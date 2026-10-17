//! Sub-issues and related issues (ISS-03, ISS-04, ADR-0027): the kinds of relation, adding and
//! taking away a relation, putting an issue under another, and what the issue page shows of them.
//!
//! Two rules run through all of it:
//! - **A viewer sees a relation or a sub-issue only when they may see the issue at the other
//!   end** (`lines.rs`): a hidden issue is left out, and so is the count of it (PRIV-13).
//! - **A relation is stored once and read from both ends**, each in its own words; adding and taking
//!   away go into both histories (the event names both issues).

mod kinds;
mod lines;
mod parents;
mod read;
mod relate;
mod store;
mod types;

pub use kinds::{
    change_relation_kind, create_relation_kind, create_starting_relation_kinds, relation_kinds,
};
pub use parents::set_parent;
pub use read::{issue_relations, related_issues, sub_issues};
pub use relate::{add_relation, remove_relation};
pub use store::RelationStore;
pub use types::*;

pub(crate) use lines::LineMaker;
