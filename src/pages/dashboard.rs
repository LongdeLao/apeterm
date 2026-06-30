use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    app::{App, Language},
    theme::current_theme,
};

pub fn render(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    if let Some(background) = theme.background {
        frame.render_widget(
            Block::default().style(Style::default().bg(background)),
            frame.area(),
        );
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Length(9),
            Constraint::Min(0),
        ])
        .split(frame.area());

    let copy = match app.language {
        Language::English => format!(
            "APETERM\n\nLanguage: {}\nTheme: {}\nOnboarding: {}\nLogged in: {}\n\nDashboard coming soon",
            app.language.label(),
            app.theme_name.label(),
            yes_no(app.onboarding_complete, app.language),
            yes_no(app.logged_in, app.language),
        ),
        Language::German => format!(
            "APETERM\n\nSprache: {}\nTheme: {}\nOnboarding: {}\nEingeloggt: {}\n\nDashboard kommt bald",
            app.language.label(),
            app.theme_name.label(),
            yes_no(app.onboarding_complete, app.language),
            yes_no(app.logged_in, app.language),
        ),
    };

    let dashboard = Paragraph::new(copy)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.foreground))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent))
                .title(" Dashboard ")
                .title_style(Style::default().add_modifier(Modifier::BOLD)),
        );

    frame.render_widget(dashboard, chunks[1]);
}

fn yes_no(value: bool, language: Language) -> &'static str {
    match (value, language) {
        (true, Language::English) => "yes",
        (false, Language::English) => "no",
        (true, Language::German) => "ja",
        (false, Language::German) => "nein",
    }
}
