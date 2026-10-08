//! The field catalogue (ADR-0025 decision 10, M2-Q7): the built-in fields that field permissions
//! apply to. Own fields join in M3 (CF-06).
//!
//! The list is declared here once. The policy engine masks by it, the API description and
//! *Check access* name the fields from it, and the screens of the field-permission matrix are
//! made from it.

use crate::people_kinds::PeopleKind;

words! {
    /// A built-in field. The word is the key of the field (`assigned-to`).
    pub enum Field {
        /// *Title*.
        Title => "title",
        /// *Description*.
        Description => "description",
        /// *Status*.
        Status => "status",
        /// *Priority*.
        Priority => "priority",
        /// *Assigned to*.
        AssignedTo => "assigned-to",
        /// *Reported by*.
        ReportedBy => "reported-by",
        /// *Due date*.
        DueDate => "due-date",
        /// *Labels*.
        Labels => "labels",
        /// *Tags*.
        Tags => "tags",
        /// *Planned for release*.
        PlannedForRelease => "planned-for-release",
        /// *Found in release*.
        FoundInRelease => "found-in-release",
        /// *Estimated time*.
        EstimatedTime => "estimated-time",
        /// *Time left*.
        TimeLeft => "time-left",
        /// *Time spent*.
        TimeSpent => "time-spent",
        /// *Activity*.
        Activity => "activity",
        /// *Cost center*.
        CostCenter => "cost-center",
    }
    messages name = "issue-field.";
}

words_in_api!(
    Field,
    "A built-in field of the field catalogue, by its key."
);

/// The thing a field belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BelongsTo {
    /// An issue.
    Issue,
    /// An issue, as the total of its time entries.
    IssueTimeTotal,
    /// A time entry (it has a default on the issue).
    TimeEntry,
    /// A time entry (it has a default on the issue and on the project).
    TimeEntryWithProjectDefault,
}

impl Field {
    /// What the field belongs to.
    #[must_use]
    pub const fn belongs_to(self) -> BelongsTo {
        match self {
            Self::TimeSpent => BelongsTo::IssueTimeTotal,
            Self::Activity => BelongsTo::TimeEntry,
            Self::CostCenter => BelongsTo::TimeEntryWithProjectDefault,
            _ => BelongsTo::Issue,
        }
    }

    /// The kind of people information the field holds, when it holds a person: a person in the
    /// field is shown only when the field is visible, and then as this kind says.
    #[must_use]
    pub const fn person_kind(self) -> Option<PeopleKind> {
        match self {
            Self::AssignedTo => Some(PeopleKind::AssignedTo),
            Self::ReportedBy => Some(PeopleKind::ReportedBy),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m2_q7_the_catalogue_has_the_sixteen_fields_in_order() {
        let keys: Vec<&str> = Field::ALL.iter().map(|field| field.code()).collect();
        assert_eq!(
            keys,
            [
                "title",
                "description",
                "status",
                "priority",
                "assigned-to",
                "reported-by",
                "due-date",
                "labels",
                "tags",
                "planned-for-release",
                "found-in-release",
                "estimated-time",
                "time-left",
                "time-spent",
                "activity",
                "cost-center",
            ]
        );
    }

    #[test]
    fn every_field_has_its_own_name_and_only_two_hold_a_person() {
        let mut seen = std::collections::HashSet::new();
        for field in Field::ALL {
            let key = field.name();
            assert_eq!(key.as_str(), format!("issue-field.{}", field.code()));
            assert!(seen.insert(key));
        }
        let with_person: Vec<Field> = Field::ALL
            .iter()
            .copied()
            .filter(|field| field.person_kind().is_some())
            .collect();
        assert_eq!(with_person, [Field::AssignedTo, Field::ReportedBy]);
        assert_eq!(Field::Activity.belongs_to(), BelongsTo::TimeEntry);
        assert_eq!(Field::TimeSpent.belongs_to(), BelongsTo::IssueTimeTotal);
    }
}
