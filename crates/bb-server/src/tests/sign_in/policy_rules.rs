//! The rules that keep the organization from locking itself out and from giving more than a person
//! has, the starting roles, and the record of every change (SEC-02, SEC-08, SEC-15, ADR-0025).

use bb_app::access::{
    CreateStartingRoles, Permission, PersonRef, StartingRole, create_starting_roles,
};
use bb_app::auth::AppError;
use bb_app::people::{AddGroupMember, CreateGroup, DeleteGroup, RemoveGroupMember, RenameGroup};
use bb_app::roles::{
    AssignmentView, AssignmentsQuery, ChangeRole, CreateRole, DeleteRole, Given, LevelsQuery,
    RemoveAssignment, ResetRole, RoleView, RolesQuery, SetFieldLevel, SetPeopleLevel, Subject,
    WhoView,
};
use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, RoleId};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_store::Verification;
use bb_testing::{TestResult, each_database};

use super::policy_rig::{Member, Org, Run};
use super::rig::{Rig, refused};

mod holding;
mod journal;
mod managing;
mod starting;

/// The roles, as the administrator lists them.
async fn roles(org: &Org<'_>) -> Run<Vec<RoleView>> {
    Ok(org
        .rig
        .api()
        .roles(
            Rig::info(),
            RolesQuery {
                session_token: org.admin.session.clone(),
                place: None,
                cursor: None,
                limit: bb_core::paging::PAGE_SIZE_MAX,
                giveable: false,
            },
        )
        .await?
        .into_value()
        .roles)
}

async fn role_named(org: &Org<'_>, name: &str) -> Run<RoleView> {
    roles(org)
        .await?
        .into_iter()
        .find(|role| role.name == name)
        .ok_or_else(|| format!("no role named {name}").into())
}

/// The rows of *Who has which role* for the whole organization, as the administrator lists them.
async fn rows_of_the_organization(org: &Org<'_>) -> Run<Vec<AssignmentView>> {
    Ok(org
        .rig
        .api()
        .assignments(
            Rig::info(),
            AssignmentsQuery {
                session_token: org.admin.session.clone(),
                project: None,
                cursor: None,
                limit: bb_core::paging::PAGE_SIZE_MAX,
            },
        )
        .await?
        .into_value()
        .rows)
}

/// The row of the administrator, found by who it is for (rows made in the same moment have no
/// order of their own).
async fn ada_row(org: &Org<'_>) -> Run<AssignmentId> {
    rows_of_the_organization(org)
        .await?
        .into_iter()
        .find(|row| {
            matches!(&row.who, WhoView::Person { person: PersonRef::Name { id, .. } } if *id == org.admin.id)
        })
        .map(|row| row.id)
        .ok_or_else(|| "Ada's row".into())
}

/// Changes a role as somebody.
async fn change(
    org: &Org<'_>,
    by: &Member,
    role: RoleId,
    permissions: Option<Vec<Permission>>,
) -> Result<RoleView, AppError> {
    let version = org
        .rig
        .role_version(role)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(org
        .rig
        .api()
        .change_role(
            Rig::info(),
            ChangeRole {
                session_token: by.session.clone(),
                role,
                version,
                name: None,
                permissions,
                two_step: None,
            },
        )
        .await?
        .into_value())
}
