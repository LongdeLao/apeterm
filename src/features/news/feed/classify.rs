//! Relevance matching and category/priority classification.

use super::*;

pub(super) fn enrich_item(
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

pub(super) fn matches_watchlist_item(
    headline: &str,
    description: &str,
    matcher: &WatchlistMatcher,
) -> bool {
    matcher.terms.iter().any(|term| {
        contains_symbol(headline, term)
            || contains_phrase(headline, term)
            || contains_phrase(description, term)
    })
}

pub(super) fn contains_phrase(text: &str, phrase: &str) -> bool {
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

pub(super) fn normalize_match_term(value: &str) -> String {
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

pub fn contains_symbol(text: &str, symbol: &str) -> bool {
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

pub(super) fn classify_category(
    headline: &str,
    relevant: bool,
    crypto_symbols: &[String],
) -> NewsCategory {
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

pub(super) fn is_crypto_headline(
    headline: &str,
    lowercase: &str,
    crypto_symbols: &[String],
) -> bool {
    lowercase.contains("crypto")
        || contains_symbol(headline, "BTC")
        || contains_symbol(headline, "ETH")
        || crypto_symbols
            .iter()
            .any(|symbol| contains_symbol(headline, symbol))
}

pub(super) fn is_macro_headline(lowercase: &str) -> bool {
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

pub(super) fn classify_priority(item: &NewsItem) -> NewsPriority {
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
