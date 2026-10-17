//! Access keys (AUTH-07, SEC-14, SEC-15, SEC-16, ADR-0025 decision 8), through the real wiring:
//! the use cases of `bb-app`, the store of `bb-store` and, for the HTTP tests, the router of
//! `bb-api`. Every flow runs on SQLite and on PostgreSQL (`each_database`, ARCH-DB-PARITY).
//!
//! The test organization of `policy_rig` has the administrator Ada and people without a role;
//! Bob gets the role *Checker*: *Check access* (a *See* permission), *Create projects* (*Change*)
//! and *Invite people* (*Manage*), one of each kind a key can keep.

use std::fmt::Write as _;

use bb_app::access::Permission;
use bb_app::auth::{
    AccessKeyRulesQuery, AccessKeyView, AccessKeysQuery, AppError, ChangePassword, CreateAccessKey,
    CreatedAccessKey, DeleteAccessKey, InvitationsQuery, ProjectChoicesQuery, RequestInfo,
    StartCodeApp, TwoStepQuery,
};
use bb_app::projects::CreateProject;
use bb_app::roles::{CheckAccessQuery, RemoveAssignment};
use bb_auth::AccessKeySecret;
use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::id::ProjectId;
use bb_core::network::NetworkPrefix;
use bb_core::sign_in::SecurityAction;
use bb_core::technical::TechnicalName;
use bb_store::Verification;
use bb_testing::{TestResult, each_database};
use jiff::Timestamp;

use super::http::ADA_EMAIL;
use super::policy_rig::{Member, Org, Run};
use super::rig::{PASSWORD, Rig, Tries, refused};

mod choices;
mod log_and_limits;
mod making;
mod scope;
mod sign_in_data;

const DAY: i64 = 24 * 3600;

/// The moment `seconds` after `moment`.
pub(super) fn later(moment: Timestamp, seconds: i64) -> Timestamp {
    Timestamp::from_second(moment.as_second() + seconds).unwrap_or(moment)
}

/// What Bob may do in these tests: one permission of each kind a key keeps.
const CHECKER: [Permission; 4] = [
    Permission::SeeProjects,
    Permission::CheckAccess,
    Permission::CreateProjects,
    Permission::ManageInvitations,
];

/// A key of `owner` that runs out when it should, for the tests that use it over HTTP.
pub(super) async fn made(
    rig: &Rig,
    owner: &Member,
    choice: KeyChoice,
    expires_at: Option<String>,
) -> Run<CreatedAccessKey> {
    Ok(create_with(rig, owner, choice, &[], &[], expires_at).await?)
}

/// The secret of a new key of `owner`.
pub(super) async fn key_of(rig: &Rig, owner: &Member, choice: KeyChoice) -> Run<String> {
    Ok(made(rig, owner, choice, None).await?.secret)
}

/// A key of `owner`: the answer, or why it was refused.
pub(super) async fn create(
    rig: &Rig,
    owner: &Member,
    choice: KeyChoice,
    permissions: &[Permission],
    projects: &[ProjectId],
) -> Result<CreatedAccessKey, AppError> {
    create_with(rig, owner, choice, permissions, projects, None).await
}

async fn create_with(
    rig: &Rig,
    owner: &Member,
    choice: KeyChoice,
    permissions: &[Permission],
    projects: &[ProjectId],
    expires_at: Option<String>,
) -> Result<CreatedAccessKey, AppError> {
    Ok(rig
        .api()
        .create_access_key(
            Rig::info(),
            CreateAccessKey {
                session_token: owner.session.clone(),
                password: PASSWORD.to_owned(),
                name: "Nightly report".to_owned(),
                choice,
                permissions: permissions.to_vec(),
                projects: projects.to_vec(),
                expires_at,
            },
        )
        .await?
        .into_value())
}

/// *Check access* for Bob at a project, asked with `credential` (a key or a session).
pub(super) async fn check(
    rig: &Rig,
    credential: &str,
    person: bb_core::id::UserId,
    place: Option<ProjectId>,
) -> Result<(), AppError> {
    rig.api()
        .check_access(
            Rig::info(),
            CheckAccessQuery {
                issue_type: None,
                session_token: credential.to_owned(),
                person,
                place,
            },
        )
        .await
        .map(|_| ())
}

/// A request that every key of a person with the role *Checker* may make, the one the tests of
/// keys use to see whether a key works: *Check access* for the administrator.
pub(super) async fn probe(rig: &Rig, credential: &str) -> Result<(), AppError> {
    let ada = rig
        .person_id(ADA_EMAIL)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    check(rig, credential, ada, None).await
}

/// *Create projects* at the top level, asked with `credential`.
async fn create_project(rig: &Rig, credential: &str, short_name: &str) -> Result<(), AppError> {
    rig.api()
        .create_project(
            Rig::info(),
            CreateProject {
                session_token: credential.to_owned(),
                parent: None,
                short_name: short_name.to_owned(),
                name: format!("Project {short_name}"),
            },
        )
        .await
        .map(|_| ())
}

/// *Invite people*'s list, asked with `credential`.
async fn invitations(rig: &Rig, credential: &str) -> Result<(), AppError> {
    rig.api()
        .invitations(
            Rig::info(),
            InvitationsQuery {
                session_token: credential.to_owned(),
                cursor: None,
                limit: 10,
            },
        )
        .await
        .map(|_| ())
}

/// Bob, with the role *Checker* for the whole organization.
pub(super) async fn bob(org: &Org<'_>) -> Run<Member> {
    let bob = org.person().await?;
    let role = org.role("Checker", &CHECKER).await?;
    org.give_everywhere(&bob, role.id).await?;
    Ok(bob)
}

async fn keys_of(rig: &Rig, owner: &Member) -> Run<Vec<AccessKeyView>> {
    Ok(rig
        .api()
        .access_keys(
            Rig::info(),
            AccessKeysQuery {
                session_token: owner.session.clone(),
                cursor: None,
                limit: 100,
            },
        )
        .await?
        .into_value()
        .items)
}
