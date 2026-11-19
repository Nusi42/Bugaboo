//! The fields of an issue in the file of *Download my data* (ADR-0044, decision 3): the file never
//! shows more than the screens would, so a field of an issue the person may not see on screen (an
//! estimate hidden from a Guest, say) is left out of the issues they reported, as their role in the
//! project of the issue, and for its type, says now.

use std::collections::HashMap;
use std::sync::Arc;

use bb_core::fields::Field;
use bb_core::id::{IssueTypeId, ProjectId, UserId};
use bb_policy::{Evaluation, Place};

use crate::auth::{AppError, AuthStore, Ctx};
use crate::privacy::walk::{FoundRow, Value};

/// The fields of an issue row that say where it is and of which type.
const PROJECT: &str = "project";
const ISSUE_TYPE: &str = "issue_type";

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

impl<S: AuthStore> Ctx<'_, S> {
    /// The issue row with only the fields `person` may see of it on screen. A row that does not say
    /// its project shows no field of the catalogue at all.
    pub(super) async fn issue_as_seen(
        &self,
        person: UserId,
        mut row: FoundRow,
        seen: &mut SeenInProjects,
    ) -> Result<FoundRow, AppError> {
        let issue_type = id_in::<IssueTypeId>(&row, ISSUE_TYPE);
        let rights = match id_in::<ProjectId>(&row, PROJECT) {
            Some(project) => {
                if let Some(known) = seen.rights.get(&project) {
                    Some(known.clone())
                } else {
                    let judged = self.evaluate(person, Place::Project(project)).await?;
                    seen.rights.insert(project, judged.clone());
                    Some(judged)
                }
            }
            None => None,
        };
        let mask = rights.map(|rights| rights.field_mask_for(issue_type));
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
    const NOT_ON_THE_ROW: [Field; 7] = [
        Field::AssignedTo,
        Field::Labels,
        Field::Tags,
        Field::PlannedForRelease,
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
