use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::Paragraph,
};

use crate::{
    app::{App, Page, WatchlistEditMode},
    i18n::Key,
    pages::{dashboard, onboarding, search, settings},
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
    render_footer(frame, app);
}

pub fn content_area(area: Rect) -> Rect {
    Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1))
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
    if let Some(editor) = &app.watchlist_editor {
        return match editor.mode {
            Some(WatchlistEditMode::Add { .. }) | Some(WatchlistEditMode::ChangeTicker { .. }) => {
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
        Page::Dashboard
            if app.focused_panel == crate::app::PanelId::Watchlist
                && app.panel_content(crate::app::PanelId::Watchlist)
                    == crate::app::WindowKind::Watchlist =>
        {
            format!(
                "{}  {}",
                app.t(Key::AppFooter),
                app.t(Key::WatchlistFooterEdit)
            )
        }
        _ => app.t(Key::AppFooter).to_string(),
    }
}
