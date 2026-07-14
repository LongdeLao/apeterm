use crate::app::{App, AppMode, Page, SelectionDirection};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenerPreset {
    #[default]
    Movers,
    UnusualVolume,
    Gainers,
    Losers,
}

impl ScreenerPreset {
    pub const ALL: [Self; 4] = [
        Self::Movers,
        Self::UnusualVolume,
        Self::Gainers,
        Self::Losers,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Movers => "Movers",
            Self::UnusualVolume => "Unusual volume",
            Self::Gainers => "Gainers",
            Self::Losers => "Losers",
        }
    }
}

#[derive(Debug, Default)]
pub struct ScreenerFeature {
    pub preset: ScreenerPreset,
    pub selection: usize,
}

impl App {
    pub fn open_screener(&mut self) {
        self.return_page = (self.page != Page::Screener).then_some(self.page);
        self.page = Page::Screener;
        self.mode = AppMode::Normal;
    }
    pub fn cycle_screener_preset(&mut self, direction: SelectionDirection) {
        let index = ScreenerPreset::ALL
            .iter()
            .position(|value| *value == self.screener.preset)
            .unwrap_or(0);
        self.screener.preset = match direction {
            SelectionDirection::Previous => {
                ScreenerPreset::ALL
                    [(index + ScreenerPreset::ALL.len() - 1) % ScreenerPreset::ALL.len()]
            }
            SelectionDirection::Next => {
                ScreenerPreset::ALL[(index + 1) % ScreenerPreset::ALL.len()]
            }
        };
        self.screener.selection = 0;
    }
    pub fn screener_rows(&self) -> Vec<&crate::features::watchlist::quotes::Quote> {
        let mut rows = self
            .watchlist
            .stock_quotes
            .iter()
            .filter(|quote| match self.screener.preset {
                ScreenerPreset::Movers => quote.change_percent.abs() >= 1.0,
                ScreenerPreset::UnusualVolume => {
                    quote.relative_volume.is_some_and(|value| value >= 1.5)
                }
                ScreenerPreset::Gainers => quote.change_percent > 0.0,
                ScreenerPreset::Losers => quote.change_percent < 0.0,
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            right
                .change_percent
                .abs()
                .total_cmp(&left.change_percent.abs())
        });
        rows
    }
    pub fn move_screener_selection(&mut self, direction: SelectionDirection) {
        let count = self.screener_rows().len();
        if count == 0 {
            return;
        }
        self.screener.selection = match direction {
            SelectionDirection::Previous => (self.screener.selection + count - 1) % count,
            SelectionDirection::Next => (self.screener.selection + 1) % count,
        };
    }
    pub fn open_screener_selection(&mut self) {
        let symbol = self
            .screener_rows()
            .get(self.screener.selection)
            .map(|quote| quote.symbol.clone());
        if let Some(symbol) = symbol {
            let _ = self.agent_open_symbol(&symbol);
        }
    }
}
