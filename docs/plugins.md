# Internal plugin architecture

ApeTerm organizes its feature areas as **internal plugins**: compiled-in
feature modules with clear boundaries. This is an organizational pattern, not
an extension mechanism.

What this is **not**:

- There is no external or dynamic plugin loading.
- There is no plugin marketplace or installer.
- There is no WASM or third-party runtime plugin execution.

Plugins exist to keep feature areas modular and contributor-friendly, and to
give future features (decision journal, should-care, what-if/backtesting, …)
an obvious place to live.

## The registry

`src/plugins/` holds the plugin layer:

- `plugin.rs` — `PluginId` and `PluginSpec`, the metadata types.
- `registry.rs` — `registered_plugins()`, the list of all feature modules.

The registry is **metadata-only**. It documents which feature areas exist and
where their code lives (each `PluginSpec` lists its `modules`), and its tests
verify those paths stay accurate. It does not route app behavior through a
central dispatcher — pages, events, and app state keep their direct wiring.

## Layering model

| Layer | Location | Responsibility |
| --- | --- | --- |
| Plugins | `src/plugins/` | Plugin metadata; future home for reusable feature/business logic |
| Feature logic | `src/news.rs`, `src/sec/`, `src/search.rs`, … | Business logic, testable without the TUI |
| App state | `src/app/` | `App` owns runtime state; `app/*_feature.rs` holds per-feature state/coordination |
| Rendering | `src/pages/`, `src/ui.rs` | Draw views from `App` state; no mutation, no business logic |
| Input | `src/event.rs` | Route keyboard/input events to `App` methods |
| Agent | `src/agent/` | Natural-language interaction; may call feature logic |
| Backend | `src/backend.rs`, API clients | Cloud/backend communication |

Keep these separate: rendering must not mutate state, event routing must not
contain business logic, and feature logic must not depend on Ratatui so it
stays testable without launching the TUI.

## Adding a new feature area

1. Put reusable business logic in its own module (under `src/plugins/<name>/`
   if it is self-contained, or a top-level module like the existing features).
2. Add per-feature `App` state and coordination in `src/app/<name>_feature.rs`.
3. Add rendering in `src/pages/<name>.rs`.
4. Route input in `src/event.rs` by calling `App` methods.
5. Register the feature: add a `PluginId` variant and a `PluginSpec` entry in
   `src/plugins/registry.rs`, listing the modules it lives in. Mark it
   `experimental: true` while its internals are still settling.

Keep plugin modules focused and small, with narrow public APIs. Prefer boring,
readable Rust over clever patterns.
