//! Deduplication and ordering of news items.

use super::*;

pub(super) fn dedupe_exact(items: Vec<(NewsItem, usize)>) -> Vec<NewsItem> {
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

pub(super) fn dedupe_normalized_titles(items: Vec<NewsItem>) -> Vec<NewsItem> {
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

pub(super) fn sort_items(items: &mut [NewsItem]) {
    items.sort_by(|left, right| {
        right
            .published_at
            .cmp(&left.published_at)
            .then_with(|| left.feed_order.cmp(&right.feed_order))
            .then_with(|| left.item_order.cmp(&right.item_order))
    });
}

pub(super) fn dedupe_key(item: &NewsItem) -> String {
    if item.id.trim().is_empty() {
        item.url.clone()
    } else {
        item.id.clone()
    }
}

pub(super) fn normalized_title_key(title: &str) -> String {
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
