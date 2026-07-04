use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{
    app::App,
    features::spotlight::engine::{MAX_RESULTS, SpotlightCategory},
    i18n::Key,
    theme::current_theme,
};

pub fn render(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    let visible_rows = app.spotlight.results.len().min(MAX_RESULTS).max(1);
    let height = (visible_rows as u16).saturating_add(4);
    let area = centered_rect(frame.area(), 60, height);

    let modal_background = theme.background.unwrap_or(Color::Rgb(0, 0, 0));

    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(modal_background)),
        area,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(modal_background))
        .border_style(Style::default().fg(theme.accent))
        .title(app.t(Key::SpotlightTitle));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let input_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let results_area = Rect::new(
        inner.x,
        inner.y.saturating_add(2),
        inner.width,
        inner.height.saturating_sub(2),
    );

    frame.render_widget(
        Paragraph::new(format!("> {}", app.spotlight.query))
            .style(Style::default().fg(theme.foreground).bg(modal_background)),
        input_area,
    );

    render_results(frame, app, results_area, modal_background);
}

fn render_results(frame: &mut Frame, app: &App, area: Rect, modal_background: Color) {
    let theme = current_theme(app.theme_name);

    if app.spotlight.results.is_empty() {
        frame.render_widget(
            Paragraph::new("No matches")
                .style(Style::default().fg(theme.muted).bg(modal_background)),
            area,
        );
        return;
    }

    let lines: Vec<ratatui::text::Line> = app
        .spotlight
        .results
        .iter()
        .enumerate()
        .take(area.height as usize)
        .map(|(index, result)| {
            let selected = index == app.spotlight.selection;
            let row_style = if selected {
                Style::default()
                    .fg(theme.foreground)
                    .bg(theme.surface)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.foreground).bg(modal_background)
            };
            let badge_color = match result.category {
                SpotlightCategory::Symbol => theme.positive,
                SpotlightCategory::Panel => theme.accent,
                SpotlightCategory::Action => theme.warning,
            };
            let marker = if selected { "▌" } else { " " };
            let mut spans = vec![
                ratatui::text::Span::styled(marker, Style::default().fg(theme.accent)),
                ratatui::text::Span::styled(
                    format!(" {:<6}", result.category.badge()),
                    Style::default()
                        .fg(badge_color)
                        .add_modifier(Modifier::BOLD),
                ),
                ratatui::text::Span::styled(result.label.clone(), row_style),
            ];
            if let Some(subtitle) = &result.subtitle {
                spans.push(ratatui::text::Span::styled(
                    format!("  {subtitle}"),
                    Style::default().fg(theme.muted),
                ));
            }
            ratatui::text::Line::from(spans).style(Style::default().bg(modal_background))
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Left), area);
}

fn centered_rect(area: Rect, width_percent: u16, height: u16) -> Rect {
    let width = (area.width.saturating_mul(width_percent) / 100).min(area.width);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height: height.min(area.height),
    }
}
