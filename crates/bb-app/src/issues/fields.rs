//! What a person asks for in the fields of an issue: the texts are cleaned the way every short
//! text is, the people have to be members of the project, and every field the person touches has to be one they may
//! change (the mask of their roles, SEC-04).

use bb_core::ErrorCode;
use bb_core::fields::Field;
use bb_core::id::{ProjectId, UserId};
use bb_core::issue::ASSIGNED_PEOPLE_MAX;
use bb_core::text::TITLE_MAX;
use bb_policy::{Evaluation, FieldMask};

use super::types::{ChangeIssue, CreateIssue};
use crate::auth::{AppError, AuthStore, Ctx};
use crate::fields::is_kept;
use crate::input::clean_text;

/// The title as it is kept, or `request.invalid`.
pub(crate) fn clean_title(title: &str) -> Result<String, AppError> {
    clean_text(title, TITLE_MAX)
}

/// Refuses (`access.forbidden`) unless the mask lets the person change every one of the fields.
pub(crate) fn ensure_may_change(mask: &FieldMask, fields: &[Field]) -> Result<(), AppError> {
    if fields.iter().all(|field| mask.can_change(*field)) {
        Ok(())
    } else {
        Err(ErrorCode::Forbidden.into())
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The people an issue of the project is assigned to, each once in the order given: those who
    /// are not assigned already have to be ones `actor` may name (`may_name`: themselves, or
    /// anybody when `viewer` is shown people by name) and able to see the issue (judged for an
    /// issue that `reported_by` reported), and there are not more than an issue may have. People
    /// assigned already stay, whatever happened to their access since, so an issue stays
    /// changeable. `request.invalid` otherwise, the same for a person who does not exist and for
    /// one who may not see the issue, so the answer is no way to find out who exists.
    pub(crate) async fn clean_assigned_people(
        &self,
        actor: UserId,
        viewer: &Evaluation,
        project: ProjectId,
        reported_by: UserId,
        people: &[UserId],
        already: &[UserId],
    ) -> Result<Vec<UserId>, AppError> {
        let mut distinct: Vec<UserId> = Vec::with_capacity(people.len());
        for person in people {
            if !distinct.contains(person) {
                distinct.push(*person);
            }
        }
        if distinct.len() > ASSIGNED_PEOPLE_MAX {
            return Err(ErrorCode::InvalidRequest.into());
        }
        for person in distinct.iter().filter(|person| !is_kept(already, person)) {
            if !self
                .may_name(actor, viewer, *person, project, Some(reported_by))
                .await?
            {
                return Err(ErrorCode::InvalidRequest.into());
            }
        }
        Ok(distinct)
    }
}

/// The fields of the catalogue that a request to make or change an issue names: the others (dates,
/// time) come with the packages that store them, and tags and labels are put on and taken off on
/// their own routes (`tags`).
pub(crate) const CHANGEABLE: [Field; 5] = [
    Field::Title,
    Field::Description,
    Field::Status,
    Field::Priority,
    Field::AssignedTo,
];

/// Which fields of the catalogue a request names: given, whether the value differs from the one
/// stored or not. The one place that says which member of a request is which field, for making
/// and for changing: one flag for each field of `CHANGEABLE`, in its order. The mask is checked
/// against these before anything in the request is read.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Given([bool; CHANGEABLE.len()]);

impl Given {
    /// The fields that are given, in the order of `CHANGEABLE`.
    pub(crate) fn fields(self) -> Vec<Field> {
        CHANGEABLE
            .into_iter()
            .zip(self.0)
            .filter_map(|(field, given)| given.then_some(field))
            .collect()
    }
}

impl From<&ChangeIssue> for Given {
    fn from(input: &ChangeIssue) -> Self {
        // Title, description (saved under its lock, never here), status, priority, assigned to.
        Self([
            input.title.is_some(),
            false,
            input.status.is_some(),
            input.priority.is_some(),
            input.assigned_to.is_some(),
        ])
    }
}

impl From<&CreateIssue> for Given {
    fn from(input: &CreateIssue) -> Self {
        Self([
            // The title is always there.
            true,
            input.description.is_some(),
            input.status.is_some(),
            input.priority.is_some(),
            input.assigned_to.is_some(),
        ])
    }
}

/// The names of the members of an issue and of an issue type in their events, which *History*
/// reads: defined once, here. A field of the catalogue has the word of the catalogue
/// (`Field::code`), so History filters by one mapping.
pub(crate) mod member {
    use bb_core::fields::Field;

    pub(crate) const PROJECT: &str = "project";
    pub(crate) const ISSUE_TYPE: &str = "issue-type";
    /// The readable ID of the issue.
    pub(crate) const KEY: &str = "key";
    pub(crate) const TITLE: &str = Field::Title.code();
    pub(crate) const DESCRIPTION: &str = Field::Description.code();
    pub(crate) const STATUS: &str = Field::Status.code();
    pub(crate) const PRIORITY: &str = Field::Priority.code();
    pub(crate) const REPORTED_BY: &str = Field::ReportedBy.code();
    pub(crate) const ASSIGNED_TO: &str = Field::AssignedTo.code();
    pub(crate) const TAGS: &str = Field::Tags.code();
    pub(crate) const LABELS: &str = Field::Labels.code();
    pub(crate) const DELETED_AT: &str = "deleted-at";
    /// The issue it sits under (the ID of the parent), in the event of the sub-issue.
    pub(crate) const PARENT: &str = "parent";
    /// The project says its issues got the IDs of the mode it has now, in the event of the project.
    pub(crate) const ISSUE_IDS: &str = "issue-ids";
    /// A sub-issue that was added or taken away, in the event of the parent as History writes it.
    pub(crate) const SUB_ISSUE: &str = "sub-issue";

    /// The members of an issue type.
    pub(crate) mod issue_type {
        pub(crate) const NAME: &str = "name";
        pub(crate) const ICON: &str = "icon";
        pub(crate) const RETIRED: &str = "retired";
        /// The list of the types the first start made.
        pub(crate) const STARTING: &str = "issue-types";
    }
}
