use crate::app::{App, AppMode, Page, SelectionDirection};
use chrono::{Duration, Utc};

#[derive(Debug, Default)]
pub struct CalendarFeature {
    pub selection: usize,
    pub watchlist_only: bool,
}

impl App {
    pub fn open_calendar(&mut self) {
        self.return_page = (self.page != Page::Calendar).then_some(self.page);
        self.page = Page::Calendar;
        self.mode = AppMode::Normal;
    }
    pub fn toggle_calendar_scope(&mut self) {
        self.calendar.watchlist_only = !self.calendar.watchlist_only;
        self.calendar.selection = 0;
    }
    pub fn calendar_rows(&self) -> Vec<(String, String, String)> {
        let watchlist = self.stock_watchlist();
        let mut rows = self
            .news
            .items
            .iter()
            .filter(|item| {
                !self.calendar.watchlist_only
                    || item.symbols.iter().any(|symbol| watchlist.contains(symbol))
            })
            .filter_map(|item| {
                item.published_at.map(|date| {
                    (
                        date.format("%Y-%m-%d %H:%M").to_string(),
                        item.symbols
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "MARKET".into()),
                        item.title.clone(),
                    )
                })
            })
            // News is already maintained newest-first. Bound cloning before
            // the small final sort so Calendar stays cheap during redraws.
            .take(100)
            .collect::<Vec<_>>();
        if let (Some(details), Some(days)) = (
            self.search.selected_details.as_ref(),
            self.search
                .selected_live_details
                .as_ref()
                .and_then(|live| live.next_earnings_days),
        ) {
            rows.push((
                (Utc::now() + Duration::days(days))
                    .format("%Y-%m-%d")
                    .to_string(),
                details.symbol.clone(),
                "Estimated earnings date".to_string(),
            ));
        }
        rows.sort_unstable_by(|left, right| right.0.cmp(&left.0));
        rows.truncate(100);
        rows
    }
    pub fn move_calendar_selection(&mut self, direction: SelectionDirection) {
        let count = self.calendar_rows().len();
        if count == 0 {
            return;
        }
        self.calendar.selection = match direction {
            SelectionDirection::Previous => (self.calendar.selection + count - 1) % count,
            SelectionDirection::Next => (self.calendar.selection + 1) % count,
        };
    }
}
