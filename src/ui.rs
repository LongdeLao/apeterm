use ratatui::Frame;

use crate::{
    app::{App, Page},
    pages::{dashboard, onboarding},
};

pub fn render(frame: &mut Frame, app: &App) {
    match app.page {
        Page::Onboarding => onboarding::render(frame, app),
        Page::Dashboard => dashboard::render(frame, app),
    }
}
