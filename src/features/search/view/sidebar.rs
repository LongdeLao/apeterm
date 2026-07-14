//! Detail sidebar: quote, key stats, profile, market context, notes.

use super::*;

pub(super) fn render_detail_sidebar(
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
    let scroll = app.search.detail_sidebar_scroll.min(max_scroll);
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

pub(super) fn detail_sidebar_lines(
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

pub(super) fn push_quote_section(
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
    if let Some(live) = &app.search.selected_live_details {
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
        let status = if app.search.live_details_loading {
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

pub(super) fn change_chip_spans(
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

pub(super) fn push_key_stats(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    theme: crate::theme::Theme,
    width: usize,
) {
    let Some(live) = &app.search.selected_live_details else {
        let loading = if app.search.live_details_loading {
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
pub(super) struct StatCell {
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

pub(super) fn stat_cell(
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

pub(super) fn stat_grid_spans(
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

pub(super) fn stat_cell_spans(
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

pub(super) fn push_single_stat_line(
    lines: &mut Vec<Line<'static>>,
    cell: StatCell,
    theme: crate::theme::Theme,
    width: usize,
) {
    lines.push(Line::from(stat_cell_spans(&cell, theme, width)));
}

pub(super) fn push_focused_metric_explanation(
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

pub(super) fn push_profile(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    details: &crate::features::search::engine::InstrumentDetails,
    theme: crate::theme::Theme,
    width: usize,
) {
    let live = app.search.selected_live_details.as_ref();
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

pub(super) fn push_company_description(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    theme: crate::theme::Theme,
    width: usize,
) {
    let Some(live) = &app.search.selected_live_details else {
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
    if !app.search.detail_description_expanded && wrapped.len() > 3 {
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

pub(super) fn push_market_context(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    symbol: &str,
    theme: crate::theme::Theme,
    width: usize,
) {
    if app.search.backend_insight_loading {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsContextLoading).to_string(),
            Style::default().fg(theme.muted),
        )));
        return;
    }
    if let Some(status) = &app.search.backend_insight_status {
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsContextBackendUnavailable)
                .replace("{status}", status),
            Style::default().fg(theme.muted),
        )));
        return;
    }
    let Some(insight) = app.search.backend_insight.as_ref() else {
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
        if !app.search.detail_context_expanded && summary.len() > 2 {
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
    let driver_limit = if app.search.detail_context_expanded {
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

pub(super) fn push_notes(
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

pub(super) fn push_section_gap(lines: &mut Vec<Line<'static>>) {
    if !lines.is_empty() {
        lines.push(Line::from(""));
    }
}

pub(super) fn push_section_separator(
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

pub(super) fn draw_scrollbar(
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

pub(super) fn push_detail_row(
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
