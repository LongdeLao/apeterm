use crate::{
    app::{App, PanelId},
    features::alerts::state::AlertDirection,
    theme::current_theme,
    ui,
};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
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
    let title = app
        .alerts
        .events
        .first()
        .map(|event| {
            format!(
                " Alerts & Activity · {} · {} ",
                event.timestamp, event.message
            )
        })
        .unwrap_or_else(|| " Alerts & Activity ".to_string());
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
    if app.alerts.rules.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No alerts. Press n for a quick +5% price alert, or ask the agent to create one.",
            )
            .style(Style::default().fg(theme.muted))
            .block(block),
            area,
        );
        return;
    }
    let rows = app.alerts.rules.iter().enumerate().map(|(index, rule)| {
        let style = if index == app.alerts.selection {
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(if rule.enabled {
                theme.foreground
            } else {
                theme.muted
            })
        };
        Row::new([
            Cell::from(if rule.enabled { "on" } else { "off" }),
            Cell::from(rule.symbol.clone()),
            Cell::from(match rule.direction {
                AlertDirection::Above => ">=",
                AlertDirection::Below => "<=",
                AlertDirection::VolumeAbove => "RV>=",
            }),
            Cell::from(format!("{:.2}", rule.threshold)),
            Cell::from(if rule.triggered { "triggered" } else { "armed" }),
        ])
        .style(style)
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(5),
                Constraint::Length(12),
                Constraint::Length(4),
                Constraint::Length(12),
                Constraint::Min(8),
            ],
        )
        .header(
            Row::new(["State", "Symbol", "", "Threshold", "Status"]).style(
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(block),
        area,
    );
}
