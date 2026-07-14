use crate::{
    app::{App, PanelId},
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
    let rows = app.screener_rows();
    let block = Block::default()
        .title(format!(" Screener · {} ", app.screener.preset.label()))
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
            Paragraph::new(
                "No live watchlist symbols match this screen. Use ←/→ to change preset.",
            )
            .style(Style::default().fg(theme.muted))
            .block(block),
            area,
        );
        return;
    }
    let table_rows = rows.into_iter().enumerate().map(|(index, quote)| {
        Row::new([
            Cell::from(quote.symbol.clone()),
            Cell::from(format!("{:.2}", quote.price)),
            Cell::from(format!("{:+.2}%", quote.change_percent)),
            Cell::from(
                quote
                    .relative_volume
                    .map(|v| format!("{v:.1}x"))
                    .unwrap_or_else(|| "—".into()),
            ),
        ])
        .style(if index == app.screener.selection {
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::REVERSED)
        } else if quote.change_percent >= 0.0 {
            Style::default().fg(theme.positive)
        } else {
            Style::default().fg(theme.negative)
        })
    });
    frame.render_widget(
        Table::new(
            table_rows,
            [
                Constraint::Percentage(40),
                Constraint::Length(14),
                Constraint::Length(12),
                Constraint::Length(10),
            ],
        )
        .header(
            Row::new(["Symbol", "Price", "Change", "Rel vol"]).style(
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(block),
        area,
    );
}
