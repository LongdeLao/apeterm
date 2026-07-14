//! Keyboard/input routing.
//!
//! This module translates terminal events into calls on `App`. It should stay
//! free of feature business logic — put that in `features/*/state.rs` or the
//! feature's own module (see `plugins::registry` for the map of feature areas).

use crossterm::event::{Event, KeyCode, KeyModifiers};

use crate::app::{
    App, AppMode, InputTarget, MoveDirection, Page, SelectionDirection, SplitDirection,
    WatchlistKind, WindowKind,
};

pub fn handle_event(app: &mut App, event: Event) {
    if let Event::Key(key) = event {
        handle_key_event(app, key.code, key.modifiers)
    }
}

fn handle_key_event(app: &mut App, key_code: KeyCode, modifiers: KeyModifiers) {
    let is_control = modifiers.contains(KeyModifiers::CONTROL);

    if key_code == KeyCode::Char('p') && is_control {
        if app.spotlight.open {
            app.close_spotlight();
        } else {
            app.open_spotlight();
        }
        return;
    }

    if app.spotlight.open {
        handle_spotlight_key(app, key_code, modifiers);
        return;
    }

    if let AppMode::TextInput(target) = app.mode
        && handle_text_input_key(app, target, key_code, modifiers)
    {
        return;
    }

    // The dashboard help overlay owns the keyboard while open: only its own
    // toggle key and Esc (handled below) may dismiss it. This runs before the
    // page dispatch so the overlay behaves the same regardless of which page
    // it was opened from.
    if app.show_help && key_code != KeyCode::Char('?') && key_code != KeyCode::Esc {
        return;
    }

    if key_code == KeyCode::Char('a')
        && !is_control
        && app.page != Page::Onboarding
        && can_open_agent(app)
    {
        app.open_agent();
        return;
    }
    if key_code == KeyCode::Char('E') && !is_control && app.page != Page::Onboarding {
        app.cycle_experience();
        return;
    }

    // Global navigation shortcuts: available from any page except while a
    // sub-modal already owns the keyboard (watchlist editor, pending split,
    // window picker, note-delete confirmation) or during onboarding. This
    // keeps `,`/`/`/`?`/`g` from being stranded behind whichever page
    // happened to define them first.
    if !blocks_global_shortcuts(app) && app.page != Page::Onboarding {
        match key_code {
            KeyCode::Char(',') if !is_control => {
                app.open_settings();
                return;
            }
            // On Dashboard, `/` with the Notes panel focused means "search
            // notes" (handled by handle_notes_key below), not "open Search".
            KeyCode::Char('/')
                if !is_control
                    && app.page != Page::Search
                    && !(app.page == Page::Dashboard
                        && app.panel_content(app.dashboard.focused_panel) == WindowKind::Notes) =>
            {
                app.open_search();
                return;
            }
            KeyCode::Char('?') => {
                app.toggle_help();
                return;
            }
            KeyCode::Char('g') if !is_control => {
                app.toggle_locale();
                return;
            }
            _ => {}
        }
    }

    match key_code {
        KeyCode::Char('q') if is_control => app.should_quit = true,
        KeyCode::Char('q')
            if !is_control
                && !app.is_editing_watchlist()
                && app.page != Page::Search
                && (app.page != Page::Settings || app.settings.reset_confirmation.is_none()) =>
        {
            app.should_quit = true
        }
        // With the agent input already blurred, Esc closes the panel before
        // falling back to the usual back-navigation.
        KeyCode::Esc => {
            if app.agent_panel_open() && !app.show_help {
                app.close_agent();
            } else {
                app.close_help();
            }
        }
        KeyCode::Enter => {
            if app.is_editing_watchlist() {
                app.activate_watchlist_editor();
            } else if app.page == Page::Onboarding {
                app.advance_onboarding();
            } else if app.page == Page::Dashboard {
                match app.panel_content(app.dashboard.focused_panel) {
                    WindowKind::News => app.open_selected_news(),
                    WindowKind::Notes => app.enter_note_insert_mode(),
                    WindowKind::Screener => app.open_screener_selection(),
                    WindowKind::Compare => app.open_compare_selection(),
                    WindowKind::Picker => app.confirm_window_picker(),
                    _ => {}
                }
            } else if app.page == Page::Search {
                app.open_selected_details();
            } else if app.page == Page::Settings {
                app.activate_settings_item();
            } else if app.page == Page::Screener {
                app.open_screener_selection();
            } else if app.page == Page::Compare {
                app.open_compare_selection();
            }
        }
        _ => match app.page {
            Page::Onboarding => handle_onboarding_key(app, key_code),
            Page::Dashboard => handle_dashboard_key(app, key_code, modifiers),
            Page::Search => handle_search_key(app, key_code),
            Page::Details => handle_details_key(app, key_code),
            Page::Settings => handle_settings_key(app, key_code),
            Page::Portfolio => {
                handle_portfolio_key(app, key_code);
            }
            Page::Alerts => {
                handle_alerts_key(app, key_code);
            }
            Page::Screener => {
                handle_screener_key(app, key_code);
            }
            Page::Compare => {
                handle_compare_key(app, key_code);
            }
            Page::Calendar => {
                handle_calendar_key(app, key_code);
            }
        },
    }
}

fn can_open_agent(app: &App) -> bool {
    !app.is_text_input_active()
}

/// True while a page-local sub-mode has claimed the keyboard for its own
/// purposes (picking a watchlist row, choosing a split direction, picking a
/// window's content, confirming a note delete) — global shortcuts defer to
/// these so e.g. `,` doesn't jump to Settings mid-command.
fn blocks_global_shortcuts(app: &App) -> bool {
    app.is_editing_watchlist()
        || app.dashboard.pending_split
        || app.is_choosing_window()
        || app.notes.pending_delete.is_some()
}

fn handle_spotlight_key(app: &mut App, key_code: KeyCode, modifiers: KeyModifiers) {
    let is_control = modifiers.contains(KeyModifiers::CONTROL);
    match key_code {
        KeyCode::Esc => app.close_spotlight(),
        KeyCode::Up => app.spotlight_move_selection(SelectionDirection::Previous),
        KeyCode::Down => app.spotlight_move_selection(SelectionDirection::Next),
        KeyCode::Enter => app.execute_spotlight_selection(),
        KeyCode::Backspace => app.spotlight_pop_char(),
        KeyCode::Char(character) if !is_control => app.spotlight_push_char(character),
        _ => {}
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

    let is_control = modifiers.contains(KeyModifiers::CONTROL);

    if app.dashboard.pending_split {
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

    if !is_control {
        let handled = match app.panel_content(app.dashboard.focused_panel) {
            WindowKind::News => handle_news_key(app, key_code),
            WindowKind::Watchlist => handle_watchlist_panel_key(app, key_code),
            WindowKind::Notes => handle_notes_key(app, key_code),
            WindowKind::Sec => handle_sec_key(app, key_code),
            WindowKind::Portfolio => handle_portfolio_key(app, key_code),
            WindowKind::Alerts => handle_alerts_key(app, key_code),
            WindowKind::Screener => handle_screener_key(app, key_code),
            WindowKind::Compare => handle_compare_key(app, key_code),
            WindowKind::Calendar => handle_calendar_key(app, key_code),
            WindowKind::Picker => false,
        };
        if handled {
            return;
        }
    }

    match (key_code, is_control) {
        (KeyCode::Tab, false) => {
            app.focus_next_panel();
            return;
        }
        (KeyCode::BackTab, false) => {
            app.focus_previous_panel();
            return;
        }
        _ => {}
    }

    match (key_code, is_control) {
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
        (KeyCode::Char('e'), false)
            if app.panel_content(app.dashboard.focused_panel) == WindowKind::Watchlist =>
        {
            app.open_watchlist_editor()
        }
        (KeyCode::Char('x'), true) => app.close_focused_panel(),
        (KeyCode::Char('r'), true) => app.reset_dashboard(),
        _ => {}
    }
}

fn handle_news_key(app: &mut App, key_code: KeyCode) -> bool {
    match key_code {
        KeyCode::Left => {
            app.cycle_news_filter(SelectionDirection::Previous);
            true
        }
        KeyCode::Right => {
            app.cycle_news_filter(SelectionDirection::Next);
            true
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_news_selection(SelectionDirection::Previous);
            true
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_news_selection(SelectionDirection::Next);
            true
        }
        KeyCode::Enter => {
            app.open_selected_news();
            true
        }
        KeyCode::Char('o') => {
            app.open_selected_news_in_browser();
            true
        }
        KeyCode::Char('r') => {
            app.refresh_news();
            true
        }
        _ => false,
    }
}

fn handle_notes_key(app: &mut App, key_code: KeyCode) -> bool {
    if app.panel_content(app.dashboard.focused_panel) != WindowKind::Notes {
        return false;
    }

    if app.notes.pending_delete.is_some() {
        match key_code {
            KeyCode::Char('d') | KeyCode::Char('y') => app.confirm_delete_note(),
            _ => app.cancel_delete_note(),
        }
        return true;
    }

    match key_code {
        KeyCode::Left => {
            app.cycle_notes_tab(SelectionDirection::Previous);
            true
        }
        KeyCode::Right => {
            app.cycle_notes_tab(SelectionDirection::Next);
            true
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_notes_selection(SelectionDirection::Previous);
            true
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_notes_selection(SelectionDirection::Next);
            true
        }
        KeyCode::Enter | KeyCode::Char('i') => {
            app.enter_note_insert_mode();
            true
        }
        KeyCode::Char('n') => {
            app.create_new_note();
            true
        }
        KeyCode::Char('d') => {
            app.begin_delete_selected_note();
            true
        }
        KeyCode::Char('p') => {
            app.toggle_selected_note_pin();
            true
        }
        KeyCode::Char('/') => {
            app.begin_notes_search();
            true
        }
        _ => false,
    }
}

fn handle_sec_key(app: &mut App, key_code: KeyCode) -> bool {
    if app.panel_content(app.dashboard.focused_panel) != crate::app::WindowKind::Sec {
        return false;
    }

    match key_code {
        KeyCode::Left => {
            app.cycle_sec_tab(SelectionDirection::Previous);
            true
        }
        KeyCode::Right => {
            app.cycle_sec_tab(SelectionDirection::Next);
            true
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_sec_selection(SelectionDirection::Previous);
            true
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_sec_selection(SelectionDirection::Next);
            true
        }
        KeyCode::Char('r') => {
            app.refresh_selected_sec_entity();
            true
        }
        _ => false,
    }
}

fn handle_text_input_key(
    app: &mut App,
    target: InputTarget,
    key_code: KeyCode,
    _modifiers: KeyModifiers,
) -> bool {
    // Notes insert mode owns its own key handling: Enter inserts a newline
    // rather than submitting, and Esc leaves insert mode instead of
    // discarding the draft.
    if target == InputTarget::Notes {
        match key_code {
            KeyCode::Esc => app.exit_note_insert_mode(),
            KeyCode::Enter => app.insert_note_draft_newline(),
            KeyCode::Backspace => app.pop_note_draft_char(),
            KeyCode::Char(character) => app.push_note_draft_char(character),
            KeyCode::Up => app.move_note_suggestion(SelectionDirection::Previous),
            KeyCode::Down => app.move_note_suggestion(SelectionDirection::Next),
            KeyCode::Tab => app.accept_note_suggestion(),
            _ => {}
        }
        return true;
    }

    match key_code {
        KeyCode::Esc => {
            app.cancel_text_input();
            true
        }
        KeyCode::Enter => {
            app.submit_text_input();
            true
        }
        KeyCode::Backspace => {
            app.pop_text_input_char();
            true
        }
        KeyCode::Char(character) => {
            app.push_text_input_char(character);
            true
        }
        KeyCode::Up if target == InputTarget::Agent => {
            app.move_agent_scroll(SelectionDirection::Previous);
            true
        }
        KeyCode::Down if target == InputTarget::Agent => {
            app.move_agent_scroll(SelectionDirection::Next);
            true
        }
        KeyCode::PageUp if target == InputTarget::Agent => {
            app.page_agent_scroll(SelectionDirection::Previous);
            true
        }
        KeyCode::PageDown if target == InputTarget::Agent => {
            app.page_agent_scroll(SelectionDirection::Next);
            true
        }
        KeyCode::End if target == InputTarget::Agent => {
            app.stick_agent_scroll_to_bottom();
            true
        }
        KeyCode::Up if target == InputTarget::Watchlist => {
            app.move_watchlist_suggestion(SelectionDirection::Previous);
            true
        }
        KeyCode::Down if target == InputTarget::Watchlist => {
            app.move_watchlist_suggestion(SelectionDirection::Next);
            true
        }
        _ if target == InputTarget::NotesSearch => true,
        _ => false,
    }
}

fn handle_search_key(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Char('/') | KeyCode::Char('i') => app.begin_text_input(InputTarget::Search),
        KeyCode::Left | KeyCode::Right => app.toggle_search_asset_kind(),
        KeyCode::Up | KeyCode::Char('k') => app.move_search_selection(SelectionDirection::Previous),
        KeyCode::Down | KeyCode::Char('j') => app.move_search_selection(SelectionDirection::Next),
        _ => {}
    }
}

fn handle_details_key(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Char('t') | KeyCode::Right => app.cycle_detail_timeframe(SelectionDirection::Next),
        KeyCode::Char('T') | KeyCode::Left => {
            app.cycle_detail_timeframe(SelectionDirection::Previous)
        }
        KeyCode::Char(character) if ('1'..='8').contains(&character) => {
            app.select_detail_timeframe((character as u8 - b'1') as usize)
        }
        KeyCode::Tab => app.cycle_detail_metric_focus(SelectionDirection::Next),
        KeyCode::BackTab => app.cycle_detail_metric_focus(SelectionDirection::Previous),
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_detail_sidebar_scroll(SelectionDirection::Previous)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_detail_sidebar_scroll(SelectionDirection::Next)
        }
        KeyCode::Char('e') => app.toggle_detail_description(),
        KeyCode::Char('x') => app.toggle_detail_context(),
        KeyCode::Char('A') => app.create_quick_alert(),
        _ => {}
    }
}

fn handle_settings_key(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Left | KeyCode::Char('h') => {
            app.adjust_settings_item(SelectionDirection::Previous)
        }
        KeyCode::Right | KeyCode::Char('l') => app.adjust_settings_item(SelectionDirection::Next),
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_settings_selection(SelectionDirection::Previous)
        }
        KeyCode::Down | KeyCode::Char('j') => app.move_settings_selection(SelectionDirection::Next),
        _ => {}
    }
}

fn handle_portfolio_key(app: &mut App, key_code: KeyCode) -> bool {
    match key_code {
        KeyCode::Char('r') => app.refresh_portfolio(),
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_portfolio_selection(SelectionDirection::Previous)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_portfolio_selection(SelectionDirection::Next)
        }
        _ => return false,
    }
    true
}

fn handle_alerts_key(app: &mut App, key_code: KeyCode) -> bool {
    match key_code {
        KeyCode::Char('n') => app.create_quick_alert(),
        KeyCode::Char('t') => app.toggle_selected_alert(),
        KeyCode::Char('d') => app.delete_selected_alert(),
        KeyCode::Up | KeyCode::Char('k') => app.move_alert_selection(SelectionDirection::Previous),
        KeyCode::Down | KeyCode::Char('j') => app.move_alert_selection(SelectionDirection::Next),
        _ => return false,
    }
    true
}

fn handle_screener_key(app: &mut App, key_code: KeyCode) -> bool {
    match key_code {
        KeyCode::Left => app.cycle_screener_preset(SelectionDirection::Previous),
        KeyCode::Right => app.cycle_screener_preset(SelectionDirection::Next),
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_screener_selection(SelectionDirection::Previous)
        }
        KeyCode::Down | KeyCode::Char('j') => app.move_screener_selection(SelectionDirection::Next),
        _ => return false,
    }
    true
}

fn handle_compare_key(app: &mut App, key_code: KeyCode) -> bool {
    match key_code {
        KeyCode::Char('d') => app.remove_compare_symbol(),
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_compare_selection(SelectionDirection::Previous)
        }
        KeyCode::Down | KeyCode::Char('j') => app.move_compare_selection(SelectionDirection::Next),
        _ => return false,
    }
    true
}

fn handle_calendar_key(app: &mut App, key_code: KeyCode) -> bool {
    match key_code {
        KeyCode::Char('f') => app.toggle_calendar_scope(),
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_calendar_selection(SelectionDirection::Previous)
        }
        KeyCode::Down | KeyCode::Char('j') => app.move_calendar_selection(SelectionDirection::Next),
        _ => return false,
    }
    true
}

fn handle_watchlist_edit_key(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Left => app.cycle_active_watchlist(SelectionDirection::Previous),
        KeyCode::Right => app.cycle_active_watchlist(SelectionDirection::Next),
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_watchlist_selection(SelectionDirection::Previous)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_watchlist_selection(SelectionDirection::Next)
        }
        KeyCode::Char('c') => app.begin_watchlist_add(WatchlistKind::Crypto),
        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char('w') => {
            app.begin_watchlist_create()
        }
        KeyCode::Char('x') => app.delete_active_watchlist(),
        KeyCode::Char('t') => app.begin_selected_watchlist_ticker_change(),
        KeyCode::Char('d') => app.delete_selected_watchlist_symbol(),
        KeyCode::Char('v') => app.jump_to_selected_watchlist_row_notes(),
        _ => {}
    }
}

fn handle_watchlist_panel_key(app: &mut App, key_code: KeyCode) -> bool {
    if !app.is_editing_watchlist() {
        match key_code {
            KeyCode::Left => {
                app.cycle_active_watchlist(SelectionDirection::Previous);
                true
            }
            KeyCode::Right => {
                app.cycle_active_watchlist(SelectionDirection::Next);
                true
            }
            KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char('w') => {
                app.open_watchlist_editor();
                app.begin_watchlist_create();
                true
            }
            KeyCode::Char('x') => {
                app.delete_active_watchlist();
                true
            }
            _ => false,
        }
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PanelId;
    use crate::config::AppConfig;
    use crossterm::event::KeyModifiers;

    fn test_app() -> App {
        let mut app = App::new(AppConfig::default().expect("default config"));
        app.onboarding_complete = true;
        app.page = Page::Dashboard;
        app
    }

    fn press(app: &mut App, key_code: KeyCode) {
        handle_key_event(app, key_code, KeyModifiers::NONE);
    }

    #[test]
    fn settings_reachable_from_details_and_returns_there() {
        let mut app = test_app();
        app.page = Page::Details;

        press(&mut app, KeyCode::Char(','));
        assert_eq!(app.page, Page::Settings);

        press(&mut app, KeyCode::Esc);
        assert_eq!(app.page, Page::Details);
    }

    #[test]
    fn settings_reachable_from_search_and_returns_there() {
        let mut app = test_app();
        app.page = Page::Search;

        press(&mut app, KeyCode::Char(','));
        assert_eq!(app.page, Page::Settings);

        press(&mut app, KeyCode::Esc);
        assert_eq!(app.page, Page::Search);
    }

    #[test]
    fn help_toggles_from_any_page_not_just_dashboard() {
        for page in [Page::Search, Page::Details, Page::Settings] {
            let mut app = test_app();
            app.page = page;

            press(&mut app, KeyCode::Char('?'));
            assert!(app.show_help, "help should open from {page:?}");

            press(&mut app, KeyCode::Char('?'));
            assert!(!app.show_help, "help should close again from {page:?}");
        }
    }

    #[test]
    fn search_reachable_from_settings_and_returns_there() {
        let mut app = test_app();
        app.page = Page::Settings;

        press(&mut app, KeyCode::Char('/'));
        assert_eq!(app.page, Page::Search);

        // Search opens straight into its text input, so the first Esc only
        // blurs it (matching the agent panel's blur-then-close pattern); the
        // second one navigates back.
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.page, Page::Settings);
    }

    #[test]
    fn slash_on_notes_panel_still_begins_notes_search_not_global_search() {
        let mut app = test_app();
        // The Notes panel slot shows Notes content by default.
        app.dashboard.focused_panel = PanelId::Notes;

        press(&mut app, KeyCode::Char('/'));

        assert_eq!(app.page, Page::Dashboard);
        assert!(app.is_text_input_target(InputTarget::NotesSearch));
    }

    #[test]
    fn locale_toggle_works_outside_dashboard() {
        let mut app = test_app();
        app.page = Page::Details;
        let before = app.preferences.language;

        press(&mut app, KeyCode::Char('g'));

        assert_ne!(app.preferences.language, before);
    }

    #[test]
    fn global_shortcuts_do_not_fire_mid_pending_split() {
        let mut app = test_app();
        app.begin_split_command();
        assert!(app.dashboard.pending_split);

        press(&mut app, KeyCode::Char(','));

        assert_eq!(app.page, Page::Dashboard);
        assert!(!app.dashboard.pending_split);
    }

    #[test]
    fn enter_is_routed_by_panel_content_not_panel_position() {
        let mut app = test_app();
        app.dashboard.focused_panel = PanelId::News;
        app.set_panel_content(PanelId::News, WindowKind::Notes);

        press(&mut app, KeyCode::Enter);

        assert!(app.is_text_input_target(InputTarget::Notes));
    }

    #[test]
    fn control_navigation_is_not_swallowed_by_panel_content() {
        let mut app = test_app();
        app.set_panel_content(PanelId::News, WindowKind::Portfolio);
        let before = app.dashboard.layout.top_height_percent;

        handle_key_event(&mut app, KeyCode::Char('j'), KeyModifiers::CONTROL);

        assert_eq!(app.dashboard.layout.top_height_percent, before + 5);
    }
}
