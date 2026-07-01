use ratatui::{Frame, layout::Rect};

use crate::{app::App, app::PanelId, pages::panel};

pub fn render(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    panel::render(frame, app, area, panel_id, "news", &["No headlines yet."]);
}
