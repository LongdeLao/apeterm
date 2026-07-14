use crate::{
    app::App,
    db,
    features::{news::feed as news, notes::repo::NoteRow},
};

use super::{
    NOTE_LIMIT,
    format::{format_notes, normalize_agent_symbol},
};

impl App {
    pub fn agent_summarize_notes_for_symbol(&self, symbol: &str) -> Result<String, String> {
        let symbol = normalize_agent_symbol(symbol)?;
        let notes = self.agent_notes_for_symbol(&symbol);
        if notes.is_empty() {
            return Ok(format!("no local notes found for {symbol}"));
        }
        let mut lines = vec![format!("local notes for {symbol}:")];
        lines.extend(format_notes(&notes, 0));
        Ok(lines.join("\n"))
    }

    pub(super) fn agent_notes_for_symbol(&self, symbol: &str) -> Vec<NoteRow> {
        let Ok(connection) = db::open(&self.ticker_db_path) else {
            return Vec::new();
        };
        let mut notes = crate::features::notes::repo::list_all(&connection).unwrap_or_default();
        notes.retain(|note| {
            note.tickers
                .iter()
                .any(|ticker| ticker.eq_ignore_ascii_case(symbol))
                || news::contains_symbol(&note.body, symbol)
        });
        notes.truncate(NOTE_LIMIT);
        notes
    }

    pub(super) fn agent_note_counts_for_symbols(&self, symbols: &[String]) -> Vec<(String, usize)> {
        symbols
            .iter()
            .map(|symbol| (symbol.clone(), self.agent_notes_for_symbol(symbol).len()))
            .collect()
    }
}
