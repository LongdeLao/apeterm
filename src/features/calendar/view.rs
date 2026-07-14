use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::{app::App, app::PanelId, theme::current_theme, ui};

pub fn render(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    if app.is_panel_open(panel_id) {
        render_area(frame, app, area, Some(panel_id));
    }
}

pub fn render_page(frame: &mut Frame, app: &App) {
    render_area(frame, app, ui::content_area(frame.area(), app), None);
}

fn render_area(frame: &mut Frame, app: &App, area: Rect, panel_id: Option<PanelId>) {
    let theme = current_theme(app.theme_name);
    let rows = app.calendar_rows();
    let title = if app.calendar.watchlist_only {
        " Calendar · watchlist "
    } else {
        " Calendar · all market events "
    };
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
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No dated events yet. Refresh News, or press f to change scope.")
                .style(Style::default().fg(theme.muted))
                .block(block),
            area,
        );
        return;
    }
    let table_rows = rows
        .into_iter()
        .enumerate()
        .map(|(index, (date, symbol, title))| {
            Row::new([Cell::from(date), Cell::from(symbol), Cell::from(title)]).style(
                if index == app.calendar.selection {
                    Style::default()
                        .fg(theme.foreground)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(theme.foreground)
                },
            )
        });
    frame.render_widget(
        Table::new(
            table_rows,
            [
                Constraint::Length(17),
                Constraint::Length(10),
                Constraint::Min(20),
            ],
        )
        .header(
            Row::new(["Date", "Symbol", "Event"]).style(
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(block),
        area,
    );
}
