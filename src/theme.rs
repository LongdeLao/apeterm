use ratatui::style::Color;

use crate::app::ThemeName;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub background: Option<Color>,
    pub foreground: Color,
    pub muted: Color,
    pub accent: Color,
    pub positive: Color,
    pub negative: Color,
    pub warning: Color,
    pub surface: Color,
    pub relevant_tint: Color,
    pub macro_tint: Color,
    pub crypto_tint: Color,
}

pub fn current_theme(theme_name: ThemeName) -> Theme {
    match theme_name {
        ThemeName::Dark => dark_theme(),
        ThemeName::Light => light_theme(),
        ThemeName::Transparent => transparent_theme(),
        ThemeName::Bloomberg => bloomberg_theme(),
    }
}

fn dark_theme() -> Theme {
    Theme {
        background: Some(Color::Rgb(12, 12, 12)),
        foreground: Color::Rgb(232, 232, 232),
        muted: Color::Rgb(144, 144, 144),
        accent: Color::Rgb(208, 208, 208),
        positive: Color::Rgb(52, 211, 153),
        negative: Color::Rgb(248, 113, 113),
        warning: Color::Rgb(245, 158, 11),
        surface: Color::Rgb(24, 24, 24),
        relevant_tint: Color::Rgb(18, 32, 24),
        macro_tint: Color::Rgb(36, 30, 16),
        crypto_tint: Color::Rgb(20, 26, 36),
    }
}

fn light_theme() -> Theme {
    Theme {
        background: Some(Color::Rgb(244, 244, 240)),
        foreground: Color::Rgb(40, 40, 40),
        muted: Color::Rgb(118, 118, 112),
        accent: Color::Rgb(88, 88, 84),
        positive: Color::Rgb(18, 128, 86),
        negative: Color::Rgb(184, 28, 28),
        warning: Color::Rgb(168, 104, 0),
        surface: Color::Rgb(232, 231, 225),
        relevant_tint: Color::Rgb(220, 236, 228),
        macro_tint: Color::Rgb(240, 232, 214),
        crypto_tint: Color::Rgb(224, 232, 242),
    }
}

fn transparent_theme() -> Theme {
    Theme {
        background: None,
        foreground: Color::Rgb(242, 242, 242),
        muted: Color::Rgb(168, 168, 168),
        accent: Color::Rgb(214, 214, 214),
        positive: Color::Rgb(74, 222, 128),
        negative: Color::Rgb(248, 113, 113),
        warning: Color::Rgb(251, 191, 36),
        surface: Color::Reset,
        relevant_tint: Color::Reset,
        macro_tint: Color::Reset,
        crypto_tint: Color::Reset,
    }
}

fn bloomberg_theme() -> Theme {
    Theme {
        background: Some(Color::Rgb(8, 8, 8)),
        foreground: Color::Rgb(255, 168, 0),
        muted: Color::Rgb(154, 154, 154),
        accent: Color::Rgb(255, 102, 0),
        positive: Color::Rgb(0, 214, 143),
        negative: Color::Rgb(255, 84, 84),
        warning: Color::Rgb(255, 168, 0),
        surface: Color::Rgb(18, 18, 18),
        relevant_tint: Color::Rgb(10, 34, 24),
        macro_tint: Color::Rgb(40, 24, 8),
        crypto_tint: Color::Rgb(14, 22, 34),
    }
}
