//! The port of the relation use cases: what they need from storage beyond the issues port.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::id::{IssueId, RelationId, RelationKindId};
use bb_core::issue::Issue;
use bb_core::relation::{IssueParent, Relation, RelationKind};
use bb_core::version::Version;
use jiff::Timestamp;

use crate::issues::IssueStore;
use crate::store_macros::{queries, reads, writes};

/// What the relation use cases need from storage.
pub trait RelationStore: IssueStore + 'static {
    queries! {
        /// Every kind of relation, in the order they are offered in (then by ID).
        fn relation_kinds() -> Vec<RelationKind> = relation_kinds_in;
        /// The relation with this ID.
        fn relation(id: RelationId) -> Option<Relation> = relation_in;
        /// The relations an issue is at either end of, oldest first.
        fn relations_of(issue: IssueId) -> Vec<Relation> = relations_of_in;
        /// The relation of this kind from one issue to the other, if there is one.
        fn relation_between(
            kind: RelationKindId,
            from: IssueId,
            to: IssueId
        ) -> Option<Relation> = relation_between_in;
        /// The issue another issue sits under.
        fn parent_of(child: IssueId) -> Option<IssueParent> = parent_of_in;
        /// The issues that sit under an issue, oldest first.
        fn sub_issues_of(parent: IssueId) -> Vec<IssueParent> = sub_issues_of_in;
    }

    reads! {
        /// The issues with these IDs that exist, also those in the trash.
        fn issues_by_ids(ids: &[IssueId]) -> Vec<Issue>;
    }

    writes! {
        /// Adds a kind of relation.
        fn insert_relation_kind(kind: &RelationKind) -> ();
        /// Changes the names and the retired date of a kind, counts its version up, and says
        /// whether the kind was still at `version`.
        fn update_relation_kind(
            id: RelationKindId,
            forward: &str,
            backward: &str,
            retired_at: Option<Timestamp>,
            version: Version
        ) -> bool;
        /// Adds a relation. The same relation twice is a duplicate key.
        fn insert_relation(relation: &Relation) -> ();
        /// Takes a relation away; `false` when it was not there.
        fn delete_relation(id: RelationId) -> bool;
        /// Puts an issue under another one, replacing the one it sat under.
        fn set_issue_parent(parent: &IssueParent) -> ();
        /// Takes an issue out from under its parent; `false` when it sat under none.
        fn clear_issue_parent(child: IssueId) -> bool;
        /// Waits for every other change of which issue sits under which, until the end of this
        /// change: the chain above a new parent is judged and written by one change at a time.
        fn lock_issue_parents() -> ();
        /// Waits for every other adding of a relation, until the end of this change: the limit of
        /// relations of an issue is counted on what is stored when the others are done.
        fn lock_issue_relations() -> ();
    }
}
