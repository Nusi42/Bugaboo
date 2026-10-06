//! *Used by* (UX §5.1, PRIV-13): what a group or a role is used for, shown before it is changed or
//! deleted ("Used in 4 projects").
//!
//! The numbers count only what the viewer may see: a row on a project the viewer cannot see is
//! left out, and so are the people and groups that only such a row names. The whole organization
//! is always seen. Whether a thing is used at all, which decides if a role can be deleted without
//! a replacement, is judged on every row, and says only yes or no.

use std::collections::BTreeSet;

use bb_core::access::{Assignment, Group};
use bb_core::id::{GroupId, ProjectId, UserId};
use bb_policy::Masked;
use serde::Serialize;

use crate::access::{Actor, ProjectLabel};
use crate::auth::{AppError, AuthStore, Ctx};

/// What a group is used for, as far as the viewer may see.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GroupUsedBy {
    /// Whether it has a role for the whole organization.
    pub whole_organization: bool,
    /// In how many projects the viewer can see that it has a role or *No access*.
    pub projects: u64,
    /// How many people are in it (people who manage groups see all of them).
    pub members: u64,
}

/// What a role is used for, as far as the viewer may see.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoleUsedBy {
    /// Whether somebody has it for the whole organization.
    pub whole_organization: bool,
    /// In how many projects the viewer can see that somebody has it.
    pub projects: u64,
    /// How many people have it through a row of their own, in the places the viewer sees.
    pub people: u64,
    /// How many groups have it, in the places the viewer sees.
    pub groups: u64,
}

/// A group as the page of a group shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GroupDetail {
    /// The group.
    pub group: super::GroupView,
    /// What it is used for.
    pub used_by: GroupUsedBy,
}

impl Masked for GroupDetail {}

/// The rows that the viewer sees, summed up.
pub(crate) struct SeenRows {
    pub(crate) whole_organization: bool,
    pub(crate) projects: BTreeSet<ProjectId>,
    pub(crate) people: BTreeSet<UserId>,
    pub(crate) groups: BTreeSet<GroupId>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The rows in the places the viewer sees: the whole organization, and the projects they
    /// may know about.
    pub(crate) async fn rows_seen_by(
        &self,
        actor: &Actor,
        rows: &[Assignment],
    ) -> Result<SeenRows, AppError> {
        let mut seen = SeenRows {
            whole_organization: false,
            projects: BTreeSet::new(),
            people: BTreeSet::new(),
            groups: BTreeSet::new(),
        };
        let mut known: BTreeSet<ProjectId> = BTreeSet::new();
        for row in rows {
            match row.project {
                None => seen.whole_organization = true,
                Some(project) => {
                    if !known.contains(&project) {
                        if !matches!(
                            self.project_label(actor, project).await?,
                            ProjectLabel::Visible { .. }
                        ) {
                            continue;
                        }
                        known.insert(project);
                    }
                    seen.projects.insert(project);
                }
            }
            seen.people.extend(row.person);
            seen.groups.extend(row.group);
        }
        Ok(seen)
    }

    /// The group with what it is used for.
    pub(crate) async fn group_used_by(
        &self,
        actor: &Actor,
        group: &Group,
        members: usize,
    ) -> Result<GroupUsedBy, AppError> {
        let rows = self
            .store()
            .assignments_of_group(group.id)
            .await
            .map_err(|error| crate::auth::internal::<S>(&error))?;
        let seen = self.rows_seen_by(actor, &rows).await?;
        Ok(GroupUsedBy {
            whole_organization: seen.whole_organization,
            projects: u64::try_from(seen.projects.len()).unwrap_or(u64::MAX),
            members: u64::try_from(members).unwrap_or(u64::MAX),
        })
    }
}
