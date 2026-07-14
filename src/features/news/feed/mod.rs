use std::collections::HashSet;

use chrono::{DateTime, Utc};
use reqwest::Url;
use reqwest::blocking::Client;
use rss::Channel;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

mod classify;
mod dedupe;
mod providers;
mod sources;

pub use classify::contains_symbol;

use classify::*;
use dedupe::*;
use providers::*;
use sources::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub source: String,
    pub source_url: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub url: String,
    pub description: Option<String>,
    pub symbols: Vec<String>,
    pub relevant: bool,
    pub category: NewsCategory,
    pub priority: NewsPriority,
    pub image_url: Option<String>,
    pub feed_order: usize,
    pub item_order: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewsCategory {
    Macro,
    Watchlist,
    Reddit,
    Crypto,
    General,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NewsPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub struct FeedSource {
    pub label: String,
    pub url: String,
    pub order: usize,
}

#[derive(Debug, Clone)]
pub struct NewsRuntimeConfig {
    pub feeds: Vec<String>,
    pub stock_symbols: Vec<String>,
    pub crypto_symbols: Vec<String>,
    pub stock_matchers: Vec<WatchlistMatcher>,
    pub crypto_matchers: Vec<WatchlistMatcher>,
    pub enable_rss: bool,
    pub enable_financial_juice: bool,
    pub enable_nasdaq: bool,
}

#[derive(Debug, Clone)]
pub struct WatchlistMatcher {
    pub symbol: String,
    pub terms: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub items: Vec<NewsItem>,
    pub status: Option<String>,
    pub source_label: String,
    pub connection_status: String,
    pub source_counts: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderFetchResult {
    pub items: Vec<NewsItem>,
    pub errors: Vec<String>,
}

pub trait NewsProvider: Send + Sync {
    fn source_name(&self) -> &'static str;
    fn fetch(&self) -> ProviderFetchResult;
}

#[allow(dead_code)]
pub fn fetch_all_news(config: &NewsRuntimeConfig) -> Result<FetchResult, String> {
    let mut providers: Vec<Box<dyn NewsProvider>> = Vec::new();

    if config.enable_rss {
        let labels = [
            "Markets",
            "Stocks",
            "Economy",
            "Earnings",
            "Federal Reserve",
            "ECB",
            "Bloomberg Markets",
            "WSJ Markets",
            "FT Markets",
        ];
        let feeds = config
            .feeds
            .iter()
            .enumerate()
            .map(|(index, url)| FeedSource {
                label: labels
                    .get(index)
                    .copied()
                    .unwrap_or("Google News")
                    .to_string(),
                url: url.clone(),
                order: index,
            })
            .collect::<Vec<_>>();
        providers.push(Box::new(RssNewsProvider::new(feeds)));
    }
    if config.enable_financial_juice {
        providers.push(Box::new(FinancialJuiceProvider));
    }
    if config.enable_nasdaq {
        providers.push(Box::new(NasdaqNewsProvider::new(
            config.stock_symbols.clone(),
        )));
    }

    Ok(fetch_from_providers(
        providers.iter().map(|provider| provider.as_ref()).collect(),
        &config.stock_symbols,
        &config.crypto_symbols,
        &config.stock_matchers,
        &config.crypto_matchers,
    ))
}

pub fn stream_all_news<F>(config: &NewsRuntimeConfig, mut emit: F) -> Result<(), String>
where
    F: FnMut(FetchResult, bool),
{
    let mut providers: Vec<Box<dyn NewsProvider>> = Vec::new();

    if config.enable_rss {
        let labels = [
            "Markets",
            "Stocks",
            "Economy",
            "Earnings",
            "Federal Reserve",
            "ECB",
            "Bloomberg Markets",
            "WSJ Markets",
            "FT Markets",
        ];
        let feeds = config
            .feeds
            .iter()
            .enumerate()
            .map(|(index, url)| FeedSource {
                label: labels
                    .get(index)
                    .copied()
                    .unwrap_or("Google News")
                    .to_string(),
                url: url.clone(),
                order: index,
            })
            .collect::<Vec<_>>();
        providers.push(Box::new(RssNewsProvider::new(feeds)));
    }
    if config.enable_financial_juice {
        providers.push(Box::new(FinancialJuiceProvider));
    }
    if config.enable_nasdaq {
        providers.push(Box::new(NasdaqNewsProvider::new(
            config.stock_symbols.clone(),
        )));
    }

    if providers.is_empty() {
        return Ok(());
    }

    let mut aggregated = Vec::new();
    let mut errors = Vec::new();
    let mut successful_sources: Vec<String> = Vec::new();
    let mut global_order = 0usize;
    let total = providers.len();

    for (index, provider) in providers.iter().enumerate() {
        let result = provider.fetch();
        if !result.items.is_empty() {
            successful_sources.push(provider.source_name().to_string());
        }
        for item in result.items {
            aggregated.push((item, global_order));
            global_order += 1;
        }
        errors.extend(
            result
                .errors
                .into_iter()
                .map(|error| format!("{}: {}", provider.source_name(), error)),
        );

        emit(
            finalize_items(
                aggregated.clone(),
                errors.clone(),
                successful_sources.clone(),
                &config.stock_symbols,
                &config.crypto_symbols,
                &config.stock_matchers,
                &config.crypto_matchers,
            ),
            index + 1 == total,
        );
    }

    Ok(())
}

fn fetch_from_providers(
    providers: Vec<&dyn NewsProvider>,
    stock_symbols: &[String],
    crypto_symbols: &[String],
    stock_matchers: &[WatchlistMatcher],
    crypto_matchers: &[WatchlistMatcher],
) -> FetchResult {
    let mut aggregated = Vec::new();
    let mut errors = Vec::new();
    let mut successful_sources: Vec<String> = Vec::new();
    let mut global_order = 0usize;

    for provider in providers {
        let result = provider.fetch();
        if !result.items.is_empty() {
            successful_sources.push(provider.source_name().to_string());
        }
        for item in result.items {
            aggregated.push((item, global_order));
            global_order += 1;
        }
        errors.extend(
            result
                .errors
                .into_iter()
                .map(|error| format!("{}: {}", provider.source_name(), error)),
        );
    }

    finalize_items(
        aggregated,
        errors,
        successful_sources,
        stock_symbols,
        crypto_symbols,
        stock_matchers,
        crypto_matchers,
    )
}

fn finalize_items(
    aggregated: Vec<(NewsItem, usize)>,
    errors: Vec<String>,
    successful_sources: Vec<String>,
    stock_symbols: &[String],
    crypto_symbols: &[String],
    stock_matchers: &[WatchlistMatcher],
    crypto_matchers: &[WatchlistMatcher],
) -> FetchResult {
    let mut items = dedupe_exact(aggregated);
    items.retain(is_whitelisted_item);
    items.retain(is_fresh_item);
    for item in &mut items {
        enrich_item(
            item,
            stock_symbols,
            crypto_symbols,
            stock_matchers,
            crypto_matchers,
        );
    }
    items = dedupe_normalized_titles(items);
    sort_items(&mut items);
    let source_counts = source_counts(&items);

    FetchResult {
        items,
        status: (!errors.is_empty()).then(|| errors.join(" | ")),
        source_label: if successful_sources.is_empty() {
            "news feed".to_string()
        } else {
            successful_sources.join(" + ").to_lowercase()
        },
        connection_status: if errors.is_empty() {
            "connected".to_string()
        } else if successful_sources.is_empty() {
            "reconnecting...".to_string()
        } else {
            "degraded".to_string()
        },
        source_counts,
    }
}

fn source_domain(source_url: &str) -> Option<String> {
    Url::parse(source_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
}

fn is_fresh_item(item: &NewsItem) -> bool {
    item.published_at
        .map(|published| Utc::now().signed_duration_since(published).num_days() <= 7)
        .unwrap_or(true)
}

fn source_counts(items: &[NewsItem]) -> Vec<(String, usize)> {
    let mut counts = std::collections::BTreeMap::new();
    for item in items {
        *counts.entry(item.source.clone()).or_insert(0usize) += 1;
    }
    let mut counts = counts.into_iter().collect::<Vec<_>>();
    counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    counts
}

#[derive(Debug, Deserialize)]
struct NasdaqResponse {
    #[serde(default)]
    data: NasdaqData,
}

#[derive(Debug, Default, Deserialize)]
struct NasdaqData {
    #[serde(default)]
    rows: Vec<NasdaqRow>,
}

#[derive(Debug, Default, Deserialize)]
struct NasdaqRow {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    summary: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    publisher: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    author: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    image: Option<String>,
    #[serde(rename = "publishedDate")]
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    published_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    date: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StaticProvider {
        name: &'static str,
        result: ProviderFetchResult,
    }

    impl NewsProvider for StaticProvider {
        fn source_name(&self) -> &'static str {
            self.name
        }

        fn fetch(&self) -> ProviderFetchResult {
            self.result.clone()
        }
    }

    #[test]
    fn parses_rss_item_preferring_guid_for_id() {
        let channel = Channel::read_from(
            br#"<?xml version="1.0"?>
            <rss version="2.0">
              <channel>
                <title>Feed</title>
                <item>
                  <title>Headline</title>
                  <link>https://example.com/story</link>
                  <guid>story-1</guid>
                  <author>Jane Doe</author>
                  <pubDate>Mon, 01 Jul 2024 12:00:00 GMT</pubDate>
                  <description><![CDATA[<p>Hello <b>world</b></p>]]></description>
                </item>
              </channel>
            </rss>"#
                .as_slice(),
        )
        .unwrap();

        let item = parse_rss_item(&channel.items()[0], "Feed", NewsPriority::Medium, 0, 0).unwrap();
        assert_eq!(item.id, "story-1");
        assert_eq!(item.title, "Headline");
        assert_eq!(item.author.as_deref(), Some("Jane Doe"));
        assert_eq!(item.description.as_deref(), Some("Hello world"));
        assert_eq!(item.priority, NewsPriority::Medium);
    }

    #[test]
    fn dedupes_by_id_then_url() {
        let first = test_item("same", "https://example.com/a", None);
        let second = test_item("same", "https://example.com/b", None);
        let third = NewsItem {
            id: String::new(),
            url: "https://example.com/c".to_string(),
            ..test_item("", "https://example.com/c", None)
        };
        let fourth = NewsItem {
            id: String::new(),
            url: "https://example.com/c".to_string(),
            ..test_item("", "https://example.com/c", None)
        };

        let items = dedupe_exact(vec![(first, 0), (second, 1), (third, 2), (fourth, 3)]);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn sorts_newest_first() {
        let older = test_item("1", "https://example.com/1", Some("2024-01-01T10:00:00Z"));
        let newer = test_item("2", "https://example.com/2", Some("2024-01-01T11:00:00Z"));
        let undated = test_item("3", "https://example.com/3", None);

        let mut items = dedupe_exact(vec![(older, 0), (newer, 1), (undated, 2)]);
        sort_items(&mut items);
        assert_eq!(
            items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["2", "1", "3"]
        );
    }

    #[test]
    fn strips_financial_juice_prefix() {
        let channel = Channel::read_from(
            br#"<?xml version="1.0"?>
            <rss version="2.0">
              <channel>
                <title>Feed</title>
                <item>
                  <title>FinancialJuice: CPI comes in hot</title>
                  <link>https://example.com/story</link>
                </item>
              </channel>
            </rss>"#
                .as_slice(),
        )
        .unwrap();

        let mut item = parse_rss_item(
            &channel.items()[0],
            "FinancialJuice",
            NewsPriority::High,
            0,
            0,
        )
        .unwrap();
        item.title = item
            .title
            .strip_prefix("FinancialJuice: ")
            .unwrap_or(item.title.as_str())
            .to_string();
        assert_eq!(item.title, "CPI comes in hot");
    }

    #[test]
    fn parses_nasdaq_json_and_attaches_symbol() {
        let response: NasdaqResponse = serde_json::from_str(
            r#"{
                "data": {
                    "rows": [{
                        "id": "abc",
                        "title": "NVDA wins again",
                        "url": "https://example.com/nvda",
                        "summary": "<p>Chip news</p>",
                        "publisher": "Reuters",
                        "author": "Staff",
                        "image": "https://example.com/image.jpg",
                        "publishedDate": "2024-07-01T12:00:00Z"
                    }]
                }
            }"#,
        )
        .unwrap();

        let items = parse_nasdaq_response(&response, "NVDA", 0);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source, "Reuters");
        assert_eq!(items[0].symbols, vec!["NVDA"]);
        assert_eq!(items[0].description.as_deref(), Some("Chip news"));
    }

    #[test]
    fn provider_failure_does_not_kill_aggregate_fetch() {
        let good = StaticProvider {
            name: "good",
            result: ProviderFetchResult {
                items: vec![NewsItem {
                    source: "Reuters".to_string(),
                    url: "https://www.reuters.com/world/markets-1".to_string(),
                    ..test_item("1", "https://www.reuters.com/world/markets-1", None)
                }],
                errors: Vec::new(),
            },
        };
        let bad = StaticProvider {
            name: "bad",
            result: ProviderFetchResult {
                items: Vec::new(),
                errors: vec!["timeout".to_string()],
            },
        };

        let result = fetch_from_providers(
            vec![&good, &bad],
            &["NVDA".to_string()],
            &[],
            &[WatchlistMatcher {
                symbol: "NVDA".to_string(),
                terms: vec!["NVDA".to_string()],
            }],
            &[],
        );
        assert_eq!(result.items.len(), 1);
        assert!(
            result
                .status
                .as_deref()
                .unwrap_or_default()
                .contains("bad: timeout")
        );
    }

    #[test]
    fn extracts_watchlist_symbols_from_headline() {
        let mut item = test_item("1", "https://example.com/1", None);
        item.title = "NVDA rises while AAPL slips".to_string();
        enrich_item(
            &mut item,
            &["NVDA".to_string(), "AAPL".to_string()],
            &[],
            &[
                WatchlistMatcher {
                    symbol: "NVDA".to_string(),
                    terms: vec!["NVDA".to_string()],
                },
                WatchlistMatcher {
                    symbol: "AAPL".to_string(),
                    terms: vec!["AAPL".to_string()],
                },
            ],
            &[],
        );
        assert_eq!(item.symbols, vec!["AAPL", "NVDA"]);
        assert!(item.relevant);
        assert_eq!(item.category, NewsCategory::Watchlist);
    }

    #[test]
    fn does_not_match_symbol_inside_other_words() {
        assert!(!contains_symbol("MUSIC rallies", "MU"));
        assert!(contains_symbol("MU rallies", "MU"));
    }

    #[test]
    fn tolerates_numeric_nasdaq_ids() {
        let response: NasdaqResponse = serde_json::from_str(
            r#"{"data":{"rows":[{"id":27867191,"title":"Headline","url":"https://example.com","publisher":"NASDAQ"}]}}"#,
        )
        .unwrap();
        let items = parse_nasdaq_response(&response, "SPY", 0);
        assert_eq!(items[0].id, "27867191");
    }

    #[test]
    fn dedupes_by_normalized_title() {
        let first = NewsItem {
            title: "Markets Live: Fed Chair speaks - Reuters".to_string(),
            ..test_item("1", "https://example.com/1", None)
        };
        let second = NewsItem {
            title: "Markets Live Fed Chair speaks".to_string(),
            ..test_item("2", "https://example.com/2", None)
        };
        assert_eq!(dedupe_normalized_titles(vec![first, second]).len(), 1);
    }

    fn test_item(id: &str, url: &str, timestamp: Option<&str>) -> NewsItem {
        NewsItem {
            id: id.to_string(),
            title: id.to_string(),
            source: "Feed".to_string(),
            source_url: None,
            author: None,
            published_at: timestamp.and_then(parse_timestamp),
            url: url.to_string(),
            description: None,
            symbols: Vec::new(),
            relevant: false,
            category: NewsCategory::General,
            priority: NewsPriority::Medium,
            image_url: None,
            feed_order: 0,
            item_order: 0,
        }
    }
}
