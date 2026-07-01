use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use serde_json::Value;
use tungstenite::{Message, connect};

const LOCAL_PYTHON: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/.venv/bin/python");
const YFINANCE_SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/yfinance_stream.py");

#[derive(Debug, Clone)]
pub enum MarketEvent {
    CryptoTicker {
        symbol: String,
        price: f64,
        price_change_percent: f64,
    },
    StockTicker {
        symbol: String,
        price: f64,
        price_change_percent: f64,
        market_session: MarketSession,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketSession {
    PreMarket,
    Regular,
    AfterHours,
}

pub fn market_channel() -> (Sender<MarketEvent>, Receiver<MarketEvent>) {
    mpsc::channel()
}

pub fn spawn_market_streams(
    sender: Sender<MarketEvent>,
    crypto_symbols: Vec<String>,
    stock_symbols: Vec<String>,
) {
    spawn_binance_stream(sender.clone(), crypto_symbols);
    spawn_yfinance_stream(sender, stock_symbols);
}

fn spawn_binance_stream(sender: Sender<MarketEvent>, symbols: Vec<String>) {
    let Some(stream_url) = binance_stream_url(&symbols) else {
        return;
    };

    thread::spawn(move || {
        loop {
            if stream_binance_prices(&sender, &stream_url).is_err() {
                thread::sleep(Duration::from_secs(3));
            }
        }
    });
}

fn stream_binance_prices(
    sender: &Sender<MarketEvent>,
    stream_url: &str,
) -> tungstenite::Result<()> {
    let (mut socket, _) = connect(stream_url)?;

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

fn binance_stream_url(symbols: &[String]) -> Option<String> {
    let streams = symbols
        .iter()
        .map(|symbol| symbol.trim().to_ascii_lowercase())
        .filter(|symbol| !symbol.is_empty())
        .map(|symbol| format!("{symbol}@ticker"))
        .collect::<Vec<_>>();

    if streams.is_empty() {
        None
    } else {
        Some(format!(
            "wss://stream.binance.com:9443/stream?streams={}",
            streams.join("/")
        ))
    }
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

impl MarketSession {
    fn from_value(value: &str) -> Self {
        match value {
            "pre_market" => Self::PreMarket,
            "regular" => Self::Regular,
            "after_hours" => Self::AfterHours,
            _ => Self::AfterHours,
        }
    }
}

fn spawn_yfinance_stream(sender: Sender<MarketEvent>, symbols: Vec<String>) {
    if symbols.is_empty() {
        return;
    }

    thread::spawn(move || {
        loop {
            if stream_yfinance_prices(&sender, &symbols).is_err() {
                thread::sleep(Duration::from_secs(5));
            }
        }
    });
}

fn stream_yfinance_prices(sender: &Sender<MarketEvent>, symbols: &[String]) -> std::io::Result<()> {
    let mut child = Command::new(python_command())
        .arg("-u")
        .arg(YFINANCE_SCRIPT)
        .args(symbols)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let Some(stdout) = child.stdout.take() else {
        return Ok(());
    };

    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = line?;
        if let Some(event) = parse_stock_message(&line) {
            if sender.send(event).is_err() {
                break;
            }
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

fn python_command() -> &'static str {
    if Path::new(LOCAL_PYTHON).exists() {
        LOCAL_PYTHON
    } else {
        "python3"
    }
}

fn parse_stock_message(text: &str) -> Option<MarketEvent> {
    let value: Value = serde_json::from_str(text).ok()?;
    let symbol = value.get("symbol")?.as_str()?.to_string();
    let price = value.get("price")?.as_f64()?;
    let price_change_percent = value
        .get("price_change_percent")
        .and_then(Value::as_f64)
        .unwrap_or_default();
    let market_session = value
        .get("market_state")
        .and_then(Value::as_str)
        .map(MarketSession::from_value)
        .unwrap_or(MarketSession::AfterHours);
    Some(MarketEvent::StockTicker {
        symbol,
        price,
        price_change_percent,
        market_session,
    })
}
