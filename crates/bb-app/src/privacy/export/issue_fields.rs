//! The fields of an issue in the file of *Download my data* (ADR-0044, decision 3): the file never
//! shows more than the screens would, so a field of an issue the person may not see on screen (an
//! estimate hidden from a Guest, say) is left out of the issues they reported, as their role in the
//! project of the issue, and for its type, says now. The same holds for the value of an own field
//! of an issue, which the walk finds as a row of its own (a field of people that names the person):
//! it is left out where the field is hidden from the person on that issue (M9-F25).

use std::collections::HashMap;
use std::sync::Arc;

use bb_core::event::RecordKind;
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{FieldId, IssueId, IssueTypeId, ProjectId, UserId};
use bb_policy::{Evaluation, FieldMask, Place};

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::privacy::walk::{FoundRow, Value};

/// The fields of an issue row that say where it is and of which type.
const PROJECT: &str = "project";
const ISSUE_TYPE: &str = "issue_type";

/// The fields of a row of an own field's value that say of which item and which field it is.
const ITEM_KIND: &str = "entity_kind";
const ITEM: &str = "entity_id";
const OWN_FIELD: &str = "field";

/// The field of the catalogue a member of the issue holds: `estimated_time` holds *Estimated time*.
/// A member that holds none (its ID, its code, when it was made) is no field a role can hide. The
/// one mapping of a member to its field: the matrix of exports reads it too.
#[must_use]
pub fn field_of(member: &str) -> Option<Field> {
    Field::from_code(&member.replace('_', "-"))
}

/// The ID a cell of the row holds, read as `T`.
fn id_in<T: std::str::FromStr>(row: &FoundRow, field: &str) -> Option<T> {
    row.cells.iter().find_map(|cell| match &cell.value {
        Value::Id(text) if cell.field == field => text.parse().ok(),
        _ => None,
    })
}

/// What a person may see in each project, judged once for the whole file.
#[derive(Default)]
pub(super) struct SeenInProjects {
    rights: HashMap<ProjectId, Arc<Evaluation>>,
}

/// The text a cell of the row holds.
fn text_in<'a>(row: &'a FoundRow, field: &str) -> Option<&'a str> {
    row.cells.iter().find_map(|cell| match &cell.value {
        Value::Text(text) if cell.field == field => Some(text.as_str()),
        _ => None,
    })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The mask of the fields `person` may see on an issue of `issue_type` in `project`, judged
    /// once for each project of the file.
    async fn mask_in(
        &self,
        person: UserId,
        (project, issue_type): (ProjectId, Option<IssueTypeId>),
        seen: &mut SeenInProjects,
    ) -> Result<FieldMask, AppError> {
        let rights = if let Some(known) = seen.rights.get(&project) {
            known.clone()
        } else {
            let judged = self.evaluate(person, Place::Project(project)).await?;
            seen.rights.insert(project, judged.clone());
            judged
        };
        Ok(rights.field_mask_for(issue_type))
    }

    /// The row of an own field's value, unless it is the value of a field of an issue that is
    /// hidden from `person` there: then none, as the screens show none. A value of a project, or
    /// of an issue that is gone, stays as it is.
    pub(super) async fn own_value_as_seen(
        &self,
        person: UserId,
        row: FoundRow,
        seen: &mut SeenInProjects,
    ) -> Result<Option<FoundRow>, AppError> {
        let of_issue = text_in(&row, ITEM_KIND) == Some(RecordKind::Issue.code());
        let (Some(issue), Some(field)) = (
            id_in::<IssueId>(&row, ITEM),
            id_in::<FieldId>(&row, OWN_FIELD),
        ) else {
            return Ok(Some(row));
        };
        if !of_issue {
            return Ok(Some(row));
        }
        let Some(found) = self
            .store()
            .issue(issue)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(Some(row));
        };
        let mask = self
            .mask_in(person, (found.project, Some(found.issue_type)), seen)
            .await?;
        Ok(mask.can_see(FieldKey::Own(field)).then_some(row))
    }

    /// The issue row with only the fields `person` may see of it on screen. A row that does not say
    /// its project shows no field of the catalogue at all.
    pub(super) async fn issue_as_seen(
        &self,
        person: UserId,
        mut row: FoundRow,
        seen: &mut SeenInProjects,
    ) -> Result<FoundRow, AppError> {
        let issue_type = id_in::<IssueTypeId>(&row, ISSUE_TYPE);
        let mask = match id_in::<ProjectId>(&row, PROJECT) {
            Some(project) => Some(self.mask_in(person, (project, issue_type), seen).await?),
            None => None,
        };
        row.cells.retain(|cell| match field_of(cell.field) {
            Some(field) => mask.as_ref().is_some_and(|mask| mask.can_see(field)),
            None => true,
        });
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priv_03_a_member_of_an_issue_is_read_as_its_field_of_the_catalogue() {
        assert_eq!(field_of("estimated_time"), Some(Field::EstimatedTime));
        assert_eq!(field_of("status"), Some(Field::Status));
        assert_eq!(field_of("reported_by"), Some(Field::ReportedBy));
        assert_eq!(field_of("project"), None);
        assert_eq!(field_of("created_at"), None);
    }

    /// The fields of the catalogue that are no member of the issue row: rows of their own hold them
    /// (the people assigned, labels, tags, releases), they are worked out (time spent), or they are
    /// kept for each person (the time factor).
    const NOT_ON_THE_ROW: [Field; 6] = [
        Field::AssignedTo,
        Field::Labels,
        Field::Tags,
        Field::FoundInRelease,
        Field::TimeSpent,
        Field::TimeFactor,
    ];

    #[test]
    fn priv_03_every_field_of_the_catalogue_on_the_issue_row_is_read_as_that_field() {
        use bb_core::personal::Entity as _;
        let on_the_row: Vec<Field> = bb_core::issue::Issue::MARKS
            .fields()
            .iter()
            .filter_map(|mark| field_of(mark.name()))
            .collect();
        // A member renamed away from its field would be shown to a person the field is hidden from.
        for field in Field::ALL {
            assert_ne!(
                on_the_row.contains(field),
                NOT_ON_THE_ROW.contains(field),
                "{field:?} is either a member of the issue row read as its field, or listed here"
            );
        }
    }
}
