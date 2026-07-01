use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{App, SearchAssetKind},
    i18n::{Key, Locale},
    pages::fill::Fill,
    search::LiveInstrumentDetails,
    theme::current_theme,
};

pub fn render(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    if let Some(background) = theme.background {
        frame.render_widget(Fill::new(background), frame.area());
    }

    let area = frame.area();
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
            Span::styled("\u{f002} ", Style::default().fg(theme.accent)),
            Span::styled(
                app.search_query.as_str(),
                Style::default().fg(theme.foreground),
            ),
        ]),
        Line::from(vec![
            filter_span(
                app.t(Key::SearchFilterStocks),
                app.search_asset_kind == SearchAssetKind::Stocks,
                theme.foreground,
                theme.muted,
            ),
            Span::raw("  "),
            filter_span(
                app.t(Key::SearchFilterEtfs),
                app.search_asset_kind == SearchAssetKind::Etfs,
                theme.foreground,
                theme.muted,
            ),
            Span::styled(
                app.t(Key::SearchHelpTabSwitches),
                Style::default().fg(theme.muted),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme.accent)),
    );
    frame.render_widget(input, chunks[0]);

    if let Some(message) = &app.search_message {
        frame.render_widget(
            Paragraph::new(message.as_str()).style(Style::default().fg(theme.muted)),
            chunks[1],
        );
    } else {
        render_results(frame, app, chunks[1]);
    }

    frame.render_widget(
        Paragraph::new(app.t(Key::SearchFooter)).style(Style::default().fg(theme.muted)),
        chunks[2],
    );
}

pub fn render_details(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    render(frame, app);

    let area = centered_percent_rect(frame.area(), 88, 78);
    let Some(details) = &app.selected_details else {
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("\u{f1ad} ", Style::default().fg(theme.accent)),
            Span::styled(
                details.symbol.as_str(),
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", details.name),
                Style::default().fg(theme.foreground),
            ),
        ]),
        Line::from(""),
    ];

    if let Some(live) = &app.selected_live_details {
        lines.push(section_title("\u{f201}", app.t(Key::DetailsSectionQuote)));
        lines.push(quote_line(live, app, theme.foreground, theme.muted));
        lines.push(Line::from(""));
        lines.push(section_title("\u{f02d}", app.t(Key::DetailsSectionSummary)));
        if let Some(summary) = live_summary(live, &app.locale) {
            for line in wrap_text(&summary, area.width.saturating_sub(6) as usize) {
                lines.push(Line::from(Span::styled(
                    line,
                    Style::default().fg(theme.foreground),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                app.t(Key::DetailsStatusNoSummary),
                Style::default().fg(theme.muted),
            )));
        }
        lines.push(Line::from(""));
        lines.push(section_title(
            "\u{f1de}",
            app.t(Key::DetailsSectionFundamentals),
        ));
        let label_width = detail_label_width(app);
        for (label, value) in fundamental_rows(live, app) {
            push_detail_row(
                &mut lines,
                label,
                value,
                label_width,
                theme.foreground,
                theme.muted,
            );
        }
        lines.push(Line::from(""));
    } else if app.live_details_loading {
        lines.push(section_title("\u{f201}", app.t(Key::DetailsSectionQuote)));
        lines.push(Line::from(Span::styled(
            app.t(Key::DetailsStatusLoading),
            Style::default().fg(theme.muted),
        )));
        lines.push(Line::from(""));
    }

    lines.push(section_title("\u{f129}", app.t(Key::DetailsSectionDetails)));
    let label_width = detail_label_width(app);
    for (label, value) in profile_rows(details, app) {
        push_detail_row(
            &mut lines,
            label,
            value,
            label_width,
            theme.foreground,
            theme.muted,
        );
    }

    let background = theme.background.unwrap_or(ratatui::style::Color::Black);
    let panel = Paragraph::new(lines)
        .style(Style::default().fg(theme.foreground).bg(background))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", app.t(Key::DetailsSectionDetails)))
                .style(Style::default().bg(background))
                .border_style(Style::default().fg(theme.accent)),
        );

    frame.render_widget(Clear, area);
    frame.render_widget(panel, area);
}

fn render_results(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let visible_rows = area.height.saturating_sub(3).max(1) as usize;

    if app.search_results.is_empty() {
        frame.render_widget(
            Paragraph::new(app.t(Key::SearchEmpty)).style(Style::default().fg(theme.muted)),
            area,
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
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.foreground)
            };
            let marker = if selected { ">" } else { " " };
            let sector = result.sector.as_deref().unwrap_or("-");
            let industry = result.industry.as_deref().unwrap_or("");
            let meta = if industry.is_empty() {
                sector.to_string()
            } else {
                format!("{sector} / {industry}")
            };

            Row::new(vec![
                Cell::from(marker),
                Cell::from(result.symbol.clone()),
                Cell::from(result.name.clone()),
                Cell::from(meta),
            ])
            .style(style)
        });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
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
        .style(Style::default().fg(theme.muted)),
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

fn filter_span(label: &str, active: bool, foreground: Color, muted: Color) -> Span<'static> {
    if active {
        Span::styled(
            format!("[{label}]"),
            Style::default().fg(foreground).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(format!(" {label} "), Style::default().fg(muted))
    }
}

fn section_title<'a>(icon: &'static str, title: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{icon} "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
    ])
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

fn quote_line<'a>(
    live: &LiveInstrumentDetails,
    app: &'a App,
    foreground: Color,
    muted: Color,
) -> Line<'a> {
    let price = live
        .price
        .map(format_price)
        .unwrap_or_else(|| "-".to_string());
    let change = price_change(live);
    let (change_text, change_color) = match change {
        Some((absolute, percent)) => {
            let sign = if absolute >= 0.0 { "+" } else { "" };
            let color = if absolute >= 0.0 {
                Color::LightGreen
            } else {
                Color::LightRed
            };
            (format!("{sign}{absolute:.2} ({sign}{percent:.2}%)"), color)
        }
        None => ("-".to_string(), muted),
    };

    let current_price_width = app.i18n.width(Key::DetailsLabelCurrentPrice).max(20);
    let change_width = app.i18n.width(Key::DetailsLabelChange).max(12);

    Line::from(vec![
        Span::styled(
            format!(
                "{}",
                pad_right(app.t(Key::DetailsLabelCurrentPrice), current_price_width)
            ),
            Style::default().fg(muted),
        ),
        Span::styled(
            format!("{price:<14}"),
            Style::default().fg(foreground).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            pad_right(app.t(Key::DetailsLabelChange), change_width),
            Style::default().fg(muted),
        ),
        Span::styled(change_text, Style::default().fg(change_color)),
    ])
}

fn fundamental_rows<'a>(
    live: &LiveInstrumentDetails,
    app: &'a App,
) -> Vec<(&'a str, Option<String>)> {
    vec![
        (
            app.t(Key::DetailsLabelPreviousClose),
            live.previous_close.map(format_price),
        ),
        (
            app.t(Key::DetailsLabelWeekHigh),
            live.week_52_high.map(format_price),
        ),
        (
            app.t(Key::DetailsLabelWeekLow),
            live.week_52_low.map(format_price),
        ),
        (
            app.t(Key::DetailsLabelMarketCap),
            live.market_cap.map(format_large_number),
        ),
        (
            app.t(Key::DetailsLabelAvgVolume),
            live.avg_volume.map(format_large_number),
        ),
        (
            app.t(Key::DetailsLabelPeRatio),
            live.trailing_pe.map(format_ratio),
        ),
        (
            app.t(Key::DetailsLabelForwardPe),
            live.forward_pe.map(format_ratio),
        ),
        (
            app.t(Key::DetailsLabelDividendYield),
            live.dividend_yield.map(format_percent),
        ),
        (
            app.t(Key::DetailsLabelEarnings),
            live.next_earnings_days.map(|days| format_days(days, app)),
        ),
        (app.t(Key::DetailsLabelBeta), live.beta.map(format_ratio)),
        (app.t(Key::DetailsLabelCountry), live.country.clone()),
        (app.t(Key::DetailsLabelWebsite), live.website.clone()),
    ]
}

fn profile_rows<'a>(
    details: &crate::search::InstrumentDetails,
    app: &'a App,
) -> Vec<(&'a str, Option<String>)> {
    vec![
        (app.t(Key::DetailsLabelExchange), details.exchange.clone()),
        (app.t(Key::DetailsLabelType), details.asset_type.clone()),
        (app.t(Key::DetailsLabelSector), details.sector.clone()),
        (app.t(Key::DetailsLabelIndustry), details.industry.clone()),
        (app.t(Key::DetailsLabelCurrency), details.currency.clone()),
        (
            app.t(Key::DetailsLabelActive),
            Some(details.active.to_string()),
        ),
        (
            app.t(Key::DetailsLabelUpdated),
            details.last_updated.clone(),
        ),
    ]
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

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.len() + word.len() + 1 > width {
            lines.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
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

fn format_days(value: i64, app: &App) -> String {
    app.t(Key::DetailsValueDays)
        .replace("{count}", &value.to_string())
}

fn search_symbol_column_width(app: &App) -> u16 {
    app.i18n
        .width(Key::SearchHeaderSymbol)
        .max(8)
        .saturating_add(1) as u16
}

fn detail_label_width(app: &App) -> usize {
    [
        Key::DetailsLabelPreviousClose,
        Key::DetailsLabelWeekHigh,
        Key::DetailsLabelWeekLow,
        Key::DetailsLabelMarketCap,
        Key::DetailsLabelAvgVolume,
        Key::DetailsLabelPeRatio,
        Key::DetailsLabelForwardPe,
        Key::DetailsLabelDividendYield,
        Key::DetailsLabelEarnings,
        Key::DetailsLabelBeta,
        Key::DetailsLabelCountry,
        Key::DetailsLabelWebsite,
        Key::DetailsLabelExchange,
        Key::DetailsLabelType,
        Key::DetailsLabelSector,
        Key::DetailsLabelIndustry,
        Key::DetailsLabelCurrency,
        Key::DetailsLabelActive,
        Key::DetailsLabelUpdated,
    ]
    .into_iter()
    .map(|key| app.i18n.width(key))
    .max()
    .unwrap_or(20)
    .max(20)
    .saturating_add(2)
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

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn centered_percent_rect(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let width = area.width.saturating_mul(width_percent).saturating_div(100);
    let height = area
        .height
        .saturating_mul(height_percent)
        .saturating_div(100);
    centered_rect(area, width.max(80), height.max(24))
}
