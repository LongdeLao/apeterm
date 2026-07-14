//! News providers: RSS, FinancialJuice, Nasdaq — HTTP fetch and parsing.

use super::*;

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

pub(super) fn build_client() -> Result<Client, String> {
    Client::builder()
        .user_agent("ApeTerm/0.1")
        .build()
        .map_err(|error| error.to_string())
}

pub(super) fn get_bytes(client: &Client, url: &str) -> Result<Vec<u8>, String> {
    client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|error| error.to_string())
}

pub(super) fn parse_rss_item(
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

pub(super) fn extract_rss_image_url(item: &rss::Item) -> Option<String> {
    item.enclosure()
        .map(|enclosure| enclosure.url().trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc2822(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .or_else(|_| {
            DateTime::parse_from_rfc3339(value).map(|timestamp| timestamp.with_timezone(&Utc))
        })
        .ok()
}

pub(super) fn strip_html(value: &str) -> String {
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

pub(super) fn is_supported_nasdaq_symbol(symbol: &str) -> bool {
    !(symbol.starts_with('^')
        || symbol.contains('.')
        || symbol.contains('-')
        || symbol.contains('='))
}

pub(super) fn parse_nasdaq_response(
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

pub(super) fn parse_nasdaq_row(
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

pub(super) fn deserialize_optional_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
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
