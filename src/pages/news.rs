use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::{
    app::{App, PanelId},
    i18n::Key,
    pages::panel,
    theme::current_theme,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    if !app.is_panel_open(panel_id) {
        return;
    }

    let theme = current_theme(app.theme_name);
    let inner = panel::content_area(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    panel::render_title(frame, app, chunks[0], panel_id, app.t(Key::PanelTitleNews));

    if app.news_items.is_empty() {
        let message = app.news_status.as_deref().unwrap_or(app.t(Key::NewsEmpty));
        frame.render_widget(
            Paragraph::new(message).style(Style::default().fg(theme.muted)),
            chunks[1],
        );
    } else {
        let visible_rows = chunks[1].height.max(1) as usize;
        let scroll = app.news_scroll.min(app.news_items.len().saturating_sub(1));
        let end = scroll
            .saturating_add(visible_rows)
            .min(app.news_items.len());
        let lines = app.news_items[scroll..end]
            .iter()
            .enumerate()
            .map(|(offset, item)| {
                let index = scroll + offset;
                let selected = index == app.news_selection;
                let marker = if selected { ">" } else { " " };
                let title_style = if selected {
                    Style::default()
                        .fg(theme.background.unwrap_or(Color::Black))
                        .bg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.foreground)
                };
                let meta_style = if selected {
                    title_style
                } else {
                    Style::default().fg(theme.muted)
                };

                Line::from(vec![
                    Span::styled(format!("{marker} "), title_style),
                    Span::styled(item.title.as_str(), title_style),
                    Span::styled(
                        format!(
                            "  {}  {}",
                            item.source,
                            app.news_timestamp(item.published_at)
                        ),
                        meta_style,
                    ),
                ])
            })
            .collect::<Vec<_>>();

        frame.render_widget(Paragraph::new(lines), chunks[1]);
    }

    if let Some(item) = &app.selected_news {
        render_detail(frame, app, area, item);
    }
}

fn render_detail(frame: &mut Frame, app: &App, area: Rect, item: &crate::news::NewsItem) {
    let theme = current_theme(app.theme_name);
    let background = theme.background.unwrap_or(Color::Black);
    let modal = centered_rect(area, 78, 18);
    let published = app.news_timestamp(item.published_at);

    let mut lines = vec![
        Line::from(Span::styled(
            item.title.as_str(),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        detail_line(
            app.t(Key::NewsDetailSource),
            &item.source,
            theme.foreground,
            theme.muted,
        ),
        detail_line(
            app.t(Key::NewsDetailPublished),
            published.as_str(),
            theme.foreground,
            theme.muted,
        ),
    ];

    if let Some(author) = &item.author {
        lines.push(detail_line(
            app.t(Key::NewsDetailAuthor),
            author,
            theme.foreground,
            theme.muted,
        ));
    }

    lines.push(detail_line(
        app.t(Key::NewsDetailLink),
        item.url.as_str(),
        theme.foreground,
        theme.muted,
    ));

    if let Some(description) = &item.description {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            app.t(Key::NewsDetailSummary),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            description.as_str(),
            Style::default().fg(theme.foreground),
        )));
    }

    let panel = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(theme.foreground).bg(background))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", app.t(Key::PanelTitleNews)))
                .style(Style::default().bg(background))
                .border_style(Style::default().fg(theme.accent)),
        );

    frame.render_widget(Clear, modal);
    frame.render_widget(panel, modal);
}

fn detail_line<'a>(label: &'a str, value: &'a str, foreground: Color, muted: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(muted)),
        Span::styled(value, Style::default().fg(foreground)),
    ])
}

fn centered_rect(area: Rect, width_percent: u16, height: u16) -> Rect {
    let width = area.width.saturating_mul(width_percent).saturating_div(100);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height: height.min(area.height),
    }
}
