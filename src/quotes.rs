use std::time::{Duration, Instant};

use crate::market::MarketEvent;

const QUOTE_FLASH_DURATION: Duration = Duration::from_millis(650);

#[derive(Debug, Clone)]
pub struct CryptoQuote {
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

pub fn update_crypto_quotes(quotes: &mut Vec<CryptoQuote>, event: MarketEvent) {
    match event {
        MarketEvent::CryptoTicker {
            symbol,
            price,
            price_change_percent,
        } => {
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
                quotes.push(CryptoQuote {
                    symbol,
                    price,
                    price_change_percent,
                    direction: PriceDirection::Flat,
                    flash_until: None,
                });
                quotes.sort_by(|left, right| left.symbol.cmp(&right.symbol));
            }
        }
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
