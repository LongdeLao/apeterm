use std::collections::HashSet;

use chrono::{DateTime, Utc};
use reqwest::Url;
use reqwest::blocking::Client;
use rss::Channel;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

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

pub struct RssNewsProvider {
    feeds: Vec<FeedSource>,
}

pub struct FinancialJuiceProvider;

pub struct NasdaqNewsProvider {
    symbols: Vec<String>,
}

impl RssNewsProvider {
    pub fn new(feeds: Vec<FeedSource>) -> Self {
        Self { feeds }
    }
}

impl NasdaqNewsProvider {
    pub fn new(symbols: Vec<String>) -> Self {
        Self { symbols }
    }
}

impl NewsProvider for RssNewsProvider {
    fn source_name(&self) -> &'static str {
        "RSS"
    }

    fn fetch(&self) -> ProviderFetchResult {
        let Ok(client) = build_client() else {
            return ProviderFetchResult {
                items: Vec::new(),
                errors: vec!["RSS: failed to build HTTP client".to_string()],
            };
        };
        let mut items = Vec::new();
        let mut errors = Vec::new();

        for feed in &self.feeds {
            let bytes = match get_bytes(&client, feed.url.as_str()) {
                Ok(bytes) => bytes,
                Err(error) => {
                    errors.push(format!("{}: {}", feed.label, error));
                    continue;
                }
            };
            let channel = match Channel::read_from(&bytes[..]) {
                Ok(channel) => channel,
                Err(error) => {
                    errors.push(format!("{}: {}", feed.label, error));
                    continue;
                }
            };

            for (item_order, item) in channel.items().iter().enumerate() {
                if let Some(parsed) = parse_rss_item(
                    item,
                    feed.label.as_str(),
                    NewsPriority::Medium,
                    feed.order,
                    item_order,
                ) {
                    items.push(parsed);
                }
            }
        }

        ProviderFetchResult { items, errors }
    }
}

impl NewsProvider for FinancialJuiceProvider {
    fn source_name(&self) -> &'static str {
        "FinancialJuice"
    }

    fn fetch(&self) -> ProviderFetchResult {
        let Ok(client) = build_client() else {
            return ProviderFetchResult {
                items: Vec::new(),
                errors: vec!["FinancialJuice: failed to build HTTP client".to_string()],
            };
        };
        let bytes = match get_bytes(&client, "https://www.financialjuice.com/feed.ashx?xy=rss") {
            Ok(bytes) => bytes,
            Err(error) => {
                return ProviderFetchResult {
                    items: Vec::new(),
                    errors: vec![format!("FinancialJuice: {error}")],
                };
            }
        };
        let channel = match Channel::read_from(&bytes[..]) {
            Ok(channel) => channel,
            Err(error) => {
                return ProviderFetchResult {
                    items: Vec::new(),
                    errors: vec![format!("FinancialJuice: {error}")],
                };
            }
        };
        let mut items = Vec::new();

        for (item_order, item) in channel.items().iter().enumerate() {
            if let Some(mut parsed) = parse_rss_item(
                item,
                "FinancialJuice",
                NewsPriority::High,
                usize::MAX - 1,
                item_order,
            ) {
                parsed.title = parsed
                    .title
                    .strip_prefix("FinancialJuice: ")
                    .unwrap_or(parsed.title.as_str())
                    .trim()
                    .to_string();
                items.push(parsed);
            }
        }

        ProviderFetchResult {
            items,
            errors: Vec::new(),
        }
    }
}

impl NewsProvider for NasdaqNewsProvider {
    fn source_name(&self) -> &'static str {
        "NASDAQ"
    }

    fn fetch(&self) -> ProviderFetchResult {
        let Ok(client) = build_client() else {
            return ProviderFetchResult {
                items: Vec::new(),
                errors: vec!["NASDAQ: failed to build HTTP client".to_string()],
            };
        };
        let mut items = Vec::new();
        let mut errors = Vec::new();

        for (feed_order, symbol) in self
            .symbols
            .iter()
            .filter(|symbol| is_supported_nasdaq_symbol(symbol))
            .enumerate()
        {
            let url = format!(
                "https://www.nasdaq.com/api/news/topic/articlebysymbol?fallback=true&offset=0&limit=20&q={}%7CSTOCKS",
                symbol
            );
            let bytes = match get_bytes(&client, &url) {
                Ok(bytes) => bytes,
                Err(error) => {
                    errors.push(format!("{symbol}: {error}"));
                    continue;
                }
            };
            let response: NasdaqResponse = match serde_json::from_slice(&bytes) {
                Ok(response) => response,
                Err(error) => {
                    errors.push(format!("{symbol}: {error}"));
                    continue;
                }
            };
            items.extend(parse_nasdaq_response(
                &response,
                symbol,
                usize::MAX / 2 + feed_order,
            ));
        }

        ProviderFetchResult { items, errors }
    }
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

fn build_client() -> Result<Client, String> {
    Client::builder()
        .user_agent("ApeTerm/0.1")
        .build()
        .map_err(|error| error.to_string())
}

fn get_bytes(client: &Client, url: &str) -> Result<Vec<u8>, String> {
    client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|error| error.to_string())
}

fn parse_rss_item(
    item: &rss::Item,
    source: &str,
    priority: NewsPriority,
    feed_order: usize,
    item_order: usize,
) -> Option<NewsItem> {
    let raw_title = item.title()?.trim();
    let url = item.link()?.trim();
    if raw_title.is_empty() || url.is_empty() {
        return None;
    }

    let id = item.guid().map(|guid| guid.value()).unwrap_or(url).trim();
    if id.is_empty() {
        return None;
    }

    let description = item
        .description()
        .map(strip_html)
        .filter(|value| !value.trim().is_empty());
    let (title, inferred_source) = infer_title_source(raw_title);
    let source_url = item
        .source()
        .map(|source_meta| source_meta.url().trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let source = item
        .source()
        .and_then(|source_meta| source_meta.title())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(non_generic_source)
        .or_else(|| {
            source_url
                .as_deref()
                .and_then(source_domain)
                .map(|domain| normalize_source(domain.as_str()))
                .filter(|candidate| allowed_source_name(candidate.as_str()))
        })
        .or(inferred_source)
        .or_else(|| {
            description
                .as_deref()
                .and_then(infer_source_from_description)
        })
        .unwrap_or_else(|| source.to_string());

    Some(NewsItem {
        id: id.to_string(),
        title,
        source: source.to_string(),
        source_url,
        author: item
            .author()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        published_at: item.pub_date().and_then(parse_timestamp),
        url: url.to_string(),
        description,
        symbols: Vec::new(),
        relevant: false,
        category: NewsCategory::General,
        priority,
        image_url: extract_rss_image_url(item),
        feed_order,
        item_order,
    })
}

fn infer_title_source(title: &str) -> (String, Option<String>) {
    let Some((headline, suffix)) = split_title_suffix(title) else {
        return (title.to_string(), None);
    };
    let suffix = suffix.trim();
    if suffix.is_empty() || suffix.len() > 48 || suffix.contains(':') {
        return (title.to_string(), None);
    }
    let normalized = normalize_source(suffix);
    if normalized == suffix || allowed_source_name(normalized.as_str()) {
        return (headline.trim().to_string(), Some(normalized));
    }
    (title.to_string(), None)
}

fn split_title_suffix(title: &str) -> Option<(&str, &str)> {
    [" - ", " | ", " — ", " – "]
        .iter()
        .find_map(|separator| title.rsplit_once(separator))
}

fn non_generic_source(source: &str) -> Option<String> {
    let trimmed = source.trim();
    if trimmed.is_empty() || is_generic_feed_label(trimmed) {
        None
    } else {
        Some(normalize_source(trimmed))
    }
}

fn is_generic_feed_label(source: &str) -> bool {
    matches!(
        source.to_ascii_lowercase().as_str(),
        "markets"
            | "stocks"
            | "economy"
            | "earnings"
            | "federal reserve"
            | "ecb"
            | "bloomberg markets"
            | "wsj markets"
            | "ft markets"
            | "google news"
    )
}

fn infer_source_from_description(description: &str) -> Option<String> {
    let candidate = description
        .rsplit("  ")
        .next()
        .or_else(|| description.rsplit(' ').next())
        .unwrap_or_default()
        .trim();
    let normalized = normalize_source(candidate);
    allowed_source_name(normalized.as_str()).then_some(normalized)
}

fn extract_rss_image_url(item: &rss::Item) -> Option<String> {
    item.enclosure()
        .map(|enclosure| enclosure.url().trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc2822(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .or_else(|_| {
            DateTime::parse_from_rfc3339(value).map(|timestamp| timestamp.with_timezone(&Utc))
        })
        .ok()
}

fn dedupe_exact(items: Vec<(NewsItem, usize)>) -> Vec<NewsItem> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for (item, order) in items {
        let key = dedupe_key(&item);
        if seen.insert(key) {
            deduped.push((item, order));
        }
    }

    deduped.into_iter().map(|(item, _)| item).collect()
}

fn dedupe_normalized_titles(items: Vec<NewsItem>) -> Vec<NewsItem> {
    let mut seen = HashSet::new();
    let mut kept = Vec::new();
    for item in items {
        let key = normalized_title_key(&item.title);
        if key.is_empty() || seen.insert(key) {
            kept.push(item);
        } else {
            continue;
        }
    }

    kept
}

fn sort_items(items: &mut [NewsItem]) {
    items.sort_by(|left, right| {
        right
            .published_at
            .cmp(&left.published_at)
            .then_with(|| left.feed_order.cmp(&right.feed_order))
            .then_with(|| left.item_order.cmp(&right.item_order))
    });
}

fn dedupe_key(item: &NewsItem) -> String {
    if item.id.trim().is_empty() {
        item.url.clone()
    } else {
        item.id.clone()
    }
}

fn strip_html(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    let mut in_tag = false;
    let mut entity = String::new();
    let mut in_entity = false;

    for character in value.chars() {
        match character {
            '<' => {
                in_tag = true;
                if !text.ends_with(' ') {
                    text.push(' ');
                }
            }
            '>' => in_tag = false,
            '&' if !in_tag => {
                in_entity = true;
                entity.clear();
            }
            ';' if in_entity => {
                in_entity = false;
                text.push_str(match entity.as_str() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "#39" | "apos" => "'",
                    "nbsp" => " ",
                    _ => "",
                });
            }
            _ if in_tag => {}
            _ if in_entity => entity.push(character),
            _ => text.push(character),
        }
    }

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_supported_nasdaq_symbol(symbol: &str) -> bool {
    !(symbol.starts_with('^')
        || symbol.contains('.')
        || symbol.contains('-')
        || symbol.contains('='))
}

fn parse_nasdaq_response(
    response: &NasdaqResponse,
    symbol: &str,
    feed_order: usize,
) -> Vec<NewsItem> {
    response
        .data
        .rows
        .iter()
        .enumerate()
        .filter_map(|(item_order, row)| parse_nasdaq_row(row, symbol, feed_order, item_order))
        .collect()
}

fn parse_nasdaq_row(
    row: &NasdaqRow,
    symbol: &str,
    feed_order: usize,
    item_order: usize,
) -> Option<NewsItem> {
    let title = row.title.as_deref()?.trim();
    let url = row.url.as_deref()?.trim();
    if title.is_empty() || url.is_empty() {
        return None;
    }

    let source = row
        .publisher
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("NASDAQ");
    let published_at = row
        .published_date
        .as_deref()
        .or(row.date.as_deref())
        .and_then(parse_timestamp);
    let id = row
        .id
        .as_deref()
        .or(row.url.as_deref())
        .unwrap_or(url)
        .trim()
        .to_string();

    Some(NewsItem {
        id,
        title: strip_html(title),
        source: source.to_string(),
        source_url: None,
        author: row
            .author
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        published_at,
        url: url.to_string(),
        description: row
            .summary
            .as_deref()
            .map(strip_html)
            .filter(|value| !value.is_empty()),
        symbols: vec![symbol.to_string()],
        relevant: true,
        category: NewsCategory::Watchlist,
        priority: NewsPriority::Medium,
        image_url: row.image.as_deref().map(str::to_string),
        feed_order,
        item_order,
    })
}

fn enrich_item(
    item: &mut NewsItem,
    stock_symbols: &[String],
    crypto_symbols: &[String],
    stock_matchers: &[WatchlistMatcher],
    crypto_matchers: &[WatchlistMatcher],
) {
    let mut matched = item.symbols.clone();
    let headline = item.title.as_str();
    let description = item.description.as_deref().unwrap_or_default();

    for symbol in stock_symbols {
        if contains_symbol(headline, symbol) && !matched.contains(symbol) {
            matched.push(symbol.clone());
        }
    }
    for symbol in crypto_symbols {
        if contains_symbol(headline, symbol) && !matched.contains(symbol) {
            matched.push(symbol.clone());
        }
    }
    for matcher in stock_matchers {
        if matches_watchlist_item(headline, description, matcher)
            && !matched.contains(&matcher.symbol)
        {
            matched.push(matcher.symbol.clone());
        }
    }
    for matcher in crypto_matchers {
        if matches_watchlist_item(headline, description, matcher)
            && !matched.contains(&matcher.symbol)
        {
            matched.push(matcher.symbol.clone());
        }
    }

    matched.sort();
    matched.dedup();
    item.symbols = matched;
    item.relevant = !item.symbols.is_empty();
    item.category = classify_category(headline, item.relevant, crypto_symbols);
    item.source = normalize_source(&item.source);
    item.priority = classify_priority(item);
}

fn matches_watchlist_item(headline: &str, description: &str, matcher: &WatchlistMatcher) -> bool {
    matcher.terms.iter().any(|term| {
        contains_symbol(headline, term)
            || contains_phrase(headline, term)
            || contains_phrase(description, term)
    })
}

fn contains_phrase(text: &str, phrase: &str) -> bool {
    let phrase = normalize_match_term(phrase);
    if phrase.is_empty() {
        return false;
    }
    normalize_match_term(text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .contains(phrase.as_str())
}

fn normalize_match_term(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character.is_ascii_whitespace() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_symbol(text: &str, symbol: &str) -> bool {
    if symbol.is_empty() {
        return false;
    }

    let mut start = 0usize;
    while let Some(found) = text[start..].find(symbol) {
        let index = start + found;
        let before = text[..index].chars().next_back();
        let after = text[index + symbol.len()..].chars().next();
        let left_ok = before.is_none_or(|character| !character.is_ascii_alphanumeric());
        let right_ok = after.is_none_or(|character| !character.is_ascii_alphanumeric());
        if left_ok && right_ok {
            return true;
        }
        start = index + symbol.len();
    }

    false
}

fn classify_category(headline: &str, relevant: bool, crypto_symbols: &[String]) -> NewsCategory {
    let lowercase = headline.to_ascii_lowercase();
    if relevant {
        return NewsCategory::Watchlist;
    }
    if lowercase.contains("reddit")
        || lowercase.contains("wallstreetbets")
        || lowercase.contains("r/")
    {
        return NewsCategory::Reddit;
    }
    if is_crypto_headline(headline, &lowercase, crypto_symbols) {
        return NewsCategory::Crypto;
    }
    if is_macro_headline(&lowercase) {
        return NewsCategory::Macro;
    }
    NewsCategory::General
}

fn is_crypto_headline(headline: &str, lowercase: &str, crypto_symbols: &[String]) -> bool {
    lowercase.contains("crypto")
        || contains_symbol(headline, "BTC")
        || contains_symbol(headline, "ETH")
        || crypto_symbols
            .iter()
            .any(|symbol| contains_symbol(headline, symbol))
}

fn is_macro_headline(lowercase: &str) -> bool {
    [
        "fed",
        "rates",
        "rate cut",
        "cpi",
        "jobs",
        "payroll",
        "treasury",
        "yield",
        "opec",
        "inflation",
        "powell",
        "ecb",
        "boj",
        "gdp",
        "pmi",
        "fomc",
    ]
    .iter()
    .any(|keyword| lowercase.contains(keyword))
}

fn classify_priority(item: &NewsItem) -> NewsPriority {
    let title = item.title.to_ascii_lowercase();
    if title.contains("breaking")
        || title.contains("halts trading")
        || title.contains("files bankruptcy")
        || title.contains("missile")
        || title.contains("explosion")
        || title.contains("emergency")
    {
        return NewsPriority::Critical;
    }

    if is_clickbait_source(&item.source) || is_clickbait_title(&title) {
        return NewsPriority::Low;
    }

    if item.category == NewsCategory::Macro {
        return NewsPriority::High;
    }

    NewsPriority::Medium
}

fn normalize_source(source: &str) -> String {
    let lowercase = source.to_ascii_lowercase();
    if lowercase.contains("reuters") {
        "Reuters".to_string()
    } else if lowercase.contains("cnbc") {
        "CNBC".to_string()
    } else if lowercase.contains("cnn") {
        "CNN".to_string()
    } else if lowercase.contains("bloomberg") {
        "Bloomberg".to_string()
    } else if lowercase.contains("wall street journal")
        || lowercase == "wsj"
        || lowercase.contains(" wsj")
    {
        "Wall Street Journal".to_string()
    } else if lowercase.contains("financial times") || lowercase == "ft" {
        "Financial Times".to_string()
    } else if lowercase.contains("financialjuice") {
        "FinancialJuice".to_string()
    } else if lowercase.contains("nasdaq") {
        "NASDAQ".to_string()
    } else if lowercase.contains("yahoo") {
        "Yahoo Finance".to_string()
    } else if lowercase.contains("investing.com") {
        "Investing.com".to_string()
    } else if lowercase.contains("seeking alpha") || lowercase.contains("seekingalpha") {
        "Seeking Alpha".to_string()
    } else if lowercase.contains("barron") {
        "Barron's".to_string()
    } else if lowercase == "sec" || lowercase.contains("securities and exchange commission") {
        "SEC".to_string()
    } else if lowercase.contains("federal reserve") {
        "Federal Reserve".to_string()
    } else if lowercase == "ecb" || lowercase.contains("european central bank") {
        "ECB".to_string()
    } else if lowercase.contains("pr newswire") || lowercase.contains("prnewswire") {
        "PR Newswire".to_string()
    } else if lowercase.contains("globenewswire") {
        "GlobeNewswire".to_string()
    } else if lowercase.contains("business wire") || lowercase.contains("businesswire") {
        "Business Wire".to_string()
    } else if lowercase.contains("reddit") {
        "Reddit".to_string()
    } else if lowercase.contains("motley") {
        "Motley Fool".to_string()
    } else {
        source.trim().to_string()
    }
}

fn is_clickbait_source(source: &str) -> bool {
    normalize_source(source) == "Motley Fool"
}

fn is_clickbait_title(title: &str) -> bool {
    [
        "should you buy",
        "top pick",
        "perfect opportunity",
        "millionaire-maker",
        "no-brainer",
        "here's why",
        "could make you rich",
        "buy before",
    ]
    .iter()
    .any(|phrase| title.contains(phrase))
}

fn normalized_title_key(title: &str) -> String {
    let normalized = title
        .to_ascii_lowercase()
        .replace("'s", " ")
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character.is_ascii_whitespace() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>();

    normalized
        .split_whitespace()
        .filter(|token| {
            !matches!(
                *token,
                "reuters"
                    | "bloomberg"
                    | "cnbc"
                    | "cnn"
                    | "wsj"
                    | "ft"
                    | "marketwatch"
                    | "yahoo"
                    | "finance"
                    | "investing"
                    | "seeking"
                    | "alpha"
                    | "barrons"
                    | "sec"
                    | "federalreserve"
                    | "ecb"
                    | "prnewswire"
                    | "globenewswire"
                    | "businesswire"
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_whitelisted_item(item: &NewsItem) -> bool {
    let source = normalize_source(&item.source);
    if allowed_source_name(source.as_str()) {
        return true;
    }

    if domain_matches_whitelist(
        item.source_url
            .as_deref()
            .and_then(source_domain)
            .as_deref(),
    ) {
        return true;
    }

    domain_matches_whitelist(
        Url::parse(&item.url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .as_deref(),
    )
}

fn allowed_source_name(source: &str) -> bool {
    matches!(
        source,
        "Reuters"
            | "Bloomberg"
            | "CNBC"
            | "CNN"
            | "Wall Street Journal"
            | "Financial Times"
            | "Yahoo Finance"
            | "Investing.com"
            | "Seeking Alpha"
            | "Barron's"
            | "SEC"
            | "Federal Reserve"
            | "ECB"
            | "PR Newswire"
            | "GlobeNewswire"
            | "Business Wire"
    )
}

fn domain_matches_whitelist(domain: Option<&str>) -> bool {
    let Some(domain) = domain else {
        return false;
    };
    let domain = domain.to_ascii_lowercase();
    [
        "reuters.com",
        "bloomberg.com",
        "cnbc.com",
        "cnn.com",
        "wsj.com",
        "ft.com",
        "finance.yahoo.com",
        "investing.com",
        "seekingalpha.com",
        "barrons.com",
        "sec.gov",
        "federalreserve.gov",
        "ecb.europa.eu",
        "prnewswire.com",
        "globenewswire.com",
        "businesswire.com",
    ]
    .iter()
    .any(|allowed| domain == *allowed || domain.ends_with(&format!(".{allowed}")))
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

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(Value::String(value)) => Some(value),
        Some(Value::Number(value)) => Some(value.to_string()),
        Some(Value::Bool(value)) => Some(value.to_string()),
        Some(Value::Null) | None => None,
        Some(other) => Some(other.to_string()),
    })
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
