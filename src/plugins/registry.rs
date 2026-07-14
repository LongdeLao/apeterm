//! Metadata registry of ApeTerm's internal feature modules.
//!
//! The registry is intentionally metadata-only: it documents which feature
//! areas exist and where their code lives, without routing app behavior
//! through a central dispatcher. Pages, events, and app state keep their
//! existing direct wiring.
//!
//! When adding a new feature area (e.g. decision journal, should-care,
//! what-if/backtesting), add a `PluginId` variant and a `PluginSpec` entry
//! here alongside the new module.

use super::plugin::{PluginId, PluginSpec};

/// All compiled-in feature modules, in display order.
pub fn registered_plugins() -> &'static [PluginSpec] {
    &[
        PluginSpec {
            id: PluginId::Watchlist,
            name: "Watchlist",
            description: "Track stocks and crypto across named watchlists with live quotes.",
            experimental: false,
            modules: &["features/watchlist/"],
        },
        PluginSpec {
            id: PluginId::Notes,
            name: "Notes",
            description: "Per-symbol and general notes with search.",
            experimental: false,
            modules: &["features/notes/"],
        },
        PluginSpec {
            id: PluginId::Insights,
            name: "Insights",
            description: "Backend-provided market insights shown on the dashboard.",
            experimental: false,
            modules: &["backend.rs", "features/dashboard/"],
        },
        PluginSpec {
            id: PluginId::News,
            name: "News",
            description: "News feed fetching, filtering, and enrichment.",
            experimental: false,
            modules: &["features/news/"],
        },
        PluginSpec {
            id: PluginId::Sec,
            name: "SEC",
            description: "SEC filings: submissions, Form 4, 13F, and local sync.",
            experimental: false,
            modules: &["features/sec/"],
        },
        PluginSpec {
            id: PluginId::Agent,
            name: "Agent",
            description: "Natural-language assistant that can call feature logic.",
            experimental: true,
            modules: &["features/agent/"],
        },
        PluginSpec {
            id: PluginId::Portfolio,
            name: "Portfolio",
            description: "Read-only broker holdings, cash, value, and P/L.",
            experimental: true,
            modules: &["features/portfolio/"],
        },
        PluginSpec {
            id: PluginId::Alerts,
            name: "Alerts",
            description: "Price alerts and a local activity inbox.",
            experimental: true,
            modules: &["features/alerts/"],
        },
        PluginSpec {
            id: PluginId::Screener,
            name: "Screener",
            description: "Live screens over watchlist price and volume signals.",
            experimental: true,
            modules: &["features/screener/"],
        },
        PluginSpec {
            id: PluginId::Compare,
            name: "Compare",
            description: "Side-by-side market comparison workspace.",
            experimental: true,
            modules: &["features/compare/"],
        },
        PluginSpec {
            id: PluginId::Broker,
            name: "Broker",
            description: "Optional read-only broker adapters.",
            experimental: true,
            modules: &["broker/"],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn registry_is_not_empty() {
        assert!(!registered_plugins().is_empty());
    }

    #[test]
    fn plugin_ids_are_unique() {
        let mut seen = HashSet::new();
        for spec in registered_plugins() {
            assert!(seen.insert(spec.id), "duplicate plugin id: {:?}", spec.id);
        }
    }

    #[test]
    fn specs_have_names_descriptions_and_modules() {
        for spec in registered_plugins() {
            assert!(!spec.name.trim().is_empty());
            assert!(!spec.description.trim().is_empty());
            assert!(!spec.modules.is_empty(), "{} lists no modules", spec.name);
        }
    }

    #[test]
    fn listed_modules_exist_on_disk() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for spec in registered_plugins() {
            for module in spec.modules {
                assert!(
                    src.join(module).exists(),
                    "{}: listed module src/{} does not exist",
                    spec.name,
                    module
                );
            }
        }
    }
}
