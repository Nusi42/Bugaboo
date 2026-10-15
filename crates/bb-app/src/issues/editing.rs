//! What locks, drafts and the check of versions need to know about an issue (ISS-17, ISS-11,
//! ADR-0019): where it sits, its version, and the permission that lets a person change it.
//!
//! `bb-app` knows the issue itself, like it knows projects, roles and groups, so the issue is not
//! registered with `EditableRecords` (that is for kinds the program adds from outside, such as the
//! notes of the tests): this file and `editing/conflict.rs` are the one home of the issue kind
//! (M3-F32).

use bb_core::event::{EntityKind, EntityRef, RecordKind};
use bb_core::id::IssueId;
use bb_policy::{Permission, Place};

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::editing::RecordFacts;

/// What a person needs to make an issue in a project (the form behind a draft of a new issue).
#[must_use]
pub(crate) fn making(kind: &EntityKind) -> Option<Permission> {
    (*kind == EntityKind::from(RecordKind::Issue)).then_some(Permission::MakeIssues)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The facts of an issue for a person, or `None` when there is no such issue (or it is in the
    /// trash), or the person may not see it: the three look the same.
    ///
    /// The permission to change is *Change issues*; for a person who reported the issue and holds
    /// only *Change own issues* it is that one. Whoever holds neither gets *Change issues*, which
    /// the check that follows refuses.
    pub(crate) async fn issue_facts(
        &self,
        actor: &Actor,
        record: &EntityRef,
    ) -> Result<Option<RecordFacts>, AppError> {
        let Ok(id) = IssueId::from_bytes(record.id.to_bytes()) else {
            return Ok(None);
        };
        let Some((issue, held)) = self.seen_issue_by_id(actor, id).await? else {
            return Ok(None);
        };
        Ok(Some(RecordFacts {
            place: Place::Project(issue.project),
            version: issue.version,
            edit: held.permission_to_change_issue(issue.reported_by),
        }))
    }
}
