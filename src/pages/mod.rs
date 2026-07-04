//! Page rendering. Modules here draw views from `App` state and should not
//! mutate it or hold feature business logic — that belongs in
//! `app/*_feature.rs` or the feature's own module.

pub mod calendar;
pub mod dashboard;
pub mod fill;
pub mod news;
pub mod notes;
pub mod onboarding;
pub mod panel;
pub mod search;
pub mod settings;
pub mod spotlight;
pub mod watchlist;
