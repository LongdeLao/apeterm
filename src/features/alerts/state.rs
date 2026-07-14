use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    app::{App, AppMode, Page, SelectionDirection},
    features::watchlist::market::MarketEvent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertDirection {
    Above,
    Below,
    VolumeAbove,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub symbol: String,
    pub direction: AlertDirection,
    pub threshold: f64,
    pub enabled: bool,
    pub triggered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Default)]
pub struct AlertsFeature {
    pub rules: Vec<AlertRule>,
    pub events: Vec<AlertEvent>,
    pub selection: usize,
}

#[cfg(not(test))]
#[derive(Debug, Default, Serialize, Deserialize)]
struct AlertsStore {
    #[serde(default)]
    rules: Vec<AlertRule>,
    #[serde(default)]
    events: Vec<AlertEvent>,
}

impl AlertsFeature {
    pub fn load() -> Self {
        #[cfg(test)]
        return Self::default();

        #[cfg(not(test))]
        let store = alerts_path()
            .and_then(|path| std::fs::read(path).map_err(std::io::Error::other))
            .ok()
            .and_then(|bytes| {
                serde_json::from_slice::<AlertsStore>(&bytes)
                    .ok()
                    .or_else(|| {
                        serde_json::from_slice::<Vec<AlertRule>>(&bytes)
                            .ok()
                            .map(|rules| AlertsStore {
                                rules,
                                events: Vec::new(),
                            })
                    })
            })
            .unwrap_or_default();
        #[cfg(not(test))]
        Self {
            rules: store.rules,
            events: store.events,
            selection: 0,
        }
    }

    fn save(&self) {
        #[cfg(test)]
        return;

        #[cfg(not(test))]
        if let Ok(path) = alerts_path()
            && let Ok(bytes) = serde_json::to_vec_pretty(&AlertsStore {
                rules: self.rules.clone(),
                events: self.events.iter().take(100).cloned().collect(),
            })
        {
            let _ = crate::config::write_atomic(&path, &bytes);
        }
    }
}

#[cfg(not(test))]
fn alerts_path() -> std::io::Result<std::path::PathBuf> {
    Ok(crate::config::data_dir()?.join("alerts.json"))
}

impl App {
    pub fn open_alerts(&mut self) {
        self.return_page = (self.page != Page::Alerts).then_some(self.page);
        self.page = Page::Alerts;
        self.mode = AppMode::Normal;
    }

    pub fn create_price_alert(
        &mut self,
        symbol: &str,
        direction: AlertDirection,
        threshold: f64,
    ) -> Result<String, String> {
        let symbol = symbol.trim().to_ascii_uppercase();
        if symbol.is_empty() || !threshold.is_finite() || threshold <= 0.0 {
            return Err("symbol and a positive threshold are required".to_string());
        }
        self.alerts.rules.push(AlertRule {
            symbol: symbol.clone(),
            direction,
            threshold,
            enabled: true,
            triggered: false,
        });
        self.alerts.save();
        self.notify(format!("Alert created for {symbol}"));
        Ok(format!(
            "created {direction:?} alert for {symbol} at {threshold:.2}"
        ))
    }

    pub fn create_quick_alert(&mut self) {
        let selected = self.search.selected_details.as_ref().and_then(|details| {
            self.search
                .selected_live_details
                .as_ref()
                .and_then(|live| live.price)
                .map(|price| (details.symbol.clone(), price))
        });
        let selected = selected.or_else(|| {
            self.watchlist
                .stock_quotes
                .first()
                .map(|quote| (quote.symbol.clone(), quote.price))
        });
        if let Some((symbol, price)) = selected {
            let _ = self.create_price_alert(&symbol, AlertDirection::Above, price * 1.05);
        } else {
            self.notify("No live quote available for a quick alert");
        }
    }

    pub fn evaluate_market_alerts(&mut self, event: &MarketEvent) {
        let (symbol, price, relative_volume) = match event {
            MarketEvent::CryptoTicker { symbol, price, .. } => (symbol, *price, None),
            MarketEvent::StockTicker {
                symbol,
                price,
                day_volume,
                avg_volume,
                ..
            } => (
                symbol,
                *price,
                day_volume.zip(*avg_volume).and_then(|(day, average)| {
                    (average > 0).then_some(day as f64 / average as f64)
                }),
            ),
        };
        let mut messages = Vec::new();
        for rule in &mut self.alerts.rules {
            if !rule.enabled || rule.triggered || !rule.symbol.eq_ignore_ascii_case(symbol) {
                continue;
            }
            let hit = match rule.direction {
                AlertDirection::Above => price >= rule.threshold,
                AlertDirection::Below => price <= rule.threshold,
                AlertDirection::VolumeAbove => {
                    relative_volume.is_some_and(|value| value >= rule.threshold)
                }
            };
            if hit {
                rule.triggered = true;
                messages.push(match rule.direction {
                    AlertDirection::VolumeAbove => format!(
                        "{} relative volume is {:.1}x (>= {:.1}x)",
                        rule.symbol,
                        relative_volume.unwrap_or_default(),
                        rule.threshold
                    ),
                    _ => format!(
                        "{} is {:.2} ({:?} {:.2})",
                        rule.symbol, price, rule.direction, rule.threshold
                    ),
                });
            }
        }
        let triggered = !messages.is_empty();
        for message in messages {
            self.alerts.events.insert(
                0,
                AlertEvent {
                    message: message.clone(),
                    timestamp: Utc::now().to_rfc3339(),
                },
            );
            self.notify(format!("ALERT · {message}"));
        }
        if triggered {
            self.alerts.save();
        }
    }

    pub fn move_alert_selection(&mut self, direction: SelectionDirection) {
        let count = self.alerts.rules.len();
        if count == 0 {
            return;
        }
        self.alerts.selection = match direction {
            SelectionDirection::Previous => (self.alerts.selection + count - 1) % count,
            SelectionDirection::Next => (self.alerts.selection + 1) % count,
        };
    }
    pub fn toggle_selected_alert(&mut self) {
        if let Some(rule) = self.alerts.rules.get_mut(self.alerts.selection) {
            rule.enabled = !rule.enabled;
            rule.triggered = false;
        }
        self.alerts.save();
    }
    pub fn delete_selected_alert(&mut self) {
        if !self.alerts.rules.is_empty() {
            self.alerts.rules.remove(self.alerts.selection);
            self.alerts.selection = self
                .alerts
                .selection
                .min(self.alerts.rules.len().saturating_sub(1));
            self.alerts.save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::AppConfig, features::watchlist::market::MarketSession};

    #[test]
    fn price_alert_triggers_once() {
        let mut app = App::new(AppConfig::default().unwrap());
        app.create_price_alert("AAPL", AlertDirection::Above, 100.0)
            .unwrap();
        let event = MarketEvent::StockTicker {
            symbol: "AAPL".to_string(),
            price: 101.0,
            price_change_percent: 1.0,
            day_volume: None,
            avg_volume: None,
            market_session: MarketSession::Regular,
        };
        app.evaluate_market_alerts(&event);
        app.evaluate_market_alerts(&event);
        assert_eq!(app.alerts.events.len(), 1);
        assert!(app.alerts.rules[0].triggered);
    }
}
