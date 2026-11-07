//! The one registry of tools and the one way to call them (AI-04, AI-05, AI-21, COMP-10).
//!
//! `registry.rs` is the table; `inputs.rs` holds the input types the schemas are generated from;
//! `read.rs` runs the read tools; `preview.rs` answers what a tool that changes something would do;
//! `call.rs` is the only path in: it checks the scope, runs the tool as the actor, takes out what
//! is kept from AI, fences the text and writes the log of tool calls (`log.rs`).

mod call;
mod inputs;
mod log;
mod preview;
mod read;
mod registry;

pub use call::{
    ToolAnswer, ToolCallQuery, ToolList, ToolListQuery, ToolListing, ToolMode, call_tool,
    list_tools,
};
pub use inputs::*;
pub use log::ToolCallStore;
pub use registry::{DESIGNED_OUT, ToolSpec, registry};
