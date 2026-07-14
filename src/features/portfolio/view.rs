use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
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
    let title = if app.portfolio.syncing {
        " Portfolio · syncing… "
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
        let message = app.portfolio.status.as_deref().unwrap_or(
            "No broker portfolio connected. Run `apeterm broker connect`, then press r to sync.",
        );
        frame.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(theme.muted))
                .block(block),
            area,
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
            area,
        );
    }
}
