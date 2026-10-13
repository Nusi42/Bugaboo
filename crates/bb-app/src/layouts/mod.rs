//! Layouts and role-based forms (CFG-01, ISS-02, ADR-0027): which fields the issue page and the
//! new-issue form show, in which sections and tabs, and for which roles.
//!
//! Three rules run through all of it:
//! - **Field permissions win** (`effective.rs`): a field shows only when the layout lists it for
//!   one of the person's roles *and* the level of the field lets them see it (on the new-issue
//!   form: change it). The levels are asked of the policy engine, never restated here.
//! - **The nearest place wins** (`bb_core::layout::find`): a sub-project uses its parent's layout
//!   until it has its own; until anybody customizes a form, the starting layout counts.
//! - **The editor holds an edit lock** (ADR-0019, ADR-0027 decision 6): a layout is saved under
//!   the lock its editor holds, so two people cannot change one layout at once.

mod effective;
mod read;
mod records;
mod store;
mod types;
mod write;

pub use read::{form_layout, layout_editor};
pub use store::LayoutStore;
pub use types::*;
pub use write::{delete_layout, make_layout, save_layout};
