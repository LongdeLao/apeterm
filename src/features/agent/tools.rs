//! Executes typed tool calls against the app. This is the only place the
//! agent touches app state, and it only does so through App methods.

use crate::{
    app::App,
    features::agent::{
        context,
        tool_call::{ToolCall, ToolResult},
    },
};

/// Tool schema given to the model in the system prompt.
pub fn catalog() -> &'static str {
    r#"- read_current_context: args {} — re-read the current page, selection, and watchlists
- summarize_watchlist: args {} — summarize active watchlist movers, quote coverage, news, and notes
- explain_watchlist_move: args {} — connect active watchlist price moves to local news and backend insight
- find_watchlist_outliers: args {} — rank gainers, losers, relative-volume spikes, and no-news moves
- compare_symbols: args {"symbols": string[]} — compare 2 to 5 tickers using local details, live data, news, notes, and backend insight
- brief_selected_symbol: args {} — build a compact brief for the selected symbol
- summarize_symbol_news: args {"symbol": string} — summarize recent local news for one ticker
- find_news_without_position: args {} — find important news symbols not on the active watchlist
- summarize_notes_for_symbol: args {"symbol": string} — summarize local notes tied to one ticker
- build_symbol_timeline: args {"symbol": string} — build a timeline from local notes, news, and SEC activity
- summarize_sec_activity: args {} — summarize the selected SEC entity's holdings or transactions
- find_sec_watchlist_matches: args {} — find SEC activity touching active watchlist symbols
- surface_attention_list: args {} — rank what the user should look at first from local app signals
- list_watchlists: args {} — list all watchlists with their symbols
- create_watchlist: args {"name": string} — create a new watchlist and make it active
- add_symbol_to_watchlist: args {"symbol": string} — add a ticker to the active watchlist
- remove_symbol_from_watchlist: args {"symbol": string} — remove a ticker from the active watchlist
- open_symbol: args {"symbol": string} — open the details page for a ticker"#
}

pub fn execute(app: &mut App, call: ToolCall) -> ToolResult {
    let tool = call.name();
    let outcome = match call {
        ToolCall::ReadCurrentContext => Ok(context::build_context(app)),
        ToolCall::SummarizeWatchlist => app.agent_summarize_watchlist(),
        ToolCall::ExplainWatchlistMove => app.agent_explain_watchlist_move(),
        ToolCall::FindWatchlistOutliers => app.agent_find_watchlist_outliers(),
        ToolCall::CompareSymbols { symbols } => app.agent_compare_symbols(&symbols),
        ToolCall::BriefSelectedSymbol => app.agent_brief_selected_symbol(),
        ToolCall::SummarizeSymbolNews { symbol } => app.agent_summarize_symbol_news(&symbol),
        ToolCall::FindNewsWithoutPosition => app.agent_find_news_without_position(),
        ToolCall::SummarizeNotesForSymbol { symbol } => {
            app.agent_summarize_notes_for_symbol(&symbol)
        }
        ToolCall::BuildSymbolTimeline { symbol } => app.agent_build_symbol_timeline(&symbol),
        ToolCall::SummarizeSecActivity => app.agent_summarize_sec_activity(),
        ToolCall::FindSecWatchlistMatches => app.agent_find_sec_watchlist_matches(),
        ToolCall::SurfaceAttentionList => app.agent_surface_attention_list(),
        ToolCall::ListWatchlists => Ok(list_watchlists(app)),
        ToolCall::CreateWatchlist { name } => app.agent_create_watchlist(&name),
        ToolCall::AddSymbolToWatchlist { symbol } => app.agent_add_symbol_to_watchlist(&symbol),
        ToolCall::RemoveSymbolFromWatchlist { symbol } => {
            app.agent_remove_symbol_from_watchlist(&symbol)
        }
        ToolCall::OpenSymbol { symbol } => app.agent_open_symbol(&symbol),
    };

    match outcome {
        Ok(message) => ToolResult::success(tool, message),
        Err(message) => ToolResult::failure(tool, message),
    }
}

fn list_watchlists(app: &App) -> String {
    let active_index = app.active_watchlist_index();
    app.watchlists()
        .iter()
        .enumerate()
        .map(|(index, list)| {
            let marker = if index == active_index {
                " (active)"
            } else {
                ""
            };
            let mut symbols = list.stock_symbols.clone();
            symbols.extend(list.crypto_symbols.iter().cloned());
            let symbols = if symbols.is_empty() {
                "(empty)".to_string()
            } else {
                symbols.join(", ")
            };
            format!("{}{marker}: {symbols}", list.name)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
