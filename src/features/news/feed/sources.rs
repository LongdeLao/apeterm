//! Source labeling: inference, normalization, whitelist, clickbait filters.

use super::*;

pub(super) fn infer_title_source(title: &str) -> (String, Option<String>) {
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

pub(super) fn split_title_suffix(title: &str) -> Option<(&str, &str)> {
    [" - ", " | ", " — ", " – "]
        .iter()
        .find_map(|separator| title.rsplit_once(separator))
}

pub(super) fn non_generic_source(source: &str) -> Option<String> {
    let trimmed = source.trim();
    if trimmed.is_empty() || is_generic_feed_label(trimmed) {
        None
    } else {
        Some(normalize_source(trimmed))
    }
}

pub(super) fn is_generic_feed_label(source: &str) -> bool {
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

pub(super) fn infer_source_from_description(description: &str) -> Option<String> {
    let candidate = description
        .rsplit("  ")
        .next()
        .or_else(|| description.rsplit(' ').next())
        .unwrap_or_default()
        .trim();
    let normalized = normalize_source(candidate);
    allowed_source_name(normalized.as_str()).then_some(normalized)
}

pub(super) fn normalize_source(source: &str) -> String {
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

pub(super) fn is_clickbait_source(source: &str) -> bool {
    normalize_source(source) == "Motley Fool"
}

pub(super) fn is_clickbait_title(title: &str) -> bool {
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

pub(super) fn is_whitelisted_item(item: &NewsItem) -> bool {
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

pub(super) fn allowed_source_name(source: &str) -> bool {
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

pub(super) fn domain_matches_whitelist(domain: Option<&str>) -> bool {
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
