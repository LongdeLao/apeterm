use crate::app::{App, AppMode, Page, SelectionDirection};

#[derive(Debug, Default)]
pub struct CompareFeature {
    pub symbols: Vec<String>,
    pub selection: usize,
}

impl App {
    pub fn open_compare(&mut self) {
        if self.compare.symbols.is_empty() {
            self.compare.symbols = self.stock_watchlist().iter().take(4).cloned().collect();
        }
        self.return_page = (self.page != Page::Compare).then_some(self.page);
        self.page = Page::Compare;
        self.mode = AppMode::Normal;
    }
    pub fn set_compare_symbols(&mut self, symbols: &[String]) -> Result<String, String> {
        let mut normalized = Vec::new();
        for symbol in symbols {
            let symbol = symbol.trim().to_ascii_uppercase();
            if !symbol.is_empty() && !normalized.contains(&symbol) {
                normalized.push(symbol);
            }
        }
        if !(2..=5).contains(&normalized.len()) {
            return Err("comparison requires 2 to 5 symbols".into());
        }
        self.compare.symbols = normalized;
        self.open_compare();
        Ok("comparison workspace opened".into())
    }
    pub fn move_compare_selection(&mut self, direction: SelectionDirection) {
        let count = self.compare.symbols.len();
        if count == 0 {
            return;
        }
        self.compare.selection = match direction {
            SelectionDirection::Previous => (self.compare.selection + count - 1) % count,
            SelectionDirection::Next => (self.compare.selection + 1) % count,
        };
    }
    pub fn remove_compare_symbol(&mut self) {
        if !self.compare.symbols.is_empty() {
            self.compare.symbols.remove(self.compare.selection);
            self.compare.selection = self
                .compare
                .selection
                .min(self.compare.symbols.len().saturating_sub(1));
        }
    }
    pub fn open_compare_selection(&mut self) {
        let symbol = self.compare.symbols.get(self.compare.selection).cloned();
        if let Some(symbol) = symbol {
            let _ = self.agent_open_symbol(&symbol);
        }
    }
}
