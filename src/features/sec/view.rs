use chrono::{DateTime, Local};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use crate::{
    app::{App, PanelId, SecTab},
    db,
    sec::types::{EntityKind, HoldingDeltaKind, SecEntity},
    theme::current_theme,
    ui::panel,
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

    panel::render_title(
        frame,
        app,
        chunks[0],
        panel_id,
        app.t(crate::i18n::Key::PanelTitleSec),
    );
    render_tabs(frame, app, chunks[1]);

    let Ok(connection) = db::open(&app.ticker_db_path) else {
        frame.render_widget(
            Paragraph::new("SEC database unavailable").style(Style::default().fg(theme.muted)),
            chunks[2],
        );
        return;
    };

    let entities = match app.sec.tab {
        SecTab::Institutional => {
            crate::features::sec::repo::list_entities(&connection, EntityKind::Institution)
        }
        SecTab::Ceos => crate::features::sec::repo::list_ceo_entities(&connection, false),
        SecTab::Congress => crate::features::sec::repo::list_ceo_entities(&connection, true),
    };
    let entities = entities.unwrap_or_default();
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
    let selected_index = app
        .active_sec_selection()
        .min(entities.len().saturating_sub(1));
    let selected_entity = &entities[selected_index];

    render_watchlist(frame, app, panes[0], &connection, &entities, selected_index);
    match app.sec.tab {
        SecTab::Institutional => {
            render_institution_detail(frame, app, panes[1], &connection, selected_entity)
        }
        SecTab::Ceos => render_ceo_detail(frame, app, panes[1], &connection, selected_entity),
        SecTab::Congress => {
            render_congress_detail(frame, app, panes[1], &connection, selected_entity)
        }
    }

    let footer = app
        .sec
        .status
        .as_deref()
        .unwrap_or("[tab] panels  [←/→] tabs  [↑↓] navigate  [r] refresh");
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().fg(theme.muted)),
        chunks[3],
    );
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let tabs = [
        (SecTab::Institutional, "INSTITUTIONAL"),
        (SecTab::Ceos, "CEOS"),
        (SecTab::Congress, "CONGRESS"),
    ];
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
                let selected = app.sec.tab == tab;
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
    let title = match app.sec.tab {
        SecTab::Institutional => " institutions ",
        SecTab::Ceos => " insiders ",
        SecTab::Congress => " members ",
    };
    let inner = area;

    let visible_height = inner.height as usize;
    if visible_height == 0 {
        return;
    }
    let mut lines = vec![Line::from(Span::styled(
        title.trim(),
        Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
    ))];
    let visible_height = visible_height.saturating_sub(1);
    if visible_height == 0 {
        frame.render_widget(Paragraph::new(lines), inner);
        return;
    }
    let scroll = selected_index
        .saturating_sub(visible_height.saturating_sub(1))
        .min(entities.len().saturating_sub(visible_height));

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
        let glyph = match app.sec.tab {
            SecTab::Institutional => {
                let deltas = crate::features::sec::repo::holding_deltas(connection, entity.id)
                    .unwrap_or_default();
                if deltas
                    .iter()
                    .any(|delta| delta.kind != HoldingDeltaKind::Unchanged)
                {
                    Span::styled("▲ ", Style::default().fg(theme.positive))
                } else {
                    Span::styled("• ", Style::default().fg(theme.muted))
                }
            }
            SecTab::Ceos => {
                let code =
                    crate::features::sec::repo::latest_transaction_code(connection, entity.id)
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
            SecTab::Congress => {
                let code = crate::features::sec::repo::latest_congress_transaction_type(
                    connection, entity.id,
                )
                .ok()
                .flatten()
                .unwrap_or_default();
                let (glyph, color) = if code.starts_with('P') {
                    ("● ", theme.positive)
                } else if code.starts_with('S') {
                    ("○ ", NEGATIVE)
                } else {
                    ("• ", theme.accent)
                };
                Span::styled(glyph, Style::default().fg(color))
            }
        };
        let mut spans = vec![glyph];
        if app.sec.tab == SecTab::Congress {
            let name_width = entities
                .iter()
                .map(|value| value.name.len())
                .max()
                .unwrap_or(10)
                .min(inner.width.saturating_sub(10) as usize);
            spans.push(Span::styled(
                pad_right(&entity.name, name_width + 1),
                base_style,
            ));
            if let Some(subtitle) = &entity.subtitle {
                spans.push(Span::styled(
                    subtitle.clone(),
                    Style::default().fg(theme.muted),
                ));
            }
        } else {
            spans.push(Span::styled(entity.name.clone(), base_style));
            if let Some(subtitle) = &entity.subtitle {
                spans.push(Span::styled(
                    format!("  {subtitle}"),
                    Style::default().fg(theme.muted),
                ));
            }
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), inner);
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
        .constraints([
            Constraint::Length(4),
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .split(area);

    render_detail_header(frame, app, sections[0], connection, entity);

    let history = crate::features::sec::repo::portfolio_value_history(connection, entity.id)
        .unwrap_or_default();
    let current_total = history
        .last()
        .map(|(_, value)| thirteenf_value_to_usd(*value))
        .unwrap_or(0.0);
    let previous_total = history
        .iter()
        .rev()
        .nth(1)
        .map(|(_, value)| thirteenf_value_to_usd(*value));
    let total_change = previous_total
        .filter(|value| *value > 0.0)
        .map(|value| (current_total - value) / value * 100.0);

    let holdings =
        crate::features::sec::repo::latest_holdings(connection, entity.id).unwrap_or_default();
    let deltas =
        crate::features::sec::repo::holding_deltas(connection, entity.id).unwrap_or_default();
    let delta_map = deltas
        .iter()
        .cloned()
        .map(|delta| (delta.cusip.clone(), delta))
        .collect::<std::collections::HashMap<_, _>>();
    let total = holdings.iter().map(|row| row.value_usd).sum::<i64>().max(1) as f64;
    let summary = summarize_holding_deltas(&deltas);
    let top_10_weight = holdings
        .iter()
        .take(10)
        .map(|row| row.value_usd as f64 / total * 100.0)
        .sum::<f64>();
    let largest_position = holdings.first().map(|row| {
        format!(
            "{} {:>4.1}%",
            row.ticker.clone().unwrap_or_else(|| row.cusip.clone()),
            row.value_usd as f64 / total * 100.0
        )
    });

    let summary_lines = vec![
        Line::from(vec![
            Span::styled("13F value ", Style::default().fg(theme.muted)),
            Span::styled(
                format_currency(current_total),
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Positions ", Style::default().fg(theme.muted)),
            Span::styled(
                holdings.len().to_string(),
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("QoQ ", Style::default().fg(theme.muted)),
            Span::styled(
                total_change
                    .map(|value| format!("{value:+.1}%"))
                    .unwrap_or_else(|| "-".to_string()),
                Style::default().fg(match total_change {
                    Some(value) if value > 0.0 => theme.positive,
                    Some(value) if value < 0.0 => NEGATIVE,
                    _ => theme.muted,
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Largest ", Style::default().fg(theme.muted)),
            Span::styled(
                largest_position.unwrap_or_else(|| "-".to_string()),
                Style::default().fg(theme.foreground),
            ),
            Span::raw("  "),
            Span::styled("Top 10 ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{top_10_weight:.1}%"),
                Style::default().fg(theme.foreground),
            ),
        ]),
        Line::from(vec![
            Span::styled("Top add ", Style::default().fg(theme.muted)),
            Span::styled(
                summary.top_add.unwrap_or_else(|| "-".to_string()),
                Style::default().fg(theme.positive),
            ),
            Span::raw("  "),
            Span::styled("Top cut ", Style::default().fg(theme.muted)),
            Span::styled(
                summary.top_cut.unwrap_or_else(|| "-".to_string()),
                Style::default().fg(NEGATIVE),
            ),
        ]),
        Line::from(vec![
            Span::styled("New ", Style::default().fg(theme.muted)),
            Span::styled(
                summary.bought.join(", ").if_empty_then("-"),
                Style::default().fg(theme.foreground),
            ),
            Span::raw("  "),
            Span::styled("Exited ", Style::default().fg(theme.muted)),
            Span::styled(
                summary.sold.join(", ").if_empty_then("-"),
                Style::default().fg(theme.foreground),
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(summary_lines).wrap(Wrap { trim: true }),
        sections[1],
    );

    let rows = holdings.into_iter().map(|row| {
        let delta = delta_map.get(&row.cusip);
        let delta_text = delta
            .map(|value| match value.kind {
                HoldingDeltaKind::New => "New".to_string(),
                HoldingDeltaKind::Increased => {
                    format!("+{}", value.current_shares - value.previous_shares)
                }
                HoldingDeltaKind::Decreased => {
                    format!("{}", value.current_shares - value.previous_shares)
                }
                HoldingDeltaKind::Exited => "Exit".to_string(),
                HoldingDeltaKind::Unchanged => "0".to_string(),
            })
            .unwrap_or_else(|| "0".to_string());
        let delta_style = match delta.map(|value| value.kind) {
            Some(HoldingDeltaKind::New | HoldingDeltaKind::Increased) => {
                Style::default().fg(theme.positive)
            }
            Some(HoldingDeltaKind::Decreased | HoldingDeltaKind::Exited) => {
                Style::default().fg(NEGATIVE)
            }
            _ => Style::default().fg(theme.muted),
        };
        Row::new(vec![
            Cell::from(row.ticker.unwrap_or_else(|| row.cusip.clone())),
            Cell::from(format_compact_number(row.shares as f64)),
            Cell::from(format_currency(thirteenf_value_to_usd(row.value_usd))),
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
    .header(
        Row::new(vec!["Ticker/Name", "Shares", "Value", "Delta", "% Port"]).style(
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::BOLD),
        ),
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

    let rows = crate::features::sec::repo::recent_insider_txs(connection, entity.id, 20)
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
                Cell::from(
                    tx.price_usd
                        .map(format_currency)
                        .unwrap_or_else(|| "-".to_string()),
                ),
                Cell::from(
                    value
                        .map(format_currency)
                        .unwrap_or_else(|| "-".to_string()),
                ),
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
    .header(
        Row::new(vec![
            "Date",
            "Type",
            "Shares",
            "Price",
            "Value",
            "Owned After",
        ])
        .style(
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .column_spacing(1);
    frame.render_widget(table, sections[1]);
}

fn render_congress_detail(
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
    let txs = crate::features::sec::repo::recent_congress_txs(connection, entity.id, 20)
        .unwrap_or_default();
    if txs.is_empty() {
        let message = if entity.subtitle.as_deref() == Some("Senate") {
            "Senate disclosures are not ingesting yet."
        } else {
            "No House PTR transactions synced yet."
        };
        frame.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(theme.muted))
                .wrap(Wrap { trim: true }),
            sections[1],
        );
        return;
    }

    let rows = txs.into_iter().map(|tx| {
        let style = if tx.transaction_type.starts_with('P') {
            Style::default().fg(theme.positive)
        } else if tx.transaction_type.starts_with('S') {
            Style::default().fg(NEGATIVE)
        } else {
            Style::default().fg(theme.muted)
        };
        Row::new(vec![
            Cell::from(tx.transaction_date),
            Cell::from(Line::from(Span::styled(tx.transaction_type, style))),
            Cell::from(tx.ticker.unwrap_or_else(|| "-".to_string())),
            Cell::from(tx.asset_name),
            Cell::from(tx.amount_range),
            Cell::from(tx.filed_at.unwrap_or_else(|| "-".to_string())),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Percentage(46),
            Constraint::Length(18),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec!["Date", "Type", "Ticker", "Asset", "Amount", "Filed"]).style(
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::BOLD),
        ),
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
    let last_synced = crate::features::sec::repo::last_polled_at(connection, entity.id)
        .ok()
        .flatten()
        .and_then(|value| parse_relative_time(&value))
        .unwrap_or_else(|| "never".to_string());
    let subtitle = entity
        .subtitle
        .as_deref()
        .unwrap_or(entity.filer_cik.as_str());
    let body = vec![
        Line::from(Span::styled(
            entity.name.as_str(),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
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
                if app.sec.loading {
                    "refreshing"
                } else {
                    "[r] refresh"
                },
                Style::default().fg(theme.accent),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(body).wrap(Wrap { trim: true }), area);
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
    if value.abs() >= 1_000_000_000_000.0 {
        format!("${:.1}T", value / 1_000_000_000_000.0)
    } else if value.abs() >= 1_000_000_000.0 {
        format!("${:.1}B", value / 1_000_000_000.0)
    } else if value.abs() >= 1_000_000.0 {
        format!("${:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("${:.1}K", value / 1_000.0)
    } else {
        format!("${value:.2}")
    }
}

fn thirteenf_value_to_usd(value: i64) -> f64 {
    value as f64 * 1_000.0
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

fn pad_right(value: &str, width: usize) -> String {
    let used = value.chars().count();
    format!("{value}{}", " ".repeat(width.saturating_sub(used)))
}

#[derive(Default)]
struct HoldingDeltaSummary {
    top_add: Option<String>,
    top_cut: Option<String>,
    bought: Vec<String>,
    sold: Vec<String>,
}

fn summarize_holding_deltas(
    deltas: &[crate::features::sec::types::HoldingDelta],
) -> HoldingDeltaSummary {
    let mut top_add: Option<(&str, i64)> = None;
    let mut top_cut: Option<(&str, i64)> = None;
    let mut bought = Vec::new();
    let mut sold = Vec::new();

    for delta in deltas {
        let label = delta.ticker.as_deref().unwrap_or(delta.cusip.as_str());
        let share_delta = delta.current_shares - delta.previous_shares;
        match delta.kind {
            HoldingDeltaKind::New => {
                if bought.len() < 3 {
                    bought.push(label.to_string());
                }
            }
            HoldingDeltaKind::Exited => {
                if sold.len() < 3 {
                    sold.push(label.to_string());
                }
            }
            HoldingDeltaKind::Increased => {
                if top_add.is_none_or(|(_, value)| share_delta > value) {
                    top_add = Some((label, share_delta));
                }
            }
            HoldingDeltaKind::Decreased => {
                if top_cut.is_none_or(|(_, value)| share_delta < value) {
                    top_cut = Some((label, share_delta));
                }
            }
            HoldingDeltaKind::Unchanged => {}
        }
    }

    HoldingDeltaSummary {
        top_add: top_add
            .map(|(label, value)| format!("{label} +{}", format_compact_number(value as f64))),
        top_cut: top_cut
            .map(|(label, value)| format!("{label} {}", format_compact_number(value as f64))),
        bought,
        sold,
    }
}

trait EmptyFallback {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl EmptyFallback for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}
