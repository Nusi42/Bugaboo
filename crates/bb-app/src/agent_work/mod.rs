//! Agent work (AI-31 to AI-35, ADR-0029): hand-overs, standing rules, the work on an issue with its
//! attempts, questions and plans, and the review. The rules themselves are in
//! `bb_core::agent_work`; this module has the port and the use cases.

mod hand_over;
mod members;
mod queue;
mod read;
mod stop;
mod store;
mod types;

pub use hand_over::hand_over;
pub use read::issue_agent_work;
pub use store::{AgentWorkRecords, GrantStore};
pub use types::*;
