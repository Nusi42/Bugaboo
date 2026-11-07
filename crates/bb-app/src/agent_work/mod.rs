//! Agent work (AI-31 to AI-35, ADR-0029): hand-overs, standing rules, the work on an issue with its
//! attempts, questions and plans, and the review. The rules themselves are in
//! `bb_core::agent_work`; this module has the port and the use cases.

mod store;

pub use store::{AgentWorkRecords, GrantStore};
