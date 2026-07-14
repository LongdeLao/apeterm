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
    let block = Block::default()
        .title(" Compare ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(
            if panel_id.is_some_and(|id| app.is_panel_focused(id)) {
                theme.accent
            } else {
                theme.muted
            },
        ));
    if app.compare.symbols.is_empty() {
        frame.render_widget(
            Paragraph::new("No symbols selected. Ask the agent to compare 2–5 tickers.")
                .style(Style::default().fg(theme.muted))
                .block(block),
            area,
        );
        return;
    }
    let rows = app
        .compare
        .symbols
        .iter()
        .enumerate()
        .map(|(index, symbol)| {
            let quote = app
                .watchlist
                .stock_quotes
                .iter()
                .find(|quote| quote.symbol.eq_ignore_ascii_case(symbol));
            let (price, change, volume) = quote
                .map(|quote| {
                    (
                        format!("{:.2}", quote.price),
                        format!("{:+.2}%", quote.change_percent),
                        quote
                            .relative_volume
                            .map(|v| format!("{v:.1}x"))
                            .unwrap_or_else(|| "—".into()),
                    )
                })
                .unwrap_or_else(|| ("—".into(), "—".into(), "—".into()));
            Row::new([
                Cell::from(symbol.clone()),
                Cell::from(price),
                Cell::from(change),
                Cell::from(volume),
            ])
            .style(if index == app.compare.selection {
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(theme.foreground)
            })
        });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Percentage(40),
                Constraint::Length(14),
                Constraint::Length(12),
                Constraint::Length(12),
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
