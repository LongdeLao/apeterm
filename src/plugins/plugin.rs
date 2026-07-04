//! Core types for ApeTerm's internal plugin system.
//!
//! A "plugin" is a compiled-in feature module with a clear boundary — not an
//! externally loadable package. There is no dynamic loading, no marketplace,
//! and no runtime plugin execution. The types here are metadata only: they
//! document which feature areas exist and where new ones should go.

/// Stable identifier for each internal feature module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginId {
    Watchlist,
    Notes,
    Insights,
    News,
    Sec,
    Agent,
}

/// Static description of one internal feature module.
///
/// `modules` lists the source paths where the feature's code lives today, so
/// contributors can navigate from the registry to the implementation. Keep it
/// up to date when feature code moves.
pub struct PluginSpec {
    pub id: PluginId,
    pub name: &'static str,
    pub description: &'static str,
    /// Marks features that are still settling; their internal APIs may change
    /// without notice.
    pub experimental: bool,
    /// Where this feature's logic currently lives, relative to `src/`.
    pub modules: &'static [&'static str],
}
