use crossterm::event::{Event, KeyCode, KeyModifiers};

use crate::app::{
    App, MoveDirection, Page, PanelId, SelectionDirection, SplitDirection, WatchlistKind,
};

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
        KeyCode::Char('q')
            if !is_control
                && !app.is_editing_watchlist()
                && app.page != Page::Search
                && (app.page != Page::Settings || app.reset_confirmation.is_none()) =>
        {
            app.should_quit = true
        }
        KeyCode::Esc => app.close_help(),
        KeyCode::Char('?') => {
            if app.page == Page::Dashboard {
                app.toggle_help();
            }
        }
        KeyCode::Enter => {
            if app.is_editing_watchlist() {
                app.activate_watchlist_editor();
            } else if app.page == Page::Onboarding {
                app.advance_onboarding();
            } else if app.page == Page::Dashboard {
                app.confirm_window_picker();
            } else if app.page == Page::Search {
                app.open_selected_details();
            } else if app.page == Page::Settings {
                app.activate_settings_item();
            }
        }
        _ => match app.page {
            Page::Onboarding => handle_onboarding_key(app, key_code),
            Page::Dashboard => handle_dashboard_key(app, key_code, modifiers),
            Page::Search => handle_search_key(app, key_code),
            Page::Details => {}
            Page::Settings => handle_settings_key(app, key_code),
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
    if app.is_editing_watchlist() {
        handle_watchlist_edit_key(app, key_code);
        return;
    }

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
        (KeyCode::Char('s'), true) => app.begin_split_command(),
        (KeyCode::Char('a'), true) => app.add_panel(),
        (KeyCode::Char('c'), true) => app.change_focused_panel_content(),
        (KeyCode::Char('g'), false) => app.toggle_locale(),
        (KeyCode::Char(','), false) => app.open_settings(),
        (KeyCode::Char('/'), false) => app.open_search(),
        (KeyCode::Char('e'), false) if app.focused_panel == PanelId::Watchlist => {
            app.open_watchlist_editor()
        }
        (KeyCode::Char('x'), true) => app.close_focused_panel(),
        (KeyCode::Char('r'), true) => app.reset_dashboard(),
        _ => {}
    }
}

fn handle_search_key(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Tab | KeyCode::BackTab => app.toggle_search_asset_kind(),
        KeyCode::Up | KeyCode::Char('k') => app.move_search_selection(SelectionDirection::Previous),
        KeyCode::Down | KeyCode::Char('j') => app.move_search_selection(SelectionDirection::Next),
        KeyCode::Backspace => app.pop_search_char(),
        KeyCode::Char(character) => app.push_search_char(character),
        _ => {}
    }
}

fn handle_settings_key(app: &mut App, key_code: KeyCode) {
    if app.reset_confirmation.is_some() {
        match key_code {
            KeyCode::Backspace => app.pop_reset_confirmation_char(),
            KeyCode::Char(character) => app.push_reset_confirmation_char(character),
            _ => {}
        }
        return;
    }

    match key_code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_settings_selection(SelectionDirection::Previous)
        }
        KeyCode::Down | KeyCode::Char('j') => app.move_settings_selection(SelectionDirection::Next),
        KeyCode::Char('g') => app.toggle_locale(),
        _ => {}
    }
}

fn handle_watchlist_edit_key(app: &mut App, key_code: KeyCode) {
    if app
        .watchlist_editor
        .as_ref()
        .is_some_and(|editor| editor.mode.is_some())
    {
        match key_code {
            KeyCode::Up => app.move_watchlist_suggestion(SelectionDirection::Previous),
            KeyCode::Down => app.move_watchlist_suggestion(SelectionDirection::Next),
            KeyCode::Backspace => app.pop_watchlist_input_char(),
            KeyCode::Char(character) => app.push_watchlist_input_char(character),
            _ => {}
        }
        return;
    }

    match key_code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_watchlist_selection(SelectionDirection::Previous)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_watchlist_selection(SelectionDirection::Next)
        }
        KeyCode::Char('a') => app.begin_watchlist_add(WatchlistKind::Stock),
        KeyCode::Char('c') => app.begin_watchlist_add(WatchlistKind::Crypto),
        KeyCode::Char('t') => app.begin_selected_watchlist_ticker_change(),
        KeyCode::Char('d') => app.delete_selected_watchlist_symbol(),
        _ => {}
    }
}
