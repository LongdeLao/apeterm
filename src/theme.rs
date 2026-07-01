use ratatui::style::Color;

use crate::app::ThemeName;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub background: Option<Color>,
    pub foreground: Color,
    pub muted: Color,
    pub accent: Color,
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
    }
}

fn light_theme() -> Theme {
    Theme {
        background: Some(Color::Rgb(244, 244, 240)),
        foreground: Color::Rgb(40, 40, 40),
        muted: Color::Rgb(118, 118, 112),
        accent: Color::Rgb(88, 88, 84),
    }
}

fn transparent_theme() -> Theme {
    Theme {
        background: None,
        foreground: Color::Rgb(242, 242, 242),
        muted: Color::Rgb(168, 168, 168),
        accent: Color::Rgb(214, 214, 214),
    }
}

fn bloomberg_theme() -> Theme {
    Theme {
        background: Some(Color::Rgb(8, 8, 8)),
        foreground: Color::Rgb(255, 168, 0),
        muted: Color::Rgb(154, 154, 154),
        accent: Color::Rgb(255, 102, 0),
    }
}
