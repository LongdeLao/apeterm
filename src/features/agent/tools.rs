//! Executes typed tool calls against the app. This is the only place the
//! agent touches app state, and it only does so through App methods.

use crate::{
    app::App,
    features::agent::{
        context,
        tool_call::{ToolCall, ToolResult},
    },
};

/// Tool schema given to the model in the system prompt.
pub fn catalog() -> &'static str {
    r#"- read_current_context: args {} — re-read the current page, selection, and watchlists
- list_watchlists: args {} — list all watchlists with their symbols
- create_watchlist: args {"name": string} — create a new watchlist and make it active
- add_symbol_to_watchlist: args {"symbol": string} — add a ticker to the active watchlist
- remove_symbol_from_watchlist: args {"symbol": string} — remove a ticker from the active watchlist
- open_symbol: args {"symbol": string} — open the details page for a ticker"#
}

pub fn execute(app: &mut App, call: ToolCall) -> ToolResult {
    let tool = call.name();
    let outcome = match call {
        ToolCall::ReadCurrentContext => Ok(context::build_context(app)),
        ToolCall::ListWatchlists => Ok(list_watchlists(app)),
        ToolCall::CreateWatchlist { name } => app.agent_create_watchlist(&name),
        ToolCall::AddSymbolToWatchlist { symbol } => app.agent_add_symbol_to_watchlist(&symbol),
        ToolCall::RemoveSymbolFromWatchlist { symbol } => {
            app.agent_remove_symbol_from_watchlist(&symbol)
        }
        ToolCall::OpenSymbol { symbol } => app.agent_open_symbol(&symbol),
    };

    match outcome {
        Ok(message) => ToolResult::success(tool, message),
        Err(message) => ToolResult::failure(tool, message),
    }
}

fn list_watchlists(app: &App) -> String {
    let active_index = app.active_watchlist_index();
    app.watchlists()
        .iter()
        .enumerate()
        .map(|(index, list)| {
            let marker = if index == active_index {
                " (active)"
            } else {
                ""
            };
            let mut symbols = list.stock_symbols.clone();
            symbols.extend(list.crypto_symbols.iter().cloned());
            let symbols = if symbols.is_empty() {
                "(empty)".to_string()
            } else {
                symbols.join(", ")
            };
            format!("{}{marker}: {symbols}", list.name)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
