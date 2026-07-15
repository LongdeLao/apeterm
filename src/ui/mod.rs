use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::Paragraph,
};

use crate::{
    app::{App, Page, PanelId, WatchlistEditMode, WindowKind},
    features::{
        agent, alerts, calendar, compare, dashboard, onboarding, portfolio, screener, search,
        settings, spotlight,
    },
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
        Page::Portfolio => portfolio::render(frame, app),
        Page::Alerts => alerts::render(frame, app),
        Page::Screener => screener::render(frame, app),
        Page::Compare => compare::render(frame, app),
        Page::Calendar => calendar::render_page(frame, app),
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

    if app.portfolio.login.is_some() {
        portfolio::render_login_overlay(frame, app);
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
    if let Some((message, _)) = &app.notification {
        return format!("● {message}");
    }
    if app.spotlight.open {
        return app.t(Key::SpotlightFooter).to_string();
    }
    if app.notes.insert_mode {
        return app.t(Key::NotesEditFooter).to_string();
    }
    if app.notes.pending_delete.is_some() {
        return app.t(Key::NotesDeleteConfirmFooter).to_string();
    }
    if app.is_text_input_target(crate::app::InputTarget::NotesSearch) {
        return app.t(Key::NotesSearchFooter).to_string();
    }
    if app.is_text_input_target(crate::app::InputTarget::BrokerLogin) {
        return "Enter submit · Esc cancel · credentials stay with pytr".to_string();
    }

    if let Some(editor) = &app.watchlist.editor {
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
        Page::Portfolio => "c connect · r sync · d disconnect · ↑↓ select · Esc back".to_string(),
        Page::Alerts => "n quick alert · t toggle · d delete · Esc back".to_string(),
        Page::Screener => "←→ screen · ↑↓ select · Esc back".to_string(),
        Page::Compare => "↑↓ select · d remove · Esc back".to_string(),
        Page::Calendar => "f scope · ↑↓ select · Esc back".to_string(),
        _ if app.is_text_input_target(crate::app::InputTarget::Agent) => {
            app.t(Key::AgentFooter).to_string()
        }
        Page::Dashboard
            if app.dashboard.focused_panel == PanelId::Watchlist
                && app.panel_content(PanelId::Watchlist) == WindowKind::Watchlist =>
        {
            format!(
                "{}  {}",
                app.t(Key::AppFooter),
                app.t(Key::WatchlistFooterEdit)
            )
        }
        Page::Dashboard
            if app.dashboard.focused_panel == PanelId::News
                && app.panel_content(PanelId::News) == WindowKind::News =>
        {
            format!("{}  {}", app.t(Key::AppFooter), app.t(Key::NewsFooter))
        }
        Page::Dashboard if app.panel_content(app.dashboard.focused_panel) == WindowKind::Sec => {
            format!("{}  {}", app.t(Key::AppFooter), app.t(Key::SecFooter))
        }
        Page::Dashboard if app.panel_content(app.dashboard.focused_panel) == WindowKind::Notes => {
            format!("{}  {}", app.t(Key::AppFooter), app.t(Key::NotesFooter))
        }
        _ => app.t(Key::AppFooter).to_string(),
    }
}

pub mod fill;
pub mod panel;
pub mod util;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::InputTarget,
        config::AppConfig,
        features::portfolio::state::{TradeRepublicLogin, TradeRepublicLoginStep},
    };
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn new_pages_render_on_compact_and_wide_terminals() {
        for (width, height) in [(60, 18), (120, 36)] {
            for page in [
                Page::Portfolio,
                Page::Alerts,
                Page::Screener,
                Page::Compare,
                Page::Calendar,
            ] {
                let mut app = App::new(AppConfig::default().unwrap());
                app.page = page;
                let backend = TestBackend::new(width, height);
                let mut terminal = Terminal::new(backend).unwrap();
                terminal.draw(|frame| render(frame, &app)).unwrap();
            }
        }
    }

    #[test]
    fn broker_login_overlay_renders_from_dashboard_panel() {
        let mut app = App::new(AppConfig::default().unwrap());
        app.page = Page::Dashboard;
        app.dashboard.focused_panel = PanelId::News;
        app.set_panel_content(PanelId::News, WindowKind::Portfolio);
        app.portfolio.login = Some(TradeRepublicLogin {
            step: TradeRepublicLoginStep::Phone,
            phone: String::new(),
            pin: String::new(),
            process_id: None,
            countdown: None,
            input: "+491234".to_string(),
        });
        app.begin_text_input(InputTarget::BrokerLogin);

        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let view = buffer_view(terminal.backend().buffer());

        assert!(view.contains("Trade Republic Login"));
        assert!(view.contains("Phone: +491234"));
    }

    fn buffer_view(buffer: &Buffer) -> String {
        let mut out = String::new();
        for row in buffer.content().chunks(buffer.area.width as usize) {
            for cell in row {
                out.push_str(cell.symbol());
            }
            out.push('\n');
        }
        out
    }
}
