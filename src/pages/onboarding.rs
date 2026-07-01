use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::{
    app::{App, OnboardingStep, ThemeName},
    i18n::{Key, Locale},
    theme::current_theme,
    ui,
};

const LOGO: &str = "／三ヽ\n(6( ･ ･|)\n|　( ┴)\n\napeterm";

pub fn render(frame: &mut Frame, app: &App) {
    let theme = current_theme(app.theme_name);
    let area = ui::content_area(frame.area());
    if let Some(background) = theme.background {
        frame.render_widget(
            Block::default().style(Style::default().bg(background)),
            area,
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
    let chunks = welcome_chunks(ui::content_area(frame.area()));

    let logo = Paragraph::new(LOGO)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.foreground));

    let prompt = Paragraph::new(app.t(Key::OnboardingPromptContinue))
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.muted).add_modifier(Modifier::DIM));

    frame.render_widget(logo, chunks[1]);
    frame.render_widget(prompt, chunks[2]);
}

fn render_language(frame: &mut Frame, app: &App) {
    let options = app
        .i18n
        .available_locales()
        .into_iter()
        .map(|locale| (locale_label(app, &locale), locale == app.locale))
        .collect::<Vec<_>>();

    render_menu(frame, app, app.t(Key::OnboardingTitleLanguage), &options);
}

fn render_theme(frame: &mut Frame, app: &App) {
    render_menu(
        frame,
        app,
        app.t(Key::OnboardingTitleTheme),
        &[
            (
                app.t(Key::AppThemeDark).to_string(),
                app.theme_name == ThemeName::Dark,
            ),
            (
                app.t(Key::AppThemeLight).to_string(),
                app.theme_name == ThemeName::Light,
            ),
            (
                app.t(Key::AppThemeTransparent).to_string(),
                app.theme_name == ThemeName::Transparent,
            ),
            (
                app.t(Key::AppThemeBloomberg).to_string(),
                app.theme_name == ThemeName::Bloomberg,
            ),
        ],
    );
}

fn render_menu(frame: &mut Frame, app: &App, title: &str, options: &[(String, bool)]) {
    let theme = current_theme(app.theme_name);
    let chunks = menu_chunks(ui::content_area(frame.area()), options.len() as u16);

    let title = Paragraph::new(title)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.foreground));

    let prompt = Paragraph::new(app.t(Key::OnboardingPromptContinue))
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.muted).add_modifier(Modifier::DIM));

    frame.render_widget(title, chunks[1]);
    render_options(frame, app, chunks[2], options);
    frame.render_widget(prompt, chunks[3]);
}

fn render_options(frame: &mut Frame, app: &App, area: Rect, options: &[(String, bool)]) {
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
            Span::styled(label.as_str(), style),
        ]);

        frame.render_widget(Paragraph::new(row), rows[index]);
    }
}

fn locale_label(app: &App, locale: &Locale) -> String {
    locale
        .language_key()
        .map(|key| app.t(key).to_string())
        .unwrap_or_else(|| locale.code().to_string())
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
