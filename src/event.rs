use crossterm::event::{Event, KeyCode};

use crate::app::{App, Page, SelectionDirection};

pub fn handle_event(app: &mut App, event: Event) {
    let Event::Key(key) = event else {
        return;
    };

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Enter => {
            if app.page == Page::Onboarding {
                app.advance_onboarding();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(SelectionDirection::Previous),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(SelectionDirection::Next),
        _ => {}
    }
}
