use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    agent::{AgentController, AgentRole, Badge},
    app::{App, InputTarget},
    i18n::Key,
    theme::{current_theme, Theme},
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);

    // Subtle left divider only; the transcript uses its own bubbles rather
    // than a heavy nested frame.
    let panel = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(theme.muted).add_modifier(Modifier::DIM))
        .style(background_style(theme));
    frame.render_widget(Clear, area);
    frame.render_widget(panel, area);

    let inner = Rect {
        x: area.x.saturating_add(2),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(3),
        height: area.height.saturating_sub(2),
    };
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // header + gap
            Constraint::Min(0),    // transcript
            Constraint::Length(3), // input + hint
        ])
        .split(inner);

    render_header(frame, app, &theme, chunks[0]);
    render_transcript(frame, app, &theme, chunks[1]);
    render_input(frame, app, &theme, chunks[2]);
}

fn background_style(theme: Theme) -> Style {
    match theme.background {
        Some(background) => Style::default().bg(background),
        None => Style::default(),
    }
}

fn render_header(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let connected = app.agent.model_label().is_some();
    let model = app.agent.model_label().unwrap_or("not configured");
    let dot_color = if connected {
        theme.positive
    } else {
        theme.negative
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            app.t(Key::PanelTitleAgent),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled("● ", Style::default().fg(dot_color)),
        Span::styled(model.to_string(), Style::default().fg(theme.muted)),
    ]));
    frame.render_widget(header, area);
}

fn render_transcript(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let lines = build_transcript(app, theme, area.width);
    let scroll = scroll_offset(app, &lines, area.width, area.height);
    let transcript = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(transcript, area);
}

fn render_input(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(1)])
        .split(area);

    let focused = app.is_text_input_target(InputTarget::Agent);
    let input_text = if app.agent.input.is_empty() {
        Span::styled("Ask anything...", Style::default().fg(theme.muted))
    } else {
        Span::styled(
            app.agent.input.clone(),
            Style::default().fg(theme.foreground),
        )
    };
    let input = Paragraph::new(Line::from(vec![
        Span::styled(
            "❯ ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        input_text,
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(if focused { theme.accent } else { theme.muted })),
    );
    frame.render_widget(input, rows[0]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("⏎", Style::default().fg(theme.accent)),
        Span::styled(" send", Style::default().fg(theme.muted)),
        Span::styled("    esc", Style::default().fg(theme.accent)),
        Span::styled(" close", Style::default().fg(theme.muted)),
    ]));
    frame.render_widget(hint, rows[1]);

    if focused {
        frame.set_cursor_position(Position::new(
            rows[0]
                .x
                .saturating_add(2 + UnicodeWidthStr::width(app.agent.input.as_str()) as u16),
            rows[0].y.saturating_add(1),
        ));
    }
}

fn build_transcript(app: &App, theme: &Theme, width: u16) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let box_width = (width as usize).saturating_sub(1);

    if app.agent.messages.is_empty() && !app.agent.busy {
        lines.push(Line::from(Span::styled(
            "How can I help?".to_string(),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        for suggestion in AgentController::suggestions() {
            push_box(
                &mut lines,
                &[(suggestion.to_string(), Style::default().fg(theme.muted))],
                box_width,
                theme.muted,
            );
            lines.push(Line::from(""));
        }
        return lines;
    }

    for message in &app.agent.messages {
        match message.role {
            AgentRole::User => {
                push_bubble(
                    &mut lines,
                    "you",
                    theme.accent,
                    &message.content,
                    theme.foreground,
                    box_width,
                    None,
                    theme,
                );
            }
            AgentRole::Assistant => {
                push_bubble(
                    &mut lines,
                    "ape",
                    theme.muted,
                    &message.content,
                    theme.foreground,
                    box_width,
                    message.badge,
                    theme,
                );
            }
        }
        lines.push(Line::from(""));
    }

    if app.agent.busy {
        let label = app.agent.status.as_deref().unwrap_or("working");
        push_bubble(
            &mut lines,
            "ape",
            theme.muted,
            &format!("{} {label}", spinner_frame()),
            theme.accent,
            box_width,
            None,
            theme,
        );
    } else if let Some(status) = &app.agent.status {
        // Errors (missing key, interrupted request) shown plainly, not boxed.
        lines.push(Line::from(Span::styled(
            format!("! {status}"),
            Style::default().fg(theme.warning),
        )));
    }

    lines
}

/// A labeled message block: a small caps label (with optional success/failure
/// badge) above wrapped text. Kept frameless to reduce visual noise.
#[allow(clippy::too_many_arguments)]
fn push_bubble(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    _accent_color: Color,
    text: &str,
    text_color: Color,
    box_width: usize,
    badge: Option<Badge>,
    theme: &Theme,
) {
    // Label line, with the badge right-aligned inside the box width.
    let mut label_spans = vec![Span::styled(
        label.to_string(),
        Style::default().fg(theme.muted),
    )];
    if let Some(badge) = badge {
        let (glyph, color) = match badge {
            Badge::Ok => ("✓", theme.positive),
            Badge::Failed => ("✗", theme.negative),
        };
        let pad = box_width
            .saturating_sub(UnicodeWidthStr::width(label))
            .saturating_sub(1);
        label_spans.push(Span::raw(" ".repeat(pad)));
        label_spans.push(Span::styled(
            glyph.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }
    lines.push(Line::from(label_spans));

    push_box(
        lines,
        &[(text.to_string(), Style::default().fg(text_color))],
        box_width,
        _accent_color,
    );
}

/// Renders wrapped `content` as plain indented text.
/// `content` is a single logical string paired with its style.
fn push_box(
    lines: &mut Vec<Line<'static>>,
    content: &[(String, Style)],
    box_width: usize,
    _accent_color: Color,
) {
    if box_width < 4 {
        for (text, style) in content {
            lines.push(Line::from(Span::styled(text.clone(), *style)));
        }
        return;
    }

    let content_width = box_width.saturating_sub(2);
    for (text, style) in content {
        for wrapped in wrap_text(text, content_width) {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(wrapped, *style),
            ]));
        }
    }
}

/// Word-wraps `text` to `width` display columns, hard-splitting any word that
/// is itself too long. Preserves explicit newlines.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        let mut current_width = 0;

        for mut word in paragraph.split_whitespace() {
            // Hard-split words wider than the box.
            while UnicodeWidthStr::width(word) > width {
                if current_width > 0 {
                    lines.push(std::mem::take(&mut current));
                    current_width = 0;
                }
                let (head, tail) = split_at_width(word, width);
                lines.push(head.to_string());
                word = tail;
            }

            let word_width = UnicodeWidthStr::width(word);
            let separator = usize::from(current_width > 0);
            if current_width + separator + word_width > width {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
                current_width = word_width;
            } else {
                if separator > 0 {
                    current.push(' ');
                    current_width += 1;
                }
                current.push_str(word);
                current_width += word_width;
            }
        }
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn split_at_width(text: &str, width: usize) -> (&str, &str) {
    let mut used = 0;
    for (index, character) in text.char_indices() {
        let char_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if used + char_width > width {
            return text.split_at(index);
        }
        used += char_width;
    }
    (text, "")
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
