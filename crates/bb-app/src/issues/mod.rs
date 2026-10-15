//! Issues, issue types and issue IDs (ISS-01, ISS-01a, ISS-02).
//!
//! The use cases are plain `async fn`s that take the context and an input (`types.rs`): *make an
//! issue* (`make.rs`), *change an issue* (`change.rs`), *delete into the trash and undo*
//! (`delete.rs`), *seeing issues* (`read.rs`) and the issue types (`issue_types.rs`). They depend
//! on storage only through the port [`IssueStore`] (`store.rs`) and on the policy engine for
//! every decision: what a person sees of an issue is `Evaluation::sees_issue`, what they change
//! is the mask of their roles.
//!
//! Three rules run through all of them:
//! - **An issue that a person may not see answers `resource.not_found`**, exactly like one that
//!   does not exist (or is in the trash).
//! - **No answer holds a count** of issues (PRIV-13): lists have a cursor and no total.
//! - **Every change is made on the version that was looked at**, and the readable ID of an issue
//!   is given out inside the change that makes it (`ids.rs`).

mod assignees;
mod change;
mod delete;
mod description;
pub(crate) mod editing;
mod fields;
mod ids;
mod issue_types;
mod make;
mod moving;
mod read;
mod store;
mod types;
mod views;
mod visible;
pub(crate) use visible::{Where, seen_by};

pub use assignees::{AssignablePeoplePage, AssignablePeopleQuery, assignable_people};
pub use change::change_issue;
pub use delete::{delete_issue, restore_issue};
pub use description::save_description;
pub use issue_types::{
    MakeStartingIssueTypes, change_issue_type, issue_types, make_issue_type,
    make_starting_issue_types,
};
pub use make::make_issue;
pub use moving::move_issue;
pub use read::{issue, issues};
pub use store::{IssueFound, IssueStore};
pub use types::*;

pub(crate) use fields::{ensure_may_change, member};
pub(crate) use ids::{ModeChange, plan_renewals_in, renew_or_queue_in};
