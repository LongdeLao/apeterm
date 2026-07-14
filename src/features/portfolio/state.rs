use std::{fs, io, path::Path, sync::mpsc, thread};

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};

use serde::{Deserialize, Serialize};

use crate::app::{App, AppMode, Page, SelectionDirection};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioPosition {
    pub name: String,
    pub isin: String,
    #[serde(default)]
    pub symbol: Option<String>,
    pub quantity: f64,
    pub price: f64,
    pub average_cost: f64,
    pub net_value: f64,
    pub cost_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub broker: String,
    pub currency: String,
    pub cash: f64,
    pub synced_at: String,
    #[serde(default)]
    pub positions: Vec<PortfolioPosition>,
}

#[derive(Debug)]
pub struct PortfolioFeature {
    pub snapshot: Option<PortfolioSnapshot>,
    pub selection: usize,
    pub status: Option<String>,
    pub syncing: bool,
    pub(crate) receiver: Option<mpsc::Receiver<Result<PortfolioSnapshot, String>>>,
}

impl PortfolioFeature {
    pub fn load(path: &Path) -> Self {
        #[cfg(test)]
        let snapshot = {
            let _ = path;
            None
        };

        #[cfg(not(test))]
        let snapshot = std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        Self {
            snapshot,
            selection: 0,
            status: None,
            syncing: false,
            receiver: None,
        }
    }
}

impl App {
    pub fn open_portfolio(&mut self) {
        self.return_page = (self.page != Page::Portfolio).then_some(self.page);
        self.page = Page::Portfolio;
        self.mode = AppMode::Normal;
        self.show_help = false;
    }

    pub fn connect_trade_republic(&mut self) {
        if self.portfolio.syncing {
            return;
        }
        if !crate::broker::trade_republic::available() {
            self.portfolio.status = Some(
                "Trade Republic needs optional deps. Reinstall with INSTALL_BROKER_DEPS=1."
                    .to_string(),
            );
            return;
        }

        let _ = suspend_terminal_for_broker_prompt();
        println!("ApeTerm Trade Republic connect");
        println!(
            "pytr owns credentials and cookies in ~/.pytr. ApeTerm stores only a portfolio snapshot."
        );
        println!();
        let result = crate::broker::trade_republic::connect();
        println!();
        println!("Press Enter to return to ApeTerm.");
        let mut line = String::new();
        let _ = io::stdin().read_line(&mut line);
        let _ = restore_terminal_after_broker_prompt();

        match result {
            Ok(()) => {
                self.config.broker.trade_republic_enabled = true;
                let _ = self.config.save();
                self.portfolio.status =
                    Some("Trade Republic connected. Syncing portfolio...".to_string());
                self.notify("Trade Republic connected");
                self.refresh_portfolio();
            }
            Err(error) => {
                self.portfolio.status = Some(format!("Trade Republic connect failed: {error}"));
            }
        }
    }

    pub fn refresh_portfolio(&mut self) {
        if self.portfolio.syncing {
            return;
        }
        if !self.config.broker.trade_republic_enabled {
            self.portfolio.status =
                Some("Trade Republic is optional. Press c to connect from this page.".to_string());
            return;
        }
        let path = self.config.broker.portfolio_cache_path.clone();
        let (sender, receiver) = mpsc::channel();
        self.portfolio.receiver = Some(receiver);
        self.portfolio.syncing = true;
        self.portfolio.status = Some("Syncing Trade Republic portfolio…".to_string());
        thread::spawn(move || {
            let result = crate::broker::trade_republic::sync(&path).map_err(|e| e.to_string());
            let _ = sender.send(result);
        });
    }

    pub fn poll_portfolio(&mut self) {
        let result = self
            .portfolio
            .receiver
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        let Some(result) = result else { return };
        self.portfolio.receiver = None;
        self.portfolio.syncing = false;
        match result {
            Ok(snapshot) => {
                let count = snapshot.positions.len();
                self.portfolio.snapshot = Some(snapshot);
                self.portfolio.status = Some(format!("Synced {count} positions"));
                self.notify(format!("Portfolio synced: {count} positions"));
            }
            Err(error) => self.portfolio.status = Some(error),
        }
    }

    pub fn disconnect_trade_republic(&mut self) {
        self.config.broker.trade_republic_enabled = false;
        let _ = self.config.save();
        self.portfolio.snapshot = None;
        self.portfolio.selection = 0;
        self.portfolio.syncing = false;
        self.portfolio.receiver = None;
        let _ = fs::remove_file(&self.config.broker.portfolio_cache_path);
        self.portfolio.status = Some(
            "Trade Republic disconnected. pytr credentials in ~/.pytr were left untouched."
                .to_string(),
        );
        self.notify("Trade Republic disconnected");
    }

    pub fn move_portfolio_selection(&mut self, direction: SelectionDirection) {
        let count = self
            .portfolio
            .snapshot
            .as_ref()
            .map_or(0, |snapshot| snapshot.positions.len());
        if count == 0 {
            return;
        }
        self.portfolio.selection = match direction {
            SelectionDirection::Previous => (self.portfolio.selection + count - 1) % count,
            SelectionDirection::Next => (self.portfolio.selection + 1) % count,
        };
    }

    pub fn portfolio_summary(&self) -> String {
        let Some(snapshot) = &self.portfolio.snapshot else {
            return "No portfolio connected".to_string();
        };
        let value: f64 = snapshot
            .positions
            .iter()
            .map(|position| position.net_value)
            .sum();
        let cost: f64 = snapshot
            .positions
            .iter()
            .map(|position| position.cost_value)
            .sum();
        format!(
            "{} positions · {} {:.2} invested · P/L {:+.2} · cash {:.2}",
            snapshot.positions.len(),
            snapshot.currency,
            value,
            value - cost,
            snapshot.cash
        )
    }
}

fn suspend_terminal_for_broker_prompt() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        SetCursorStyle::DefaultUserShape,
        Show,
        MoveTo(0, 0),
        Clear(ClearType::All),
        LeaveAlternateScreen
    )
}

fn restore_terminal_after_broker_prompt() -> io::Result<()> {
    enable_raw_mode()?;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        Clear(ClearType::All),
        MoveTo(0, 0),
        SetCursorStyle::SteadyBar,
        Hide
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_schema_deserializes_pytr_bridge_output() {
        let snapshot: PortfolioSnapshot = serde_json::from_str(
            r#"{"broker":"trade_republic","currency":"EUR","cash":12.5,"synced_at":"2026-01-01T00:00:00Z","positions":[{"name":"Example","isin":"DE0000000001","symbol":null,"quantity":2.0,"price":10.0,"average_cost":8.0,"net_value":20.0,"cost_value":16.0}]}"#,
        )
        .unwrap();
        assert_eq!(snapshot.positions.len(), 1);
        assert_eq!(snapshot.positions[0].net_value, 20.0);
    }
}
