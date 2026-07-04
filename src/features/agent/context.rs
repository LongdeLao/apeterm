//! Builds the compact snapshot of local app state the model sees each turn.

use std::fmt::Write;

use crate::app::{App, Page, WindowKind};

pub fn build_context(app: &App) -> String {
    let mut context = String::new();

    let page = match app.page {
        Page::Onboarding => "onboarding",
        Page::Dashboard => "dashboard",
        Page::Search => "search",
        Page::Details => "symbol details",
        Page::Settings => "settings",
    };
    let _ = writeln!(context, "page: {page}");

    if app.page == Page::Dashboard {
        let focused = match app.panel_content(app.dashboard.focused_panel) {
            WindowKind::News => "news",
            WindowKind::Watchlist => "watchlist",
            WindowKind::Calendar => "calendar",
            WindowKind::Notes => "notes",
            WindowKind::Sec => "sec filings",
            WindowKind::Picker => "window picker",
        };
        let _ = writeln!(context, "focused panel: {focused}");
    }

    if let Some(details) = &app.search.selected_details {
        let _ = writeln!(context, "selected symbol: {}", details.symbol);
    }

    let lists = app.watchlists();
    let active_index = app.active_watchlist_index();
    if let Some(active) = lists.get(active_index) {
        let _ = writeln!(context, "active watchlist: {}", active.name);
        let _ = writeln!(context, "  stocks: {}", join_or_none(&active.stock_symbols));
        let _ = writeln!(
            context,
            "  crypto: {}",
            join_or_none(&active.crypto_symbols)
        );
    }
    let _ = writeln!(
        context,
        "all watchlists: {}",
        lists
            .iter()
            .map(|list| list.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    context
}

fn join_or_none(symbols: &[String]) -> String {
    if symbols.is_empty() {
        "(none)".to_string()
    } else {
        symbols.join(", ")
    }
}
