use std::time::{Duration, Instant};

use crate::features::watchlist::market::{MarketEvent, MarketSession};

const QUOTE_FLASH_DURATION: Duration = Duration::from_millis(650);

#[derive(Debug, Clone)]
pub struct Quote {
    pub symbol: String,
    pub price: f64,
    pub change_percent: f64,
    pub day_volume: Option<u64>,
    pub avg_volume: Option<u64>,
    pub relative_volume: Option<f64>,
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
    stock_market_session: &mut Option<MarketSession>,
    event: MarketEvent,
) {
    match event {
        MarketEvent::CryptoTicker {
            symbol,
            price,
            price_change_percent,
        } => update_quotes(
            crypto_quotes,
            symbol,
            price,
            price_change_percent,
            None,
            None,
        ),
        MarketEvent::StockTicker {
            symbol,
            price,
            price_change_percent,
            day_volume,
            avg_volume,
            market_session,
        } => {
            *stock_market_session = Some(market_session);
            update_quotes(
                stock_quotes,
                symbol,
                price,
                price_change_percent,
                day_volume,
                avg_volume,
            );
        }
    }
}

fn update_quotes(
    quotes: &mut Vec<Quote>,
    symbol: String,
    price: f64,
    change_percent: f64,
    day_volume: Option<u64>,
    avg_volume: Option<u64>,
) {
    if let Some(quote) = quotes.iter_mut().find(|quote| quote.symbol == symbol) {
        quote.direction = PriceDirection::from_prices(quote.price, price);
        quote.price = price;
        quote.change_percent = change_percent;
        quote.day_volume = day_volume.or(quote.day_volume);
        quote.avg_volume = avg_volume.or(quote.avg_volume);
        quote.relative_volume = relative_volume(quote.day_volume, quote.avg_volume);
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
            change_percent,
            day_volume,
            avg_volume,
            relative_volume: relative_volume(day_volume, avg_volume),
            direction: PriceDirection::Flat,
            flash_until: None,
        });
        quotes.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    }
}

fn relative_volume(day_volume: Option<u64>, avg_volume: Option<u64>) -> Option<f64> {
    let day_volume = day_volume?;
    let avg_volume = avg_volume?;
    if avg_volume == 0 {
        return None;
    }
    Some(day_volume as f64 / avg_volume as f64)
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
