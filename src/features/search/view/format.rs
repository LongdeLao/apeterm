//! Text/number formatting helpers for the search and detail views.

use super::*;

pub(super) fn inverse_foreground(theme: crate::theme::Theme) -> Color {
    theme.background.unwrap_or(theme.surface)
}

pub(super) fn text_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

pub(super) fn truncate_to_width(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if text_width(value) <= width {
        return value.to_string();
    }
    let suffix = "...";
    if width <= suffix.len() {
        return ".".repeat(width);
    }
    let target = width - suffix.len();
    let mut out = String::new();
    for character in value.chars() {
        let next = format!("{out}{character}");
        if text_width(&next) > target {
            break;
        }
        out.push(character);
    }
    if let Some(index) = out.trim_end().rfind(char::is_whitespace) {
        out.truncate(index);
    }
    out.push_str(suffix);
    out
}

pub(super) fn wrap_words(value: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in value.split_whitespace() {
        let word = truncate_to_width(word, width);
        if current.is_empty() {
            current = word;
            continue;
        }
        let candidate = format!("{current} {word}");
        if text_width(&candidate) <= width {
            current = candidate;
        } else {
            lines.push(current);
            current = word;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() && !value.trim().is_empty() {
        lines.push(truncate_to_width(value.trim(), width));
    }
    lines
}

pub(super) fn append_suffix_to_last_line(lines: &mut Vec<String>, suffix: &str, width: usize) {
    if lines.is_empty() {
        lines.push(truncate_to_width(suffix, width));
        return;
    }
    let suffix_width = text_width(suffix);
    if suffix_width >= width {
        if let Some(last) = lines.last_mut() {
            *last = truncate_to_width(suffix, width);
        }
        return;
    }
    if let Some(last) = lines.last_mut() {
        let base_width = width.saturating_sub(suffix_width + 1);
        let mut base = truncate_to_width(last.trim_end(), base_width);
        if suffix == "..." {
            base = base.trim_end_matches('.').trim_end().to_string();
            *last = format!("{base}{suffix}");
        } else {
            *last = format!("{base} {suffix}");
        }
    }
}

pub(super) fn price_change(live: &LiveInstrumentDetails) -> Option<(f64, f64)> {
    let price = live.price?;
    let previous = live.previous_close?;
    if previous == 0.0 {
        return None;
    }
    let absolute = price - previous;
    Some((absolute, absolute / previous * 100.0))
}

pub(super) fn live_summary(live: &LiveInstrumentDetails, locale: &Locale) -> Option<String> {
    match locale {
        Locale::De => live.summary_de.clone().or_else(|| live.summary.clone()),
        Locale::En => live.summary.clone(),
        Locale::Other(_) => live.summary.clone(),
    }
}

pub(super) fn relative_volume(live: &LiveInstrumentDetails) -> Option<f64> {
    let day = live.day_volume?;
    let avg = live.avg_volume?;
    if avg == 0.0 { None } else { Some(day / avg) }
}

pub(super) fn day_range(live: &LiveInstrumentDetails) -> Option<String> {
    Some(format!(
        "{} - {}",
        format_price(live.day_low?),
        format_price(live.day_high?)
    ))
}

pub(super) fn format_price(value: f64) -> String {
    format!("{value:.2}")
}

pub(super) fn format_ratio(value: f64) -> String {
    format!("{value:.2}")
}

pub(super) fn format_percent(value: f64) -> String {
    format!("{value:.2}%")
}

pub(super) fn format_headquarters(live: &LiveInstrumentDetails) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(city) = &live.city
        && !city.is_empty()
    {
        parts.push(city.clone());
    }
    if let Some(state) = &live.state
        && !state.is_empty()
    {
        parts.push(state.clone());
    }
    if let Some(country) = &live.country
        && !country.is_empty()
    {
        parts.push(country.clone());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

pub(super) fn format_large_number(value: f64) -> String {
    if value.abs() >= 1_000_000_000_000.0 {
        format!("{:.2}T", value / 1_000_000_000_000.0)
    } else if value.abs() >= 1_000_000_000.0 {
        format!("{:.2}B", value / 1_000_000_000.0)
    } else if value.abs() >= 1_000_000.0 {
        format!("{:.2}M", value / 1_000_000.0)
    } else {
        format!("{value:.0}")
    }
}
