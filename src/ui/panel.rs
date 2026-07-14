use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    app::{App, PanelId},
    theme::current_theme,
};

#[allow(dead_code)]
pub fn render(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    panel_id: PanelId,
    title: &str,
    lines: &[&str],
) {
    if !app.is_panel_open(panel_id) {
        return;
    }

    let theme = current_theme(app.theme_name);
    let inner = content_area(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    render_title(frame, app, chunks[0], panel_id, title);

    let body = Paragraph::new(lines.join("\n")).style(Style::default().fg(theme.muted));

    frame.render_widget(body, chunks[1]);
}

pub fn render_title(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId, title: &str) {
    let theme = current_theme(app.theme_name);
    let title_style = if app.is_panel_focused(panel_id) {
        Style::default()
            .fg(theme.foreground)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    };

    let title = Paragraph::new(Line::from(Span::styled(format!(" {title} "), title_style)));
    frame.render_widget(title, area);
}

pub fn content_area(area: Rect) -> Rect {
    inset(area, 2, 1)
}

fn inset(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal),
        y: area.y.saturating_add(vertical),
        width: area.width.saturating_sub(horizontal * 2),
        height: area.height.saturating_sub(vertical * 2),
    }
}
