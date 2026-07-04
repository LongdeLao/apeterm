//! Feature modules: one folder per feature, colocating state (`state.rs`),
//! rendering (`view.rs`), persistence (`repo.rs`), and any feature-specific
//! logic. `ui` routes to `view` renderers; `event` routes input to state
//! methods. See `docs/plugins.md` for the layering model.

pub mod agent;
pub mod sec;
