use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{App, InputTarget, PanelId, WatchlistEditMode, WatchlistEditRow, WatchlistKind},
    features::watchlist::market::MarketSession,
    features::watchlist::quotes::{PriceDirection, Quote},
    i18n::Key,
    theme::current_theme,
    ui::panel,
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
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    panel::render_title(
        frame,
        app,
        chunks[0],
        panel_id,
        app.t(Key::PanelTitleWatchlist),
    );
    render_watchlist_tabs(frame, app, chunks[1]);

    let notes_symbols = app.notes_ticker_symbols();
    let mut lines = Vec::new();
    push_watchlist_rows(
        &mut lines,
        app,
        &notes_symbols,
        theme.foreground,
        theme.muted,
    );
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    frame.render_widget(Paragraph::new(lines), chunks[2]);
    render_market_state(frame, app, chunks[3]);

    if watchlist_input_active(app) {
        render_watchlist_input(frame, app, inner);
    }
}

fn render_watchlist_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme.muted));
    frame.render_widget(block, area);

    let tab_count = app.watchlists().len();
    let line = Line::from(
        app.watchlists()
            .iter()
            .enumerate()
            .flat_map(|(index, watchlist)| {
                let selected = index == app.active_watchlist_index();
                let style = if selected {
                    Style::default()
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(theme.muted)
                };

                let mut spans = vec![Span::styled(
                    format!(" {} ", watchlist.name.to_uppercase()),
                    style,
                )];
                if index + 1 < tab_count {
                    spans.push(Span::raw("  "));
                }
                spans
            })
            .chain([
                Span::raw("  "),
                Span::styled("+", Style::default().fg(theme.muted)),
            ])
            .collect::<Vec<_>>(),
    );
    frame.render_widget(Paragraph::new(line), area);
}

fn push_watchlist_rows<'a>(
    lines: &mut Vec<Line<'a>>,
    app: &'a App,
    notes_symbols: &std::collections::HashSet<String>,
    foreground: Color,
    muted: Color,
) {
    let symbol_width = app
        .stock_watchlist()
        .iter()
        .chain(app.crypto_watchlist().iter())
        .map(|symbol| {
            let label = app.watchlist_display_name(symbol).unwrap_or(symbol);
            UnicodeWidthStr::width(label)
        })
        .max()
        .unwrap_or(SYMBOL_WIDTH)
        .max(SYMBOL_WIDTH);

    if app.watchlist.editor.is_some() {
        lines.push(edit_action_line(
            app,
            WatchlistEditRow::AddStock,
            app.t(Key::WatchlistEditAddStock),
        ));
        lines.push(edit_action_line(
            app,
            WatchlistEditRow::AddCrypto,
            app.t(Key::WatchlistEditAddCrypto),
        ));
        lines.push(Line::from(""));
    }

    for (index, symbol) in app.stock_watchlist().iter().enumerate() {
        let quote = app
            .watchlist
            .stock_quotes
            .iter()
            .find(|quote| quote.symbol == *symbol);
        lines.push(symbol_line(
            app,
            WatchlistEditRow::Stock(index),
            app.watchlist_display_name(symbol).unwrap_or(symbol),
            symbol,
            quote,
            notes_symbols,
            foreground,
            muted,
            symbol_width,
        ));
    }

    for (index, symbol) in app.crypto_watchlist().iter().enumerate() {
        let quote = app
            .watchlist
            .crypto_quotes
            .iter()
            .find(|quote| quote.symbol == *symbol);
        lines.push(symbol_line(
            app,
            WatchlistEditRow::Crypto(index),
            app.watchlist_display_name(symbol).unwrap_or(symbol),
            symbol,
            quote,
            notes_symbols,
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
            .fg(theme.background.unwrap_or(Color::Black))
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
    symbol: &'a str,
    quote: Option<&'a Quote>,
    notes_symbols: &std::collections::HashSet<String>,
    foreground: Color,
    muted: Color,
    symbol_width: usize,
) -> Line<'a> {
    let theme = current_theme(app.theme_name);
    let selected = app.selected_watchlist_row() == Some(row);
    let prefix = if app.watchlist.editor.is_some() {
        if selected { "> " } else { "  " }
    } else {
        ""
    };

    let base_style = if selected {
        Style::default()
            .fg(theme.background.unwrap_or(Color::Black))
            .bg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(foreground)
    };

    if let Some(quote) = quote {
        quote_line(
            app,
            prefix,
            quote,
            display_symbol,
            symbol,
            notes_symbols,
            base_style,
            selected,
            foreground,
            symbol_width,
        )
    } else {
        Line::from(vec![
            notes_indicator(app, symbol, notes_symbols),
            Span::styled(
                format_symbol_label(prefix, display_symbol, symbol, symbol_width),
                base_style,
            ),
            Span::styled(
                app.t(Key::WatchlistStatusLoadingQuote),
                Style::default().fg(muted),
            ),
        ])
    }
}

fn notes_indicator(
    app: &App,
    symbol: &str,
    notes_symbols: &std::collections::HashSet<String>,
) -> Span<'static> {
    if notes_symbols.contains(symbol) {
        let theme = current_theme(app.theme_name);
        Span::styled("● ", Style::default().fg(theme.accent))
    } else {
        Span::raw("  ")
    }
}

fn quote_line<'a>(
    app: &'a App,
    prefix: &str,
    quote: &'a Quote,
    display_symbol: &'a str,
    symbol: &'a str,
    notes_symbols: &std::collections::HashSet<String>,
    base_style: Style,
    selected: bool,
    foreground: Color,
    symbol_width: usize,
) -> Line<'a> {
    let theme = current_theme(app.theme_name);
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
    let percent_color = if quote.change_percent >= 0.0 {
        Color::Rgb(34, 197, 94)
    } else {
        Color::Rgb(239, 68, 68)
    };
    let percent_arrow = if quote.change_percent >= 0.0 {
        "▲"
    } else {
        "▼"
    };
    let symbol_style = if selected {
        base_style
    } else {
        Style::default().fg(foreground)
    };
    let mut spans = vec![
        notes_indicator(app, symbol, notes_symbols),
        Span::styled(
            format_symbol_label(prefix, display_symbol, symbol, symbol_width),
            symbol_style,
        ),
        Span::styled(format!("{arrow} "), style),
        Span::styled(format!("{:>12.2}  ", quote.price), style),
        Span::styled(
            format!("{percent_arrow} {:+.2}%", quote.change_percent),
            Style::default().fg(percent_color),
        ),
    ];

    if let Some(volume) = quote.day_volume {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("VOL ", Style::default().fg(foreground)));
        spans.push(Span::styled(
            format_compact_volume(volume),
            Style::default().fg(theme.muted),
        ));
    }

    if let Some(rvol) = quote.relative_volume {
        let rvol_color = if rvol > 2.0 {
            Color::Rgb(249, 115, 22)
        } else {
            theme.muted
        };
        spans.push(Span::raw("  "));
        spans.push(Span::styled("RV ", Style::default().fg(foreground)));
        spans.push(Span::styled(
            format!("{rvol:.1}x"),
            Style::default().fg(rvol_color),
        ));
    }

    Line::from(spans)
}

fn format_compact_volume(value: u64) -> String {
    let value = value as f64;
    if value >= 1_000_000_000.0 {
        format!("{:.1}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.0}K", value / 1_000.0)
    } else {
        format!("{}", value as u64)
    }
}

fn render_market_state(frame: &mut Frame, app: &App, area: Rect) {
    let (icon, label, color) = match app.watchlist.stock_market_session {
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
    let line = Line::from(vec![
        Span::styled(icon, Style::default().fg(color)),
        Span::styled(format!(" {label}"), Style::default().fg(color)),
    ]);
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Right), area);
}

fn format_symbol_label(
    prefix: &str,
    display_symbol: &str,
    symbol: &str,
    symbol_width: usize,
) -> String {
    let label = if display_symbol.eq(symbol) {
        display_symbol.to_string()
    } else {
        format!("{display_symbol} ({symbol})")
    };
    format!("{prefix}{label:<symbol_width$}")
}

fn watchlist_input_active(app: &App) -> bool {
    app.watchlist
        .editor
        .as_ref()
        .and_then(|editor| editor.mode.as_ref())
        .is_some()
}

fn render_watchlist_input(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let background = theme.background.unwrap_or(Color::Black);
    let Some(mode) = app
        .watchlist
        .editor
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
        WatchlistEditMode::CreateWatchlist { input } => ("New Watchlist", input.as_str()),
    };
    let suggestion_count = app.watchlist.suggestions.len().min(6) as u16;
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

    for (index, suggestion) in app.watchlist.suggestions.iter().take(6).enumerate() {
        let selected = index == app.watchlist.suggestion_selection;
        let style = if selected {
            Style::default()
                .fg(theme.background.unwrap_or(Color::Black))
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

    let focused = app.is_text_input_target(InputTarget::Watchlist);
    let panel = Paragraph::new(lines)
        .style(Style::default().bg(background))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Watchlist Input ")
                .border_style(Style::default().fg(if focused { theme.accent } else { theme.muted }))
                .style(Style::default().bg(background)),
        );

    frame.render_widget(Clear, modal);
    frame.render_widget(panel, modal);
    if focused {
        frame.set_cursor_position(Position::new(
            modal.x.saturating_add(
                1 + UnicodeWidthStr::width(label_text.as_str()) as u16
                    + UnicodeWidthStr::width(input) as u16,
            ),
            modal.y.saturating_add(1),
        ));
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}
