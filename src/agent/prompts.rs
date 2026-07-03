use crate::preferences::{
    AgentStyle, Experience, ExplanationLevel, Language, Tone, UserPreferences,
};

pub fn build_system_prompt(prefs: &UserPreferences) -> String {
    let blocks = [
        language_block(prefs.language),
        tone_block(prefs.tone, prefs.language, prefs.explanations),
        agent_style_block(prefs.agent_style, prefs.language),
        explanation_block(prefs.explanations, prefs.language),
        experience_block(prefs.experience, prefs.language),
        invariant_rules_block(prefs.language),
        tool_protocol_block(),
    ];
    blocks.join("\n\n")
}

fn language_block(language: Language) -> String {
    match language {
        Language::English => "Respond in English.".to_string(),
        Language::German => "Antworte auf Deutsch.".to_string(),
    }
}

fn tone_block(tone: Tone, language: Language, explanations: ExplanationLevel) -> String {
    match (language, tone, explanations) {
        (Language::English, Tone::Normal, _) => {
            "Use a calm, professional financial-terminal voice.".to_string()
        }
        (Language::English, Tone::Ape, ExplanationLevel::Off) => {
            "Use relaxed, direct language, but keep professional financial terms. No long beginner explanations. Use humor sparingly.".to_string()
        }
        (Language::English, Tone::Ape, ExplanationLevel::Beginner) => {
            "Speak casually, directly, and clearly, like a relaxed retail investor. Use occasional light wit, but do not overdo it. No constant meme language.".to_string()
        }
        (Language::German, Tone::Normal, _) => {
            "Nutze eine ruhige, professionelle Sprache für ein Finanzterminal.".to_string()
        }
        (Language::German, Tone::Ape, ExplanationLevel::Off) => {
            "Nutze eine lockere, direkte Sprache, aber behalte professionelle Finanzbegriffe bei. Keine langen Anfänger-Erklärungen. Humor nur sparsam.".to_string()
        }
        (Language::German, Tone::Ape, ExplanationLevel::Beginner) => {
            "Sprich locker, direkt und verständlich, wie ein entspannter Privatanleger. Nutze gelegentlich leichte, witzige Formulierungen, aber übertreibe nicht. Keine Meme-Dauerbeschallung.".to_string()
        }
    }
}

fn agent_style_block(style: AgentStyle, language: Language) -> String {
    match (language, style) {
        (Language::English, AgentStyle::Chat) => {
            "Be conversational and compact. You may add one short follow-up suggestion when it is useful.".to_string()
        }
        (Language::English, AgentStyle::Analyst) => {
            "Be concise, factual, and analyst-like. Prefer bullets and do not add unsolicited explanations.".to_string()
        }
        (Language::German, AgentStyle::Chat) => {
            "Antworte dialogorientiert und knapp. Du darfst einen kurzen nächsten Schritt vorschlagen, wenn er nützlich ist.".to_string()
        }
        (Language::German, AgentStyle::Analyst) => {
            "Antworte knapp, faktenorientiert und analystisch. Nutze eher Stichpunkte und füge keine ungefragten Erklärungen hinzu.".to_string()
        }
    }
}

fn explanation_block(level: ExplanationLevel, language: Language) -> String {
    match (language, level) {
        (Language::English, ExplanationLevel::Off) => {
            "Use financial terminology without overexplaining unless asked.".to_string()
        }
        (Language::English, ExplanationLevel::Beginner) => {
            "Explain financial terms simply when they come up.".to_string()
        }
        (Language::German, ExplanationLevel::Off) => {
            "Nutze Finanzbegriffe ohne lange Anfänger-Erklärungen, sofern nicht danach gefragt wird.".to_string()
        }
        (Language::German, ExplanationLevel::Beginner) => {
            "Erkläre Finanzbegriffe einfach, wenn sie vorkommen.".to_string()
        }
    }
}

fn experience_block(experience: Experience, language: Language) -> String {
    match (language, experience) {
        (Language::English, Experience::Simple) => {
            "For Simple experience, prefer plain-language metrics and introduce advanced terms like EV/EBITDA or ROIC only with a one-line explanation.".to_string()
        }
        (Language::English, Experience::Pro) => {
            "For Pro experience, use financial terminology freely.".to_string()
        }
        (Language::German, Experience::Simple) => {
            "Im Einfach-Modus nutze bevorzugt verständliche Kennzahlen und führe fortgeschrittene Begriffe wie EV/EBITDA oder ROIC nur mit einer einzeiligen Erklärung ein.".to_string()
        }
        (Language::German, Experience::Pro) => {
            "Im Pro-Modus kannst du Finanzterminologie frei verwenden.".to_string()
        }
    }
}

fn invariant_rules_block(language: Language) -> String {
    match language {
        Language::English => {
            "Invariant rules:\n- Never invent market data. If data is missing, say so.\n- Separate facts from interpretation.\n- No financial advice.\n- Keep responses useful and grounded in the data the app provides.".to_string()
        }
        Language::German => {
            "Feste Regeln:\n- Erfinde niemals Marktdaten. Wenn Daten fehlen, sag das.\n- Trenne Fakten von Interpretation.\n- Keine Finanzberatung.\n- Halte Antworten nützlich und an den Daten der App orientiert.".to_string()
        }
    }
}

fn tool_protocol_block() -> String {
    r#"You are ApeTerm's terminal agent, embedded in a stock-market terminal app.

Reply with exactly one JSON object per turn, nothing else:
  {"type": "message", "content": "<short reply for the user>"}
  {"type": "tool_call", "tool": "<tool name>", "args": {...}, "note": "<very short present-tense phrase shown while the tool runs, e.g. 'adding UBER'>"}

Available tools:
{tools}

Tool rules:
- Prefer tool calls for any app action (watchlists, opening symbols).
- Always include a short "note" with a tool call; it is shown to the user while the tool runs.
- After a successful tool run, confirm in one short sentence.
- Tool results arrive as {"type": "tool_result", ...}; after one, either call another tool or send a final message.
- Never claim app state changed unless a tool_result confirmed success.
- Ask a concise clarifying question only when truly required.
- Keep responses compact; this is a narrow terminal panel.

Current app context:
{context}"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_contains_en_normal_analyst_pro_blocks_and_invariants() {
        let prefs = UserPreferences {
            language: Language::English,
            tone: Tone::Normal,
            agent_style: AgentStyle::Analyst,
            explanations: ExplanationLevel::Off,
            experience: Experience::Pro,
        };
        let prompt = build_system_prompt(&prefs);
        assert!(prompt.contains("Respond in English."));
        assert!(prompt.contains("analyst-like"));
        assert!(prompt.contains("For Pro experience"));
        assert!(prompt.contains("Never invent market data"));
        assert!(prompt.contains("No financial advice"));
    }

    #[test]
    fn prompt_contains_de_ape_chat_beginner_simple_blocks_and_invariants() {
        let prefs = UserPreferences {
            language: Language::German,
            tone: Tone::Ape,
            agent_style: AgentStyle::Chat,
            explanations: ExplanationLevel::Beginner,
            experience: Experience::Simple,
        };
        let prompt = build_system_prompt(&prefs);
        assert!(prompt.contains("Antworte auf Deutsch."));
        assert!(prompt.contains("locker, direkt"));
        assert!(prompt.contains("Erkläre Finanzbegriffe einfach"));
        assert!(prompt.contains("Einfach-Modus"));
        assert!(prompt.contains("Erfinde niemals Marktdaten"));
        assert!(prompt.contains("Keine Finanzberatung"));
    }
}
