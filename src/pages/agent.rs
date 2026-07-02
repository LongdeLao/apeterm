use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{AgentRole, App, InputTarget, Page},
    i18n::Key,
    theme::current_theme,
    ui,
};

pub fn render(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    let area = ui::content_area(frame.area());
    let overlay = centered_rect(
        area,
        area.width.saturating_sub(8).min(96),
        area.height.min(18),
    );
    let inner = inset(overlay, 2, 1);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    let mut lines = Vec::new();
    if app.agent_messages.is_empty() {
        lines.push(Line::from(Span::styled(
            app.t(Key::AgentStatusEmpty),
            Style::default().fg(theme.muted),
        )));
    } else {
        for message in &app.agent_messages {
            let (label, label_style, content_style) = match message.role {
                AgentRole::User => (
                    "you",
                    Style::default()
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(theme.foreground),
                ),
                AgentRole::Assistant => (
                    "agent",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(theme.foreground),
                ),
            };
            lines.push(Line::from(vec![
                Span::styled("[", Style::default().fg(theme.muted)),
                Span::styled(label, label_style),
                Span::styled("]", Style::default().fg(theme.muted)),
            ]));
            lines.push(Line::from(Span::styled(
                message.content.as_str(),
                content_style,
            )));
            lines.push(Line::from(""));
        }
    }

    if let Some(status) = &app.agent_status {
        lines.push(Line::from(Span::styled(
            status.as_str(),
            Style::default().fg(theme.muted),
        )));
    } else if app.agent_loading {
        lines.push(Line::from(Span::styled(
            app.t(Key::AgentStatusLoading),
            Style::default().fg(theme.muted),
        )));
    }

    let transcript_scroll = scroll_offset(app, &lines, chunks[1].width, chunks[1].height);
    let transcript = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((transcript_scroll, 0));
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            app.t(Key::PanelTitleAgent),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  streaming", Style::default().fg(theme.muted)),
    ]));
    let input_focused = app.is_text_input_target(InputTarget::Agent);
    let input = Paragraph::new(Line::from(vec![
        Span::styled(
            "> ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.agent_input.as_str(),
            Style::default().fg(theme.foreground),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if input_focused { theme.accent } else { theme.muted }))
            .title(Span::styled("Agent Input", Style::default().fg(theme.muted))),
    );

    let panel = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::DIM),
        )
        .style(Style::default().bg(theme.background.unwrap_or(Color::Black)));
    frame.render_widget(Clear, overlay);
    frame.render_widget(panel, overlay);
    frame.render_widget(header, chunks[0]);
    frame.render_widget(transcript, chunks[1]);
    frame.render_widget(input, chunks[2]);

    if app.page == Page::Agent && input_focused {
        frame.set_cursor_position(Position::new(
            chunks[2]
                .x
                .saturating_add(3 + UnicodeWidthStr::width(app.agent_input.as_str()) as u16),
            chunks[2].y.saturating_add(1),
        ));
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn inset(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal),
        y: area.y.saturating_add(vertical),
        width: area.width.saturating_sub(horizontal * 2),
        height: area.height.saturating_sub(vertical * 2),
    }
}

fn scroll_offset(app: &App, lines: &[Line<'_>], width: u16, height: u16) -> u16 {
    if width == 0 || height == 0 {
        return 0;
    }

    let total_lines = wrapped_line_count(lines, width);
    let max_scroll = total_lines.saturating_sub(height as usize) as u16;
    if app.agent_auto_scroll {
        max_scroll
    } else {
        app.agent_scroll.min(max_scroll)
    }
}

fn wrapped_line_count(lines: &[Line<'_>], width: u16) -> usize {
    let width = width.max(1) as usize;
    lines
        .iter()
        .map(|line| {
            let content_width: usize = line.spans.iter().map(|span| span.content.width()).sum();
            content_width.max(1).div_ceil(width)
        })
        .sum()
}
