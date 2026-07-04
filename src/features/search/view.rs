use chrono::{Datelike, Local, TimeZone};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Cell, Paragraph, Row, Table,
        canvas::{Canvas, Line as CanvasLine},
    },
};
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{App, DetailTimeframe, InputTarget, Page, SearchAssetKind},
    backend::InsightArticle,
    db,
    features::search::engine::{HistoryPoint, LiveInstrumentDetails},
    i18n::{Key, Locale},
    metrics::{
        MetricId, metric_explanation_key, metric_label_key, visible_key_stats, visible_metrics,
    },
    preferences::ExplanationLevel,
    theme::current_theme,
    ui,
    ui::fill::Fill,
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
            Constraint::Length(4),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

    let input = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", app.t(Key::CopySearchPlaceholder)),
                Style::default().fg(theme.muted),
            ),
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
    .style(background_style(theme));
    frame.render_widget(input, chunks[0]);
    if app.page == Page::Search && app.is_text_input_target(InputTarget::Search) {
        let prompt_width =
            UnicodeWidthStr::width(format!(" {}  ", app.t(Key::CopySearchPlaceholder)).as_str());
        frame.set_cursor_position(Position::new(
            chunks[0].x.saturating_add(
                prompt_width as u16 + UnicodeWidthStr::width(app.search_query.as_str()) as u16,
            ),
            chunks[0].y,
        ));
    }

    if let Some(message) = &app.search_message {
        frame.render_widget(
            Paragraph::new(message.as_str()).style(background_style(theme).fg(theme.muted)),
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

    if let Some(background_fill) = theme.background {
        frame.render_widget(Fill::new(background_fill), area);
    }

    let page = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    let inner = page[0];

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(68),
            Constraint::Length(2),
            Constraint::Percentage(32),
        ])
        .split(inner);
    let chart_column = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(columns[0]);

    render_chart_header(frame, app, theme, chart_column[0]);
    render_chart_panel(frame, app, theme, chart_column[1]);
    render_vertical_divider(frame, theme, columns[1]);
    render_detail_sidebar(frame, app, details, theme, columns[2]);
    render_detail_footer(frame, app, theme, page[1]);
}

fn render_detail_footer(frame: &mut Frame, app: &App, theme: crate::theme::Theme, area: Rect) {
    let key = if area.width < 100 {
        Key::DetailsFooterCompact
    } else {
        Key::DetailsFooter
    };
    frame.render_widget(
        Paragraph::new(app.t(key))
            .style(background_style(theme).fg(theme.muted))
            .alignment(Alignment::Center),
        area,
    );
}

fn render_vertical_divider(frame: &mut Frame, theme: crate::theme::Theme, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let x = area.x + area.width / 2;
    let divider = Rect::new(x, area.y, 1, area.height);
    let lines = (0..area.height)
        .map(|_| Line::from("│"))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(theme.border_dim)),
        divider,
    );
}

fn render_chart_header(frame: &mut Frame, app: &App, theme: crate::theme::Theme, area: Rect) {
    let history = visible_history(app);
    frame.render_widget(
        Paragraph::new(chart_header_line(app, theme, &history, area.width))
            .style(background_style(theme)),
        area,
    );
}

fn render_chart_panel(frame: &mut Frame, app: &App, theme: crate::theme::Theme, area: Rect) {
    if area.width < 12 || area.height < 5 {
        return;
    }

    let history = visible_history(app);

    if history.len() < 2 {
        let message = if app.live_details_loading {
            app.t(Key::DetailsStatusLoading)
        } else {
            app.t(Key::DetailsChartNoData)
        };
        frame.render_widget(
            Paragraph::new(message)
                .alignment(Alignment::Center)
                .style(background_style(theme).fg(theme.muted)),
            area,
        );
        return;
    }

    let first = history.first().expect("history length checked");
    let last = history.last().expect("history length checked");
    let close_min = history
        .iter()
        .map(|point| point.close)
        .fold(f64::INFINITY, f64::min);
    let close_max = history
        .iter()
        .map(|point| point.close)
        .fold(f64::NEG_INFINITY, f64::max);
    let current_price = app
        .selected_live_details
        .as_ref()
        .and_then(|live| live.price)
        .unwrap_or(last.close);
    let reference = app
        .selected_live_details
        .as_ref()
        .and_then(|live| live.previous_close);
    let tick_min_input = reference
        .map_or(close_min, |value| close_min.min(value))
        .min(current_price);
    let tick_max_input = reference
        .map_or(close_max, |value| close_max.max(value))
        .max(current_price);
    let ticks = nice_y_ticks(tick_min_input, tick_max_input, 7);
    let y_min = *ticks.first().unwrap_or(&tick_min_input);
    let y_max = *ticks.last().unwrap_or(&tick_max_input);
    let line_color = if last.close >= first.close {
        theme.positive
    } else {
        theme.negative
    };
    let fill_color = if last.close >= first.close {
        theme.positive_dim
    } else {
        theme.negative_dim
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(1)])
        .split(area);
    let y_label_width = if area.width < 60 { 6 } else { 8 };
    let price_areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(y_label_width), Constraint::Min(4)])
        .split(chunks[0]);
    let x_axis_areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(y_label_width), Constraint::Min(4)])
        .split(chunks[1]);

    draw_price_grid(
        frame.buffer_mut(),
        price_areas[0],
        price_areas[1],
        theme,
        &ticks,
        y_min,
        y_max,
    );
    draw_month_grid(frame.buffer_mut(), price_areas[1], theme, &history);

    let x_min = first.ts as f64;
    let x_max = last.ts as f64;
    let canvas = Canvas::default()
        .marker(symbols::Marker::Braille)
        .x_bounds([x_min, x_max])
        .y_bounds([y_min, y_max])
        .paint(|ctx| {
            for point in &history {
                ctx.draw(&CanvasLine::new(
                    point.ts as f64,
                    y_min,
                    point.ts as f64,
                    point.close,
                    fill_color,
                ));
            }
            ctx.layer();
            for pair in history.windows(2) {
                ctx.draw(&CanvasLine::new(
                    pair[0].ts as f64,
                    pair[0].close,
                    pair[1].ts as f64,
                    pair[1].close,
                    line_color,
                ));
            }
        });
    frame.render_widget(canvas, price_areas[1]);

    draw_previous_close(
        frame.buffer_mut(),
        app,
        theme,
        price_areas[1],
        reference,
        y_min,
        y_max,
    );
    draw_current_price_marker(
        frame.buffer_mut(),
        theme,
        price_areas[1],
        current_price,
        y_min,
        y_max,
        line_color,
    );
    draw_x_axis(frame.buffer_mut(), app, theme, x_axis_areas[1], &history);
}

fn chart_header_line(
    app: &App,
    theme: crate::theme::Theme,
    history: &[HistoryPoint],
    width: u16,
) -> Line<'static> {
    let mut spans = Vec::new();
    let mut used = 0usize;
    push_span_with_width(
        &mut spans,
        &mut used,
        format!(" {}   ", app.t(Key::DetailsChartTitle)),
        Style::default()
            .fg(theme.foreground)
            .add_modifier(Modifier::BOLD),
    );

    for (index, timeframe) in DetailTimeframe::ALL.iter().enumerate() {
        if index > 0 {
            push_span_with_width(&mut spans, &mut used, "  ", Style::default());
        }
        let label = app.t(timeframe.label_key());
        if *timeframe == app.detail_timeframe {
            push_span_with_width(
                &mut spans,
                &mut used,
                format!("[{label}]"),
                Style::default()
                    .fg(inverse_foreground(theme))
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            );
        } else {
            push_span_with_width(
                &mut spans,
                &mut used,
                label.to_string(),
                Style::default().fg(theme.muted),
            );
        }
    }

    let summary = chart_summary_spans(app, theme, history);
    let summary_width = summary
        .iter()
        .map(|(text, _)| text_width(text))
        .sum::<usize>();
    let available = width as usize;
    if !summary.is_empty() && used.saturating_add(summary_width).saturating_add(2) < available {
        let gap = available.saturating_sub(used + summary_width);
        push_span_with_width(&mut spans, &mut used, " ".repeat(gap), Style::default());
        for (text, style) in summary {
            push_span_with_width(&mut spans, &mut used, text, style);
        }
    }

    Line::from(spans)
}

fn chart_summary_spans(
    app: &App,
    theme: crate::theme::Theme,
    history: &[HistoryPoint],
) -> Vec<(String, Style)> {
    let (Some(first), Some(last)) = (history.first(), history.last()) else {
        return Vec::new();
    };
    if first.close == 0.0 {
        return Vec::new();
    }
    let high = history
        .iter()
        .map(|point| point.close)
        .fold(f64::NEG_INFINITY, f64::max);
    let low = history
        .iter()
        .map(|point| point.close)
        .fold(f64::INFINITY, f64::min);
    let period_return = (last.close - first.close) / first.close * 100.0;
    let sign_color = if period_return >= 0.0 {
        theme.positive
    } else {
        theme.negative
    };
    vec![
        (
            format!("{}: ", app.t(app.detail_timeframe.label_key())),
            Style::default().fg(theme.muted),
        ),
        (
            format!("{period_return:+.1}%"),
            Style::default().fg(sign_color).add_modifier(Modifier::BOLD),
        ),
        (
            format!(
                "   {} {}  {} {}",
                app.t(Key::DetailsChartHigh),
                format_price(high),
                app.t(Key::DetailsChartLow),
                format_price(low),
            ),
            Style::default().fg(theme.muted),
        ),
    ]
}

fn push_span_with_width(
    spans: &mut Vec<Span<'static>>,
    used: &mut usize,
    text: impl Into<String>,
    style: Style,
) {
    let text = text.into();
    *used += text_width(&text);
    spans.push(Span::styled(text, style));
}

fn visible_history(app: &App) -> Vec<HistoryPoint> {
    let all = app
        .selected_live_details
        .as_ref()
        .map(|live| normalize_history(&live.history))
        .unwrap_or_default();
    let Some(days) = app.detail_timeframe.day_window() else {
        return all;
    };
    let Some(last) = all.last() else {
        return all;
    };
    let cutoff = last.ts.saturating_sub(days * 86_400);
    let filtered = all
        .iter()
        .filter(|point| point.ts >= cutoff)
        .cloned()
        .collect::<Vec<_>>();
    if filtered.len() >= 2 {
        return filtered;
    }
    let fallback = match app.detail_timeframe {
        DetailTimeframe::OneDay => 2,
        DetailTimeframe::OneWeek => 5,
        DetailTimeframe::OneMonth => 22,
        DetailTimeframe::ThreeMonths => 66,
        DetailTimeframe::SixMonths => 132,
        DetailTimeframe::OneYear => 252,
        DetailTimeframe::FiveYears | DetailTimeframe::Max => all.len(),
    };
    all[all.len().saturating_sub(fallback)..].to_vec()
}

fn draw_price_grid(
    buffer: &mut Buffer,
    label_area: Rect,
    plot_area: Rect,
    theme: crate::theme::Theme,
    ticks: &[f64],
    y_min: f64,
    y_max: f64,
) {
    for tick in ticks {
        let y = price_to_y(plot_area, *tick, y_min, y_max);
        for x in plot_area.x..plot_area.right() {
            set_cell(buffer, x, y, "┄", Style::default().fg(theme.border_dim));
        }
        let label = format_axis_price(*tick, y_max - y_min, axis_precision(y_min, y_max));
        let x = label_area
            .right()
            .saturating_sub(text_width(&label) as u16)
            .saturating_sub(1);
        buffer.set_stringn(
            x,
            y,
            label,
            label_area.width.saturating_sub(1) as usize,
            Style::default().fg(theme.muted),
        );
    }
}

fn draw_month_grid(
    buffer: &mut Buffer,
    plot_area: Rect,
    theme: crate::theme::Theme,
    history: &[HistoryPoint],
) {
    let mut seen = HashSet::new();
    for point in history {
        let Some(datetime) = Local.timestamp_opt(point.ts, 0).single() else {
            continue;
        };
        let key = (datetime.year(), datetime.month());
        if !seen.insert(key) {
            continue;
        }
        let x = time_to_x(plot_area, point.ts, history);
        for y in plot_area.y..plot_area.bottom() {
            set_cell(buffer, x, y, "┆", Style::default().fg(theme.border_dim));
        }
    }
}

fn draw_previous_close(
    buffer: &mut Buffer,
    app: &App,
    theme: crate::theme::Theme,
    area: Rect,
    previous_close: Option<f64>,
    y_min: f64,
    y_max: f64,
) {
    let Some(previous_close) = previous_close else {
        return;
    };
    if previous_close < y_min || previous_close > y_max {
        return;
    }
    let y = price_to_y(area, previous_close, y_min, y_max);
    for x in area.x..area.right() {
        set_cell(buffer, x, y, "·", Style::default().fg(theme.muted));
    }
    let label = format!(
        "{} {}",
        app.t(Key::DetailsLabelPreviousClose).to_lowercase(),
        format_price(previous_close)
    );
    buffer.set_stringn(
        area.x,
        y,
        label,
        area.width as usize,
        Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
    );
}

fn draw_current_price_marker(
    buffer: &mut Buffer,
    theme: crate::theme::Theme,
    area: Rect,
    price: f64,
    y_min: f64,
    y_max: f64,
    color: Color,
) {
    let y = price_to_y(area, price, y_min, y_max);
    let label = format!("▶{}", format_price(price));
    let width = text_width(&label).min(area.width as usize);
    let x = area.right().saturating_sub(width as u16);
    buffer.set_stringn(
        x,
        y,
        label,
        width,
        Style::default()
            .fg(inverse_foreground(theme))
            .bg(color)
            .add_modifier(Modifier::BOLD),
    );
}

fn draw_x_axis(
    buffer: &mut Buffer,
    app: &App,
    theme: crate::theme::Theme,
    area: Rect,
    history: &[HistoryPoint],
) {
    if history.is_empty() || area.width == 0 {
        return;
    }
    for (ts, label) in x_axis_labels(app.detail_timeframe, history, area.width) {
        let x = time_to_x(area, ts, history);
        let width = text_width(&label) as u16;
        let start = x.saturating_sub(width / 2).max(area.x);
        let max_width = area.right().saturating_sub(start) as usize;
        buffer.set_stringn(
            start,
            area.y,
            label,
            max_width,
            Style::default().fg(theme.muted),
        );
    }
}

fn render_detail_sidebar(
    frame: &mut Frame,
    app: &App,
    details: &crate::features::search::engine::InstrumentDetails,
    theme: crate::theme::Theme,
    area: Rect,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let inner = if area.width > 3 {
        Rect::new(
            area.x + 1,
            area.y,
            area.width.saturating_sub(3),
            area.height,
        )
    } else {
        area
    };
    let lines = detail_sidebar_lines(app, details, theme, inner.width as usize);
    let max_scroll = lines.len().saturating_sub(inner.height as usize);
    let scroll = app.detail_sidebar_scroll.min(max_scroll);
    let end = scroll
        .saturating_add(inner.height as usize)
        .min(lines.len());
    let visible = lines[scroll..end].to_vec();
    frame.render_widget(
        Paragraph::new(visible).style(background_style(theme).fg(theme.foreground)),
        inner,
    );
    if lines.len() > inner.height as usize && area.width > 1 {
        draw_scrollbar(frame.buffer_mut(), theme, area, scroll, lines.len());
    }
}

fn detail_sidebar_lines(
    app: &App,
    details: &crate::features::search::engine::InstrumentDetails,
    theme: crate::theme::Theme,
    width: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    push_quote_section(&mut lines, app, details, theme, width);
    push_section_gap(&mut lines);
    push_section_separator(&mut lines, app.t(Key::DetailsSectionKeyStats), theme, width);
    push_key_stats(&mut lines, app, theme, width);
    push_section_gap(&mut lines);
    push_section_separator(&mut lines, app.t(Key::DetailsSectionProfile), theme, width);
    push_profile(&mut lines, app, details, theme, width);
    push_section_gap(&mut lines);
    push_section_separator(&mut lines, app.t(Key::DetailsSectionCompany), theme, width);
    push_company_description(&mut lines, app, theme, width);
    push_section_gap(&mut lines);
    push_section_separator(
        &mut lines,
        app.t(Key::DetailsSectionMarketContext),
        theme,
        width,
    );
    push_market_context(&mut lines, app, details.symbol.as_str(), theme, width);
    push_section_gap(&mut lines);
    push_section_separator(
        &mut lines,
        app.t(Key::DetailsSectionHeadlines),
        theme,
        width,
    );
    push_headlines(&mut lines, app, details.symbol.as_str(), theme, width);
    push_section_gap(&mut lines);
    push_section_separator(&mut lines, app.t(Key::DetailsSectionNotes), theme, width);
    push_notes(&mut lines, app, details.symbol.as_str(), theme, width);
    lines
}

fn push_quote_section(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    details: &crate::features::search::engine::InstrumentDetails,
    theme: crate::theme::Theme,
    width: usize,
) {
    push_section_separator(lines, app.t(Key::DetailsSectionQuote), theme, width);
    lines.push(Line::from(Span::styled(
        truncate_to_width(details.symbol.as_str(), width),
        Style::default()
            .fg(theme.foreground)
            .add_modifier(Modifier::BOLD),
    )));
    let exchange = details.exchange.as_deref().unwrap_or("-");
    let currency = details.currency.as_deref().unwrap_or("-");
    lines.push(Line::from(Span::styled(
        truncate_to_width(
            &format!("{}  {}  {}", details.name, exchange, currency),
            width,
        ),
        Style::default().fg(theme.muted),
    )));
    if let Some(live) = &app.selected_live_details {
        lines.push(Line::from(Span::styled(
            live.price
                .map(format_price)
                .unwrap_or_else(|| "-".to_string()),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(change_chip_spans(live, theme)));
        if let Some(extended_price) = live.extended_price {
            let percent = live
                .extended_change_percent
                .map(|value| format!("{value:+.2}%"))
                .unwrap_or_else(|| "-".to_string());
            lines.push(Line::from(Span::styled(
                truncate_to_width(
                    &format!(
                        "{}  {}  {}",
                        app.t(Key::DetailsLabelAfterHours),
                        format_price(extended_price),
                        percent
                    ),
                    width,
                ),
                Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
            )));
        }
    } else {
        let status = if app.live_details_loading {
            app.t(Key::DetailsStatusLoading)
        } else {
            app.t(Key::DetailsChartNoData)
        };
        lines.push(Line::from(Span::styled(
            status.to_string(),
            Style::default().fg(theme.muted),
        )));
    }
}

fn change_chip_spans(
    live: &LiveInstrumentDetails,
    theme: crate::theme::Theme,
) -> Vec<Span<'static>> {
    match price_change(live) {
        Some((absolute, percent)) => {
            let positive = absolute >= 0.0;
            let color = if positive {
                theme.positive
            } else {
                theme.negative
            };
            let direction = if positive { "▲" } else { "▼" };
            let sign = if positive { "+" } else { "" };
            vec![Span::styled(
                format!(" {direction} {sign}{absolute:.2} ({sign}{percent:.2}%) "),
                Style::default()
                    .fg(inverse_foreground(theme))
                    .bg(color)
                    .add_modifier(Modifier::BOLD),
            )]
        }
        None => vec![Span::styled(
            " - ".to_string(),
            Style::default().fg(theme.muted),
        )],
    }
}

fn push_key_stats(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    theme: crate::theme::Theme,
    width: usize,
) {
    let Some(live) = &app.selected_live_details else {
        let loading = if app.live_details_loading {
            app.t(Key::DetailsStatusLoading)
        } else {
            app.t(Key::DetailsChartNoData)
        };
        lines.push(Line::from(Span::styled(
            loading.to_string(),
            Style::default().fg(theme.muted),
        )));
        return;
    };
    let visible = visible_key_stats(app.preferences.experience);
    debug_assert!(visible_metrics(app.preferences.experience).len() >= visible.len());
    let focused_metric = app.focused_detail_metric();
    let stats = visible
        .iter()
        .copied()
        .map(|metric| stat_cell(app, live, metric, theme, focused_metric == Some(metric)))
        .collect::<Vec<_>>();
    if width < 34 {
        for cell in stats {
            push_single_stat_line(lines, cell, theme, width);
        }
    } else {
        for pair in stats.chunks(2) {
            let left = &pair[0];
            let right = pair.get(1);
            lines.push(Line::from(stat_grid_spans(left, right, theme, width)));
        }
    }
    push_focused_metric_explanation(lines, app, theme, width, focused_metric);
}

#[derive(Debug, Clone)]
struct StatCell {
    label: String,
    value: String,
    color: Color,
    focused: bool,
}

impl StatCell {
    fn new(label: &str, value: Option<String>, color: Color, focused: bool) -> Self {
        Self {
            label: label.to_string(),
            value: value.unwrap_or_else(|| "-".to_string()),
            color,
            focused,
        }
    }
}

fn stat_cell(
    app: &App,
    live: &LiveInstrumentDetails,
    metric: MetricId,
    theme: crate::theme::Theme,
    focused: bool,
) -> StatCell {
    let value = match metric {
        MetricId::Price => live.price.map(format_price),
        MetricId::ChangePercent => price_change(live).map(|(_, percent)| format!("{percent:+.2}%")),
        MetricId::Volume => live.day_volume.map(format_large_number),
        MetricId::AverageVolume => live.avg_volume.map(format_large_number),
        MetricId::RelativeVolume => relative_volume(live).map(|value| format!("{value:.2}x")),
        MetricId::MarketCap => live.market_cap.map(format_large_number),
        MetricId::PreviousClose => live.previous_close.map(format_price),
        MetricId::Open => live.open.map(format_price),
        MetricId::DayRange => day_range(live),
        MetricId::Week52High => live.week_52_high.map(format_price),
        MetricId::Week52Low => live.week_52_low.map(format_price),
        MetricId::PeRatio => live.trailing_pe.map(format_ratio),
        MetricId::ForwardPe => live.forward_pe.map(format_ratio),
        MetricId::DividendYield => live.dividend_yield.map(format_percent),
        MetricId::Beta => live.beta.map(format_ratio),
    };
    let color = match metric {
        MetricId::RelativeVolume => relative_volume(live)
            .map(|value| {
                if value > 2.0 {
                    theme.positive
                } else if value < 0.8 {
                    theme.negative
                } else {
                    theme.foreground
                }
            })
            .unwrap_or(theme.foreground),
        _ => theme.foreground,
    };
    StatCell::new(app.t(metric_label_key(metric)), value, color, focused)
}

fn stat_grid_spans(
    left: &StatCell,
    right: Option<&StatCell>,
    theme: crate::theme::Theme,
    width: usize,
) -> Vec<Span<'static>> {
    let gap = 2usize;
    let cell_width = width.saturating_sub(gap) / 2;
    let mut spans = stat_cell_spans(left, theme, cell_width);
    spans.push(Span::raw(" ".repeat(gap)));
    if let Some(right) = right {
        spans.extend(stat_cell_spans(
            right,
            theme,
            width.saturating_sub(cell_width + gap),
        ));
    }
    spans
}

fn stat_cell_spans(
    cell: &StatCell,
    theme: crate::theme::Theme,
    width: usize,
) -> Vec<Span<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let value = truncate_to_width(&cell.value, width);
    let value_width = text_width(&value);
    let label_width = width.saturating_sub(value_width.saturating_add(1));
    let label = truncate_to_width(&cell.label, label_width);
    let padding = width.saturating_sub(text_width(&label) + value_width);
    let label_style = if cell.focused {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };
    let value_style = if cell.focused {
        Style::default().fg(cell.color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(cell.color)
    };
    vec![
        Span::styled(label, label_style),
        Span::raw(" ".repeat(padding)),
        Span::styled(value, value_style),
    ]
}

fn push_single_stat_line(
    lines: &mut Vec<Line<'static>>,
    cell: StatCell,
    theme: crate::theme::Theme,
    width: usize,
) {
    lines.push(Line::from(stat_cell_spans(&cell, theme, width)));
}

fn push_focused_metric_explanation(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    theme: crate::theme::Theme,
    width: usize,
    focused_metric: Option<MetricId>,
) {
    if app.preferences.explanations != ExplanationLevel::Beginner {
        return;
    }
    let Some(metric) = focused_metric else {
        return;
    };
    let Some(key) = metric_explanation_key(metric) else {
        return;
    };
    let Some(explanation) = app.i18n.metric_explanation(key) else {
        return;
    };
    lines.push(Line::from(""));
    let label = app.t(metric_label_key(metric));
    let text = format!("{label}: {explanation}");
    for line in wrap_words(&text, width) {
        lines.push(Line::from(Span::styled(
            line,
            Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
        )));
    }
}

fn push_profile(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    details: &crate::features::search::engine::InstrumentDetails,
    theme: crate::theme::Theme,
    width: usize,
) {
    let live = app.selected_live_details.as_ref();
    let headquarters = live.and_then(format_headquarters);
    let employees = live
        .and_then(|live| live.full_time_employees)
        .map(format_compact_number);
    let rows = vec![
        (app.t(Key::DetailsLabelSector), details.sector.clone()),
        (app.t(Key::DetailsLabelIndustry), details.industry.clone()),
        (
            app.t(Key::DetailsLabelCountry),
            live.and_then(|live| live.country.clone()),
        ),
        (app.t(Key::DetailsLabelHeadquarters), headquarters),
        (app.t(Key::DetailsLabelEmployees), employees),
    ];
    let label_width = rows
        .iter()
        .map(|(label, _)| text_width(label))
        .max()
        .unwrap_or(8)
        .min(width.saturating_div(2))
        .saturating_add(2);
    for (label, value) in rows {
        push_detail_row(
            lines,
            label,
            value,
            label_width,
            width,
            theme.foreground,
            theme.muted,
        );
    }
}

fn push_company_description(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    theme: crate::theme::Theme,
    width: usize,
) {
    let Some(live) = &app.selected_live_details else {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsStatusLoading).to_string(),
            Style::default().fg(theme.muted),
        )));
        return;
    };
    let Some(summary) = live_summary(live, &app.locale) else {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsStatusNoSummary).to_string(),
            Style::default().fg(theme.muted),
        )));
        return;
    };
    let mut wrapped = wrap_words(&summary, width);
    if !app.detail_description_expanded && wrapped.len() > 3 {
        wrapped.truncate(3);
        append_suffix_to_last_line(&mut wrapped, "...", width);
    }
    for line in wrapped {
        lines.push(Line::from(Span::styled(
            line,
            Style::default().fg(theme.foreground),
        )));
    }
}

fn push_market_context(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    symbol: &str,
    theme: crate::theme::Theme,
    width: usize,
) {
    if app.backend_insight_loading {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsContextLoading).to_string(),
            Style::default().fg(theme.muted),
        )));
        return;
    }
    if let Some(status) = &app.backend_insight_status {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsContextBackendUnavailable)
                .replace("{status}", status),
            Style::default().fg(theme.muted),
        )));
        return;
    }
    let Some(insight) = app.backend_insight.as_ref() else {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsContextEmpty).to_string(),
            Style::default().fg(theme.muted),
        )));
        return;
    };
    if insight.ticker != symbol {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsContextEmpty).to_string(),
            Style::default().fg(theme.muted),
        )));
        return;
    }
    let Some(explanation) = &insight.explanation else {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsContextEmpty).to_string(),
            Style::default().fg(theme.muted),
        )));
        return;
    };

    if !explanation.summary.trim().is_empty() {
        let mut summary = wrap_words(&explanation.summary, width);
        if !app.detail_context_expanded && summary.len() > 2 {
            summary.truncate(2);
            append_suffix_to_last_line(&mut summary, "...", width);
        }
        for line in summary {
            lines.push(Line::from(line));
        }
    }
    if explanation.stale_context {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsContextStale).to_string(),
            Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
        )));
    } else {
        let mut metadata = app
            .t(Key::DetailsContextConfidence)
            .replace("{confidence}", &explanation.confidence);
        if explanation.cache_hit {
            metadata.push_str(" | ");
            metadata.push_str(app.t(Key::DetailsContextCache));
        }
        lines.push(Line::from(Span::styled(
            metadata,
            Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
        )));
    }
    let driver_limit = if app.detail_context_expanded {
        usize::MAX
    } else {
        2
    };
    for driver in explanation.key_drivers.iter().take(driver_limit) {
        for (index, wrapped) in wrap_words(driver, width.saturating_sub(2))
            .into_iter()
            .enumerate()
        {
            let prefix = if index == 0 { "• " } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(prefix.to_string(), Style::default().fg(theme.accent)),
                Span::styled(wrapped, Style::default().fg(theme.foreground)),
            ]));
        }
    }
}

fn push_headlines(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    symbol: &str,
    theme: crate::theme::Theme,
    width: usize,
) {
    let rows = detail_headlines(app, symbol);
    if rows.is_empty() {
        let empty = backend_headlines_empty_message(app, symbol)
            .unwrap_or_else(|| app.t(Key::DetailsHeadlinesEmpty).to_string());
        lines.push(Line::from(Span::styled(
            empty,
            Style::default().fg(theme.muted),
        )));
        return;
    }
    for (index, row) in rows.iter().take(3).enumerate() {
        if index > 0 {
            lines.push(Line::from(""));
        }
        let prefix = format!("{}. ", index + 1);
        let title_width = width.saturating_sub(text_width(&prefix));
        let mut title_lines = wrap_words(&row.title, title_width);
        if title_lines.len() > 2 {
            title_lines.truncate(2);
            append_suffix_to_last_line(&mut title_lines, "...", title_width);
        }
        if title_lines.is_empty() {
            title_lines.push(row.title.clone());
        }
        for (line_index, title) in title_lines.into_iter().enumerate() {
            let line_prefix = if line_index == 0 {
                prefix.clone()
            } else {
                " ".repeat(text_width(&prefix))
            };
            lines.push(Line::from(vec![
                Span::styled(line_prefix, Style::default().fg(theme.accent)),
                Span::styled(title, Style::default().fg(theme.foreground)),
            ]));
        }
        let metadata = format!("{} | {}", row.sources.join(", "), row.age);
        lines.push(Line::from(Span::styled(
            truncate_to_width(&metadata, width),
            Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
        )));
    }
}

#[derive(Debug, Clone)]
struct DetailHeadline {
    title: String,
    sources: Vec<String>,
    age: String,
}

fn detail_headlines(app: &App, symbol: &str) -> Vec<DetailHeadline> {
    if let Some(insight) = app.backend_insight.as_ref()
        && insight.ticker == symbol
        && let Some(context) = &insight.context
    {
        return dedupe_backend_headlines(app, &context.articles);
    }
    dedupe_local_headlines(app, symbol)
}

fn dedupe_backend_headlines(app: &App, articles: &[InsightArticle]) -> Vec<DetailHeadline> {
    let mut rows = Vec::new();
    for article in articles {
        merge_headline(
            &mut rows,
            article.title.clone(),
            backend_source_label(app, article),
            backend_age_label(app, article),
        );
    }
    rows
}

fn dedupe_local_headlines(app: &App, symbol: &str) -> Vec<DetailHeadline> {
    let mut rows = Vec::new();
    for item in app
        .news_items
        .iter()
        .filter(|item| item.symbols.iter().any(|candidate| candidate == symbol))
    {
        let age = app.news_timestamp(item.published_at);
        merge_headline(
            &mut rows,
            item.title.clone(),
            if item.source.trim().is_empty() {
                app.t(Key::DetailsHeadlinesLocalFeed).to_string()
            } else {
                item.source.clone()
            },
            if age.is_empty() {
                app.t(Key::DetailsHeadlinesFresh).to_string()
            } else {
                age
            },
        );
    }
    rows
}

fn merge_headline(rows: &mut Vec<DetailHeadline>, title: String, source: String, age: String) {
    let key = normalize_headline_key(&title);
    if let Some(existing) = rows
        .iter_mut()
        .find(|row| normalize_headline_key(&row.title) == key)
    {
        if !existing
            .sources
            .iter()
            .any(|candidate| candidate == &source)
        {
            existing.sources.push(source);
        }
        return;
    }
    rows.push(DetailHeadline {
        title,
        sources: vec![source],
        age,
    });
}

fn backend_headlines_empty_message(app: &App, symbol: &str) -> Option<String> {
    let insight = app.backend_insight.as_ref()?;
    if insight.ticker != symbol {
        return None;
    }
    let context = insight.context.as_ref()?;
    if context.stale_context || context.articles.is_empty() {
        Some(app.t(Key::DetailsHeadlinesNoFresh).to_string())
    } else {
        None
    }
}

fn push_notes(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    symbol: &str,
    theme: crate::theme::Theme,
    width: usize,
) {
    let Ok(connection) = db::open(&app.ticker_db_path) else {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsNotesUnavailable).to_string(),
            Style::default().fg(theme.muted),
        )));
        return;
    };
    let notes = crate::features::notes::repo::list_all(&connection)
        .unwrap_or_default()
        .into_iter()
        .filter(|note| note.tickers.iter().any(|ticker| ticker == symbol))
        .take(2)
        .collect::<Vec<_>>();
    if notes.is_empty() {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsNotesEmpty).to_string(),
            Style::default().fg(theme.muted),
        )));
        return;
    }
    for note in notes {
        let preview = note.body.lines().next().unwrap_or("").trim();
        let text = if preview.is_empty() {
            app.t(Key::DetailsNotesEmptyNote).to_string()
        } else {
            truncate_to_width(preview, width.saturating_sub(2))
        };
        lines.push(Line::from(vec![
            Span::styled("• ".to_string(), Style::default().fg(theme.accent)),
            Span::styled(text, Style::default().fg(theme.foreground)),
        ]));
    }
}

fn push_section_gap(lines: &mut Vec<Line<'static>>) {
    if !lines.is_empty() {
        lines.push(Line::from(""));
    }
}

fn push_section_separator(
    lines: &mut Vec<Line<'static>>,
    title: &str,
    theme: crate::theme::Theme,
    width: usize,
) {
    let title = title.to_uppercase();
    let left = "─ ";
    let right_prefix = " ";
    let used = text_width(left) + text_width(&title) + text_width(right_prefix);
    let rule = "─".repeat(width.saturating_sub(used));
    lines.push(Line::from(vec![
        Span::styled(left.to_string(), Style::default().fg(theme.border_dim)),
        Span::styled(
            title,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{right_prefix}{rule}"),
            Style::default().fg(theme.border_dim),
        ),
    ]));
}

fn draw_scrollbar(
    buffer: &mut Buffer,
    theme: crate::theme::Theme,
    area: Rect,
    scroll: usize,
    total_lines: usize,
) {
    if area.height == 0 {
        return;
    }
    let x = area.right().saturating_sub(1);
    for y in area.y..area.bottom() {
        set_cell(buffer, x, y, "│", Style::default().fg(theme.border_dim));
    }
    let height = area.height as usize;
    let thumb_height = (height * height / total_lines).clamp(1, height);
    let max_scroll = total_lines.saturating_sub(height);
    let thumb_top = (scroll * height.saturating_sub(thumb_height))
        .checked_div(max_scroll)
        .unwrap_or(0);
    for offset in 0..thumb_height {
        let y = area.y + (thumb_top + offset) as u16;
        set_cell(
            buffer,
            x,
            y,
            "█",
            Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
        );
    }
}

fn render_results(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let inner = area;

    let visible_rows = inner.height.saturating_sub(1).max(1) as usize;

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

fn background_style(theme: crate::theme::Theme) -> Style {
    match theme.background {
        Some(background) => Style::default().bg(background),
        None => Style::default(),
    }
}

fn push_detail_row(
    lines: &mut Vec<Line<'_>>,
    label: &str,
    value: Option<String>,
    label_width: usize,
    row_width: usize,
    foreground: Color,
    muted: Color,
) {
    let label_text = pad_right(label, label_width.min(row_width));
    let remaining = row_width.saturating_sub(text_width(&label_text));
    let value = truncate_to_width(&value.unwrap_or_else(|| "-".to_string()), remaining);
    lines.push(Line::from(vec![
        Span::styled(label_text, Style::default().fg(muted)),
        Span::styled(value, Style::default().fg(foreground)),
    ]));
}

fn backend_source_label(app: &App, article: &InsightArticle) -> String {
    if article.source.trim().is_empty() {
        app.t(Key::DetailsHeadlinesSourceUnknown).to_string()
    } else {
        article.source.clone()
    }
}

fn backend_age_label(app: &App, article: &InsightArticle) -> String {
    if let Some(age_hours) = article.age_hours {
        format!("{age_hours:.0}h")
    } else if let Some(published_at) = &article.published_at {
        published_at.clone()
    } else {
        app.t(Key::DetailsHeadlinesFresh).to_string()
    }
}

fn normalize_headline_key(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| character.to_lowercase())
        .filter(|character| character.is_ascii_alphanumeric() || character.is_ascii_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn nice_y_ticks(min: f64, max: f64, desired_ticks: usize) -> Vec<f64> {
    if !min.is_finite() || !max.is_finite() {
        return Vec::new();
    }
    let mut low = min.min(max);
    let mut high = min.max(max);
    if (high - low).abs() < f64::EPSILON {
        let padding = (high.abs() * 0.02).max(1.0);
        low -= padding;
        high += padding;
    }
    let mut step = nice_step((high - low).abs() / desired_ticks.saturating_sub(1).max(1) as f64);
    for _ in 0..8 {
        let start = (low / step).floor() * step;
        let end = (high / step).ceil() * step;
        let count = ((end - start) / step).round() as usize + 1;
        if (6..=8).contains(&count) {
            return tick_values(start, step, count);
        }
        step *= if count > 8 { 2.0 } else { 0.5 };
    }
    let start = (low / step).floor() * step;
    let end = (high / step).ceil() * step;
    let count = ((end - start) / step).round() as usize + 1;
    tick_values(start, step, count.clamp(2, 10))
}

fn tick_values(start: f64, step: f64, count: usize) -> Vec<f64> {
    (0..count)
        .map(|index| start + step * index as f64)
        .collect()
}

fn nice_step(raw: f64) -> f64 {
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }
    let exponent = raw.log10().floor();
    let magnitude = 10_f64.powf(exponent);
    let fraction = raw / magnitude;
    let nice_fraction = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 2.5 {
        2.5
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice_fraction * magnitude
}

fn axis_precision(min: f64, max: f64) -> usize {
    let range = (max - min).abs();
    if range >= 100.0 {
        0
    } else if range >= 10.0 {
        1
    } else {
        2
    }
}

fn price_to_y(area: Rect, value: f64, min: f64, max: f64) -> u16 {
    if area.height <= 1 || (max - min).abs() < f64::EPSILON {
        return area.y;
    }
    let ratio = ((max - value) / (max - min)).clamp(0.0, 1.0);
    area.y + (ratio * f64::from(area.height - 1)).round() as u16
}

fn time_to_x(area: Rect, ts: i64, history: &[HistoryPoint]) -> u16 {
    let (Some(first), Some(last)) = (history.first(), history.last()) else {
        return area.x;
    };
    if area.width <= 1 || first.ts == last.ts {
        return area.x;
    }
    let ratio = ((ts - first.ts) as f64 / (last.ts - first.ts) as f64).clamp(0.0, 1.0);
    area.x + (ratio * f64::from(area.width - 1)).round() as u16
}

fn x_axis_labels(
    timeframe: DetailTimeframe,
    history: &[HistoryPoint],
    width: u16,
) -> Vec<(i64, String)> {
    if history.is_empty() {
        return Vec::new();
    }
    let target = if width >= 100 {
        7
    } else if width >= 60 {
        6
    } else {
        5
    }
    .min(history.len());
    if target <= 1 {
        return vec![(
            history[0].ts,
            format_ts_label_for_timeframe(history[0].ts, timeframe),
        )];
    }
    (0..target)
        .map(|index| {
            let point_index = index * (history.len() - 1) / (target - 1);
            let ts = history[point_index].ts;
            (ts, format_ts_label_for_timeframe(ts, timeframe))
        })
        .collect()
}

fn format_ts_label_for_timeframe(ts: i64, timeframe: DetailTimeframe) -> String {
    let Some(datetime) = Local.timestamp_opt(ts, 0).single() else {
        return "-".to_string();
    };
    let format = match timeframe {
        DetailTimeframe::OneYear | DetailTimeframe::FiveYears | DetailTimeframe::Max => "%b '%y",
        _ => "%b %-d",
    };
    datetime.format(format).to_string()
}

fn set_cell(buffer: &mut Buffer, x: u16, y: u16, symbol: &str, style: Style) {
    if let Some(cell) = buffer.cell_mut((x, y)) {
        cell.set_symbol(symbol);
        cell.set_style(style);
    }
}

fn inverse_foreground(theme: crate::theme::Theme) -> Color {
    theme.background.unwrap_or(theme.surface)
}

fn text_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

fn truncate_to_width(value: &str, width: usize) -> String {
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

fn wrap_words(value: &str, width: usize) -> Vec<String> {
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

fn append_suffix_to_last_line(lines: &mut Vec<String>, suffix: &str, width: usize) {
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
    if avg == 0.0 { None } else { Some(day / avg) }
}

fn day_range(live: &LiveInstrumentDetails) -> Option<String> {
    Some(format!(
        "{} - {}",
        format_price(live.day_low?),
        format_price(live.day_high?)
    ))
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

fn format_axis_price(value: f64, range: f64, precision: usize) -> String {
    if range >= 100.0 && precision == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.precision$}")
    }
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

#[cfg(test)]
mod detail_render_tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    use crate::{
        app::Page,
        backend::{BackendInsight, InsightContextResponse, InsightExplanationResponse},
        config::AppConfig,
        features::search::engine::{InstrumentDetails, LiveInstrumentDetails},
    };

    #[test]
    fn detail_view_renders_at_requested_sizes() {
        for (width, height) in [(80, 24), (140, 40), (200, 60)] {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("test terminal");
            let app = detail_app();
            terminal
                .draw(|frame| render_details(frame, &app))
                .expect("detail render");
            println!(
                "\n--- {width}x{height} ---\n{}",
                buffer_view(terminal.backend().buffer())
            );
        }
    }

    #[test]
    fn beginner_metric_explanation_only_appears_when_enabled() {
        let mut app = App::new(AppConfig::default().expect("default config"));
        let theme = crate::theme::current_theme(app.theme_name);
        let mut lines = Vec::new();

        push_focused_metric_explanation(&mut lines, &app, theme, 80, Some(MetricId::MarketCap));
        assert!(lines.is_empty());

        app.preferences.explanations = crate::preferences::ExplanationLevel::Beginner;
        push_focused_metric_explanation(&mut lines, &app, theme, 80, Some(MetricId::MarketCap));
        assert!(!lines.is_empty());
    }

    fn detail_app() -> App {
        let mut app = App::new(AppConfig::default().expect("default config"));
        app.page = Page::Details;
        app.selected_details = Some(InstrumentDetails {
            symbol: "AAPL".to_string(),
            name: "Apple Inc.".to_string(),
            exchange: Some("NASDAQ".to_string()),
            asset_type: Some("stock".to_string()),
            sector: Some("Technology".to_string()),
            industry: Some("Consumer Electronics".to_string()),
            currency: Some("USD".to_string()),
            active: true,
            last_updated: Some("2026-07-03".to_string()),
        });
        app.selected_live_details = Some(LiveInstrumentDetails {
            price: Some(194.83),
            previous_close: Some(197.04),
            day_volume: Some(142_390_000.0),
            open: Some(198.12),
            day_high: Some(199.21),
            day_low: Some(193.88),
            market_cap: Some(4_720_000_000_000.0),
            avg_volume: Some(158_970_000.0),
            extended_price: Some(195.12),
            extended_change_percent: Some(0.15),
            week_52_high: Some(235.50),
            week_52_low: Some(177.40),
            trailing_pe: Some(29.84),
            forward_pe: Some(27.12),
            dividend_yield: Some(0.51),
            beta: Some(2.21),
            next_earnings_days: Some(24),
            summary: Some("Apple Inc. designs, manufactures, and markets smartphones, personal computers, tablets, wearables, and accessories worldwide. The company also sells related services including AppleCare, cloud services, content, and payment services.".to_string()),
            summary_de: None,
            city: Some("Cupertino".to_string()),
            state: Some("CA".to_string()),
            country: Some("United States".to_string()),
            website: Some("https://www.apple.com".to_string()),
            full_time_employees: Some(164_000.0),
            history: sample_history(),
        });
        app.backend_insight = Some(BackendInsight {
            ticker: "AAPL".to_string(),
            context: Some(InsightContextResponse {
                ticker: "AAPL".to_string(),
                stale_context: false,
                article_count: 2,
                articles: vec![
                    InsightArticle {
                        title: "Apple shares rise as services revenue offsets device weakness".to_string(),
                        url: "https://example.com/a".to_string(),
                        source: "Reuters".to_string(),
                        published_at: None,
                        age_hours: Some(1.0),
                        ticker: "AAPL".to_string(),
                        relevance_score: Some(0.9),
                        reason: None,
                    },
                    InsightArticle {
                        title: "Apple shares rise as services revenue offsets device weakness".to_string(),
                        url: "https://example.com/b".to_string(),
                        source: "The Globe and Mail".to_string(),
                        published_at: None,
                        age_hours: Some(2.0),
                        ticker: "AAPL".to_string(),
                        relevance_score: Some(0.8),
                        reason: None,
                    },
                    InsightArticle {
                        title: "Analysts focus on AI features ahead of the next iPhone cycle".to_string(),
                        url: "https://example.com/c".to_string(),
                        source: "The Motley Fool".to_string(),
                        published_at: None,
                        age_hours: Some(4.0),
                        ticker: "AAPL".to_string(),
                        relevance_score: Some(0.7),
                        reason: None,
                    },
                ],
            }),
            explanation: Some(InsightExplanationResponse {
                ticker: "AAPL".to_string(),
                model: "test".to_string(),
                cache_hit: true,
                stale_context: false,
                summary: "The market is weighing resilient services growth against softer hardware demand and margin pressure.".to_string(),
                key_drivers: vec![
                    "Services revenue continues to support multiple expansion.".to_string(),
                    "Hardware replacement cycles remain the near-term swing factor.".to_string(),
                    "AI-related product expectations are influencing sentiment.".to_string(),
                ],
                sources_used: vec!["Reuters".to_string()],
                confidence: "Moderate".to_string(),
            }),
        });
        app
    }

    fn sample_history() -> Vec<HistoryPoint> {
        let start = Local
            .with_ymd_and_hms(2025, 1, 2, 21, 0, 0)
            .single()
            .expect("date")
            .timestamp();
        (0..390)
            .map(|day| {
                let wave = (day as f64 / 17.0).sin() * 7.5;
                let close = 178.0 + day as f64 * 0.045 + wave;
                HistoryPoint {
                    ts: start + day * 86_400,
                    close,
                    volume: Some(110_000_000.0 + (day % 37) as f64 * 2_300_000.0),
                }
            })
            .collect()
    }

    fn buffer_view(buffer: &Buffer) -> String {
        let mut out = String::new();
        for row in buffer.content().chunks(buffer.area.width as usize) {
            for cell in row {
                out.push_str(cell.symbol());
            }
            out.push('\n');
        }
        out
    }
}
