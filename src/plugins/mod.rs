//! ApeTerm's internal plugin architecture.
//!
//! "Plugin" here means a compiled-in feature module with clear boundaries —
//! nothing is loaded dynamically, downloaded, or installed. See
//! `docs/plugins.md` for the full layering model.
//!
//! Layering, in short:
//! - `plugins/*` (this module): plugin metadata; future home for reusable
//!   feature/business logic that doesn't depend on `App` state.
//! - `features/*/view.rs` and `ui/`: rendering only.
//! - `event.rs`: keyboard/input routing.
//! - `app/`: high-level app state; `features/*/state.rs` holds per-feature
//!   state and coordination.
//! - `agent/*`: natural-language interaction.
//! - `backend.rs` and API clients: cloud/backend communication.

pub mod plugin;
pub mod registry;

pub use plugin::{PluginId, PluginSpec};
pub use registry::registered_plugins;
