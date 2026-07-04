use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::Paragraph,
};

use crate::{
    app::{App, Page, PanelId, WatchlistEditMode, WindowKind},
    features::agent::view as agent,
    features::dashboard::view as dashboard,
    features::onboarding::view as onboarding,
    features::search::view as search,
    features::settings::view as settings,
    features::spotlight::view as spotlight,
    i18n::Key,
    theme::current_theme,
};

pub fn render(frame: &mut Frame, app: &App) {
    match app.page {
        Page::Onboarding => onboarding::render(frame, app),
        Page::Dashboard => dashboard::render(frame, app),
        Page::Search => search::render(frame, app),
        Page::Details => search::render_details(frame, app),
        Page::Settings => settings::render(frame, app),
    }

    if app.agent.panel_open {
        let [_main_area, agent_area] = split_content_area(frame.area(), app);
        if agent_area.width > 0 {
            if let Some(background) = current_theme(app.theme_name).background {
                frame.render_widget(
                    Paragraph::new("").style(Style::default().bg(background)),
                    agent_area,
                );
            }
            agent::render(frame, app, agent_area);
        }
    }

    if app.show_help {
        dashboard::render_help(frame, app);
    }

    if app.spotlight.open {
        spotlight::render(frame, app);
    }

    render_footer(frame, app);
}

pub fn content_area(area: Rect, app: &App) -> Rect {
    split_content_area(area, app)[0]
}

pub fn split_content_area(area: Rect, app: &App) -> [Rect; 2] {
    let content = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    if !app.agent.panel_open || content.width < 72 {
        return [
            content,
            Rect::new(content.right(), content.y, 0, content.height),
        ];
    }

    let agent_width = content.width.saturating_mul(32) / 100;
    let agent_width = agent_width
        .clamp(36, 44)
        .min(content.width.saturating_sub(24));
    let main_width = content.width.saturating_sub(agent_width);
    [
        Rect::new(content.x, content.y, main_width, content.height),
        Rect::new(
            content.x + main_width,
            content.y,
            agent_width,
            content.height,
        ),
    ]
}

fn render_footer(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.height == 0 {
        return;
    }

    let theme = current_theme(app.theme_name);
    let footer_area = Rect::new(
        area.x,
        area.y.saturating_add(area.height.saturating_sub(1)),
        area.width,
        1,
    );
    frame.render_widget(
        Paragraph::new(footer_text(app)).style(
            Style::default()
                .fg(theme.muted)
                .bg(theme.background.unwrap_or_default())
                .add_modifier(Modifier::DIM),
        ),
        footer_area,
    );
}

fn footer_text(app: &App) -> String {
    if app.spotlight.open {
        return app.t(Key::SpotlightFooter).to_string();
    }
    if app.notes_insert_mode {
        return app.t(Key::NotesEditFooter).to_string();
    }
    if app.pending_note_delete.is_some() {
        return app.t(Key::NotesDeleteConfirmFooter).to_string();
    }
    if app.is_text_input_target(crate::app::InputTarget::NotesSearch) {
        return app.t(Key::NotesSearchFooter).to_string();
    }

    if let Some(editor) = &app.watchlist_editor {
        return match editor.mode {
            Some(WatchlistEditMode::Add { .. }) | Some(WatchlistEditMode::ChangeTicker { .. }) => {
                app.t(Key::WatchlistEditInputFooter).to_string()
            }
            Some(WatchlistEditMode::CreateWatchlist { .. }) => {
                app.t(Key::WatchlistEditInputFooter).to_string()
            }
            Some(WatchlistEditMode::EditAlias { .. }) | None => {
                app.t(Key::WatchlistEditHelp).to_string()
            }
        };
    }

    match app.page {
        Page::Search => app.t(Key::SearchFooter).to_string(),
        Page::Settings => app.t(Key::SettingsFooter).to_string(),
        _ if app.is_text_input_target(crate::app::InputTarget::Agent) => {
            app.t(Key::AgentFooter).to_string()
        }
        Page::Dashboard
            if app.focused_panel == PanelId::Watchlist
                && app.panel_content(PanelId::Watchlist) == WindowKind::Watchlist =>
        {
            format!(
                "{}  {}",
                app.t(Key::AppFooter),
                app.t(Key::WatchlistFooterEdit)
            )
        }
        Page::Dashboard
            if app.focused_panel == PanelId::News
                && app.panel_content(PanelId::News) == WindowKind::News =>
        {
            format!("{}  {}", app.t(Key::AppFooter), app.t(Key::NewsFooter))
        }
        Page::Dashboard if app.panel_content(app.focused_panel) == WindowKind::Sec => {
            format!("{}  {}", app.t(Key::AppFooter), app.t(Key::SecFooter))
        }
        Page::Dashboard if app.panel_content(app.focused_panel) == WindowKind::Notes => {
            format!("{}  {}", app.t(Key::AppFooter), app.t(Key::NotesFooter))
        }
        _ => app.t(Key::AppFooter).to_string(),
    }
}

pub mod fill;
pub mod panel;
