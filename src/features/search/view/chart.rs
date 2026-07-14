//! Price chart panel: canvas drawing, axes, grids, and tick math.

use super::*;

pub(super) fn render_chart_header(
    frame: &mut Frame,
    app: &App,
    theme: crate::theme::Theme,
    area: Rect,
) {
    let history = visible_history(app);
    frame.render_widget(
        Paragraph::new(chart_header_line(app, theme, &history, area.width))
            .style(background_style(theme)),
        area,
    );
}

pub(super) fn render_chart_panel(
    frame: &mut Frame,
    app: &App,
    theme: crate::theme::Theme,
    area: Rect,
) {
    if area.width < 12 || area.height < 5 {
        return;
    }

    let history = visible_history(app);

    if history.len() < 2 {
        let message = if app.search.live_details_loading {
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
        .search
        .selected_live_details
        .as_ref()
        .and_then(|live| live.price)
        .unwrap_or(last.close);
    let reference = app
        .search
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
    let show_fill = app.search.detail_timeframe != DetailTimeframe::OneDay;
    let fill_color = if show_fill {
        Some(if last.close >= first.close {
            theme.positive_dim
        } else {
            theme.negative_dim
        })
    } else {
        None
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
            if let Some(fill_color) = fill_color {
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
            }
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

pub(super) fn chart_header_line(
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
        if *timeframe == app.search.detail_timeframe {
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

pub(super) fn chart_summary_spans(
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
            format!("{}: ", app.t(app.search.detail_timeframe.label_key())),
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

pub(super) fn push_span_with_width(
    spans: &mut Vec<Span<'static>>,
    used: &mut usize,
    text: impl Into<String>,
    style: Style,
) {
    let text = text.into();
    *used += text_width(&text);
    spans.push(Span::styled(text, style));
}

pub(super) fn visible_history(app: &App) -> Vec<HistoryPoint> {
    let all = app
        .search
        .selected_live_details
        .as_ref()
        .map(|live| normalize_history(&live.history))
        .unwrap_or_default();
    let Some(days) = app.search.detail_timeframe.day_window() else {
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
    if app.search.detail_timeframe == DetailTimeframe::OneDay {
        let intraday = intraday_points(&filtered);
        if intraday.len() >= 2 {
            return intraday;
        }
    }
    if filtered.len() >= 2 {
        return filtered;
    }
    let fallback = match app.search.detail_timeframe {
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

pub(super) fn intraday_points(history: &[HistoryPoint]) -> Vec<HistoryPoint> {
    const MAX_INTRADAY_GAP_SECONDS: i64 = 30 * 60;

    history
        .iter()
        .enumerate()
        .filter(|(index, point)| {
            let previous_gap = index
                .checked_sub(1)
                .map(|previous| point.ts.saturating_sub(history[previous].ts));
            let next_gap = history
                .get(index + 1)
                .map(|next| next.ts.saturating_sub(point.ts));
            previous_gap.is_some_and(|gap| gap <= MAX_INTRADAY_GAP_SECONDS)
                || next_gap.is_some_and(|gap| gap <= MAX_INTRADAY_GAP_SECONDS)
        })
        .map(|(_, point)| point.clone())
        .collect()
}

pub(super) fn draw_price_grid(
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

pub(super) fn draw_month_grid(
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

pub(super) fn draw_previous_close(
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

pub(super) fn draw_current_price_marker(
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

pub(super) fn draw_x_axis(
    buffer: &mut Buffer,
    app: &App,
    theme: crate::theme::Theme,
    area: Rect,
    history: &[HistoryPoint],
) {
    if history.is_empty() || area.width == 0 {
        return;
    }
    for (ts, label) in x_axis_labels(app.search.detail_timeframe, history, area.width) {
        let x = time_to_x(area, ts, history);
        let width = text_width(&label) as u16;
        let max_start = area.right().saturating_sub(width);
        let start = x.saturating_sub(width / 2).clamp(area.x, max_start);
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

pub(super) fn nice_y_ticks(min: f64, max: f64, desired_ticks: usize) -> Vec<f64> {
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

pub(super) fn tick_values(start: f64, step: f64, count: usize) -> Vec<f64> {
    (0..count)
        .map(|index| start + step * index as f64)
        .collect()
}

pub(super) fn nice_step(raw: f64) -> f64 {
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

pub(super) fn axis_precision(min: f64, max: f64) -> usize {
    let range = (max - min).abs();
    if range >= 100.0 {
        0
    } else if range >= 10.0 {
        1
    } else {
        2
    }
}

pub(super) fn price_to_y(area: Rect, value: f64, min: f64, max: f64) -> u16 {
    if area.height <= 1 || (max - min).abs() < f64::EPSILON {
        return area.y;
    }
    let ratio = ((max - value) / (max - min)).clamp(0.0, 1.0);
    area.y + (ratio * f64::from(area.height - 1)).round() as u16
}

pub(super) fn time_to_x(area: Rect, ts: i64, history: &[HistoryPoint]) -> u16 {
    let (Some(first), Some(last)) = (history.first(), history.last()) else {
        return area.x;
    };
    if area.width <= 1 || first.ts == last.ts {
        return area.x;
    }
    let ratio = ((ts - first.ts) as f64 / (last.ts - first.ts) as f64).clamp(0.0, 1.0);
    area.x + (ratio * f64::from(area.width - 1)).round() as u16
}

pub(super) fn x_axis_labels(
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

pub(super) fn format_ts_label_for_timeframe(ts: i64, timeframe: DetailTimeframe) -> String {
    let Some(datetime) = Local.timestamp_opt(ts, 0).single() else {
        return "-".to_string();
    };
    let format = match timeframe {
        DetailTimeframe::OneDay => "%H:%M",
        DetailTimeframe::OneYear | DetailTimeframe::FiveYears | DetailTimeframe::Max => "%b '%y",
        _ => "%b %-d",
    };
    datetime.format(format).to_string()
}

pub(super) fn set_cell(buffer: &mut Buffer, x: u16, y: u16, symbol: &str, style: Style) {
    if let Some(cell) = buffer.cell_mut((x, y)) {
        cell.set_symbol(symbol);
        cell.set_style(style);
    }
}

pub(super) fn normalize_history(history: &[HistoryPoint]) -> Vec<HistoryPoint> {
    let mut normalized = history.to_vec();
    normalized.sort_by_key(|point| point.ts);
    normalized.dedup_by(|left, right| left.ts == right.ts);
    normalized
}

pub(super) fn format_axis_price(value: f64, range: f64, precision: usize) -> String {
    if range >= 100.0 && precision == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.precision$}")
    }
}
