use chrono::{DateTime, Utc};

use crate::{
    backend::BackendInsight,
    features::{
        news::feed::{NewsItem, NewsPriority},
        notes::repo::NoteRow,
        watchlist::quotes::Quote,
    },
};

use super::NOTE_LIMIT;

pub(super) fn format_news_items(items: &[NewsItem], indent: usize) -> Vec<String> {
    let prefix = " ".repeat(indent);
    if items.is_empty() {
        return vec![format!("{prefix}- no matching local news")];
    }

    items
        .iter()
        .map(|item| {
            let symbols = if item.symbols.is_empty() {
                String::new()
            } else {
                format!(" [{}]", item.symbols.join(","))
            };
            format!(
                "{prefix}- [{}]{} {} ({}, {})",
                priority_label(item.priority),
                symbols,
                trim_chars(&item.title, 140),
                item.source,
                item.published_at
                    .map(|date| date.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "undated".to_string())
            )
        })
        .collect()
}

pub(super) fn format_notes(notes: &[NoteRow], indent: usize) -> Vec<String> {
    let prefix = " ".repeat(indent);
    if notes.is_empty() {
        return vec![format!("{prefix}- no matching local notes")];
    }

    notes
        .iter()
        .take(NOTE_LIMIT)
        .map(|note| {
            format!(
                "{prefix}- {} · {}",
                timestamp_label(note.updated_at),
                trim_chars(note.body.lines().next().unwrap_or(""), 160)
            )
        })
        .collect()
}

pub(super) fn format_backend_insight(insight: &BackendInsight, indent: usize) -> Vec<String> {
    let prefix = " ".repeat(indent);
    let Some(explanation) = &insight.explanation else {
        return vec![format!("{prefix}backend insight: unavailable")];
    };

    let mut lines = vec![format!(
        "{prefix}backend insight: confidence {} · cache_hit {} · stale {}",
        explanation.confidence, explanation.cache_hit, explanation.stale_context
    )];
    if !explanation.summary.trim().is_empty() {
        lines.push(format!(
            "{prefix}summary: {}",
            trim_chars(&explanation.summary, 260)
        ));
    }
    for driver in explanation.key_drivers.iter().take(4) {
        lines.push(format!("{prefix}driver: {}", trim_chars(driver, 180)));
    }
    lines
}

pub(super) fn quote_rank<'a>(quotes: impl Iterator<Item = &'a Quote>) -> String {
    let rows = quotes
        .map(|quote| {
            format!(
                "{} {} {}",
                quote.symbol,
                percent(quote.change_percent),
                relative_volume_label(quote.relative_volume)
            )
        })
        .collect::<Vec<_>>();
    join_or_none(&rows)
}

pub(super) fn normalize_agent_symbol(symbol: &str) -> Result<String, String> {
    let symbol = symbol.trim().to_ascii_uppercase();
    if symbol.is_empty() {
        Err("symbol must not be empty".to_string())
    } else {
        Ok(symbol)
    }
}

pub(super) fn percent(value: f64) -> String {
    format!("{value:+.2}%")
}

pub(super) fn relative_volume_label(value: Option<f64>) -> String {
    value
        .map(|value| format!("rel vol {value:.2}x"))
        .unwrap_or_else(|| "rel vol n/a".to_string())
}

pub(super) fn priority_score(priority: NewsPriority) -> u8 {
    match priority {
        NewsPriority::Critical => 4,
        NewsPriority::High => 3,
        NewsPriority::Medium => 2,
        NewsPriority::Low => 1,
    }
}

pub(super) fn priority_label(priority: NewsPriority) -> &'static str {
    match priority {
        NewsPriority::Critical => "critical",
        NewsPriority::High => "high",
        NewsPriority::Medium => "medium",
        NewsPriority::Low => "low",
    }
}

pub(super) fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

pub(super) fn option_label(value: Option<&str>) -> &str {
    value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("n/a")
}

pub(super) fn money_opt(value: Option<f64>) -> String {
    value
        .map(|value| format!("${}", compact_number(value)))
        .unwrap_or_else(|| "n/a".to_string())
}

pub(super) fn number_opt(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "n/a".to_string())
}

pub(super) fn compact_number(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1_000_000_000_000.0 {
        format!("{:.2}T", value / 1_000_000_000_000.0)
    } else if abs >= 1_000_000_000.0 {
        format!("{:.2}B", value / 1_000_000_000.0)
    } else if abs >= 1_000_000.0 {
        format!("{:.2}M", value / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.2}K", value / 1_000.0)
    } else {
        format!("{value:.2}")
    }
}

pub(super) fn trim_chars(value: &str, max: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut trimmed = value
        .chars()
        .take(max.saturating_sub(3))
        .collect::<String>();
    trimmed.push_str("...");
    trimmed
}

pub(super) fn timestamp_label(ts: i64) -> String {
    if ts <= 0 {
        return "undated".to_string();
    }
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "undated".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_number_formats_large_values() {
        assert_eq!(compact_number(1_500_000.0), "1.50M");
        assert_eq!(compact_number(2_000_000_000.0), "2.00B");
    }

    #[test]
    fn trim_chars_preserves_short_text() {
        assert_eq!(trim_chars("short", 10), "short");
    }

    #[test]
    fn trim_chars_shortens_long_text() {
        assert_eq!(trim_chars("abcdef", 4), "a...");
    }

    #[test]
    fn normalizes_agent_symbols() {
        assert_eq!(normalize_agent_symbol(" nvda ").unwrap(), "NVDA");
        assert!(normalize_agent_symbol(" ").is_err());
    }
}
