//! The MCP server for outside AI apps, built on the shared tool registry (ADR-0013, ADR-0042).
//!
//! It turns the one registry of `bb-app` into MCP at `/mcp` over streamable HTTP: it lists the tools
//! the scope of a token allows, calls them as the person behind the token, offers the resources and
//! the prompts of AI-21, and keeps the run of a session of an app. It speaks the protocol itself
//! (`protocol.rs`), because it needs no more of it than requests with answers, and it depends on
//! `bb-app` only: the use cases it calls are those of [`bb_app::auth::AuthApi`].

#![forbid(unsafe_code)]

mod methods;
mod protocol;
mod server;
mod session;

pub use server::{McpOptions, McpService, router};
