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
    }
}

fn dark_theme() -> Theme {
    Theme {
        background: Some(Color::Rgb(0, 0, 0)),
        foreground: Color::Rgb(255, 255, 255),
        muted: Color::Rgb(120, 120, 120),
        accent: Color::Rgb(255, 255, 255),
    }
}

fn light_theme() -> Theme {
    Theme {
        background: Some(Color::Rgb(255, 255, 255)),
        foreground: Color::Rgb(0, 0, 0),
        muted: Color::Rgb(120, 120, 120),
        accent: Color::Rgb(80, 80, 80),
    }
}

fn transparent_theme() -> Theme {
    Theme {
        background: None,
        foreground: Color::Rgb(255, 255, 255),
        muted: Color::Rgb(120, 120, 120),
        accent: Color::Rgb(180, 180, 180),
    }
}
