//! Tags and labels (TAG-01, TAG-02, TAG-03): putting them on issues, the tags to offer while a
//! person types, the organization's list of labels, and renaming and combining across all issues.
//!
//! The use cases are plain `async fn`s that take the context and an input (`types.rs`): *tags and
//! labels of one issue* (`issue.rs`), *suggestions* (`suggest.rs`), *looking after tags*
//! (`manage.rs`) and *looking after labels* (`labels.rs`). They depend on storage only through the
//! port [`TagStore`] (`store.rs`). Three rules run through all of them:
//! - **What is judged is one function of what is stored** (`judge.rs`): before a change is written
//!   and again inside it, after the lock, so a preview and the change cannot differ.
//! - **A rename or a combine changes every issue and says so in each one** (`rewrite.rs`): one
//!   event for each issue, with its names before and after, whether the person who made the change
//!   sees the issue or not.
//! - **The tags to offer never come from a project the person cannot see** (`suggest.rs`).

mod issue;
mod judge;
mod labels;
mod manage;
mod refs;
mod rewrite;
mod store;
mod suggest;
mod types;

pub(crate) use issue::labels_changed;
pub use issue::{add_label, add_tag, issue_labels_and_tags, remove_label, remove_tag};
pub use labels::{
    change_label, change_label_group, combine_labels, create_label, create_label_group,
    label_change_preview, label_groups, labels,
};
pub use manage::{change_tag, combine_tags, tag_change_preview, tags};
pub use store::{TagLimit, TagStore, TagUse};
pub use suggest::tag_suggestions;
pub use types::*;

pub(crate) use refs::{EventLabel, event_labels};
