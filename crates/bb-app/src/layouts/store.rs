//! The port of the layout use cases: what they need from storage beyond the own-field port.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use bb_core::id::LayoutId;
use bb_core::layout::{Layout, LayoutContent, LayoutKind, StoredLayout};

use crate::fields::FieldStore;
use crate::store_macros::{queries, writes};

/// What the layout use cases need from storage.
pub trait LayoutStore: FieldStore + 'static {
    queries! {
        /// The headers of every layout of one kind: where each is set. The lookup of ADR-0027
        /// (`bb_core::layout::find`) picks the one that counts.
        fn layout_headers(kind: LayoutKind) -> Vec<Layout> = layout_headers_in;
        /// One layout with its sections and fields.
        fn stored_layout(id: LayoutId) -> Option<StoredLayout> = stored_layout_in;
    }

    writes! {
        /// Adds a layout with its sections and fields.
        fn insert_layout(layout: &Layout, content: &LayoutContent) -> ();
        /// Replaces the sections and fields of a layout, counts its version up, and says whether
        /// the layout was still at `version`.
        fn replace_layout(id: LayoutId, content: &LayoutContent, version: i64) -> bool;
        /// Takes a layout away (the place uses the layout it inherits again), and says whether
        /// the layout was still at `version`.
        fn delete_layout(id: LayoutId, version: i64) -> bool;
    }
}
