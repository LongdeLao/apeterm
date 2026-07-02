use chrono::{DateTime, Local};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table, Wrap},
};

use crate::{
    app::{App, PanelId, SecTab},
    db,
    pages::panel,
    sec::types::{EntityKind, HoldingDeltaKind, SecEntity},
    theme::current_theme,
};

const NEGATIVE: Color = Color::Rgb(239, 68, 68);

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

    panel::render_title(frame, app, chunks[0], panel_id, app.t(crate::i18n::Key::PanelTitleSec));
    render_tabs(frame, app, chunks[1]);

    let Ok(connection) = db::open(&app.ticker_db_path) else {
        frame.render_widget(
            Paragraph::new("SEC database unavailable").style(Style::default().fg(theme.muted)),
            chunks[2],
        );
        return;
    };

    let kind = match app.sec_tab {
        SecTab::Institutional => EntityKind::Institution,
        SecTab::Ceos => EntityKind::Ceo,
    };
    let entities = db::sec_repo::list_entities(&connection, kind).unwrap_or_default();
    if entities.is_empty() {
        frame.render_widget(
            Paragraph::new("No SEC watchlist entities seeded")
                .style(Style::default().fg(theme.muted)),
            chunks[2],
        );
        return;
    }

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[2]);
    let selected_index = app.active_sec_selection().min(entities.len().saturating_sub(1));
    let selected_entity = &entities[selected_index];

    render_watchlist(frame, app, panes[0], &connection, &entities, selected_index);
    match app.sec_tab {
        SecTab::Institutional => render_institution_detail(frame, app, panes[1], &connection, selected_entity),
        SecTab::Ceos => render_ceo_detail(frame, app, panes[1], &connection, selected_entity),
    }

    let footer = app
        .sec_status
        .as_deref()
        .unwrap_or("[tab] panels  [←/→] tabs  [↑↓] navigate  [r] refresh");
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().fg(theme.muted)),
        chunks[3],
    );
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let tabs = [(SecTab::Institutional, "INSTITUTIONAL"), (SecTab::Ceos, "CEOS")];
    frame.render_widget(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme.muted)),
        area,
    );

    let line = Line::from(
        tabs.into_iter()
            .enumerate()
            .flat_map(|(index, (tab, label))| {
                let selected = app.sec_tab == tab;
                let style = if selected {
                    Style::default()
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(theme.muted)
                };
                let mut spans = vec![Span::styled(format!(" {label} "), style)];
                if index == 0 {
                    spans.push(Span::raw("  "));
                }
                spans
            })
            .collect::<Vec<_>>(),
    );
    frame.render_widget(Paragraph::new(line), area);
}

fn render_watchlist(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    connection: &rusqlite::Connection,
    entities: &[SecEntity],
    selected_index: usize,
) {
    let theme = current_theme(app.theme_name);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" watchlist ")
        .border_style(Style::default().fg(theme.muted));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_height = inner.height as usize;
    if visible_height == 0 {
        return;
    }
    let scroll = selected_index
        .saturating_sub(visible_height.saturating_sub(1))
        .min(entities.len().saturating_sub(visible_height));

    let mut lines = Vec::new();
    for (index, entity) in entities
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
    {
        let selected = index == selected_index;
        let base_style = if selected {
            Style::default().bg(theme.surface).fg(theme.foreground)
        } else {
            Style::default().fg(theme.foreground)
        };
        let glyph = match app.sec_tab {
            SecTab::Institutional => {
                let deltas = db::sec_repo::holding_deltas(connection, entity.id).unwrap_or_default();
                if deltas.iter().any(|delta| delta.kind != HoldingDeltaKind::Unchanged) {
                    Span::styled("▲ ", Style::default().fg(theme.positive))
                } else {
                    Span::styled("• ", Style::default().fg(theme.muted))
                }
            }
            SecTab::Ceos => {
                let code = db::sec_repo::latest_transaction_code(connection, entity.id)
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                let (glyph, color) = if code == "P" {
                    ("● ", theme.positive)
                } else if code == "S" {
                    ("○ ", NEGATIVE)
                } else {
                    ("• ", theme.muted)
                };
                Span::styled(glyph, Style::default().fg(color))
            }
        };
        let mut spans = vec![glyph, Span::styled(entity.name.clone(), base_style)];
        if let Some(subtitle) = &entity.subtitle {
            spans.push(Span::styled(
                format!("  {subtitle}"),
                Style::default().fg(theme.muted),
            ));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }),
        inner,
    );
}

fn render_institution_detail(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    connection: &rusqlite::Connection,
    entity: &SecEntity,
) {
    let theme = current_theme(app.theme_name);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    render_detail_header(frame, app, sections[0], connection, entity);

    let history = db::sec_repo::portfolio_value_history(connection, entity.id).unwrap_or_default();
    let values = history
        .iter()
        .map(|(_, value)| (*value as u64).max(1))
        .collect::<Vec<_>>();
    frame.render_widget(
        Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" portfolio value ")
                    .border_style(Style::default().fg(theme.muted)),
            )
            .data(&values)
            .style(Style::default().fg(theme.positive)),
        sections[1],
    );

    let holdings = db::sec_repo::latest_holdings(connection, entity.id).unwrap_or_default();
    let deltas = db::sec_repo::holding_deltas(connection, entity.id).unwrap_or_default();
    let delta_map = deltas
        .into_iter()
        .map(|delta| (delta.cusip.clone(), delta))
        .collect::<std::collections::HashMap<_, _>>();
    let total = holdings.iter().map(|row| row.value_usd).sum::<i64>().max(1) as f64;

    let rows = holdings.into_iter().map(|row| {
        let delta = delta_map.get(&row.cusip);
        let delta_text = delta
            .map(|value| match value.kind {
                HoldingDeltaKind::New => "New".to_string(),
                HoldingDeltaKind::Increased => format!("+{}", value.current_shares - value.previous_shares),
                HoldingDeltaKind::Decreased => format!("{}", value.current_shares - value.previous_shares),
                HoldingDeltaKind::Exited => "Exit".to_string(),
                HoldingDeltaKind::Unchanged => "0".to_string(),
            })
            .unwrap_or_else(|| "0".to_string());
        let delta_style = match delta.map(|value| value.kind) {
            Some(HoldingDeltaKind::New | HoldingDeltaKind::Increased) => Style::default().fg(theme.positive),
            Some(HoldingDeltaKind::Decreased | HoldingDeltaKind::Exited) => Style::default().fg(NEGATIVE),
            _ => Style::default().fg(theme.muted),
        };
        Row::new(vec![
            Cell::from(row.ticker.unwrap_or_else(|| row.cusip.clone())),
            Cell::from(format_compact_number(row.shares as f64)),
            Cell::from(format_currency(row.value_usd as f64)),
            Cell::from(Line::from(Span::styled(delta_text, delta_style))),
            Cell::from(format!("{:>5.1}%", row.value_usd as f64 / total * 100.0)),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Length(12),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" latest holdings ")
            .border_style(Style::default().fg(theme.muted)),
    )
    .header(
        Row::new(vec!["Ticker/Name", "Shares", "Value", "Delta", "% Port"])
            .style(Style::default().fg(theme.muted).add_modifier(Modifier::BOLD)),
    )
    .column_spacing(1);
    frame.render_widget(table, sections[2]);
}

fn render_ceo_detail(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    connection: &rusqlite::Connection,
    entity: &SecEntity,
) {
    let theme = current_theme(app.theme_name);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(area);
    render_detail_header(frame, app, sections[0], connection, entity);

    let rows = db::sec_repo::recent_insider_txs(connection, entity.id, 20)
        .unwrap_or_default()
        .into_iter()
        .map(|tx| {
            let value = tx.price_usd.map(|price| price * tx.shares);
            let style = match tx.code.as_str() {
                "P" => Style::default().fg(theme.positive),
                "S" => Style::default().fg(NEGATIVE),
                _ => Style::default().fg(theme.muted),
            };
            Row::new(vec![
                Cell::from(tx.transaction_date),
                Cell::from(Line::from(Span::styled(tx.code, style))),
                Cell::from(format_compact_number(tx.shares)),
                Cell::from(tx.price_usd.map(format_currency).unwrap_or_else(|| "-".to_string())),
                Cell::from(value.map(format_currency).unwrap_or_else(|| "-".to_string())),
                Cell::from(
                    tx.shares_owned_after
                        .map(format_compact_number)
                        .unwrap_or_else(|| "-".to_string()),
                ),
            ])
        });

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Length(16),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" recent form 4 transactions ")
            .border_style(Style::default().fg(theme.muted)),
    )
    .header(
        Row::new(vec!["Date", "Type", "Shares", "Price", "Value", "Owned After"])
            .style(Style::default().fg(theme.muted).add_modifier(Modifier::BOLD)),
    )
    .column_spacing(1);
    frame.render_widget(table, sections[1]);
}

fn render_detail_header(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    connection: &rusqlite::Connection,
    entity: &SecEntity,
) {
    let theme = current_theme(app.theme_name);
    let last_synced = db::sec_repo::last_polled_at(connection, entity.id)
        .ok()
        .flatten()
        .and_then(|value| parse_relative_time(&value))
        .unwrap_or_else(|| "never".to_string());
    let subtitle = entity.subtitle.as_deref().unwrap_or(entity.filer_cik.as_str());
    let body = vec![
        Line::from(Span::styled(
            entity.name.as_str(),
            Style::default().fg(theme.foreground).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(subtitle, Style::default().fg(theme.muted)),
            Span::raw("  "),
            Span::styled(
                format!("Last synced: {last_synced}"),
                Style::default().fg(theme.muted),
            ),
            Span::raw("  "),
            Span::styled(
                if app.sec_loading { "refreshing" } else { "[r] refresh" },
                Style::default().fg(theme.accent),
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(body)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" detail ")
                    .border_style(Style::default().fg(theme.muted)),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn parse_relative_time(value: &str) -> Option<String> {
    let timestamp = DateTime::parse_from_rfc3339(value).ok()?;
    let delta = Local::now().signed_duration_since(timestamp.with_timezone(&Local));
    Some(if delta.num_minutes() < 1 {
        "now".to_string()
    } else if delta.num_minutes() < 60 {
        format!("{}m ago", delta.num_minutes())
    } else if delta.num_hours() < 24 {
        format!("{}h ago", delta.num_hours())
    } else {
        format!("{}d ago", delta.num_days())
    })
}

fn format_currency(value: f64) -> String {
    if value.abs() >= 1_000_000_000.0 {
        format!("${:.1}B", value / 1_000_000_000.0)
    } else if value.abs() >= 1_000_000.0 {
        format!("${:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("${:.1}K", value / 1_000.0)
    } else {
        format!("${value:.2}")
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
