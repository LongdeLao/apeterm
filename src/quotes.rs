use std::time::{Duration, Instant};

use crate::market::{MarketEvent, MarketSession};

const QUOTE_FLASH_DURATION: Duration = Duration::from_millis(650);

#[derive(Debug, Clone)]
pub struct Quote {
    pub symbol: String,
    pub price: f64,
    pub price_change_percent: f64,
    pub direction: PriceDirection,
    pub flash_until: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceDirection {
    Up,
    Down,
    Flat,
}

pub fn update_market_quotes(
    crypto_quotes: &mut Vec<Quote>,
    stock_quotes: &mut Vec<Quote>,
    stock_market_session: &mut MarketSession,
    event: MarketEvent,
) {
    match event {
        MarketEvent::CryptoTicker {
            symbol,
            price,
            price_change_percent,
        } => update_quotes(crypto_quotes, symbol, price, price_change_percent),
        MarketEvent::StockTicker {
            symbol,
            price,
            price_change_percent,
            market_session,
        } => {
            *stock_market_session = market_session;
            update_quotes(stock_quotes, symbol, price, price_change_percent);
        }
    }
}

fn update_quotes(quotes: &mut Vec<Quote>, symbol: String, price: f64, price_change_percent: f64) {
    if let Some(quote) = quotes.iter_mut().find(|quote| quote.symbol == symbol) {
        quote.direction = PriceDirection::from_prices(quote.price, price);
        quote.price = price;
        quote.price_change_percent = price_change_percent;
        quote.flash_until = match quote.direction {
            PriceDirection::Up | PriceDirection::Down => {
                Some(Instant::now() + QUOTE_FLASH_DURATION)
            }
            PriceDirection::Flat => quote.flash_until,
        };
    } else {
        quotes.push(Quote {
            symbol,
            price,
            price_change_percent,
            direction: PriceDirection::Flat,
            flash_until: None,
        });
        quotes.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    }
}

impl PriceDirection {
    fn from_prices(previous: f64, current: f64) -> Self {
        if current > previous {
            Self::Up
        } else if current < previous {
            Self::Down
        } else {
            Self::Flat
        }
    }
}
