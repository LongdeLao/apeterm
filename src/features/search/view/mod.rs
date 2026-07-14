use crate::ui::util::{background_style, format_compact_number, pad_right};
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

mod chart;
mod format;
mod headlines;
mod sidebar;

use chart::*;
use format::*;
use headlines::*;
use sidebar::*;

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
                app.search.query.as_str(),
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            filter_span(
                app.t(Key::SearchFilterStocks),
                app.search.asset_kind == SearchAssetKind::Stocks,
                theme,
            ),
            Span::raw("  "),
            filter_span(
                app.t(Key::SearchFilterEtfs),
                app.search.asset_kind == SearchAssetKind::Etfs,
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
                prompt_width as u16 + UnicodeWidthStr::width(app.search.query.as_str()) as u16,
            ),
            chunks[0].y,
        ));
    }

    if let Some(message) = &app.search.message {
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
    let Some(details) = &app.search.selected_details else {
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

fn render_results(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let inner = area;

    let visible_rows = inner.height.saturating_sub(1).max(1) as usize;

    if app.search.results.is_empty() {
        frame.render_widget(
            Paragraph::new(app.t(Key::SearchEmpty)).style(Style::default().fg(theme.muted)),
            inner,
        );
        return;
    }

    let scroll = visible_scroll_start(app, visible_rows);
    let end = scroll
        .saturating_add(visible_rows)
        .min(app.search.results.len());
    let rows = app.search.results[scroll..end]
        .iter()
        .enumerate()
        .map(|(offset, result)| {
            let index = scroll + offset;
            let selected = index == app.search.selection;
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
                .replace("{total}", &app.search.results.len().to_string()),
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

fn search_symbol_column_width(app: &App) -> u16 {
    app.i18n
        .width(Key::SearchHeaderSymbol)
        .max(8)
        .saturating_add(1) as u16
}

fn visible_scroll_start(app: &App, visible_rows: usize) -> usize {
    if visible_rows == 0 {
        return app.search.selection.min(app.search.results.len());
    }

    let mut scroll = app.search.scroll.min(app.search.results.len());
    if app.search.selection < scroll {
        scroll = app.search.selection;
    } else if app.search.selection >= scroll + visible_rows {
        scroll = app.search.selection + 1 - visible_rows;
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

    #[test]
    fn one_day_history_uses_intraday_points_when_available() {
        let mut app = detail_app();
        app.search.detail_timeframe = DetailTimeframe::OneDay;
        let start = Local
            .with_ymd_and_hms(2026, 7, 2, 13, 30, 0)
            .single()
            .expect("date")
            .timestamp();
        let mut history = vec![
            HistoryPoint {
                ts: start - 2 * 86_400,
                close: 197.0,
                volume: Some(100_000_000.0),
            },
            HistoryPoint {
                ts: start - 86_400,
                close: 196.0,
                volume: Some(100_000_000.0),
            },
            HistoryPoint {
                ts: start - 6 * 60 * 60,
                close: 199.0,
                volume: Some(100_000_000.0),
            },
        ];
        history.extend((0..120).map(|minute| HistoryPoint {
            ts: start + minute * 60,
            close: 195.0 + minute as f64 * 0.01,
            volume: Some(250_000.0),
        }));
        app.search
            .selected_live_details
            .as_mut()
            .expect("live details")
            .history = history;

        let visible = visible_history(&app);

        assert_eq!(visible.len(), 120);
        assert_eq!(visible.first().expect("first").ts, start);
        assert_eq!(
            format_ts_label_for_timeframe(start, DetailTimeframe::OneDay).len(),
            5
        );
    }

    fn detail_app() -> App {
        let mut app = App::new(AppConfig::default().expect("default config"));
        app.page = Page::Details;
        app.search.selected_details = Some(InstrumentDetails {
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
        app.search.selected_live_details = Some(LiveInstrumentDetails {
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
        app.search.backend_insight = Some(BackendInsight {
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
