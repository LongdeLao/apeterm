//! Global command palette: fuzzy-search symbols, panels, and actions from
//! anywhere in the app. Symbol data comes from an indexed prefix query
//! (`search::spotlight_prefix_search`), panels are a static table, and
//! actions are a small `Vec` built at startup — mirrors the
//! `agent::tool_call`/`agent::tools` split (registry vs. execution, only
//! touching `App` through its existing methods).

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

use crate::{
    app::{App, PanelId, SecTab, WindowKind},
    db, search,
};

pub const MAX_RESULTS: usize = 10;
const SYMBOL_CANDIDATE_LIMIT: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpotlightCategory {
    Symbol,
    Panel,
    Action,
}

impl SpotlightCategory {
    pub fn badge(&self) -> &'static str {
        match self {
            SpotlightCategory::Symbol => "SYM",
            SpotlightCategory::Panel => "PANEL",
            SpotlightCategory::Action => "ACT",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SpotlightEntry {
    Symbol(String),
    Panel(SpotlightPanel),
    Action(usize),
}

#[derive(Debug, Clone)]
pub struct SpotlightResult {
    pub category: SpotlightCategory,
    pub label: String,
    pub subtitle: Option<String>,
    pub entry: SpotlightEntry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotlightPanel {
    Watchlist,
    News,
    Congress,
    Sec13F,
    Notes,
    Search,
    Settings,
    AgentPanel,
}

impl SpotlightPanel {
    pub const ALL: [SpotlightPanel; 8] = [
        SpotlightPanel::Watchlist,
        SpotlightPanel::News,
        SpotlightPanel::Congress,
        SpotlightPanel::Sec13F,
        SpotlightPanel::Notes,
        SpotlightPanel::Search,
        SpotlightPanel::Settings,
        SpotlightPanel::AgentPanel,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            SpotlightPanel::Watchlist => "Watchlist",
            SpotlightPanel::News => "News",
            SpotlightPanel::Congress => "Congress",
            SpotlightPanel::Sec13F => "SEC 13F",
            SpotlightPanel::Notes => "Notes",
            SpotlightPanel::Search => "Search",
            SpotlightPanel::Settings => "Settings",
            SpotlightPanel::AgentPanel => "AI Agent",
        }
    }

    fn keywords(&self) -> &'static str {
        match self {
            SpotlightPanel::Watchlist => "watchlist stocks crypto",
            SpotlightPanel::News => "news feed headlines",
            SpotlightPanel::Congress => "congress politicians trades congressional",
            SpotlightPanel::Sec13F => "sec 13f institutional filings form4",
            SpotlightPanel::Notes => "notes journal",
            SpotlightPanel::Search => "search find symbol",
            SpotlightPanel::Settings => "settings preferences theme locale",
            SpotlightPanel::AgentPanel => "agent ai assistant chat",
        }
    }

    pub fn apply(&self, app: &mut App) {
        match self {
            SpotlightPanel::Watchlist => app.spotlight_focus_panel(PanelId::Watchlist, WindowKind::Watchlist),
            SpotlightPanel::News => app.spotlight_focus_panel(PanelId::News, WindowKind::News),
            SpotlightPanel::Notes => app.spotlight_focus_panel(PanelId::Notes, WindowKind::Notes),
            SpotlightPanel::Congress => {
                app.spotlight_focus_panel(PanelId::Calendar, WindowKind::Sec);
                app.sec_tab = SecTab::Congress;
            }
            SpotlightPanel::Sec13F => {
                app.spotlight_focus_panel(PanelId::Calendar, WindowKind::Sec);
                app.sec_tab = SecTab::Institutional;
            }
            SpotlightPanel::Search => app.open_search(),
            SpotlightPanel::Settings => app.open_settings(),
            SpotlightPanel::AgentPanel => app.open_agent(),
        }
    }
}

pub struct SpotlightAction {
    pub label: &'static str,
    pub keywords: &'static str,
    pub run: fn(&mut App),
}

pub fn actions() -> Vec<SpotlightAction> {
    vec![
        SpotlightAction {
            label: "New Note",
            keywords: "note create write",
            run: |app| {
                app.spotlight_focus_panel(PanelId::Notes, WindowKind::Notes);
                app.create_new_note();
            },
        },
        SpotlightAction {
            label: "Toggle Theme",
            keywords: "theme color dark light bloomberg degen",
            run: |app| app.cycle_theme(),
        },
        SpotlightAction {
            label: "Open AI Agent Panel",
            keywords: "agent ai chat assistant",
            run: |app| app.open_agent(),
        },
        SpotlightAction {
            label: "Reset Dashboard Layout",
            keywords: "reset layout panels",
            run: |app| app.reset_dashboard(),
        },
        SpotlightAction {
            label: "Toggle Language",
            keywords: "locale language german english de en",
            run: |app| app.toggle_locale(),
        },
    ]
}

#[derive(Debug, Clone, Default)]
pub struct SpotlightState {
    pub open: bool,
    pub query: String,
    pub selection: usize,
    pub results: Vec<SpotlightResult>,
}

/// Recomputes `app.spotlight.results` for the current query, merging
/// symbols/panels/actions into one ranked list. Called after every edit to
/// `app.spotlight.query`.
pub fn refresh(app: &mut App) {
    let query = app.spotlight.query.trim().to_string();

    let results = if query.is_empty() {
        default_results()
    } else {
        ranked_results(app, &query)
    };

    app.spotlight.results = results;
    if app.spotlight.selection >= app.spotlight.results.len() {
        app.spotlight.selection = app.spotlight.results.len().saturating_sub(1);
    }
}

fn default_results() -> Vec<SpotlightResult> {
    let mut results: Vec<SpotlightResult> = SpotlightPanel::ALL
        .iter()
        .map(|panel| SpotlightResult {
            category: SpotlightCategory::Panel,
            label: panel.label().to_string(),
            subtitle: None,
            entry: SpotlightEntry::Panel(*panel),
        })
        .collect();

    for (index, action) in actions().into_iter().enumerate() {
        results.push(SpotlightResult {
            category: SpotlightCategory::Action,
            label: action.label.to_string(),
            subtitle: None,
            entry: SpotlightEntry::Action(index),
        });
    }

    results.truncate(MAX_RESULTS);
    results
}

/// Scores a match against its primary text (symbol/label) first; only falls
/// back to the secondary keyword text (with a penalty) if the primary text
/// doesn't match at all. Without this, a panel/action whose *keywords*
/// happen to contain the query (e.g. Settings' "...theme...") could
/// outrank an entry whose *name* is a direct match (e.g. "Toggle Theme").
fn score(matcher: &SkimMatcherV2, primary: &str, secondary: Option<&str>, query: &str) -> Option<i64> {
    if let Some(score) = matcher.fuzzy_match(primary, query) {
        return Some(score * 2);
    }
    secondary.and_then(|secondary| matcher.fuzzy_match(secondary, query))
}

fn ranked_results(app: &App, query: &str) -> Vec<SpotlightResult> {
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, SpotlightResult)> = Vec::new();

    if let Ok(connection) = db::open(&app.ticker_db_path) {
        if let Ok(symbols) = search::spotlight_prefix_search(&connection, query, SYMBOL_CANDIDATE_LIMIT) {
            for symbol_result in symbols {
                let haystack = format!("{} {}", symbol_result.symbol, symbol_result.name);
                if let Some(score) = score(&matcher, &haystack, None, query) {
                    scored.push((
                        score,
                        SpotlightResult {
                            category: SpotlightCategory::Symbol,
                            label: format!("{} — {}", symbol_result.symbol, symbol_result.name),
                            subtitle: symbol_result.exchange,
                            entry: SpotlightEntry::Symbol(symbol_result.symbol),
                        },
                    ));
                }
            }
        }
    }

    for panel in SpotlightPanel::ALL {
        if let Some(score) = score(&matcher, panel.label(), Some(panel.keywords()), query) {
            scored.push((
                score,
                SpotlightResult {
                    category: SpotlightCategory::Panel,
                    label: panel.label().to_string(),
                    subtitle: None,
                    entry: SpotlightEntry::Panel(panel),
                },
            ));
        }
    }

    for (index, action) in actions().into_iter().enumerate() {
        if let Some(score) = score(&matcher, action.label, Some(action.keywords), query) {
            scored.push((
                score,
                SpotlightResult {
                    category: SpotlightCategory::Action,
                    label: action.label.to_string(),
                    subtitle: None,
                    entry: SpotlightEntry::Action(index),
                },
            ));
        }
    }

    scored.sort_by(|(left_score, left), (right_score, right)| {
        right_score
            .cmp(left_score)
            .then_with(|| left.category.cmp(&right.category))
            .then_with(|| left.label.cmp(&right.label))
    });

    scored
        .into_iter()
        .take(MAX_RESULTS)
        .map(|(_, result)| result)
        .collect()
}
