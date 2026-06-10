//! Application state, message handling, and update loop.
//!
//! Contains the core [`App`] struct, all [`Message`] variants dispatched by
//! views, the `update` function that processes messages, and supporting
//! sub-modules for filters, derived state, helper functions, and types.

mod derived;
/// Filter and sort enums for inventory view queries.
pub mod filters;
/// Helper functions for install target resolution, scaffold requests, and search.
pub mod helpers;
/// Message variants dispatched by views and async task completions.
pub mod message;
/// UI state structures for each view and workflow.
pub mod state;
/// Shared UI type definitions for views, sources, and targets.
pub mod types;
/// Application update loop processing all message variants.
pub mod update;

pub use derived::{filtered_marketplace_indices, filtered_mcp_indices, filtered_plugin_indices};
/// Filter and sort enums re-exported for convenience.
pub use filters::*;
/// Helper functions re-exported for convenience.
pub use helpers::*;
/// Application message variants.
pub use message::Message;
/// UI state structures re-exported for convenience.
pub use state::*;
/// Shared UI types re-exported for convenience.
pub use types::*;
/// Application state and update loop.
pub use update::{App, update};
