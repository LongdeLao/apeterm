use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::{
    app::{App, Language, OnboardingStep, ThemeName},
    theme::current_theme,
};

const LOGO: &str = "／三ヽ\n(6( ･ ･|)\n|　( ┴)\n\napeterm";

pub fn render(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    if let Some(background) = theme.background {
        frame.render_widget(
            Block::default().style(Style::default().bg(background)),
            frame.area(),
        );
    }

    match app.onboarding_step {
        OnboardingStep::Welcome => render_welcome(frame, app),
        OnboardingStep::Language => render_language(frame, app),
        OnboardingStep::Theme => render_theme(frame, app),
    }
}

fn render_welcome(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    let chunks = welcome_chunks(frame.area());

    let logo = Paragraph::new(LOGO)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.foreground));

    let prompt = Paragraph::new("press \u{21B5} to continue")
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.muted).add_modifier(Modifier::DIM));

    frame.render_widget(logo, chunks[1]);
    frame.render_widget(prompt, chunks[2]);
}

fn render_language(frame: &mut Frame, app: &App) {
    let title = match app.language {
        Language::English => "\u{eb01} language",
        Language::German => "\u{eb01} sprache",
    };

    render_menu(
        frame,
        app,
        title,
        &[
            ("English", app.language == Language::English),
            ("Deutsch", app.language == Language::German),
        ],
    );
}

fn render_theme(frame: &mut Frame, app: &App) {
    let title = match app.language {
        Language::English => "\u{25D0} theme",
        Language::German => " \u{24D0} theme",
    };

    render_menu(
        frame,
        app,
        title,
        &[
            ("Dark", app.theme_name == ThemeName::Dark),
            ("Light", app.theme_name == ThemeName::Light),
            ("Transparent", app.theme_name == ThemeName::Transparent),
        ],
    );
}

fn render_menu(frame: &mut Frame, app: &App, title: &'static str, options: &[(&str, bool)]) {
    let theme = current_theme(app.theme_name);
    let chunks = menu_chunks(frame.area(), options.len() as u16);

    let title = Paragraph::new(title)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.foreground));

    let prompt = Paragraph::new("press \u{21B5} to continue")
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.muted).add_modifier(Modifier::DIM));

    frame.render_widget(title, chunks[1]);
    render_options(frame, app, chunks[2], options);
    frame.render_widget(prompt, chunks[3]);
}

fn render_options(frame: &mut Frame, app: &App, area: Rect, options: &[(&str, bool)]) {
    let theme = current_theme(app.theme_name);
    let option_area = centered_rect(area, 20, options.len() as u16);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); options.len()])
        .split(option_area);

    for (index, (label, selected)) in options.iter().enumerate() {
        let marker = if *selected { ">" } else { " " };
        let style = if *selected {
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted)
        };

        let row = Line::from(vec![
            Span::styled(format!("{marker} "), style),
            Span::styled(*label, style),
        ]);

        frame.render_widget(Paragraph::new(row), rows[index]);
    }
}

fn welcome_chunks(area: Rect) -> std::rc::Rc<[Rect]> {
    let content_height = 7;

    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(content_height)) / 2),
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area)
}

fn menu_chunks(area: Rect, option_count: u16) -> std::rc::Rc<[Rect]> {
    let content_height = option_count + 4;

    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(content_height)) / 2),
            Constraint::Length(1),
            Constraint::Length(option_count + 2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area)
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vertical[1]);

    horizontal[1]
}
