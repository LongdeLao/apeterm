#![allow(dead_code)]

#[path = "../news.rs"]
mod news;

use news::{NewsRuntimeConfig, WatchlistMatcher, fetch_all_news};

fn main() -> Result<(), String> {
    let feeds = vec![
        "https://news.google.com/rss/search?q=markets&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=stocks&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=economy&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=earnings&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=%22Federal+Reserve%22&hl=en-US&gl=US&ceid=US:en"
            .to_string(),
        "https://news.google.com/rss/search?q=ECB&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=site%3Abloomberg.com+markets&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=site%3Awsj.com+markets&hl=en-US&gl=US&ceid=US:en".to_string(),
        "https://news.google.com/rss/search?q=site%3Aft.com+markets&hl=en-US&gl=US&ceid=US:en".to_string(),
    ];

    let stock_symbols = vec![
        "SPY".to_string(),
        "QQQ".to_string(),
        "NVDA".to_string(),
        "AAPL".to_string(),
        "MSFT".to_string(),
        "AMZN".to_string(),
        "META".to_string(),
        "GOOGL".to_string(),
        "TSLA".to_string(),
        "JPM".to_string(),
    ];

    let config = NewsRuntimeConfig {
        feeds,
        stock_symbols: stock_symbols.clone(),
        crypto_symbols: vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()],
        stock_matchers: stock_symbols
            .into_iter()
            .map(|symbol| WatchlistMatcher {
                terms: vec![symbol.clone()],
                symbol,
            })
            .collect(),
        crypto_matchers: vec![
            WatchlistMatcher {
                symbol: "BTCUSDT".to_string(),
                terms: vec!["BTCUSDT".to_string(), "Bitcoin".to_string()],
            },
            WatchlistMatcher {
                symbol: "ETHUSDT".to_string(),
                terms: vec!["ETHUSDT".to_string(), "Ethereum".to_string()],
            },
        ],
        enable_rss: true,
        enable_financial_juice: false,
        enable_nasdaq: false,
    };

    let result = fetch_all_news(&config)?;
    println!("items={}", result.items.len());
    println!("connection={}", result.connection_status);
    if let Some(status) = result.status.as_deref() {
        println!("status={status}");
    }
    println!("sources={:?}", result.source_counts);
    for item in result.items.iter().take(20) {
        println!(
            "{} | {} | {}",
            item.source,
            item.published_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "undated".to_string()),
            item.title
        );
    }
    Ok(())
}
