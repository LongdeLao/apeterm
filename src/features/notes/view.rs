use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{App, InputTarget, NotesFilterTab, PanelId},
    db::notes_repo::NoteRow,
    i18n::Key,
    theme::{Theme, current_theme},
    ui::panel,
};

const PIN_COL_WIDTH: u16 = 2;
const TIME_COL_WIDTH: u16 = 5;
const TICKER_COL_WIDTH: u16 = 8;

pub fn render(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    if !app.is_panel_open(panel_id) {
        return;
    }

    let theme = current_theme(app.theme_name);
    let inner = panel::content_area(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    panel::render_title(frame, app, chunks[0], panel_id, app.t(Key::PanelTitleNotes));
    render_tabs(frame, app, chunks[1]);

    let rows = app.notes_visible();
    let selected = rows.get(app.notes_selection.min(rows.len().saturating_sub(1)));

    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(app.t(Key::NotesEmpty))
                .style(Style::default().fg(theme.muted))
                .wrap(Wrap { trim: true }),
            chunks[2],
        );
    } else if chunks[2].width >= 96 {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(32),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(chunks[2]);
        render_notes_list(frame, app, panes[0], &rows);
        render_gap(frame, panes[1]);
        render_document(frame, app, panes[2], selected);
    } else if app.notes_insert_mode {
        render_document(frame, app, chunks[2], selected);
    } else {
        render_notes_list(frame, app, chunks[2], &rows);
    }

    if app.pending_note_delete.is_some() {
        render_delete_confirm(frame, app, area);
    }
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let tabs = [
        (NotesFilterTab::All, "ALL"),
        (NotesFilterTab::Tickers, "TICKERS"),
        (NotesFilterTab::Journal, "JOURNAL"),
        (NotesFilterTab::Pinned, "PINNED"),
    ];
    let tab_count = tabs.len();
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme.muted));
    frame.render_widget(block, area);

    let line = Line::from(
        tabs.into_iter()
            .enumerate()
            .flat_map(|(index, (tab, label))| {
                let selected = app.notes_tab == tab;
                let style = if selected {
                    Style::default()
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(theme.muted)
                };

                let mut spans = vec![Span::styled(format!(" {label} "), style)];
                if index + 1 < tab_count {
                    spans.push(Span::raw("  "));
                }
                spans
            })
            .collect::<Vec<_>>(),
    );
    frame.render_widget(Paragraph::new(line), area);
}

fn render_notes_list(frame: &mut Frame, app: &App, area: Rect, rows: &[NoteRow]) {
    let inner = area;

    if inner.height == 0 {
        return;
    }

    let scroll = app.notes_scroll.min(rows.len().saturating_sub(1));
    let table_rows = rows
        .iter()
        .skip(scroll)
        .take(inner.height as usize)
        .enumerate()
        .map(|(offset, note)| {
            let selected = scroll + offset == app.notes_selection;
            render_note_row(app, note, selected, inner.width)
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        table_rows,
        [
            Constraint::Length(PIN_COL_WIDTH),
            Constraint::Length(TIME_COL_WIDTH),
            Constraint::Length(TICKER_COL_WIDTH),
            Constraint::Min(12),
        ],
    )
    .column_spacing(1);
    frame.render_widget(table, inner);
}

fn render_note_row(app: &App, note: &NoteRow, selected: bool, width: u16) -> Row<'static> {
    let theme = current_theme(app.theme_name);
    let selected_style = if selected {
        Style::default().bg(theme.surface)
    } else {
        Style::default()
    };

    let pin = if note.pinned {
        Span::styled("★", Style::default().fg(theme.accent))
    } else {
        Span::raw(" ")
    };

    let updated = app.news_timestamp(chrono::DateTime::from_timestamp(note.updated_at, 0));
    let ticker = note
        .tickers
        .first()
        .cloned()
        .unwrap_or_else(|| "—".to_string());
    let ticker_style = if note.tickers.is_empty() {
        Style::default().fg(theme.muted)
    } else {
        Style::default()
            .fg(theme.positive)
            .add_modifier(Modifier::BOLD)
    };

    let preview_width = width
        .saturating_sub(PIN_COL_WIDTH + TIME_COL_WIDTH + TICKER_COL_WIDTH + 5)
        .max(8) as usize;
    let flattened = note.body.replace(['\n', '\r'], " ");
    let preview_source = if flattened.trim().is_empty() {
        "New Note"
    } else {
        flattened.trim()
    };
    let preview = truncate_with_ellipsis(preview_source, preview_width);
    let preview_style = if selected {
        Style::default()
            .fg(theme.foreground)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.foreground)
    };
    let ticker_width = TICKER_COL_WIDTH as usize;

    Row::new(vec![
        Cell::from(Line::from(pin)),
        Cell::from(Line::from(Span::styled(
            format!("{updated:>4}"),
            Style::default().fg(theme.muted),
        ))),
        Cell::from(Line::from(Span::styled(
            format!("{ticker:<ticker_width$}"),
            ticker_style,
        ))),
        Cell::from(Line::from(Span::styled(preview, preview_style))),
    ])
    .style(selected_style)
}

fn render_gap(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(" ").style(Style::default().bg(Color::Reset)),
        area,
    );
}

/// The right pane: no border, no "editor" framing — just the note's
/// metadata and body sitting directly on the page, exactly like Apple
/// Notes. Live-editable in place when insert mode is active.
fn render_document(frame: &mut Frame, app: &App, area: Rect, row: Option<&NoteRow>) {
    let theme = current_theme(app.theme_name);
    let inset = Rect {
        x: area.x.saturating_add(1),
        y: area.y,
        width: area.width.saturating_sub(2),
        height: area.height,
    };

    let is_draft = app
        .notes_draft
        .as_ref()
        .is_some_and(|draft| row.is_some_and(|note| note.id == draft.note_id) || row.is_none());

    let Some(body) = (if is_draft {
        app.notes_draft.as_ref().map(|draft| draft.body.as_str())
    } else {
        row.map(|note| note.body.as_str())
    }) else {
        frame.render_widget(
            Paragraph::new("Select a note, or press n to create one.")
                .style(Style::default().fg(theme.muted)),
            inset,
        );
        return;
    };

    let mut lines: Vec<Line> = Vec::new();
    if let Some(note) = row {
        lines.extend(metadata_lines(app, note, theme));
        lines.push(Line::from(Span::styled(
            "─".repeat(inset.width as usize),
            Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
        )));
        lines.push(Line::from(""));
    }

    let header_rows = lines.len() as u16;
    let body_width = inset.width.max(1) as usize;
    let wrapped = wrap_text(body, body_width);
    lines.extend(wrapped.iter().map(|line| {
        Line::from(Span::styled(
            line.clone(),
            Style::default().fg(theme.foreground),
        ))
    }));

    frame.render_widget(Paragraph::new(lines), inset);

    let insert_active =
        app.notes_insert_mode && is_draft && app.is_text_input_target(InputTarget::Notes);
    if insert_active {
        let last_line = wrapped.last().map(String::as_str).unwrap_or("");
        let cursor_row = header_rows + wrapped.len().saturating_sub(1) as u16;
        frame.set_cursor_position(Position::new(
            inset
                .x
                .saturating_add(UnicodeWidthStr::width(last_line) as u16),
            inset
                .y
                .saturating_add(cursor_row)
                .min(inset.y + inset.height.saturating_sub(1)),
        ));
    }

    if insert_active && !app.notes_suggestions.is_empty() {
        render_suggestions(frame, app, inset, header_rows + wrapped.len() as u16, theme);
    }
}

fn metadata_lines(app: &App, note: &NoteRow, theme: Theme) -> Vec<Line<'static>> {
    let created = app.news_absolute_timestamp(chrono::DateTime::from_timestamp(note.created_at, 0));
    let updated = app.news_absolute_timestamp(chrono::DateTime::from_timestamp(note.updated_at, 0));
    let tickers = if note.tickers.is_empty() {
        "-".to_string()
    } else {
        note.tickers.join(" ")
    };
    let tags = if note.tags.is_empty() {
        "-".to_string()
    } else {
        note.tags.join(" ")
    };

    let mut lines = vec![
        meta_line(app.t(Key::NotesDetailTickers), &tickers, theme),
        meta_line(app.t(Key::NotesDetailTags), &tags, theme),
        meta_line(app.t(Key::NotesDetailCreated), &created, theme),
        meta_line(app.t(Key::NotesDetailUpdated), &updated, theme),
    ];
    if note.pinned {
        lines.push(Line::from(Span::styled(
            "★ pinned",
            Style::default().fg(theme.accent),
        )));
    }
    lines
}

fn meta_line(label: &str, value: &str, theme: Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(theme.muted)),
        Span::styled(
            value.to_string(),
            Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
        ),
    ])
}

fn render_suggestions(frame: &mut Frame, app: &App, inset: Rect, offset_row: u16, theme: Theme) {
    let count = app.notes_suggestions.len().min(6) as u16;
    let area = Rect {
        x: inset.x,
        y: inset
            .y
            .saturating_add(offset_row)
            .min(inset.y + inset.height.saturating_sub(count.max(1))),
        width: inset.width.min(40),
        height: count.min(inset.height),
    };
    if area.height == 0 {
        return;
    }

    let background = theme.background.unwrap_or(Color::Black);
    let lines: Vec<Line> = app
        .notes_suggestions
        .iter()
        .take(6)
        .enumerate()
        .map(|(index, suggestion)| {
            let selected = index == app.notes_suggestion_selection;
            let style = if selected {
                Style::default()
                    .fg(theme.background.unwrap_or(Color::Black))
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.foreground)
            };
            let marker = if selected { ">" } else { " " };
            Line::from(vec![
                Span::styled(format!("{marker} {:<8}", suggestion.symbol), style),
                Span::styled(format!(" {}", suggestion.name), style),
            ])
        })
        .collect();

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(background)),
        area,
    );
}

fn render_delete_confirm(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let background = theme.background.unwrap_or(Color::Black);
    let modal = centered_rect(area, 50, 5);
    let lines = vec![
        Line::from(Span::styled(
            "Delete this note?",
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.t(Key::NotesDeleteConfirmFooter),
            Style::default().fg(theme.muted),
        )),
    ];

    let panel = Paragraph::new(lines)
        .style(Style::default().bg(background))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm Delete ")
                .border_style(Style::default().fg(Color::LightRed))
                .style(Style::default().bg(background)),
        );

    frame.render_widget(Clear, modal);
    frame.render_widget(panel, modal);
}

/// Greedy word-wrap used for the document body. We wrap by hand (rather
/// than relying on `Paragraph::wrap`) so the exact same line breaks are
/// used both for rendering and for placing the cursor.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in paragraph.split(' ') {
            let candidate_width = if current.is_empty() {
                UnicodeWidthStr::width(word)
            } else {
                UnicodeWidthStr::width(current.as_str()) + 1 + UnicodeWidthStr::width(word)
            };
            if current.is_empty() || candidate_width <= width {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            } else {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
            }
        }
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn truncate_with_ellipsis(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(value) <= width {
        return value.to_string();
    }

    let mut result = String::new();
    for character in value.chars() {
        let next = format!("{result}{character}");
        if UnicodeWidthStr::width(next.as_str()) + 1 > width {
            break;
        }
        result.push(character);
    }
    result.push('…');
    result
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
