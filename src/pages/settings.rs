use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{App, InputTarget, SettingsItem},
    i18n::{Key, Locale},
    pages::fill::Fill,
    theme::current_theme,
    ui,
};

pub fn render(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    let area = ui::content_area(frame.area());
    if let Some(background) = theme.background {
        frame.render_widget(Fill::new(background), area);
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Line::from(Span::styled(
        app.t(Key::SettingsTitle),
        Style::default()
            .fg(theme.foreground)
            .add_modifier(Modifier::BOLD),
    )))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme.accent)),
    );
    frame.render_widget(title, chunks[0]);

    render_rows(frame, app, chunks[1]);
    if app.reset_confirmation.is_some() {
        render_reset_confirmation(frame, app);
    }
}

fn render_rows(frame: &mut Frame, app: &App, area: Rect) {
    let theme = current_theme(app.theme_name);
    let rows = settings_rows(app).into_iter().enumerate().map(
        |(index, (item, label, value, adjustable))| {
            let selected = app.settings_selection == index;
            let is_reset = item == SettingsItem::Reset;
            let style = if is_reset && selected {
                Style::default()
                    .fg(theme.foreground)
                    .bg(Color::Rgb(120, 22, 22))
                    .add_modifier(Modifier::BOLD)
            } else if selected {
                Style::default()
                    .fg(theme.background.unwrap_or(Color::Black))
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else if is_reset {
                Style::default()
                    .fg(Color::Rgb(196, 82, 82))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.foreground)
            };
            let marker = if selected { ">" } else { " " };
            let value = if adjustable {
                format!("< {value} >")
            } else {
                value
            };
            Row::new(vec![
                Cell::from(marker),
                Cell::from(label),
                Cell::from(value),
            ])
            .style(style)
        },
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Percentage(44),
            Constraint::Percentage(54),
        ],
    )
    .block(
        Block::default()
            .title(app.t(Key::SettingsSectionPreferences))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent)),
    )
    .column_spacing(1);

    frame.render_widget(table, inset(area, 4, 2));
}

fn settings_rows(app: &App) -> Vec<(SettingsItem, String, String, bool)> {
    vec![
        (
            SettingsItem::Language,
            app.t(Key::SettingsRowLanguage).to_string(),
            locale_label(app, &app.locale),
            true,
        ),
        (
            SettingsItem::Theme,
            app.t(Key::SettingsRowTheme).to_string(),
            app.t(app.theme_name.label_key()).to_string(),
            true,
        ),
        (
            SettingsItem::Onboarding,
            app.t(Key::SettingsRowOnboarding).to_string(),
            if app.onboarding_complete {
                app.t(Key::SettingsValueOff).to_string()
            } else {
                app.t(Key::SettingsValueOn).to_string()
            },
            true,
        ),
        (
            SettingsItem::Reset,
            app.t(Key::SettingsSectionDanger).to_string(),
            app.t(Key::SettingsRowReset).to_string(),
            false,
        ),
    ]
}

fn render_reset_confirmation(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    let background = theme.background.unwrap_or(Color::Black);
    let area = centered_rect(ui::content_area(frame.area()), 64, 9);
    let input = app
        .reset_confirmation
        .as_ref()
        .map(|input| input.input.as_str())
        .unwrap_or_default();
    let lines = vec![
        Line::from(Span::styled(
            app.t(Key::SettingsResetPrompt),
            Style::default().fg(theme.foreground),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.t(Key::SettingsResetWarning),
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("{}: ", app.t(Key::SettingsResetInputLabel)),
                Style::default().fg(theme.muted),
            ),
            Span::styled(input, Style::default().fg(theme.foreground)),
        ]),
    ];

    let panel = Paragraph::new(lines)
        .style(Style::default().bg(background))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm Reset ")
                .border_style(Style::default().fg(if app.is_text_input_target(InputTarget::ResetConfirmation) {
                    Color::LightRed
                } else {
                    theme.muted
                }))
                .style(Style::default().bg(background)),
        );

    frame.render_widget(Clear, area);
    frame.render_widget(panel, area);
    if app.is_text_input_target(InputTarget::ResetConfirmation) {
        let label_width =
            UnicodeWidthStr::width(format!("{}: ", app.t(Key::SettingsResetInputLabel)).as_str())
                as u16;
        frame.set_cursor_position(Position::new(
            area.x.saturating_add(1 + label_width + UnicodeWidthStr::width(input) as u16),
            area.y.saturating_add(5),
        ));
    }
}

fn locale_label(app: &App, locale: &Locale) -> String {
    locale
        .language_key()
        .map(|key| app.t(key).to_string())
        .unwrap_or_else(|| locale.code().to_string())
}

fn inset(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal),
        y: area.y.saturating_add(vertical),
        width: area.width.saturating_sub(horizontal * 2),
        height: area.height.saturating_sub(vertical * 2),
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
