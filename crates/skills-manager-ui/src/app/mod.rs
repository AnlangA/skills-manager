//! Application state, message handling, and update loop.
//!
//! Contains the core [`App`] struct, all [`Message`] variants dispatched by
//! views, the `update` function that processes messages, and supporting
//! sub-modules for filters, derived state, helper functions, and types.

mod derived;
pub mod filters;
pub mod helpers;
pub mod message;
pub mod state;
pub mod types;
pub mod update;

pub use derived::{filtered_marketplace_indices, filtered_mcp_indices, filtered_plugin_indices};
pub use filters::*;
pub use helpers::*;
pub use message::Message;
pub use state::*;
pub use types::*;
pub use update::{App, update};
