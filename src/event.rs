use crossterm::event::{Event, KeyCode, KeyModifiers};

use crate::app::{App, MoveDirection, Page, SelectionDirection, SplitDirection};

pub fn handle_event(app: &mut App, event: Event) {
    match event {
        Event::Key(key) => handle_key_event(app, key.code, key.modifiers),
        _ => {}
    }
}

fn handle_key_event(app: &mut App, key_code: KeyCode, modifiers: KeyModifiers) {
    let is_control = modifiers.contains(KeyModifiers::CONTROL);

    match key_code {
        KeyCode::Char('q') if is_control => app.should_quit = true,
        KeyCode::Esc => app.close_help(),
        KeyCode::Char('?') => {
            if app.page == Page::Dashboard {
                app.toggle_help();
            }
        }
        KeyCode::Enter => {
            if app.page == Page::Onboarding {
                app.advance_onboarding();
            } else if app.page == Page::Dashboard {
                app.confirm_window_picker();
            }
        }
        _ => match app.page {
            Page::Onboarding => handle_onboarding_key(app, key_code),
            Page::Dashboard => handle_dashboard_key(app, key_code, modifiers),
        },
    }
}

fn handle_onboarding_key(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(SelectionDirection::Previous),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(SelectionDirection::Next),
        _ => {}
    }
}

fn handle_dashboard_key(app: &mut App, key_code: KeyCode, modifiers: KeyModifiers) {
    if app.show_help && key_code != KeyCode::Char('?') {
        return;
    }

    let is_control = modifiers.contains(KeyModifiers::CONTROL);

    if app.pending_split {
        match key_code {
            KeyCode::Char('h') => app.split_focused_panel(SplitDirection::Horizontal),
            KeyCode::Char('v') => app.split_focused_panel(SplitDirection::Vertical),
            KeyCode::Esc => app.cancel_pending_command(),
            _ => app.cancel_pending_command(),
        }
        return;
    }

    if app.is_choosing_window() {
        match key_code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.move_window_picker(SelectionDirection::Previous)
            }
            KeyCode::Down | KeyCode::Char('j') => app.move_window_picker(SelectionDirection::Next),
            KeyCode::Char('c') => app.cancel_pending_command(),
            _ => {}
        }
        return;
    }

    match (key_code, is_control) {
        (KeyCode::Tab, false) => app.focus_next_panel(),
        (KeyCode::BackTab, false) => app.focus_previous_panel(),
        (KeyCode::Char('n'), false) => app.focus_next_panel(),
        (KeyCode::Char('p'), false) => app.focus_previous_panel(),
        (KeyCode::Char('h'), true) => app.resize_dashboard(MoveDirection::Left),
        (KeyCode::Char('j'), true) => app.resize_dashboard(MoveDirection::Down),
        (KeyCode::Char('k'), true) => app.resize_dashboard(MoveDirection::Up),
        (KeyCode::Char('l'), true) => app.resize_dashboard(MoveDirection::Right),
        (KeyCode::Char('h'), false) | (KeyCode::Left, false) => {
            app.focus_panel_in_direction(MoveDirection::Left)
        }
        (KeyCode::Char('j'), false) | (KeyCode::Down, false) => {
            app.focus_panel_in_direction(MoveDirection::Down)
        }
        (KeyCode::Char('k'), false) | (KeyCode::Up, false) => {
            app.focus_panel_in_direction(MoveDirection::Up)
        }
        (KeyCode::Char('l'), false) | (KeyCode::Right, false) => {
            app.focus_panel_in_direction(MoveDirection::Right)
        }
        (KeyCode::Char('s'), false) => app.begin_split_command(),
        (KeyCode::Char('a'), false) => app.add_panel(),
        (KeyCode::Char('c'), false) => app.change_focused_panel_content(),
        (KeyCode::Char('x'), false) => app.close_focused_panel(),
        (KeyCode::Char('r'), false) => app.reset_dashboard(),
        _ => {}
    }
}
