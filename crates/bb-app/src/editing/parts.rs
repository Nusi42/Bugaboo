//! Which parts of a record can be edited under a lock or kept as a draft (ISS-17, ISS-11, SEC-06).
//!
//! Each kind of record lists its valid parts: an issue has its `description` and the long text of
//! an own field (`field-<id>`) that the role of the person lets them see (to read a lock) or change
//! (to take one or keep a draft); a comment has its `text`, a layout the `layout`, a workflow the `workflow`, a project its
//! `description`. A part that is not valid, or that the person may not see, is refused like a record
//! that is not there, so a lock or a draft cannot be used to find a hidden field or to fill the disk.

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::comment::TEXT_PART;
use bb_core::edit::{EditWhat, LAYOUT_PART, WORKFLOW_PART};
use bb_core::event::{EntityKind, RecordKind};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{FieldId, IssueId};
use bb_core::own_fields::EDIT_PART_PREFIX;

use super::records::RecordFacts;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::issues::member;

/// What a person wants to do with a part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Need {
    /// Read who is editing it.
    See,
    /// Edit it: take the lock, keep a draft.
    Change,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The facts of the record, when the part is one the person may use as they want to.
    ///
    /// # Errors
    /// `resource.not_found` for a record the person may not know about and for a part that does not
    /// exist or that they may not see; `access.forbidden` for a part they see and may not change.
    pub(crate) async fn facts_for_part(
        &self,
        actor: &Actor,
        what: &EditWhat,
        need: Need,
    ) -> Result<RecordFacts, AppError> {
        let facts = self.facts_of(actor, &what.record).await?;
        let level = self.level_on_part(actor, what).await?;
        if level < FieldLevel::CanSee {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        if need == Need::Change && level < FieldLevel::CanChange {
            return Err(AppError::Refused(ErrorCode::Forbidden));
        }
        Ok(facts)
    }

    /// The level the person has on the part: *Hidden* for a part the kind does not list.
    async fn level_on_part(&self, actor: &Actor, what: &EditWhat) -> Result<FieldLevel, AppError> {
        let (kind, part) = (&what.record.kind, what.part.as_str());
        if *kind == EntityKind::from(RecordKind::Issue) {
            return self.level_on_issue_part(actor, what).await;
        }
        let listed = if *kind == EntityKind::from(RecordKind::Layout) {
            part == LAYOUT_PART
        } else if *kind == EntityKind::from(RecordKind::Workflow) {
            part == WORKFLOW_PART
        } else if *kind == EntityKind::from(RecordKind::Comment) {
            part == TEXT_PART
        } else if *kind == EntityKind::from(RecordKind::Project) {
            // The one long text a project can have so far (M3-F49); no screen edits it yet.
            part == member::DESCRIPTION
        } else {
            self.services.records.has_part(&what.record, part)
        };
        Ok(if listed {
            FieldLevel::CanChange
        } else {
            FieldLevel::Hidden
        })
    }

    async fn level_on_issue_part(
        &self,
        actor: &Actor,
        what: &EditWhat,
    ) -> Result<FieldLevel, AppError> {
        let Ok(id) = IssueId::from_bytes(what.record.id.to_bytes()) else {
            return Ok(FieldLevel::Hidden);
        };
        let Some((issue, held)) = self.seen_issue_by_id(actor, id).await? else {
            return Ok(FieldLevel::Hidden);
        };
        let issue_type = Some(issue.issue_type);
        let part = what.part.as_str();
        if part == member::DESCRIPTION {
            return Ok(held.field_level_for(issue_type, FieldKey::BuiltIn(Field::Description)));
        }
        let Some(field) = part
            .strip_prefix(EDIT_PART_PREFIX)
            .and_then(|text| text.parse::<FieldId>().ok())
        else {
            return Ok(FieldLevel::Hidden);
        };
        let long_text =
            self.field_catalogue().await?.iter().any(|definition| {
                definition.field.id == field && definition.field.kind.is_long_text()
            });
        Ok(if long_text {
            held.field_level_for(issue_type, FieldKey::Own(field))
        } else {
            FieldLevel::Hidden
        })
    }
}
