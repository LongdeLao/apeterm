use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::{
    app::{App, PanelId},
    theme::current_theme,
    ui,
};

pub fn render(frame: &mut Frame, app: &App) {
    render_area(frame, app, ui::content_area(frame.area(), app), None);
}

pub fn render_panel(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    if app.is_panel_open(panel_id) {
        render_area(frame, app, area, Some(panel_id));
    }
}

fn render_area(frame: &mut Frame, app: &App, area: Rect, panel_id: Option<PanelId>) {
    let theme = current_theme(app.theme_name);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if panel_id.is_some() { 0 } else { 5 }),
            Constraint::Min(0),
        ])
        .split(area);
    if panel_id.is_none() {
        render_broker_header(frame, app, chunks[0]);
    }

    let title = if app.portfolio.syncing {
        " Portfolio - syncing... "
    } else {
        " Portfolio "
    };
    let rows = app
        .portfolio
        .snapshot
        .as_ref()
        .into_iter()
        .flat_map(|snapshot| {
            snapshot
                .positions
                .iter()
                .enumerate()
                .map(|(index, position)| {
                    let pnl = position.net_value - position.cost_value;
                    let style = if index == app.portfolio.selection {
                        Style::default()
                            .fg(theme.foreground)
                            .add_modifier(Modifier::REVERSED)
                    } else if pnl >= 0.0 {
                        Style::default().fg(theme.positive)
                    } else {
                        Style::default().fg(theme.negative)
                    };
                    Row::new(vec![
                        Cell::from(
                            position
                                .symbol
                                .as_deref()
                                .unwrap_or(&position.name)
                                .to_string(),
                        ),
                        Cell::from(format!("{:.4}", position.quantity)),
                        Cell::from(format!("{:.2}", position.net_value)),
                        Cell::from(format!("{pnl:+.2}")),
                    ])
                    .style(style)
                })
        });
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if panel_id.is_some_and(|id| app.is_panel_focused(id)) {
                theme.accent
            } else {
                theme.muted
            },
        ));
    if app.portfolio.snapshot.is_none() {
        let message = app
            .portfolio
            .status
            .as_deref()
            .unwrap_or("No broker portfolio connected. Press c to connect Trade Republic.");
        frame.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(theme.muted))
                .block(block),
            chunks[1],
        );
    } else {
        frame.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Percentage(44),
                    Constraint::Length(12),
                    Constraint::Length(14),
                    Constraint::Length(13),
                ],
            )
            .header(
                Row::new(["Position", "Quantity", "Value", "P/L"]).style(
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .block(block),
            chunks[1],
        );
    }
}

fn render_broker_header(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    let theme = current_theme(app.theme_name);
    let status = if app.config.broker.trade_republic_enabled {
        "connected"
    } else {
        "not connected"
    };
    let sync = if app.portfolio.syncing {
        "syncing"
    } else {
        "idle"
    };
    let last = app
        .portfolio
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.synced_at.as_str())
        .unwrap_or("never");
    let status_line = app.portfolio.status.as_deref().unwrap_or(
        "Press c to connect, r to sync, d to disconnect. pytr keeps credentials in ~/.pytr.",
    );
    let lines = vec![
        Line::from(vec![
            Span::styled(
                "Trade Republic",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("  {status}  {sync}")),
        ]),
        Line::from(format!("Last sync: {last}")),
        Line::from(status_line.to_string()),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(theme.foreground))
            .block(
                Block::default()
                    .title(" Broker ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(
                        if app.config.broker.trade_republic_enabled {
                            theme.positive
                        } else {
                            Color::Rgb(120, 120, 120)
                        },
                    )),
            ),
        area,
    );
}
