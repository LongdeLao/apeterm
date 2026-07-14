use std::{fs, path::Path, sync::mpsc, thread};

use serde::{Deserialize, Serialize};

use crate::{
    app::{App, AppMode, InputTarget, Page, SelectionDirection},
    broker::trade_republic::LoginStartResult,
};

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
    pub login: Option<TradeRepublicLogin>,
    pub login_busy: bool,
    pub(crate) receiver: Option<mpsc::Receiver<Result<PortfolioSnapshot, String>>>,
    pub(crate) login_receiver: Option<mpsc::Receiver<Result<BrokerLoginEvent, String>>>,
}

#[derive(Debug, Clone)]
pub struct TradeRepublicLogin {
    pub step: TradeRepublicLoginStep,
    pub phone: String,
    pub pin: String,
    pub process_id: Option<String>,
    pub countdown: Option<u64>,
    pub input: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeRepublicLoginStep {
    Phone,
    Pin,
    Code,
}

#[derive(Debug)]
pub(crate) enum BrokerLoginEvent {
    Connected,
    CodeRequired {
        process_id: String,
        countdown: Option<u64>,
    },
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
            login: None,
            login_busy: false,
            receiver: None,
            login_receiver: None,
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
        self.portfolio.login = Some(TradeRepublicLogin {
            step: TradeRepublicLoginStep::Phone,
            phone: String::new(),
            pin: String::new(),
            process_id: None,
            countdown: None,
            input: String::new(),
        });
        self.portfolio.status = Some("Enter your Trade Republic phone number.".to_string());
        self.begin_text_input(InputTarget::BrokerLogin);
    }

    pub fn cancel_trade_republic_login(&mut self) {
        if self.portfolio.login_busy {
            self.portfolio.status =
                Some("Trade Republic login is running; wait for the current step.".to_string());
            return;
        }
        self.portfolio.login = None;
        self.portfolio.login_receiver = None;
        self.clear_text_input_mode();
        self.portfolio.status = Some("Trade Republic login cancelled.".to_string());
    }

    pub fn push_trade_republic_login_char(&mut self, character: char) {
        if let Some(login) = &mut self.portfolio.login {
            login.input.push(character);
        }
    }

    pub fn pop_trade_republic_login_char(&mut self) {
        if let Some(login) = &mut self.portfolio.login {
            login.input.pop();
        }
    }

    pub fn submit_trade_republic_login_input(&mut self) {
        if self.portfolio.login_busy {
            return;
        }
        let Some(login) = &mut self.portfolio.login else {
            self.clear_text_input_mode();
            return;
        };
        let value = login.input.trim().to_string();
        match login.step {
            TradeRepublicLoginStep::Phone => {
                if value.is_empty() {
                    self.portfolio.status = Some("Phone number is required.".to_string());
                    return;
                }
                login.phone = value;
                login.input.clear();
                login.step = TradeRepublicLoginStep::Pin;
                self.portfolio.status = Some("Enter your Trade Republic PIN.".to_string());
            }
            TradeRepublicLoginStep::Pin => {
                if value.is_empty() {
                    self.portfolio.status = Some("PIN is required.".to_string());
                    return;
                }
                login.pin = value;
                login.input.clear();
                self.start_trade_republic_login();
            }
            TradeRepublicLoginStep::Code => {
                if value.is_empty() {
                    self.portfolio.status = Some("Code/TAN is required.".to_string());
                    return;
                }
                login.input.clear();
                self.complete_trade_republic_login(value);
            }
        }
    }

    fn start_trade_republic_login(&mut self) {
        let Some(login) = &self.portfolio.login else {
            return;
        };
        let phone = login.phone.clone();
        let pin = login.pin.clone();
        let (sender, receiver) = mpsc::channel();
        self.portfolio.login_receiver = Some(receiver);
        self.portfolio.login_busy = true;
        self.clear_text_input_mode();
        self.portfolio.status = Some("Starting Trade Republic login...".to_string());
        thread::spawn(move || {
            let result = crate::broker::trade_republic::login_start(&phone, &pin)
                .map(|result| match result {
                    LoginStartResult::Connected => BrokerLoginEvent::Connected,
                    LoginStartResult::CodeRequired {
                        process_id,
                        countdown,
                    } => BrokerLoginEvent::CodeRequired {
                        process_id,
                        countdown,
                    },
                })
                .map_err(|error| error.to_string());
            let _ = sender.send(result);
        });
    }

    fn complete_trade_republic_login(&mut self, code: String) {
        let Some(login) = &self.portfolio.login else {
            return;
        };
        let Some(process_id) = login.process_id.clone() else {
            self.portfolio.status = Some("Missing Trade Republic login process.".to_string());
            return;
        };
        let phone = login.phone.clone();
        let pin = login.pin.clone();
        let (sender, receiver) = mpsc::channel();
        self.portfolio.login_receiver = Some(receiver);
        self.portfolio.login_busy = true;
        self.clear_text_input_mode();
        self.portfolio.status = Some("Completing Trade Republic login...".to_string());
        thread::spawn(move || {
            let result =
                crate::broker::trade_republic::login_complete(&phone, &pin, &process_id, &code)
                    .map(|()| BrokerLoginEvent::Connected)
                    .map_err(|error| error.to_string());
            let _ = sender.send(result);
        });
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
        self.poll_trade_republic_login();
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

    fn poll_trade_republic_login(&mut self) {
        let result = self
            .portfolio
            .login_receiver
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        let Some(result) = result else { return };
        self.portfolio.login_receiver = None;
        self.portfolio.login_busy = false;
        match result {
            Ok(BrokerLoginEvent::Connected) => {
                self.config.broker.trade_republic_enabled = true;
                let _ = self.config.save();
                self.portfolio.login = None;
                self.clear_text_input_mode();
                self.portfolio.status =
                    Some("Trade Republic connected. Syncing portfolio...".to_string());
                self.notify("Trade Republic connected");
                self.refresh_portfolio();
            }
            Ok(BrokerLoginEvent::CodeRequired {
                process_id,
                countdown,
            }) => {
                if let Some(login) = &mut self.portfolio.login {
                    login.process_id = Some(process_id);
                    login.countdown = countdown;
                    login.step = TradeRepublicLoginStep::Code;
                    login.input.clear();
                }
                self.portfolio.status = Some("Enter the code/TAN from Trade Republic.".to_string());
                self.begin_text_input(InputTarget::BrokerLogin);
            }
            Err(error) => {
                self.portfolio.status = Some(format!("Trade Republic login failed: {error}"));
                self.begin_text_input(InputTarget::BrokerLogin);
            }
        }
    }

    pub fn disconnect_trade_republic(&mut self) {
        self.config.broker.trade_republic_enabled = false;
        let _ = self.config.save();
        self.portfolio.snapshot = None;
        self.portfolio.selection = 0;
        self.portfolio.syncing = false;
        self.portfolio.receiver = None;
        self.portfolio.login = None;
        self.portfolio.login_busy = false;
        self.portfolio.login_receiver = None;
        self.clear_text_input_mode();
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
