use crate::app::*;
use crate::db;
use crate::features::news::feed as news;
use crate::features::search::engine as search;
use crate::features::search::engine::SearchResult;

/// UI state owned by the notes feature.
#[derive(Debug, Default)]
pub struct NotesFeature {
    pub tab: NotesFilterTab,
    pub selection: usize,
    pub scroll: usize,
    pub search_query: String,
    pub ticker_filter: Option<String>,
    pub insert_mode: bool,
    pub draft: Option<NotesDraft>,
    pub suggestions: Vec<SearchResult>,
    pub suggestion_selection: usize,
    pub pending_delete: Option<i64>,
}

impl App {
    pub fn notes_visible(&self) -> Vec<crate::features::notes::repo::NoteRow> {
        let Ok(connection) = db::open(&self.ticker_db_path) else {
            return Vec::new();
        };
        let all = crate::features::notes::repo::list_all(&connection).unwrap_or_default();

        let mut filtered: Vec<crate::features::notes::repo::NoteRow> =
            if let Some(symbol) = &self.notes.ticker_filter {
                all.into_iter()
                    .filter(|note| note.tickers.iter().any(|ticker| ticker == symbol))
                    .collect()
            } else {
                all.into_iter()
                    .filter(|note| self.notes_matches_tab(note))
                    .collect()
            };

        let query = self.notes.search_query.trim();
        if !query.is_empty() {
            let mut ids =
                crate::features::notes::repo::search_fts(&connection, query).unwrap_or_default();
            if ids.is_empty() {
                let lowered = query.to_ascii_lowercase();
                ids = filtered
                    .iter()
                    .filter(|note| note.body.to_ascii_lowercase().contains(&lowered))
                    .map(|note| note.id)
                    .collect();
            }
            filtered.retain(|note| ids.contains(&note.id));
        }

        filtered
    }
    pub(crate) fn notes_matches_tab(&self, note: &crate::features::notes::repo::NoteRow) -> bool {
        match self.notes.tab {
            NotesFilterTab::All => true,
            NotesFilterTab::Tickers => !note.tickers.is_empty(),
            NotesFilterTab::Journal => note.tickers.is_empty(),
            NotesFilterTab::Pinned => note.pinned,
        }
    }
    pub fn notes_selected_row(&self) -> Option<crate::features::notes::repo::NoteRow> {
        let rows = self.notes_visible();
        rows.get(self.notes.selection.min(rows.len().saturating_sub(1)))
            .cloned()
    }
    pub fn cycle_notes_tab(&mut self, direction: SelectionDirection) {
        self.notes.tab = match (self.notes.tab, direction) {
            (NotesFilterTab::All, SelectionDirection::Previous) => NotesFilterTab::Pinned,
            (NotesFilterTab::All, SelectionDirection::Next) => NotesFilterTab::Tickers,
            (NotesFilterTab::Tickers, SelectionDirection::Previous) => NotesFilterTab::All,
            (NotesFilterTab::Tickers, SelectionDirection::Next) => NotesFilterTab::Journal,
            (NotesFilterTab::Journal, SelectionDirection::Previous) => NotesFilterTab::Tickers,
            (NotesFilterTab::Journal, SelectionDirection::Next) => NotesFilterTab::Pinned,
            (NotesFilterTab::Pinned, SelectionDirection::Previous) => NotesFilterTab::Journal,
            (NotesFilterTab::Pinned, SelectionDirection::Next) => NotesFilterTab::All,
        };
        self.notes.ticker_filter = None;
        self.notes.selection = 0;
        self.notes.scroll = 0;
    }
    pub fn move_notes_selection(&mut self, direction: SelectionDirection) {
        let count = self.notes_visible().len();
        if count == 0 {
            self.notes.selection = 0;
            self.notes.scroll = 0;
            return;
        }

        self.notes.selection = match direction {
            SelectionDirection::Previous => self.notes.selection.saturating_sub(1),
            SelectionDirection::Next => (self.notes.selection + 1).min(count - 1),
        };
        self.sync_notes_scroll(6);
    }
    pub fn sync_notes_scroll(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            self.notes.scroll = self.notes.selection;
            return;
        }

        if self.notes.selection < self.notes.scroll {
            self.notes.scroll = self.notes.selection;
        } else if self.notes.selection >= self.notes.scroll + visible_rows {
            self.notes.scroll = self.notes.selection + 1 - visible_rows;
        }
    }
    /// Creates an empty note, inserts it into the list, selects it, and
    /// drops straight into insert mode — mirrors "New Note" in Apple Notes.
    pub fn create_new_note(&mut self) {
        let now = chrono::Utc::now().timestamp();
        let Ok(connection) = db::open(&self.ticker_db_path) else {
            return;
        };
        let Ok(id) = crate::features::notes::repo::insert(&connection, "", &[], &[], now) else {
            return;
        };

        self.notes.tab = NotesFilterTab::All;
        self.notes.ticker_filter = None;
        self.notes.search_query.clear();

        let rows = self.notes_visible();
        self.notes.selection = rows.iter().position(|note| note.id == id).unwrap_or(0);
        self.sync_notes_scroll(6);

        self.notes.draft = Some(NotesDraft {
            note_id: id,
            body: String::new(),
        });
        self.notes.suggestions.clear();
        self.notes.suggestion_selection = 0;
        self.notes.insert_mode = true;
        self.begin_text_input(InputTarget::Notes);
    }
    /// Enters insert mode on the currently selected note (Enter / `i`).
    /// Falls back to creating a new note if the list is empty.
    pub fn enter_note_insert_mode(&mut self) {
        let Some(note) = self.notes_selected_row() else {
            self.create_new_note();
            return;
        };
        self.notes.draft = Some(NotesDraft {
            note_id: note.id,
            body: note.body,
        });
        self.notes.suggestions.clear();
        self.notes.suggestion_selection = 0;
        self.notes.insert_mode = true;
        self.begin_text_input(InputTarget::Notes);
    }
    /// Leaves insert mode (Esc): persists the draft, recomputes
    /// tickers/tags, and drops empty notes rather than leaving clutter.
    pub fn exit_note_insert_mode(&mut self) {
        self.finalize_note_draft();
        self.notes.insert_mode = false;
        self.notes.suggestions.clear();
        self.notes.suggestion_selection = 0;
        self.clear_text_input_mode();
    }
    pub(crate) fn finalize_note_draft(&mut self) {
        let Some(draft) = self.notes.draft.take() else {
            return;
        };
        let Ok(connection) = db::open(&self.ticker_db_path) else {
            return;
        };

        if draft.body.trim().is_empty() {
            let _ = crate::features::notes::repo::delete(&connection, draft.note_id);
        } else {
            let tickers = self.extract_note_tickers(&draft.body);
            let tags = extract_note_tags(&draft.body);
            let now = chrono::Utc::now().timestamp();
            let _ = crate::features::notes::repo::update(
                &connection,
                draft.note_id,
                &draft.body,
                &tickers,
                &tags,
                now,
            );
        }

        let visible = self.notes_visible().len();
        if self.notes.selection >= visible {
            self.notes.selection = visible.saturating_sub(1);
        }
        self.sync_notes_scroll(6);
    }
    pub fn insert_note_draft_newline(&mut self) {
        if let Some(draft) = &mut self.notes.draft {
            draft.body.push('\n');
        }
        self.refresh_note_suggestions();
    }
    pub fn push_note_draft_char(&mut self, character: char) {
        if character.is_control() {
            return;
        }
        if let Some(draft) = &mut self.notes.draft {
            draft.body.push(character);
        }
        self.refresh_note_suggestions();
    }
    pub fn pop_note_draft_char(&mut self) {
        if let Some(draft) = &mut self.notes.draft {
            draft.body.pop();
        }
        self.refresh_note_suggestions();
    }
    pub(crate) fn refresh_note_suggestions(&mut self) {
        let Some(draft) = &self.notes.draft else {
            self.notes.suggestions.clear();
            self.notes.suggestion_selection = 0;
            return;
        };

        let last_token = draft.body.split_whitespace().last().unwrap_or("");
        let Some(query) = last_token
            .strip_prefix('$')
            .filter(|query| !query.is_empty())
        else {
            self.notes.suggestions.clear();
            self.notes.suggestion_selection = 0;
            return;
        };

        match db::open(&self.ticker_db_path)
            .and_then(|connection| search::search_assets(&connection, query, &["stock", "etf"], 6))
        {
            Ok(results) => {
                self.notes.suggestions = results;
                self.notes.suggestion_selection = self
                    .notes.suggestion_selection
                    .min(self.notes.suggestions.len().saturating_sub(1));
            }
            Err(_) => {
                self.notes.suggestions.clear();
                self.notes.suggestion_selection = 0;
            }
        }
    }
    pub fn move_note_suggestion(&mut self, direction: SelectionDirection) {
        if self.notes.suggestions.is_empty() {
            self.notes.suggestion_selection = 0;
            return;
        }

        self.notes.suggestion_selection = match direction {
            SelectionDirection::Previous => self.notes.suggestion_selection.saturating_sub(1),
            SelectionDirection::Next => (self.notes.suggestion_selection + 1)
                .min(self.notes.suggestions.len().saturating_sub(1)),
        };
    }
    pub fn accept_note_suggestion(&mut self) {
        let Some(suggestion) = self
            .notes.suggestions
            .get(self.notes.suggestion_selection)
            .cloned()
        else {
            return;
        };
        let Some(draft) = &mut self.notes.draft else {
            return;
        };

        match draft.body.rfind(char::is_whitespace) {
            Some(index) => draft.body.truncate(index + 1),
            None => draft.body.clear(),
        }
        draft.body.push('$');
        draft.body.push_str(&suggestion.symbol);
        draft.body.push(' ');

        self.notes.suggestions.clear();
        self.notes.suggestion_selection = 0;
    }
    pub(crate) fn extract_note_tickers(&self, body: &str) -> Vec<String> {
        let Ok(connection) = db::open(&self.ticker_db_path) else {
            return Vec::new();
        };
        let Ok(mut statement) =
            connection.prepare("SELECT symbol FROM instruments WHERE active = 1")
        else {
            return Vec::new();
        };
        let Ok(rows) = statement.query_map([], |row| row.get::<_, String>(0)) else {
            return Vec::new();
        };

        let mut matched: Vec<String> = rows
            .filter_map(std::result::Result::ok)
            .filter(|symbol| news::contains_symbol(body, symbol))
            .collect();
        matched.sort();
        matched.dedup();
        matched
    }
    pub fn toggle_selected_note_pin(&mut self) {
        let Some(note) = self.notes_selected_row() else {
            return;
        };
        if let Ok(connection) = db::open(&self.ticker_db_path) {
            let _ = crate::features::notes::repo::set_pinned(&connection, note.id, !note.pinned);
        }
    }
    pub fn begin_delete_selected_note(&mut self) {
        let Some(note) = self.notes_selected_row() else {
            return;
        };
        self.notes.pending_delete = Some(note.id);
    }
    pub fn confirm_delete_note(&mut self) {
        let Some(id) = self.notes.pending_delete.take() else {
            return;
        };
        if let Ok(connection) = db::open(&self.ticker_db_path) {
            let _ = crate::features::notes::repo::delete(&connection, id);
        }

        let visible = self.notes_visible().len();
        if self.notes.selection >= visible {
            self.notes.selection = visible.saturating_sub(1);
        }
        self.sync_notes_scroll(6);
    }
    pub fn cancel_delete_note(&mut self) {
        self.notes.pending_delete = None;
    }
    pub fn begin_notes_search(&mut self) {
        self.begin_text_input(InputTarget::NotesSearch);
    }
    pub fn notes_ticker_symbols(&self) -> std::collections::HashSet<String> {
        db::open(&self.ticker_db_path)
            .and_then(|connection| crate::features::notes::repo::all_ticker_symbols(&connection))
            .unwrap_or_default()
    }
    pub fn jump_to_notes_for_symbol(&mut self, symbol: &str) {
        self.notes.tab = NotesFilterTab::Tickers;
        self.notes.ticker_filter = Some(symbol.to_string());
        self.notes.search_query.clear();
        self.notes.selection = 0;
        self.notes.scroll = 0;
        self.set_panel_content(PanelId::Notes, WindowKind::Notes);
        self.focus_panel(PanelId::Notes);
    }
    pub fn jump_to_selected_watchlist_row_notes(&mut self) {
        let symbol = match self.selected_watchlist_row() {
            Some(WatchlistEditRow::Stock(index)) => self.stock_watchlist().get(index).cloned(),
            Some(WatchlistEditRow::Crypto(index)) => self.crypto_watchlist().get(index).cloned(),
            _ => None,
        };
        let Some(symbol) = symbol else {
            return;
        };
        self.close_watchlist_editor();
        self.jump_to_notes_for_symbol(&symbol);
    }
}

fn extract_note_tags(body: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for token in body.split_whitespace() {
        let trimmed = token
            .trim_matches(|character: char| character.is_ascii_punctuation() && character != '#');
        if trimmed.len() > 1 && trimmed.starts_with('#') && !tags.contains(&trimmed.to_string()) {
            tags.push(trimmed.to_string());
        }
    }
    tags
}
