//! Releases of a project (ISS-08, ADR-0049): the list offered in a project, making, changing,
//! marking, retiring, deleting into the trash and taking out again, and the one rule of which
//! releases an issue may name.
//!
//! The record, its states and where it is offered are `bb_core::project_release`; the use cases
//! depend on storage only through the port [`ReleaseStore`] (`store.rs`).

mod choose;
mod manage;
mod read;
mod store;
mod types;
mod write;

pub(crate) use choose::{Naming, ReleaseChoice, ensure_still_choosable_in};
pub(crate) use manage::member;
pub use manage::{change_release, create_release, delete_release, restore_release};
pub use read::releases;
pub use store::ReleaseStore;
pub use types::*;
