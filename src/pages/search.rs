use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    symbols,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Cell, Chart, Dataset, GraphType, Paragraph, Row, Table, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{App, InputTarget, Page, SearchAssetKind},
    db,
    i18n::{Key, Locale},
    pages::fill::Fill,
    search::{HistoryPoint, LiveInstrumentDetails},
    theme::current_theme,
    ui,
};

pub fn render(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    let area = ui::content_area(frame.area(), app);
    if let Some(background) = theme.background {
        frame.render_widget(Fill::new(background), area);
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

    let input = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" query ", Style::default().fg(theme.muted)),
            Span::raw(" "),
            Span::styled(
                app.search_query.as_str(),
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            filter_span(
                app.t(Key::SearchFilterStocks),
                app.search_asset_kind == SearchAssetKind::Stocks,
                theme,
            ),
            Span::raw("  "),
            filter_span(
                app.t(Key::SearchFilterEtfs),
                app.search_asset_kind == SearchAssetKind::Etfs,
                theme,
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" search ")
            .border_style(Style::default().fg(search_border_color(
                app.is_text_input_target(InputTarget::Search),
                theme.muted,
                theme.accent,
            ))),
    );
    frame.render_widget(input, chunks[0]);
    if app.page == Page::Search && app.is_text_input_target(InputTarget::Search) {
        frame.set_cursor_position(Position::new(
            chunks[0]
                .x
                .saturating_add(9 + UnicodeWidthStr::width(app.search_query.as_str()) as u16),
            chunks[0].y.saturating_add(1),
        ));
    }

    if let Some(message) = &app.search_message {
        frame.render_widget(
            Paragraph::new(message.as_str())
                .style(Style::default().fg(theme.muted))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" results ")
                        .border_style(Style::default().fg(theme.muted)),
                ),
            chunks[1],
        );
    } else {
        render_results(frame, app, chunks[1]);
    }
}

pub fn render_details(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    let area = ui::content_area(frame.area(), app);
    let Some(details) = &app.selected_details else {
        return;
    };

    let background = theme
        .background
        .unwrap_or(ratatui::style::Color::Rgb(24, 24, 24));

    if let Some(background_fill) = theme.background {
        frame.render_widget(Fill::new(background_fill), area);
    }

    frame.render_widget(
        Block::default().style(Style::default().bg(background)),
        area,
    );
    let inner = area;

    if inner.width < 120 {
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)])
            .split(inner);
        frame.render_widget(detail_header(details, theme), root[0]);

        let body = root[1];
        let stacked = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Min(12),
                Constraint::Min(12),
            ])
            .split(body);

        frame.render_widget(price_panel(app, theme), stacked[0]);
        frame.render_widget(fundamentals_panel(details, app, theme), stacked[1]);
        render_chart_panel(frame, app, theme, stacked[2]);
        frame.render_widget(intel_panel(app, details.symbol.as_str(), theme), stacked[3]);
        frame.render_widget(bottom_meta(details, theme), root[2]);
    } else {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
            .split(inner);
        let chart_column = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(12), Constraint::Length(1)])
            .split(columns[0]);
        let sidebar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Min(12),
            ])
            .split(columns[1]);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    " PRICE CHART ",
                    Style::default()
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("3mo", Style::default().fg(theme.muted)),
            ])),
            chart_column[0],
        );
        render_chart_panel(frame, app, theme, chart_column[1]);
        frame.render_widget(
            detail_header(details, theme),
            sidebar[0],
        );
        frame.render_widget(price_panel(app, theme), sidebar[1]);
        frame.render_widget(fundamentals_panel(details, app, theme), sidebar[2]);
        frame.render_widget(intel_panel(app, details.symbol.as_str(), theme), sidebar[3]);
        frame.render_widget(bottom_meta(details, theme), chart_column[2]);
    }
}

fn detail_header(
    details: &crate::search::InstrumentDetails,
    theme: crate::theme::Theme,
) -> Paragraph<'_> {
    let exchange = details.exchange.as_deref().unwrap_or("-");
    let currency = details.currency.as_deref().unwrap_or("-");
    Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", details.symbol),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            details.name.clone(),
            Style::default().fg(theme.foreground),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{}   {}", exchange, currency),
            Style::default().fg(theme.muted),
        ),
    ]))
    .alignment(Alignment::Left)
    .style(Style::default().bg(theme.background.unwrap_or_default()))
}

fn price_panel(app: &App, theme: crate::theme::Theme) -> Paragraph<'_> {
    let mut lines = Vec::new();
    let symbol = app
        .selected_details
        .as_ref()
        .map(|details| details.symbol.as_str())
        .unwrap_or("-");
    if let Some(live) = &app.selected_live_details {
        let price = live
            .price
            .map(|value| format!("${}", format_price(value)))
            .unwrap_or_else(|| "-".to_string());
        let (change_text, change_color) = match price_change(live) {
            Some((absolute, percent)) => {
                let sign = if absolute >= 0.0 { "+" } else { "" };
                let color = if absolute >= 0.0 {
                    theme.positive
                } else {
                    theme.negative
                };
                (format!("{sign}{absolute:.2} ({sign}{percent:.2}%)"), color)
            }
            None => ("-".to_string(), theme.muted),
        };
        let rvol = relative_volume(live);
        lines.push(Line::from(vec![
            Span::styled(
                symbol,
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                price,
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                if change_text.starts_with('-') { "▼ " } else { "▲ " },
                Style::default().fg(change_color),
            ),
            Span::styled(change_text, Style::default().fg(change_color)),
        ]));
        if let Some(extended_price) = live.extended_price {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("After Hours", Style::default().fg(theme.muted)),
                Span::raw("  "),
                Span::styled(
                    format!("${}", format_price(extended_price)),
                    Style::default().fg(theme.foreground),
                ),
                Span::raw("  "),
                Span::styled(
                    live.extended_change_percent
                        .map(|value| format!("{value:+.2}%"))
                        .unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(match live.extended_change_percent {
                        Some(value) if value > 0.0 => theme.positive,
                        Some(value) if value < 0.0 => theme.negative,
                        _ => theme.muted,
                    }),
                ),
            ]));
        }
        lines.push(Line::from(""));

        let rows: Vec<(&str, Option<String>, Option<Color>)> = vec![
            ("Vol", live.day_volume.map(format_large_number), None),
            ("Avg Vol", live.avg_volume.map(format_large_number), None),
            (
                "RVOL",
                rvol.map(|value| format!("{value:.2}x")),
                rvol.map(|value| {
                    if value > 2.0 {
                        theme.positive
                    } else if value < 0.8 {
                        theme.negative
                    } else {
                        theme.foreground
                    }
                }),
            ),
            (
                app.t(Key::DetailsLabelPreviousClose),
                live.previous_close.map(|value| format!("${}", format_price(value))),
                None,
            ),
            (
                app.t(Key::DetailsLabelOpen),
                live.open.map(|value| format!("${}", format_price(value))),
                None,
            ),
            (
                "Day Range",
                Some(format!(
                    "${} - ${}",
                    live.day_low.map(format_price).unwrap_or_else(|| "-".to_string()),
                    live.day_high.map(format_price).unwrap_or_else(|| "-".to_string())
                )),
                None,
            ),
        ];
        let label_width = rows
            .iter()
            .map(|(label, _, _)| UnicodeWidthStr::width(*label))
            .max()
            .unwrap_or(10)
            .saturating_add(2);
        for (label, value, color) in rows {
            push_detail_row(
                &mut lines,
                label,
                value,
                label_width,
                color.unwrap_or(theme.foreground),
                theme.muted,
            );
        }
    } else if app.live_details_loading {
        lines.push(Line::from(Span::styled(
            symbol,
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "...",
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            symbol,
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsStatusLoading),
            Style::default().fg(theme.muted),
        )));
    }

    Paragraph::new(lines)
        .style(Style::default().fg(theme.foreground).bg(theme.background.unwrap_or_default()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" quote ")
                .border_style(Style::default().fg(theme.muted)),
        )
}

fn fundamentals_panel(
    details: &crate::search::InstrumentDetails,
    app: &App,
    theme: crate::theme::Theme,
) -> Paragraph<'static> {
    let mut lines = Vec::new();
    let rows = if let Some(live) = &app.selected_live_details {
        compact_fundamental_rows(details, live, app)
    } else if app.live_details_loading {
        compact_loading_rows(app)
    } else {
        compact_profile_rows(details, app)
    };
    let label_width = rows
        .iter()
        .map(|(label, _)| UnicodeWidthStr::width(*label))
        .max()
        .unwrap_or(10)
        .saturating_add(2);
    for (label, value) in rows {
        push_detail_row(
            &mut lines,
            label,
            value,
            label_width,
            theme.foreground,
            theme.muted,
        );
    }
    Paragraph::new(lines)
        .style(Style::default().fg(theme.foreground).bg(theme.background.unwrap_or_default()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" metrics ")
                .border_style(Style::default().fg(theme.muted)),
        )
}

fn render_chart_panel(frame: &mut Frame, app: &App, theme: crate::theme::Theme, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.background.unwrap_or_default()))
        .border_style(Style::default().fg(theme.muted));

    let history = app
        .selected_live_details
        .as_ref()
        .map(|live| normalize_history(&live.history))
        .unwrap_or_default();

    if history.len() < 2 {
        let message = if app.live_details_loading {
            app.t(Key::DetailsStatusLoading)
        } else {
            "No chart data available"
        };
        frame.render_widget(
            Paragraph::new(message)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.muted).bg(theme.background.unwrap_or_default()))
                .block(block),
            area,
        );
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);
    let chart_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(1)])
        .split(inner);

    let first = history.first().expect("history length checked");
    let last = history.last().expect("history length checked");
    let x_min = first.ts as f64;
    let x_max = last.ts as f64;
    let min = history.iter().map(|point| point.close).fold(f64::INFINITY, f64::min);
    let max = history
        .iter()
        .map(|point| point.close)
        .fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).abs();
    let y_padding = if range > 0.0 {
        (range * 0.06).max(0.5)
    } else {
        (min.abs() * 0.02).max(0.5)
    };
    let line_color = if last.close >= first.close {
        theme.positive
    } else {
        theme.negative
    };
    let marker = if chart_chunks[0].width < 60 {
        symbols::Marker::Dot
    } else {
        symbols::Marker::Braille
    };
    let points: Vec<(f64, f64)> = history
        .iter()
        .map(|point| (point.ts as f64, point.close))
        .collect();
    let dataset = Dataset::default()
        .marker(marker)
        .graph_type(GraphType::Line)
        .style(
            Style::default()
                .fg(line_color)
                .bg(theme.background.unwrap_or_default()),
        )
        .data(&points);

    let chart = Chart::new(vec![dataset])
        .style(Style::default().bg(theme.background.unwrap_or_default()))
        .x_axis(
            Axis::default()
                .style(Style::default().fg(theme.muted))
                .bounds([x_min, x_max])
                .labels(build_x_axis_labels(&history))
                .labels_alignment(Alignment::Center),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(theme.muted))
                .bounds([min - y_padding, max + y_padding])
                .labels(build_y_axis_labels(min, max))
                .labels_alignment(Alignment::Right),
        );

    frame.render_widget(chart, chart_chunks[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("low ${}", format_price(min)),
                Style::default().fg(theme.muted),
            ),
            Span::raw("  "),
            Span::styled(
                format_ts_label(first.ts),
                Style::default().fg(theme.muted),
            ),
            Span::raw("  "),
            Span::styled(
                format_ts_label(last.ts),
                Style::default().fg(theme.muted),
            ),
            Span::raw("  "),
            Span::styled(
                format!("high ${}", format_price(max)),
                Style::default().fg(theme.muted),
            ),
        ])),
        chart_chunks[1],
    );
}

fn intel_panel(app: &App, symbol: &str, theme: crate::theme::Theme) -> Paragraph<'static> {
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            " Summary ",
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("|", Style::default().fg(theme.muted)),
        Span::styled(" News ", Style::default().fg(theme.muted)),
        Span::styled("|", Style::default().fg(theme.muted)),
        Span::styled(" Notes ", Style::default().fg(theme.muted)),
    ]));
    lines.push(Line::from(""));
    lines.extend(structured_summary_lines(app));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("News", Style::default().fg(theme.muted))));
    lines.extend(detail_news_lines(app, symbol));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Notes", Style::default().fg(theme.muted))));
    lines.extend(detail_note_lines(app, symbol));

    Paragraph::new(lines)
        .style(Style::default().fg(theme.foreground).bg(theme.background.unwrap_or_default()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" intel ")
                .border_style(Style::default().fg(theme.muted)),
        )
        .wrap(Wrap { trim: false })
}

fn bottom_meta(
    details: &crate::search::InstrumentDetails,
    theme: crate::theme::Theme,
) -> Paragraph<'_> {
    Paragraph::new(Line::from(vec![
        Span::styled(
            details.exchange.as_deref().unwrap_or("-"),
            Style::default().fg(theme.muted),
        ),
        Span::raw("  "),
        Span::styled(
            details.currency.as_deref().unwrap_or("-"),
            Style::default().fg(theme.muted),
        ),
    ]))
}

fn render_results(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" results ")
        .border_style(Style::default().fg(search_border_color(
            !app.is_text_input_target(InputTarget::Search),
            theme.muted,
            theme.accent,
        )));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_rows = inner.height.saturating_sub(2).max(1) as usize;

    if app.search_results.is_empty() {
        frame.render_widget(
            Paragraph::new(app.t(Key::SearchEmpty)).style(Style::default().fg(theme.muted)),
            inner,
        );
        return;
    }

    let scroll = visible_scroll_start(app, visible_rows);
    let end = scroll
        .saturating_add(visible_rows)
        .min(app.search_results.len());
    let rows = app.search_results[scroll..end]
        .iter()
        .enumerate()
        .map(|(offset, result)| {
            let index = scroll + offset;
            let selected = index == app.search_selection;
            let style = if selected {
                Style::default()
                    .fg(theme.foreground)
                    .bg(theme.surface)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.foreground)
            };
            let marker = if selected { "▌" } else { " " };
            let sector = result.sector.as_deref().unwrap_or("-");
            let industry = result.industry.as_deref().unwrap_or("");
            let meta = if industry.is_empty() {
                sector.to_string()
            } else {
                format!("{sector} / {industry}")
            };

            Row::new(vec![
                Cell::from(Span::styled(marker, Style::default().fg(theme.accent))),
                Cell::from(Span::styled(
                    result.symbol.clone(),
                    Style::default()
                        .fg(theme.positive)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    result.name.clone(),
                    Style::default().fg(theme.foreground),
                )),
                Cell::from(Span::styled(meta, Style::default().fg(theme.muted))),
            ])
            .style(style)
        });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(search_symbol_column_width(app)),
            Constraint::Percentage(42),
            Constraint::Percentage(58),
        ],
    )
    .header(
        Row::new(vec![
            "",
            app.t(Key::SearchHeaderSymbol),
            app.t(Key::SearchHeaderName),
            app.t(Key::SearchHeaderSectorIndustry),
        ])
        .style(
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .column_spacing(1);
    frame.render_widget(table, chunks[0]);
    frame.render_widget(
        Paragraph::new(
            app.t(Key::SearchStatusLoaded)
                .replace("{start}", &(scroll + 1).to_string())
                .replace("{end}", &end.to_string())
                .replace("{total}", &app.search_results.len().to_string()),
        )
        .style(Style::default().fg(theme.muted)),
        chunks[1],
    );
}

fn search_border_color(active: bool, muted: Color, accent: Color) -> Color {
    if active { accent } else { muted }
}

fn filter_span(label: &str, active: bool, theme: crate::theme::Theme) -> Span<'static> {
    if active {
        Span::styled(
            format!(" {label} "),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
    } else {
        Span::styled(format!(" {label} "), Style::default().fg(theme.muted))
    }
}

fn push_detail_row(
    lines: &mut Vec<Line<'_>>,
    label: &str,
    value: Option<String>,
    label_width: usize,
    foreground: Color,
    muted: Color,
) {
    lines.push(Line::from(vec![
        Span::styled(pad_right(label, label_width), Style::default().fg(muted)),
        Span::styled(
            value.unwrap_or_else(|| "-".to_string()),
            Style::default().fg(foreground),
        ),
    ]));
}

fn structured_summary_lines(app: &App) -> Vec<Line<'static>> {
    let Some(live) = &app.selected_live_details else {
        return vec![Line::from(app.t(Key::DetailsStatusNoSummary).to_string())];
    };

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        "Business",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(
        live_summary(live, &app.locale)
            .unwrap_or_else(|| app.t(Key::DetailsStatusNoSummary).to_string()),
    ));

    if let Some(headquarters) = format_headquarters(live) {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Headquarters",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(headquarters));
    }

    if let Some(employees) = live.full_time_employees {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Employees",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(format_compact_number(employees)));
    }

    lines
}

fn detail_news_lines(app: &App, symbol: &str) -> Vec<Line<'static>> {
    let items = app
        .news_items
        .iter()
        .filter(|item| item.symbols.iter().any(|candidate| candidate == symbol))
        .take(3)
        .map(|item| Line::from(format!("• {}", item.title)))
        .collect::<Vec<_>>();
    if items.is_empty() {
        vec![Line::from("No related headlines loaded")]
    } else {
        items
    }
}

fn detail_note_lines(app: &App, symbol: &str) -> Vec<Line<'static>> {
    let Ok(connection) = db::open(&app.ticker_db_path) else {
        return vec![Line::from("Notes unavailable")];
    };
    let notes = db::notes_repo::list_all(&connection)
        .unwrap_or_default()
        .into_iter()
        .filter(|note| note.tickers.iter().any(|ticker| ticker == symbol))
        .take(2)
        .map(|note| {
            let preview = note.body.lines().next().unwrap_or("").trim();
            if preview.is_empty() {
                Line::from("• Empty note")
            } else {
                Line::from(format!("• {}", truncate_text(preview, 90)))
            }
        })
        .collect::<Vec<_>>();
    if notes.is_empty() {
        vec![Line::from("No notes for this symbol")]
    } else {
        notes
    }
}

fn compact_fundamental_rows<'a>(
    details: &'a crate::search::InstrumentDetails,
    live: &'a LiveInstrumentDetails,
    app: &'a App,
) -> Vec<(&'a str, Option<String>)> {
    vec![
        (
            app.t(Key::DetailsLabelMarketCap),
            live.market_cap.map(format_large_number),
        ),
        (
            app.t(Key::DetailsLabelAvgVolume),
            live.avg_volume.map(format_large_number),
        ),
        (app.t(Key::DetailsLabelPeRatio), live.trailing_pe.map(format_ratio)),
        (
            app.t(Key::DetailsLabelDividendYield),
            live.dividend_yield.map(format_percent),
        ),
        (app.t(Key::DetailsLabelBeta), live.beta.map(format_ratio)),
        (app.t(Key::DetailsLabelCountry), live.country.clone()),
        (app.t(Key::DetailsLabelSector), details.sector.clone()),
        (app.t(Key::DetailsLabelIndustry), details.industry.clone()),
        (app.t(Key::DetailsLabelExchange), details.exchange.clone()),
    ]
}

fn compact_loading_rows(app: &App) -> Vec<(&str, Option<String>)> {
    vec![
        (app.t(Key::DetailsLabelMarketCap), Some("...".to_string())),
        (app.t(Key::DetailsLabelAvgVolume), Some("...".to_string())),
        (app.t(Key::DetailsLabelPeRatio), Some("...".to_string())),
        (app.t(Key::DetailsLabelDividendYield), Some("...".to_string())),
        (app.t(Key::DetailsLabelBeta), Some("...".to_string())),
        (app.t(Key::DetailsLabelCountry), Some("...".to_string())),
        (app.t(Key::DetailsLabelSector), Some("...".to_string())),
        (app.t(Key::DetailsLabelIndustry), Some("...".to_string())),
        (app.t(Key::DetailsLabelExchange), Some("...".to_string())),
    ]
}

fn compact_profile_rows<'a>(
    details: &'a crate::search::InstrumentDetails,
    app: &'a App,
) -> Vec<(&'a str, Option<String>)> {
    vec![
        (app.t(Key::DetailsLabelSector), details.sector.clone()),
        (app.t(Key::DetailsLabelIndustry), details.industry.clone()),
        (app.t(Key::DetailsLabelCountry), None),
        (app.t(Key::DetailsLabelExchange), details.exchange.clone()),
        (app.t(Key::DetailsLabelCurrency), details.currency.clone()),
        (app.t(Key::DetailsLabelType), details.asset_type.clone()),
        (app.t(Key::DetailsLabelActive), Some(details.active.to_string())),
        (app.t(Key::DetailsLabelUpdated), details.last_updated.clone()),
    ]
}

fn normalize_history(history: &[HistoryPoint]) -> Vec<HistoryPoint> {
    let mut normalized = history.to_vec();
    normalized.sort_by_key(|point| point.ts);
    normalized.dedup_by(|left, right| left.ts == right.ts);
    normalized
}

fn price_change(live: &LiveInstrumentDetails) -> Option<(f64, f64)> {
    let price = live.price?;
    let previous = live.previous_close?;
    if previous == 0.0 {
        return None;
    }
    let absolute = price - previous;
    Some((absolute, absolute / previous * 100.0))
}

fn live_summary(live: &LiveInstrumentDetails, locale: &Locale) -> Option<String> {
    match locale {
        Locale::De => live.summary_de.clone().or_else(|| live.summary.clone()),
        Locale::En => live.summary.clone(),
        Locale::Other(_) => live.summary.clone(),
    }
}

fn relative_volume(live: &LiveInstrumentDetails) -> Option<f64> {
    let day = live.day_volume?;
    let avg = live.avg_volume?;
    if avg == 0.0 {
        None
    } else {
        Some(day / avg)
    }
}

fn format_price(value: f64) -> String {
    format!("{value:.2}")
}

fn format_ratio(value: f64) -> String {
    format!("{value:.2}")
}

fn format_percent(value: f64) -> String {
    format!("{value:.2}%")
}

fn format_headquarters(live: &LiveInstrumentDetails) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(city) = &live.city {
        if !city.is_empty() {
            parts.push(city.clone());
        }
    }
    if let Some(state) = &live.state {
        if !state.is_empty() {
            parts.push(state.clone());
        }
    }
    if let Some(country) = &live.country {
        if !country.is_empty() {
            parts.push(country.clone());
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn format_axis_price(value: f64, range: f64, precision: usize) -> String {
    if range >= 100.0 && precision == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.precision$}")
    }
}

fn build_y_axis_labels(min: f64, max: f64) -> Vec<Line<'static>> {
    let range = (max - min).abs();
    let values = [
        min,
        min + range * 0.25,
        min + range * 0.5,
        min + range * 0.75,
        max,
    ];
    let mut precision = if range >= 100.0 {
        0
    } else if range >= 10.0 {
        1
    } else {
        2
    };
    let labels = loop {
        let rendered: Vec<String> = values
            .iter()
            .map(|value| format_axis_price(*value, range, precision))
            .collect();
        let mut deduped = rendered.clone();
        deduped.sort();
        deduped.dedup();
        if deduped.len() == rendered.len() || precision >= 4 {
            break rendered;
        }
        precision += 1;
    };
    labels.into_iter().map(Line::from).collect()
}

fn build_x_axis_labels(history: &[HistoryPoint]) -> Vec<Line<'static>> {
    if history.is_empty() {
        return vec![Line::from("-"), Line::from("-"), Line::from("-")];
    }
    let mid = history.len() / 2;
    vec![
        Line::from(format_ts_label(history[0].ts)),
        Line::from(format_ts_label(history[mid].ts)),
        Line::from(format_ts_label(history[history.len() - 1].ts)),
    ]
}

fn format_ts_label(ts: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
        .map(|datetime| {
            datetime
                .with_timezone(&chrono::Local)
                .format("%b %-d")
                .to_string()
        })
        .unwrap_or_else(|| "-".to_string())
}

fn search_symbol_column_width(app: &App) -> u16 {
    app.i18n
        .width(Key::SearchHeaderSymbol)
        .max(8)
        .saturating_add(1) as u16
}

fn pad_right(value: &str, width: usize) -> String {
    let used = UnicodeWidthStr::width(value);
    format!("{value}{}", " ".repeat(width.saturating_sub(used)))
}

fn format_large_number(value: f64) -> String {
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

fn format_compact_number(value: f64) -> String {
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

fn truncate_text(value: &str, max_chars: usize) -> String {
    let truncated: String = value.chars().take(max_chars).collect();
    if value.chars().count() > max_chars {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn visible_scroll_start(app: &App, visible_rows: usize) -> usize {
    if visible_rows == 0 {
        return app.search_selection.min(app.search_results.len());
    }

    let mut scroll = app.search_scroll.min(app.search_results.len());
    if app.search_selection < scroll {
        scroll = app.search_selection;
    } else if app.search_selection >= scroll + visible_rows {
        scroll = app.search_selection + 1 - visible_rows;
    }
    scroll
}
