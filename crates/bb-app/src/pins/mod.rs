//! Pins (CFG-05, ADR-0050, decision 12): a person pins projects and saved searches to their
//! sidebar, in their own order, at most [`PINS_MAX`](bb_core::pin::PINS_MAX).
//!
//! - **The person's own.** Nobody pins for somebody else; only a session reaches the routes.
//! - **Each target's own rule.** A project is shown to the person who sees it in the tree, a saved
//!   search to the person who may open it (`may_open` of the saved searches); a pin to something
//!   the person may not open is left out of every answer, without a word and without a count, and
//!   comes back when they may open it again (`read.rs`).
//! - **A pin goes with its target** when that is deleted: the trash of a saved search and the
//!   removal of a person whose private searches go delete the pins to them.

mod read;
mod store;
mod types;
mod write;

pub use read::my_pins;
pub use store::PinStore;
pub use types::*;
pub use write::{order_pins, pin_to_sidebar, unpin};
