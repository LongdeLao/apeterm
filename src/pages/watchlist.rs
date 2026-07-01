use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{App, PanelId},
    i18n::Key,
    market::MarketSession,
    pages::panel,
    quotes::{PriceDirection, Quote},
    theme::current_theme,
};

const SYMBOL_WIDTH: usize = 8;

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

    panel::render_title(
        frame,
        app,
        chunks[0],
        panel_id,
        app.t(Key::PanelTitleWatchlist),
    );

    let symbol_width = SYMBOL_WIDTH.max(app.i18n.width(Key::WatchlistSectionStocks));

    let mut lines = vec![Line::from(Span::styled(
        app.t(Key::WatchlistSectionCrypto),
        Style::default()
            .fg(theme.foreground)
            .add_modifier(Modifier::BOLD),
    ))];

    if app.crypto_quotes.is_empty() {
        lines.push(Line::from(Span::styled(
            app.t(Key::WatchlistStatusConnectingBinance),
            Style::default().fg(theme.muted),
        )));
    } else {
        for quote in &app.crypto_quotes {
            lines.push(quote_line(quote, theme.foreground, symbol_width));
        }
    }

    lines.push(Line::from(""));
    lines.push(stocks_title(
        app,
        app.stock_market_session,
        theme.foreground,
        symbol_width,
    ));
    if app.stock_quotes.is_empty() {
        lines.push(Line::from(Span::styled(
            app.t(Key::WatchlistStatusLiveQuotesPending),
            Style::default().fg(theme.muted),
        )));
    } else {
        for quote in &app.stock_quotes {
            lines.push(quote_line(quote, theme.foreground, symbol_width));
        }
    }

    frame.render_widget(Paragraph::new(lines), chunks[1]);
}

fn stocks_title(
    app: &App,
    session: MarketSession,
    foreground: Color,
    symbol_width: usize,
) -> Line<'_> {
    let (icon, label, color) = match session {
        MarketSession::PreMarket => (
            "\u{F05A8}",
            app.t(Key::WatchlistMarketPreMarket),
            Color::Rgb(249, 115, 22),
        ),
        MarketSession::Regular => (
            "\u{F19C}",
            app.t(Key::WatchlistMarketRegular),
            Color::Rgb(156, 163, 175),
        ),
        MarketSession::AfterHours => (
            "\u{F4EE}",
            app.t(Key::WatchlistMarketAfterHours),
            Color::Rgb(168, 85, 247),
        ),
    };

    Line::from(vec![
        Span::styled(
            pad_right(app.t(Key::WatchlistSectionStocks), symbol_width),
            Style::default().fg(foreground).add_modifier(Modifier::BOLD),
        ),
        Span::styled(icon, Style::default().fg(color)),
        Span::styled(format!(" {label}"), Style::default().fg(color)),
    ])
}

fn pad_right(value: &str, width: usize) -> String {
    let used = UnicodeWidthStr::width(value);
    format!("{value}{}", " ".repeat(width.saturating_sub(used)))
}

fn quote_line(quote: &Quote, foreground: Color, symbol_width: usize) -> Line<'static> {
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
            format!("{:<symbol_width$}", quote.symbol),
            Style::default().fg(foreground),
        ),
        Span::styled(format!("{arrow} "), style),
        Span::styled(format!("{:>12.2}  ", quote.price), style),
        Span::styled(
            format!("{percent_arrow} {:+.2}%", quote.price_change_percent),
            Style::default().fg(percent_color),
        ),
    ])
}
