//! Typed tool calls. The model requests one of these by emitting a JSON
//! object; the app executes it and answers with a [`ToolResult`]. The model
//! never mutates app state directly.

use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCall {
    ReadCurrentContext,
    ListWatchlists,
    CreateWatchlist { name: String },
    AddSymbolToWatchlist { symbol: String },
    RemoveSymbolFromWatchlist { symbol: String },
    OpenSymbol { symbol: String },
}

impl ToolCall {
    pub fn name(&self) -> &'static str {
        match self {
            ToolCall::ReadCurrentContext => "read_current_context",
            ToolCall::ListWatchlists => "list_watchlists",
            ToolCall::CreateWatchlist { .. } => "create_watchlist",
            ToolCall::AddSymbolToWatchlist { .. } => "add_symbol_to_watchlist",
            ToolCall::RemoveSymbolFromWatchlist { .. } => "remove_symbol_from_watchlist",
            ToolCall::OpenSymbol { .. } => "open_symbol",
        }
    }

}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool: String,
    pub success: bool,
    pub message: String,
}

impl ToolResult {
    pub fn success(tool: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            success: true,
            message: message.into(),
        }
    }

    pub fn failure(tool: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            success: false,
            message: message.into(),
        }
    }

    /// Wire form fed back to the model as the tool turn.
    pub fn to_json(&self) -> String {
        json!({
            "type": "tool_result",
            "tool": self.tool,
            "success": self.success,
            "message": self.message,
        })
        .to_string()
    }
}

/// What the assistant's raw reply decodes to.
#[derive(Debug, Clone)]
pub enum AssistantAction {
    Message(String),
    ToolCall {
        call: ToolCall,
        /// Optional short phrase the model wants shown while the tool runs,
        /// e.g. "adding UBER".
        note: Option<String>,
    },
    /// Well-formed JSON that requested an unknown tool or omitted required
    /// args; reported back to the model so it can correct itself.
    Invalid { reason: String },
}

#[derive(Debug, Deserialize)]
struct WireAction {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    args: serde_json::Value,
}

pub fn parse_assistant_action(raw: &str) -> AssistantAction {
    let text = strip_code_fences(raw.trim());

    // Models routinely wrap the JSON in prose or append stray braces, so
    // decode the first complete JSON object and ignore whatever surrounds it.
    let Some(wire) = extract_wire_action(text) else {
        // No usable JSON: fall back to showing the raw reply as plain text.
        return AssistantAction::Message(text.to_string());
    };

    // Models frequently drop the "type" field; infer it from what is present.
    let kind = wire.kind.clone().unwrap_or_else(|| {
        if wire.tool.is_some() {
            "tool_call".to_string()
        } else {
            "message".to_string()
        }
    });

    match kind.as_str() {
        "message" => {
            let content = wire
                .content
                .or(wire.message)
                .map(|content| content.trim().to_string())
                .filter(|content| !content.is_empty());
            AssistantAction::Message(content.unwrap_or_else(|| text.to_string()))
        }
        "tool_call" => {
            let tool = wire.tool.unwrap_or_default();
            match decode_tool(&tool, &wire.args) {
                Ok(call) => AssistantAction::ToolCall {
                    call,
                    note: wire
                        .note
                        .map(|note| note.trim().to_string())
                        .filter(|note| !note.is_empty()),
                },
                Err(reason) => AssistantAction::Invalid { reason },
            }
        }
        other => AssistantAction::Invalid {
            reason: format!("unknown action type `{other}`"),
        },
    }
}

/// Parses the first complete JSON object in `text`, tolerating leading
/// prose and trailing garbage (extra braces, commentary, a second object).
fn extract_wire_action(text: &str) -> Option<WireAction> {
    let start = text.find('{')?;
    serde_json::Deserializer::from_str(&text[start..])
        .into_iter::<WireAction>()
        .next()?
        .ok()
}

fn decode_tool(tool: &str, args: &serde_json::Value) -> Result<ToolCall, String> {
    let string_arg = |key: &str| -> Result<String, String> {
        args.get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .ok_or_else(|| format!("tool `{tool}` requires a string arg `{key}`"))
    };

    match tool {
        "read_current_context" => Ok(ToolCall::ReadCurrentContext),
        "list_watchlists" => Ok(ToolCall::ListWatchlists),
        "create_watchlist" => Ok(ToolCall::CreateWatchlist {
            name: string_arg("name")?,
        }),
        "add_symbol_to_watchlist" => Ok(ToolCall::AddSymbolToWatchlist {
            symbol: string_arg("symbol")?,
        }),
        "remove_symbol_from_watchlist" => Ok(ToolCall::RemoveSymbolFromWatchlist {
            symbol: string_arg("symbol")?,
        }),
        "open_symbol" => Ok(ToolCall::OpenSymbol {
            symbol: string_arg("symbol")?,
        }),
        other => Err(format!("unknown tool `{other}`")),
    }
}

fn strip_code_fences(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("```") else {
        return text;
    };
    let rest = rest
        .split_once('\n')
        .map(|(_language, body)| body)
        .unwrap_or(rest);
    rest.trim().trim_end_matches("```").trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_text_as_message() {
        match parse_assistant_action("hello there") {
            AssistantAction::Message(text) => assert_eq!(text, "hello there"),
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn parses_tool_call_with_args() {
        let raw = r#"{"type":"tool_call","tool":"add_symbol_to_watchlist","args":{"symbol":"TSLA"},"note":"adding TSLA"}"#;
        match parse_assistant_action(raw) {
            AssistantAction::ToolCall {
                call: ToolCall::AddSymbolToWatchlist { symbol },
                note,
            } => {
                assert_eq!(symbol, "TSLA");
                assert_eq!(note.as_deref(), Some("adding TSLA"));
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn infers_message_without_type_field() {
        let raw = r#"{"message": "Reminder: you added UBER."}"#;
        match parse_assistant_action(raw) {
            AssistantAction::Message(text) => assert_eq!(text, "Reminder: you added UBER."),
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn infers_tool_call_without_type_field() {
        let raw = r#"{"tool": "list_watchlists", "args": {}}"#;
        assert!(matches!(
            parse_assistant_action(raw),
            AssistantAction::ToolCall {
                call: ToolCall::ListWatchlists,
                ..
            }
        ));
    }

    #[test]
    fn parses_fenced_json_message() {
        let raw = "```json\n{\"type\":\"message\",\"content\":\"done\"}\n```";
        match parse_assistant_action(raw) {
            AssistantAction::Message(text) => assert_eq!(text, "done"),
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn tolerates_trailing_garbage_after_json() {
        let raw = r#"{"type": "tool_call", "tool": "add_symbol_to_watchlist", "args": {"symbol": "UBER"}}}"#;
        match parse_assistant_action(raw) {
            AssistantAction::ToolCall {
                call: ToolCall::AddSymbolToWatchlist { symbol },
                ..
            } => {
                assert_eq!(symbol, "UBER");
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn tolerates_prose_around_json() {
        let raw = "Sure, adding it now:\n{\"type\":\"tool_call\",\"tool\":\"open_symbol\",\"args\":{\"symbol\":\"TSLA\"}}\nDone!";
        match parse_assistant_action(raw) {
            AssistantAction::ToolCall {
                call: ToolCall::OpenSymbol { symbol },
                ..
            } => {
                assert_eq!(symbol, "TSLA");
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn unknown_tool_is_invalid() {
        let raw = r#"{"type":"tool_call","tool":"summarize_news","args":{}}"#;
        assert!(matches!(
            parse_assistant_action(raw),
            AssistantAction::Invalid { .. }
        ));
    }

    #[test]
    fn broken_json_falls_back_to_message() {
        let raw = r#"{"type":"tool_call","#;
        assert!(matches!(
            parse_assistant_action(raw),
            AssistantAction::Message(_)
        ));
    }
}
