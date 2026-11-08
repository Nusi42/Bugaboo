//! The one registry of tools and the one way to call them (AI-04, AI-05, AI-21, COMP-10).
//!
//! `registry.rs` is the table; `inputs.rs` holds the input types the schemas are generated from;
//! `read.rs` runs the read tools; `preview.rs` answers what a tool that changes something would do;
//! `call.rs` is the only path in: it checks the scope, runs the tool as the actor, takes out what
//! is kept from AI, fences the text and writes the log of tool calls (`log.rs`).

mod agents;
mod apply;
mod call;
mod inputs;
mod log;
mod preview;
mod read;
mod registry;
mod requests;

pub use call::{
    ToolAnswer, ToolCallQuery, ToolList, ToolListQuery, ToolListing, ToolMode, ToolUndo, UndoKind,
    call_tool, list_tools,
};
pub use inputs::*;
pub use log::ToolCallStore;
pub use preview::ProposedChange;
pub use registry::{DESIGNED_OUT, ToolSpec, registry};
