//! Dashboards (CFG-03, ADR-0050): a name, boxes on a grid of 12 columns, the person who made it and
//! whom it is shared with. *Home* is each person's own dashboard.
//!
//! Three rules run through all of it, the rules of saved searches (ADR-0034) and their code:
//! - **Private when it is made.** Sharing needs *Share dashboards* at the place, through the one
//!   check of saved searches; only the maker changes, shares or deletes a dashboard, and a person
//!   with *Change organization settings* may delete any (`share.rs`, `write.rs`).
//! - **Each box runs as the viewer.** A box reads a search text or a saved search through the
//!   reads that exist (search, totals, the timeline, the timer, the history), with the viewer's
//!   session; a box that does not check for them is left out and the grid closes up (`open.rs`,
//!   `boxes.rs`).
//! - **A dashboard that is not for a person is not there for them.** It is not listed, and its
//!   address answers exactly like one that does not exist (`open.rs`, `read.rs`).
//!
//! No route of dashboards takes an access key, and no tool of AI apps reaches them (ADR-0050,
//! decision 10).

mod boxes;
mod events;
mod open;
mod read;
mod share;
mod store;
mod types;
mod write;

pub use boxes::{box_preview, dashboard_box};
pub use read::{dashboard, dashboards, home, ready_made_boxes};
pub use share::{share_dashboard, stop_sharing_dashboard};
pub use store::DashboardStore;
pub use types::*;
pub use write::{
    change_dashboard, copy_dashboard, create_dashboard, delete_dashboard, restore_dashboard,
};
