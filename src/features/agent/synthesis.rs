//! Read-heavy agent tools that gather structured local context for the model.

mod format;
mod market;
mod news;
mod notes;
mod sec;

const NEWS_LIMIT: usize = 8;
const NOTE_LIMIT: usize = 6;
const SEC_LIMIT: usize = 8;
