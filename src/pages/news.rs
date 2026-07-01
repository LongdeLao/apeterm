use ratatui::{Frame, layout::Rect};

use crate::{app::App, app::PanelId, i18n::Key, pages::panel};

pub fn render(frame: &mut Frame, app: &App, area: Rect, panel_id: PanelId) {
    panel::render(
        frame,
        app,
        area,
        panel_id,
        app.t(Key::PanelTitleNews),
        &[app.t(Key::NewsEmpty)],
    );
}
