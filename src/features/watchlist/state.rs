use crate::app::*;
use crate::config::NamedWatchlist;
use crate::features::search::engine as search;

impl App {
    pub fn stock_watchlist(&self) -> &[String] {
        &self.active_watchlist().stock_symbols
    }
    pub fn crypto_watchlist(&self) -> &[String] {
        &self.active_watchlist().crypto_symbols
    }
    pub fn watchlists(&self) -> &[NamedWatchlist] {
        &self.config.watchlist.lists
    }
    pub fn active_watchlist_index(&self) -> usize {
        self.config.watchlist.active
    }
    pub fn cycle_active_watchlist(&mut self, direction: SelectionDirection) {
        let count = self.config.watchlist.lists.len();
        if count <= 1 {
            return;
        }

        self.config.watchlist.active = match direction {
            SelectionDirection::Previous => {
                if self.config.watchlist.active == 0 {
                    count - 1
                } else {
                    self.config.watchlist.active - 1
                }
            }
            SelectionDirection::Next => (self.config.watchlist.active + 1) % count,
        };
        self.retain_configured_quotes();
        self.request_market_refresh();
        let _ = self.config.save();
    }
    pub fn delete_active_watchlist(&mut self) {
        if self.config.watchlist.lists.len() <= 1 {
            return;
        }

        let active = self.config.watchlist.active;
        self.config.watchlist.lists.remove(active);
        if self.config.watchlist.active >= self.config.watchlist.lists.len() {
            self.config.watchlist.active = self.config.watchlist.lists.len().saturating_sub(1);
        }
        self.retain_configured_quotes();
        self.clamp_watchlist_selection();
        self.request_market_refresh();
        let _ = self.config.save();
    }
    pub fn watchlist_display_name(&self, symbol: &str) -> Option<&str> {
        self.active_watchlist()
            .display_names
            .get(symbol)
            .map(String::as_str)
            .filter(|name| !name.trim().is_empty())
    }
    pub fn is_editing_watchlist(&self) -> bool {
        self.watchlist_editor.is_some()
    }
    pub fn open_watchlist_editor(&mut self) {
        self.clear_text_input_mode();
        self.watchlist_editor = Some(WatchlistEditor {
            selection: 0,
            mode: None,
        });
    }
    pub fn close_watchlist_editor(&mut self) {
        if let Some(editor) = &mut self.watchlist_editor {
            if editor.mode.is_some() {
                editor.mode = None;
                self.watchlist_suggestions.clear();
                self.watchlist_suggestion_selection = 0;
                self.clear_text_input_mode();
                return;
            }
        }

        self.watchlist_editor = None;
    }
    pub fn watchlist_rows(&self) -> Vec<WatchlistEditRow> {
        let mut rows = Vec::new();
        rows.push(WatchlistEditRow::AddStock);
        rows.extend((0..self.stock_watchlist().len()).map(WatchlistEditRow::Stock));
        rows.push(WatchlistEditRow::AddCrypto);
        rows.extend((0..self.crypto_watchlist().len()).map(WatchlistEditRow::Crypto));
        rows
    }
    pub fn selected_watchlist_row(&self) -> Option<WatchlistEditRow> {
        let editor = self.watchlist_editor.as_ref()?;
        self.watchlist_rows().get(editor.selection).copied()
    }
    pub fn move_watchlist_selection(&mut self, direction: SelectionDirection) {
        if self
            .watchlist_editor
            .as_ref()
            .is_some_and(|editor| editor.mode.is_some())
        {
            return;
        }

        let count = self.watchlist_rows().len();
        let Some(editor) = &mut self.watchlist_editor else {
            return;
        };

        editor.selection = match direction {
            SelectionDirection::Previous => {
                if editor.selection == 0 {
                    count - 1
                } else {
                    editor.selection - 1
                }
            }
            SelectionDirection::Next => (editor.selection + 1) % count,
        };
    }
    pub fn activate_watchlist_editor(&mut self) {
        if self
            .watchlist_editor
            .as_ref()
            .is_some_and(|editor| editor.mode.is_some())
        {
            self.save_watchlist_input();
            return;
        }

        match self.selected_watchlist_row() {
            Some(WatchlistEditRow::AddStock) => self.begin_watchlist_add(WatchlistKind::Stock),
            Some(WatchlistEditRow::AddCrypto) => self.begin_watchlist_add(WatchlistKind::Crypto),
            Some(WatchlistEditRow::Stock(index)) => {
                self.begin_watchlist_alias_edit(WatchlistKind::Stock, index)
            }
            Some(WatchlistEditRow::Crypto(index)) => {
                self.begin_watchlist_alias_edit(WatchlistKind::Crypto, index)
            }
            None => {}
        }
    }
    pub fn delete_selected_watchlist_symbol(&mut self) {
        match self.selected_watchlist_row() {
            Some(WatchlistEditRow::Stock(index)) => {
                if index < self.stock_watchlist().len() {
                    self.active_watchlist_mut().stock_symbols.remove(index);
                }
            }
            Some(WatchlistEditRow::Crypto(index)) => {
                if index < self.crypto_watchlist().len() {
                    self.active_watchlist_mut().crypto_symbols.remove(index);
                }
            }
            _ => return,
        }

        self.cleanup_watchlist_aliases();
        self.clamp_watchlist_selection();
        self.retain_configured_quotes();
        self.request_market_refresh();
        let _ = self.config.save();
    }
    pub fn begin_watchlist_add(&mut self, kind: WatchlistKind) {
        if let Some(editor) = &mut self.watchlist_editor {
            editor.mode = Some(WatchlistEditMode::Add {
                kind,
                input: String::new(),
            });
        }
        self.begin_text_input(InputTarget::Watchlist);
        self.refresh_watchlist_suggestions();
    }
    pub fn begin_watchlist_create(&mut self) {
        if let Some(editor) = &mut self.watchlist_editor {
            editor.mode = Some(WatchlistEditMode::CreateWatchlist {
                input: String::new(),
            });
        }
        self.begin_text_input(InputTarget::Watchlist);
        self.watchlist_suggestions.clear();
        self.watchlist_suggestion_selection = 0;
    }
    pub(crate) fn begin_watchlist_alias_edit(&mut self, kind: WatchlistKind, index: usize) {
        let symbol = match kind {
            WatchlistKind::Stock => self.stock_watchlist().get(index),
            WatchlistKind::Crypto => self.crypto_watchlist().get(index),
        }
        .cloned()
        .unwrap_or_default();
        let input = self
            .watchlist_display_name(&symbol)
            .unwrap_or(symbol.as_str())
            .to_string();

        if let Some(editor) = &mut self.watchlist_editor {
            editor.mode = Some(WatchlistEditMode::EditAlias { symbol, input });
        }
        self.begin_text_input(InputTarget::Watchlist);
        self.watchlist_suggestions.clear();
        self.watchlist_suggestion_selection = 0;
    }
    pub fn begin_selected_watchlist_ticker_change(&mut self) {
        match self.selected_watchlist_row() {
            Some(WatchlistEditRow::Stock(index)) => {
                self.begin_watchlist_ticker_change(WatchlistKind::Stock, index)
            }
            Some(WatchlistEditRow::Crypto(index)) => {
                self.begin_watchlist_ticker_change(WatchlistKind::Crypto, index)
            }
            _ => {}
        }
    }
    pub(crate) fn begin_watchlist_ticker_change(&mut self, kind: WatchlistKind, index: usize) {
        let input = match kind {
            WatchlistKind::Stock => self.stock_watchlist().get(index),
            WatchlistKind::Crypto => self.crypto_watchlist().get(index),
        }
        .cloned()
        .unwrap_or_default();

        if let Some(editor) = &mut self.watchlist_editor {
            editor.mode = Some(WatchlistEditMode::ChangeTicker { kind, index, input });
        }
        self.begin_text_input(InputTarget::Watchlist);
        self.refresh_watchlist_suggestions();
    }
    pub fn move_watchlist_suggestion(&mut self, direction: SelectionDirection) {
        if self.watchlist_suggestions.is_empty() {
            self.watchlist_suggestion_selection = 0;
            return;
        }

        self.watchlist_suggestion_selection = match direction {
            SelectionDirection::Previous => self.watchlist_suggestion_selection.saturating_sub(1),
            SelectionDirection::Next => (self.watchlist_suggestion_selection + 1)
                .min(self.watchlist_suggestions.len().saturating_sub(1)),
        };
    }
    pub(crate) fn selected_watchlist_input_symbol(&self, input: &str) -> Option<String> {
        self.watchlist_suggestions
            .get(self.watchlist_suggestion_selection)
            .map(|suggestion| suggestion.symbol.clone())
            .or_else(|| normalize_symbol(input))
    }
    pub(crate) fn save_watchlist_input(&mut self) {
        let Some(mode) = self
            .watchlist_editor
            .as_mut()
            .and_then(|editor| editor.mode.take())
        else {
            return;
        };

        match mode {
            WatchlistEditMode::Add { kind, input } => {
                if let Some(symbol) = self.selected_watchlist_input_symbol(&input) {
                    let list = self.watchlist_mut(kind);
                    if !list.contains(&symbol) {
                        list.push(symbol);
                    }
                }
            }
            WatchlistEditMode::EditAlias { symbol, input } => {
                let alias = input.trim();
                if alias.is_empty() || alias.eq_ignore_ascii_case(&symbol) {
                    self.active_watchlist_mut().display_names.remove(&symbol);
                } else {
                    self.active_watchlist_mut()
                        .display_names
                        .insert(symbol, alias.to_string());
                }
            }
            WatchlistEditMode::ChangeTicker { kind, index, input } => {
                if let Some(symbol) = self.selected_watchlist_input_symbol(&input) {
                    let alias_migration = {
                        let list = self.watchlist_mut(kind);
                        if index < list.len() {
                            let old_symbol = std::mem::replace(&mut list[index], symbol);
                            let new_symbol = list[index].clone();
                            Some((old_symbol, new_symbol))
                        } else {
                            None
                        }
                    };
                    if let Some((old_symbol, new_symbol)) = alias_migration {
                        self.migrate_watchlist_alias(&old_symbol, &new_symbol);
                    }
                }
            }
            WatchlistEditMode::CreateWatchlist { input } => {
                let name = input.trim();
                if !name.is_empty() {
                    self.config.watchlist.lists.push(NamedWatchlist {
                        name: name.to_string(),
                        crypto_symbols: Vec::new(),
                        stock_symbols: Vec::new(),
                        display_names: std::collections::HashMap::new(),
                    });
                    self.config.watchlist.active = self.config.watchlist.lists.len() - 1;
                }
            }
        }

        self.active_watchlist_mut().stock_symbols.sort();
        self.active_watchlist_mut().stock_symbols.dedup();
        self.active_watchlist_mut().crypto_symbols.sort();
        self.active_watchlist_mut().crypto_symbols.dedup();
        self.cleanup_watchlist_aliases();
        self.retain_configured_quotes();
        self.clamp_watchlist_selection();
        self.watchlist_suggestions.clear();
        self.watchlist_suggestion_selection = 0;
        self.clear_text_input_mode();
        self.request_market_refresh();
        let _ = self.config.save();
    }
    pub fn agent_create_watchlist(&mut self, name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("watchlist name must not be empty".to_string());
        }
        if self
            .config
            .watchlist
            .lists
            .iter()
            .any(|list| list.name.eq_ignore_ascii_case(name))
        {
            return Err(format!("watchlist `{name}` already exists"));
        }

        self.config.watchlist.lists.push(NamedWatchlist {
            name: name.to_string(),
            crypto_symbols: Vec::new(),
            stock_symbols: Vec::new(),
            display_names: std::collections::HashMap::new(),
        });
        self.config.watchlist.active = self.config.watchlist.lists.len() - 1;
        self.retain_configured_quotes();
        self.clamp_watchlist_selection();
        self.request_market_refresh();
        let _ = self.config.save();
        Ok(format!("created watchlist `{name}` and made it active"))
    }
    pub fn agent_add_symbol_to_watchlist(&mut self, symbol: &str) -> Result<String, String> {
        let Some(symbol) = normalize_symbol(symbol) else {
            return Err("symbol must not be empty".to_string());
        };

        let kind = if symbol.contains('-') {
            WatchlistKind::Crypto
        } else {
            WatchlistKind::Stock
        };
        let list = self.watchlist_mut(kind);
        if list.contains(&symbol) {
            return Err(format!("{symbol} is already on the active watchlist"));
        }
        list.push(symbol.clone());
        list.sort();

        self.retain_configured_quotes();
        self.request_market_refresh();
        let _ = self.config.save();
        Ok(format!("added {symbol} to the active watchlist"))
    }
    pub fn agent_remove_symbol_from_watchlist(&mut self, symbol: &str) -> Result<String, String> {
        let Some(symbol) = normalize_symbol(symbol) else {
            return Err("symbol must not be empty".to_string());
        };

        let mut removed = false;
        for kind in [WatchlistKind::Stock, WatchlistKind::Crypto] {
            let list = self.watchlist_mut(kind);
            let before = list.len();
            list.retain(|existing| !existing.eq_ignore_ascii_case(&symbol));
            removed |= list.len() != before;
        }
        if !removed {
            return Err(format!("{symbol} is not on the active watchlist"));
        }

        self.cleanup_watchlist_aliases();
        self.clamp_watchlist_selection();
        self.retain_configured_quotes();
        self.request_market_refresh();
        let _ = self.config.save();
        Ok(format!("removed {symbol} from the active watchlist"))
    }
    pub(crate) fn watchlist_mut(&mut self, kind: WatchlistKind) -> &mut Vec<String> {
        match kind {
            WatchlistKind::Stock => &mut self.active_watchlist_mut().stock_symbols,
            WatchlistKind::Crypto => &mut self.active_watchlist_mut().crypto_symbols,
        }
    }
    pub(crate) fn active_watchlist(&self) -> &NamedWatchlist {
        &self.config.watchlist.lists[self.config.watchlist.active]
    }
    pub(crate) fn active_watchlist_mut(&mut self) -> &mut NamedWatchlist {
        let active = self.config.watchlist.active;
        &mut self.config.watchlist.lists[active]
    }
    pub(crate) fn retain_configured_quotes(&mut self) {
        let stock_symbols = self.stock_watchlist().to_vec();
        let crypto_symbols = self.crypto_watchlist().to_vec();
        self.stock_quotes
            .retain(|quote| stock_symbols.contains(&quote.symbol));
        self.crypto_quotes
            .retain(|quote| crypto_symbols.contains(&quote.symbol));
    }
    pub(crate) fn refresh_watchlist_suggestions(&mut self) {
        let Some(mode) = self
            .watchlist_editor
            .as_ref()
            .and_then(|editor| editor.mode.as_ref())
        else {
            self.watchlist_suggestions.clear();
            self.watchlist_suggestion_selection = 0;
            return;
        };

        let (kind, input) = match mode {
            WatchlistEditMode::Add { kind, input }
            | WatchlistEditMode::ChangeTicker { kind, input, .. } => (*kind, input.as_str()),
            WatchlistEditMode::EditAlias { .. } | WatchlistEditMode::CreateWatchlist { .. } => {
                self.watchlist_suggestions.clear();
                self.watchlist_suggestion_selection = 0;
                return;
            }
        };

        if kind != WatchlistKind::Stock {
            self.watchlist_suggestions.clear();
            self.watchlist_suggestion_selection = 0;
            return;
        }

        match crate::db::open(&self.ticker_db_path)
            .and_then(|connection| search::search_assets(&connection, input, &["stock", "etf"], 6))
        {
            Ok(results) => {
                self.watchlist_suggestions = results;
                self.watchlist_suggestion_selection = self
                    .watchlist_suggestion_selection
                    .min(self.watchlist_suggestions.len().saturating_sub(1));
            }
            Err(_) => {
                self.watchlist_suggestions.clear();
                self.watchlist_suggestion_selection = 0;
            }
        }
    }
    pub(crate) fn migrate_watchlist_alias(&mut self, old_symbol: &str, new_symbol: &str) {
        if old_symbol == new_symbol {
            return;
        }

        if let Some(alias) = self.active_watchlist_mut().display_names.remove(old_symbol) {
            self.active_watchlist_mut()
                .display_names
                .entry(new_symbol.to_string())
                .or_insert(alias);
        }
    }
    pub(crate) fn cleanup_watchlist_aliases(&mut self) {
        let stock_symbols = self.stock_watchlist().to_vec();
        let crypto_symbols = self.crypto_watchlist().to_vec();
        self.active_watchlist_mut()
            .display_names
            .retain(|symbol, _| stock_symbols.contains(symbol) || crypto_symbols.contains(symbol));
    }
    pub(crate) fn request_market_refresh(&mut self) {
        self.market_refresh_requested = true;
    }
    pub(crate) fn clamp_watchlist_selection(&mut self) {
        let count = self.watchlist_rows().len();
        if let Some(editor) = &mut self.watchlist_editor {
            editor.selection = editor.selection.min(count.saturating_sub(1));
        }
    }
}

pub(crate) fn normalize_symbol(input: &str) -> Option<String> {
    let symbol = input.trim().to_ascii_uppercase();
    if symbol.is_empty() {
        None
    } else {
        Some(symbol)
    }
}
