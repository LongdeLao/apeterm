//! Small rendering helpers shared across feature views.

use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::i18n::Locale;
use crate::theme::Theme;

/// A rect of `width` x `height` centered inside `area`, clamped to fit.
pub fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

/// A rect spanning `width_percent` of `area`'s width, centered, clamped to fit.
pub fn centered_rect_percent(area: Rect, width_percent: u16, height: u16) -> Rect {
    let width = (area.width.saturating_mul(width_percent) / 100).min(area.width);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height: height.min(area.height),
    }
}

/// Fill style for the theme's background color, if the theme defines one.
pub fn background_style(theme: Theme) -> Style {
    match theme.background {
        Some(background) => Style::default().bg(background),
        None => Style::default(),
    }
}

/// 1.2B / 3.4M / 5.6K style abbreviation for large magnitudes.
pub fn format_compact_number(value: f64) -> String {
    if value.abs() >= 1_000_000_000.0 {
        format!("{:.1}B", value / 1_000_000_000.0)
    } else if value.abs() >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}

/// Pad with trailing spaces to `width` terminal columns (unicode-width aware).
pub fn pad_right(value: &str, width: usize) -> String {
    let used = UnicodeWidthStr::width(value);
    format!("{value}{}", " ".repeat(width.saturating_sub(used)))
}

/// Localized display name for a locale, falling back to its code.
pub fn locale_label(app: &App, locale: &Locale) -> String {
    locale
        .language_key()
        .map(|key| app.t(key).to_string())
        .unwrap_or_else(|| locale.code().to_string())
}
