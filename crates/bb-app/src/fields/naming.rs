//! Whether a person, a group or an issue that the value of an own field names may be named by the
//! viewer (CF-06, ADR-0022): the rule that decides, split from `values.rs` to keep that file under
//! the ceiling.

use bb_core::id::{GroupId, IssueId, UserId};
use bb_core::own_fields::FieldType;
use bb_policy::Evaluation;

use super::values::Item;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether a person, group or issue a value names exists and may be named by this viewer.
    /// A person has to be one the viewer may name (themselves, or someone they are shown by name)
    /// and, with `access`, who may see the item: the rule of *Assigned to* (`may_name`); a group
    /// one whose name the viewer may read. Without `access` (*Undo*) a person who lost access
    /// since is still named, but one who is gone is not.
    pub(super) async fn reference_may_be_named(
        &self,
        actor: &Actor,
        viewer: &Evaluation,
        item: &Item,
        kind: FieldType,
        bytes: [u8; 16],
        access: bool,
    ) -> Result<bool, AppError> {
        let failed = |error| internal::<S>(&error);
        Ok(match kind {
            FieldType::Person | FieldType::People => {
                let Ok(person) = UserId::from_bytes(bytes) else {
                    return Ok(false);
                };
                let Some(project) = item.chain.last() else {
                    return Ok(false);
                };
                if access {
                    self.may_name(actor.person.id, viewer, person, *project, item.reported_by)
                        .await?
                } else {
                    self.may_name_person(actor.person.id, viewer, person)
                        .await?
                }
            }
            FieldType::Group => match GroupId::from_bytes(bytes) {
                Ok(group) => {
                    viewer.sees_names_in(kind)
                        && self.store().group(group).await.map_err(failed)?.is_some()
                }
                Err(_) => false,
            },
            FieldType::Issue => match IssueId::from_bytes(bytes) {
                // The viewer may only name an issue they see.
                Ok(id) => self.seen_issue_by_id(actor, id).await?.is_some(),
                Err(_) => false,
            },
            _ => true,
        })
    }
}
