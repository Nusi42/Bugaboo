//! Probes from the review of WP-05: each one tried to get around a rule of the management use
//! cases (give no more than you hold, a *No access* is only lifted by somebody who decides above
//! it, a level is not raised by taking an override away, *Manage groups* is not a way to lift a
//! *No access*) and must be refused. Every probe runs on both databases (ARCH-DB-PARITY).

use bb_app::access::{
    CreateStartingRoles, Permission, PolicyStore as _, StartingRole, create_starting_roles,
};
use bb_app::people::{AddGroupMember, DeleteGroup, PeopleQuery, RemoveGroupMember};
use bb_app::projects::CreateProject;
use bb_app::roles::{
    DeleteRole, Given, ReasonView, RemoveAssignment, ResetRole, RolesQuery, SetFieldLevel,
    SetPeopleLevel, Subject,
};
use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::event::DomainEvent;
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_testing::{TestResult, each_database};

use super::policy_rig::{Member, Org, Run};
use super::rig::{Rig, refused};

mod levels;
mod log;
mod no_access;
mod people;
mod races;
mod sub_projects;

/// What a person who decides roles in a project may hold there.
const LEAD: [Permission; 3] = [
    Permission::SeeProjects,
    Permission::DecideRoles,
    Permission::CheckAccess,
];

/// Refused because the person may not do it, or may not even know the project.
fn not_allowed(code: Option<ErrorCode>) -> bool {
    matches!(
        code,
        Some(ErrorCode::Forbidden | ErrorCode::NotFound | ErrorCode::CannotLift)
    )
}

async fn remove(
    org: &Org<'_>,
    by: &Member,
    row: AssignmentId,
) -> Result<(), bb_app::auth::AppError> {
    org.rig
        .api()
        .remove_assignment(
            Rig::info(),
            RemoveAssignment {
                session_token: by.session.clone(),
                assignment: row,
            },
        )
        .await
        .map(|_| ())
}
