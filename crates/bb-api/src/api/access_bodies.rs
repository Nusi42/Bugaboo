//! The small shapes that the routes of people, groups, roles and *Who has which role* share: a
//! group, a role and a place named in an answer, and where a role counts.
//!
//! A schema whose name ends in `Ref` is what a name in a sentence needs (ARCH-LEAN), and so is
//! allowed inside the items of a list.

use bb_app::access::ProjectLabel;
use bb_app::people::{GroupView, HeldRole};
use bb_app::roles::RoleRef as AppRoleRef;
use bb_app::workflows::WorkflowListItem;
use bb_core::ErrorCode;
use bb_core::access::AppliesTo;
use bb_policy::Masked;
use serde::Serialize;
use utoipa::ToSchema;

use crate::errors::ApiError;

/// A role or a group named in an answer: its ID and what it is called (empty when it was deleted
/// meanwhile). One schema for everything that is only an ID and a name.
#[derive(Serialize, ToSchema)]
pub(crate) struct NamedRef {
    /// The ID.
    pub(crate) id: String,
    /// What it is called.
    pub(crate) name: String,
}

impl From<&GroupView> for NamedRef {
    fn from(group: &GroupView) -> Self {
        Self {
            id: group.id.to_string(),
            name: group.name.clone(),
        }
    }
}

impl From<&AppRoleRef> for NamedRef {
    fn from(role: &AppRoleRef) -> Self {
        Self {
            id: role.id.to_string(),
            name: role.name.clone(),
        }
    }
}

/// Where a row of *Who has which role* is: the whole organization, a project, or a project the
/// person cannot see (which is not named).
#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum PlaceRef {
    /// The whole organization.
    Organization,
    /// A project the person may know about.
    Project {
        /// The ID of the project.
        id: String,
        /// The letters of its issue IDs and address.
        short_name: String,
        /// What it is called.
        name: String,
    },
    /// A project the person cannot see: it is not named.
    Hidden,
}

impl Masked for PlaceRef {}

impl From<Option<&ProjectLabel>> for PlaceRef {
    fn from(label: Option<&ProjectLabel>) -> Self {
        match label {
            None => Self::Organization,
            Some(ProjectLabel::Visible {
                id,
                short_name,
                name,
            }) => Self::Project {
                id: id.to_string(),
                short_name: short_name.clone(),
                name: name.clone(),
            },
            Some(ProjectLabel::Hidden) => Self::Hidden,
        }
    }
}

/// A group or a workflow in a list: the members that were asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct NamedVersionItem {
    /// The ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<String>,
    /// What it is called.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    /// Counts up with every change. A change is made on the version that was looked at.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) version: Option<i64>,
}

impl Masked for NamedVersionItem {}

impl From<&GroupView> for NamedVersionItem {
    fn from(group: &GroupView) -> Self {
        Self {
            id: Some(group.id.to_string()),
            name: Some(group.name.clone()),
            version: Some(group.version),
        }
    }
}

impl From<&WorkflowListItem> for NamedVersionItem {
    fn from(item: &WorkflowListItem) -> Self {
        Self {
            id: Some(item.id.to_string()),
            name: Some(item.name.clone()),
            version: Some(item.version),
        }
    }
}

/// A role (or *No access*) that somebody has, and where: what a sentence like "Team member in
/// Website" needs.
#[derive(Serialize, ToSchema)]
pub(crate) struct RoleAtRef {
    /// The ID of the row of *Who has which role*.
    id: String,
    /// The role; `null` for *No access*.
    role: Option<NamedRef>,
    /// Where the row is.
    place: PlaceRef,
    /// Where it counts.
    applies_to: AppliesTo,
}

impl From<&HeldRole> for RoleAtRef {
    fn from(held: &HeldRole) -> Self {
        Self {
            id: held.id.to_string(),
            role: held.role.as_ref().map(NamedRef::from),
            place: PlaceRef::from(held.project.as_ref()),
            applies_to: held.applies_to,
        }
    }
}

/// The error a use case gives for a cursor it cannot read is `request.invalid`; for a list route
/// that is a cursor that is not one of ours.
pub(crate) fn cursor_error(error: bb_app::auth::AppError) -> ApiError {
    match error {
        bb_app::auth::AppError::Refused(ErrorCode::InvalidRequest) => {
            ApiError::new(ErrorCode::BadCursor)
        }
        other => ApiError::from(other),
    }
}
