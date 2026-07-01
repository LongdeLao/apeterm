use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    app::{App, PanelId},
    pages::panel,
    quotes::{CryptoQuote, PriceDirection},
    theme::current_theme,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    if !app.is_panel_open(panel_id) {
        return;
    }

    let theme = current_theme(app.theme_name);
    let inner = panel::content_area(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    panel::render_title(frame, app, chunks[0], panel_id, "watchlist");

    let mut lines = vec![Line::from(Span::styled(
        "crypto",
        Style::default()
            .fg(theme.foreground)
            .add_modifier(Modifier::BOLD),
    ))];

    if app.crypto_quotes.is_empty() {
        lines.push(Line::from(Span::styled(
            "connecting to Binance...",
            Style::default().fg(theme.muted),
        )));
    } else {
        for quote in &app.crypto_quotes {
            lines.push(crypto_quote_line(quote, theme.foreground, theme.muted));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "stocks",
        Style::default()
            .fg(theme.foreground)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "coming later via yfinance",
        Style::default().fg(theme.muted),
    )));

    frame.render_widget(Paragraph::new(lines), chunks[1]);
}

fn crypto_quote_line(quote: &CryptoQuote, foreground: Color, muted: Color) -> Line<'static> {
    let is_flashing = quote
        .flash_until
        .is_some_and(|flash_until| flash_until > Instant::now());
    let color = match quote.direction {
        PriceDirection::Up => Color::Rgb(34, 197, 94),
        PriceDirection::Down => Color::Rgb(239, 68, 68),
        PriceDirection::Flat => foreground,
    };
    let mut style = Style::default().fg(color);

    if is_flashing {
        style = style.add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK);
    }

    let arrow = match quote.direction {
        PriceDirection::Up => "▲",
        PriceDirection::Down => "▼",
        PriceDirection::Flat => "•",
    };
    let percent_color = if quote.price_change_percent >= 0.0 {
        Color::Rgb(34, 197, 94)
    } else {
        Color::Rgb(239, 68, 68)
    };
    let percent_arrow = if quote.price_change_percent >= 0.0 {
        "▲"
    } else {
        "▼"
    };

    Line::from(vec![
        Span::styled(
            format!("{:<8}", quote.symbol),
            Style::default().fg(foreground),
        ),
        Span::styled(format!("{arrow} "), style),
        Span::styled(format!("{:>12.2}  ", quote.price), style),
        Span::styled(
            format!("{percent_arrow} {:+.2}%", quote.price_change_percent),
            Style::default().fg(percent_color),
        ),
        Span::styled("", Style::default().fg(muted)),
    ])
}
