use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use serde_json::Value;
use tungstenite::{Message, connect};

const BINANCE_STREAM_URL: &str =
    "wss://stream.binance.com:9443/stream?streams=btcusdt@ticker/ethusdt@ticker/solusdt@ticker";

#[derive(Debug, Clone)]
pub enum MarketEvent {
    CryptoTicker {
        symbol: String,
        price: f64,
        price_change_percent: f64,
    },
}

pub fn spawn_binance_stream() -> Receiver<MarketEvent> {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        loop {
            if stream_binance_prices(&sender).is_err() {
                thread::sleep(Duration::from_secs(3));
            }
        }
    });

    receiver
}

fn stream_binance_prices(sender: &Sender<MarketEvent>) -> tungstenite::Result<()> {
    let (mut socket, _) = connect(BINANCE_STREAM_URL)?;

    loop {
        let message = socket.read()?;
        let Message::Text(text) = message else {
            continue;
        };

        if let Some(event) = parse_ticker_message(&text) {
            if sender.send(event).is_err() {
                break;
            }
        }
    }

    Ok(())
}

fn parse_ticker_message(text: &str) -> Option<MarketEvent> {
    let value: Value = serde_json::from_str(text).ok()?;
    let data = value.get("data")?;
    let symbol = data.get("s")?.as_str()?.to_string();
    let price = data.get("c")?.as_str()?.parse::<f64>().ok()?;
    let price_change_percent = data.get("P")?.as_str()?.parse::<f64>().ok()?;

    Some(MarketEvent::CryptoTicker {
        symbol,
        price,
        price_change_percent,
    })
}
