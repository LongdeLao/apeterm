use ratatui::{Frame, layout::Rect};

use crate::{app::App, app::PanelId, pages::panel};

pub fn render(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    let session = if app.logged_in {
        "session: logged in"
    } else {
        "session: logged out"
    };
    let lines = [
        "Write market thoughts soon.".to_string(),
        format!("language: {}", app.language.label()),
        format!("theme: {}", app.theme_name.label()),
        session.to_string(),
    ];
    let line_refs: Vec<&str> = lines.iter().map(String::as_str).collect();

    panel::render(frame, app, area, panel_id, "notes", &line_refs);
}
