use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{App, PanelId, WatchlistEditMode, WatchlistEditRow, WatchlistKind},
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

    let mut lines = Vec::new();
    lines.push(stocks_title(
        app,
        app.stock_market_session,
        theme.foreground,
        symbol_width,
    ));
    push_watchlist_rows(
        &mut lines,
        app,
        WatchlistKind::Stock,
        symbol_width,
        theme.foreground,
        theme.muted,
    );

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        app.t(Key::WatchlistSectionCrypto),
        Style::default()
            .fg(theme.foreground)
            .add_modifier(Modifier::BOLD),
    )));
    push_watchlist_rows(
        &mut lines,
        app,
        WatchlistKind::Crypto,
        symbol_width,
        theme.foreground,
        theme.muted,
    );
    frame.render_widget(Paragraph::new(lines), chunks[1]);

    if watchlist_input_active(app) {
        render_watchlist_input(frame, app, inner);
    }
}

fn stocks_title(
    app: &App,
    session: Option<MarketSession>,
    foreground: Color,
    symbol_width: usize,
) -> Line<'_> {
    let (icon, label, color) = match session {
        Some(MarketSession::PreMarket) => (
            "\u{F05A8}",
            app.t(Key::WatchlistMarketPreMarket),
            Color::Rgb(249, 115, 22),
        ),
        Some(MarketSession::Regular) => (
            "\u{F19C}",
            app.t(Key::WatchlistMarketRegular),
            Color::Rgb(156, 163, 175),
        ),
        Some(MarketSession::AfterHours) => (
            "\u{F4EE}",
            app.t(Key::WatchlistMarketAfterHours),
            Color::Rgb(168, 85, 247),
        ),
        None => (
            "\u{F110}",
            app.t(Key::WatchlistMarketPending),
            Color::Rgb(156, 163, 175),
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

fn push_watchlist_rows<'a>(
    lines: &mut Vec<Line<'a>>,
    app: &'a App,
    kind: WatchlistKind,
    symbol_width: usize,
    foreground: Color,
    muted: Color,
) {
    if app.watchlist_editor.is_some() {
        let add_row = match kind {
            WatchlistKind::Stock => WatchlistEditRow::AddStock,
            WatchlistKind::Crypto => WatchlistEditRow::AddCrypto,
        };
        let label = match kind {
            WatchlistKind::Stock => app.t(Key::WatchlistEditAddStock),
            WatchlistKind::Crypto => app.t(Key::WatchlistEditAddCrypto),
        };
        lines.push(edit_action_line(app, add_row, label));
    }

    let symbols = match kind {
        WatchlistKind::Stock => app.stock_watchlist(),
        WatchlistKind::Crypto => app.crypto_watchlist(),
    };
    let quotes = match kind {
        WatchlistKind::Stock => &app.stock_quotes,
        WatchlistKind::Crypto => &app.crypto_quotes,
    };

    for (index, symbol) in symbols.iter().enumerate() {
        let row = match kind {
            WatchlistKind::Stock => WatchlistEditRow::Stock(index),
            WatchlistKind::Crypto => WatchlistEditRow::Crypto(index),
        };
        let quote = quotes.iter().find(|quote| quote.symbol == *symbol);
        lines.push(symbol_line(
            app,
            row,
            app.watchlist_display_name(symbol).unwrap_or(symbol),
            quote,
            foreground,
            muted,
            symbol_width,
        ));
    }
}

fn edit_action_line<'a>(app: &'a App, row: WatchlistEditRow, label: &'a str) -> Line<'a> {
    let theme = current_theme(app.theme_name);
    let selected = app.selected_watchlist_row() == Some(row);
    let style = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };
    let marker = if selected { ">" } else { " " };
    Line::from(Span::styled(format!("{marker} {label}"), style))
}

fn symbol_line<'a>(
    app: &'a App,
    row: WatchlistEditRow,
    display_symbol: &'a str,
    quote: Option<&'a Quote>,
    foreground: Color,
    muted: Color,
    symbol_width: usize,
) -> Line<'a> {
    let theme = current_theme(app.theme_name);
    let selected = app.selected_watchlist_row() == Some(row);
    let prefix = if app.watchlist_editor.is_some() {
        if selected { "> " } else { "  " }
    } else {
        ""
    };

    let base_style = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(foreground)
    };

    if let Some(quote) = quote {
        quote_line(
            prefix,
            quote,
            display_symbol,
            base_style,
            selected,
            foreground,
            symbol_width,
        )
    } else {
        Line::from(vec![
            Span::styled(
                format!("{prefix}{display_symbol:<symbol_width$}"),
                base_style,
            ),
            Span::styled(
                app.t(Key::WatchlistStatusLoadingQuote),
                Style::default().fg(muted),
            ),
        ])
    }
}

fn pad_right(value: &str, width: usize) -> String {
    let used = UnicodeWidthStr::width(value);
    format!("{value}{}", " ".repeat(width.saturating_sub(used)))
}

fn quote_line<'a>(
    prefix: &str,
    quote: &'a Quote,
    display_symbol: &'a str,
    base_style: Style,
    selected: bool,
    foreground: Color,
    symbol_width: usize,
) -> Line<'a> {
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
    let symbol_style = if selected {
        base_style
    } else {
        Style::default().fg(foreground)
    };
    Line::from(vec![
        Span::styled(
            format!("{prefix}{:<symbol_width$}", display_symbol),
            symbol_style,
        ),
        Span::styled(format!("{arrow} "), style),
        Span::styled(format!("{:>12.2}  ", quote.price), style),
        Span::styled(
            format!("{percent_arrow} {:+.2}%", quote.price_change_percent),
            Style::default().fg(percent_color),
        ),
    ])
}

fn watchlist_input_active(app: &App) -> bool {
    app.watchlist_editor
        .as_ref()
        .and_then(|editor| editor.mode.as_ref())
        .is_some()
}

fn render_watchlist_input(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let background = theme.background.unwrap_or(Color::Black);
    let Some(mode) = app
        .watchlist_editor
        .as_ref()
        .and_then(|editor| editor.mode.as_ref())
    else {
        return;
    };
    let (label, input) = match mode {
        WatchlistEditMode::Add {
            kind: WatchlistKind::Stock,
            input,
        } => (app.t(Key::WatchlistEditInputStock), input.as_str()),
        WatchlistEditMode::Add {
            kind: WatchlistKind::Crypto,
            input,
        } => (app.t(Key::WatchlistEditInputCrypto), input.as_str()),
        WatchlistEditMode::EditAlias { input, .. } => {
            (app.t(Key::WatchlistEditInputRename), input.as_str())
        }
        WatchlistEditMode::ChangeTicker { input, .. } => {
            (app.t(Key::WatchlistEditInputTicker), input.as_str())
        }
    };
    let suggestion_count = app.watchlist_suggestions.len().min(6) as u16;
    let modal = centered_rect(area, 62, 3 + suggestion_count);
    let label_text = format!("{label}: ");
    let mut lines = vec![Line::from(vec![
        Span::styled(
            label_text.as_str(),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(input, Style::default().fg(theme.foreground)),
    ])];

    for (index, suggestion) in app.watchlist_suggestions.iter().take(6).enumerate() {
        let selected = index == app.watchlist_suggestion_selection;
        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.foreground)
        };
        let marker = if selected { ">" } else { " " };
        lines.push(Line::from(vec![
            Span::styled(format!("{marker} {:<8}", suggestion.symbol), style),
            Span::styled(format!(" {}", suggestion.name), style),
        ]));
    }

    let panel = Paragraph::new(lines)
        .style(Style::default().bg(background))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent))
                .style(Style::default().bg(background)),
        );

    frame.render_widget(Clear, modal);
    frame.render_widget(panel, modal);
    frame.set_cursor_position(Position::new(
        modal
            .x
            .saturating_add(1 + label_text.len() as u16 + input.len() as u16),
        modal.y.saturating_add(1),
    ));
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}
