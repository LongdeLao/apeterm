# ApeTerm

## UI Localization

Static UI copy is localized through `src/i18n/keys.rs` and the flat JSON maps in
`locales/`. Add a new string by creating a `Key` variant with a dotted
snake-case `#[strum(serialize = "...")]` value, then add that exact key to every
locale JSON file. Run `cargo run -- --check-locales` to verify completeness.

To add a new locale, add `locales/<code>.json` with the same keys as
`locales/en.json`. The build script embeds every locale JSON file with
`include_str!`, so no Rust source changes are needed for additional locales.
