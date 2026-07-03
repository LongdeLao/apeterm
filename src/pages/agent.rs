use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    agent::AgentRole,
    app::{App, InputTarget},
    i18n::Key,
    theme::current_theme,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let inner = inset(area, 1, 1);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(inner);

    let mut lines = Vec::new();
    if app.agent.messages.is_empty() && !app.agent.busy {
        lines.push(Line::from(Span::styled(
            app.t(Key::AgentStatusEmpty),
            Style::default().fg(theme.muted),
        )));
    } else {
        for message in &app.agent.messages {
            match message.role {
                AgentRole::User => {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "❯ ",
                            Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            message.content.as_str(),
                            Style::default()
                                .fg(theme.foreground)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
                AgentRole::Assistant => {
                    lines.push(Line::from(Span::styled(
                        message.content.as_str(),
                        Style::default().fg(theme.foreground),
                    )));
                }
                AgentRole::Tool => {
                    let (marker, rest, marker_color) =
                        if let Some(rest) = message.content.strip_prefix("✓ ") {
                            ("✓", rest, theme.positive)
                        } else if let Some(rest) = message.content.strip_prefix("✗ ") {
                            ("✗", rest, theme.negative)
                        } else {
                            ("·", message.content.as_str(), theme.muted)
                        };
                    lines.push(Line::from(vec![
                        Span::styled(format!("{marker} "), Style::default().fg(marker_color)),
                        Span::styled(rest, Style::default().fg(theme.muted)),
                    ]));
                }
            }
            lines.push(Line::from(""));
        }
    }

    if app.agent.busy {
        let label = app.agent.status.as_deref().unwrap_or("working");
        lines.push(Line::from(Span::styled(
            format!("{} {label}", spinner_frame()),
            Style::default().fg(theme.accent),
        )));
    } else if let Some(status) = &app.agent.status {
        lines.push(Line::from(Span::styled(
            status.as_str(),
            Style::default().fg(theme.warning),
        )));
    }

    let transcript_scroll = scroll_offset(app, &lines, chunks[1].width, chunks[1].height);
    let transcript = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((transcript_scroll, 0));
    let model_label = app
        .agent
        .model_label()
        .map(|label| format!("  {label}"))
        .unwrap_or_default();
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            app.t(Key::PanelTitleAgent),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(model_label, Style::default().fg(theme.muted)),
    ]));
    let input_focused = app.is_text_input_target(InputTarget::Agent);
    let input_text = if app.agent.input.is_empty() {
        Span::styled("Ask anything", Style::default().fg(theme.muted))
    } else {
        Span::styled(
            app.agent.input.as_str(),
            Style::default().fg(theme.foreground),
        )
    };
    let input = Paragraph::new(Line::from(vec![
        Span::styled(
            "> ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        input_text,
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(if input_focused {
                theme.accent
            } else {
                theme.muted
            })),
    );

    let panel = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::DIM),
        )
        .style(Style::default().bg(theme.background.unwrap_or(Color::Black)));
    frame.render_widget(Clear, area);
    frame.render_widget(panel, area);
    frame.render_widget(header, chunks[0]);
    frame.render_widget(transcript, chunks[1]);
    frame.render_widget(input, chunks[2]);

    if input_focused {
        frame.set_cursor_position(Position::new(
            chunks[2]
                .x
                .saturating_add(2 + UnicodeWidthStr::width(app.agent.input.as_str()) as u16),
            chunks[2].y.saturating_add(1),
        ));
    }
}

/// The main loop redraws roughly every 100ms, so a time-based frame gives a
/// smooth spinner without extra state.
fn spinner_frame() -> &'static str {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    FRAMES[(millis / 120) as usize % FRAMES.len()]
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
    if app.agent.auto_scroll {
        max_scroll
    } else {
        app.agent.scroll.min(max_scroll)
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
