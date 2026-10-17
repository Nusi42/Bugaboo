//! One scenario for every command use case (M1-F18): the preparation, the call, and how many events
//! the call added. The table at the end names each use case once; `events.rs` runs them all and
//! checks that the table is complete (against the list of use cases of `AuthApi`, and against the
//! code through ARCH-EVENTS).

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_auth::TotpSecret;
use bb_core::access::AppliesTo;
use bb_core::access_key::KeyChoice;
use bb_core::edit::EditWhat;
use bb_core::event::{EntityRef, RecordKind};
use bb_core::fields::Field;
use bb_core::id::{EditorId, EntityId, GroupId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::settings::SettingValue;
use bb_core::technical::TechnicalName;
use bb_core::version::FIRST_VERSION;
use serde_json::Value;
use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator as _};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;

use super::fake::FakeStore;
use super::world::{OTHER_PASSWORD, PASSWORD, PUBLIC_URL, World};
use crate::access::{CreateStartingRoles, Permission, create_starting_roles};
use crate::auth::{
    AddPasskey, ChangePassword, ChangeSignUpPolicy, CompleteSetup, ConfirmCodeApp, CreateAccessKey,
    CreateInvitation, DeleteAccessKey, EndSession, IssueSetupCode, PasskeyRegistrationQuery,
    PasskeySecondStep, PasskeySecondStepQuery, PasskeySignIn, PasskeySignInQuery, RemoveCodeApp,
    RemovePasskey, RenewBackupCodes, RevokeInvitation, SecondStep, SignIn, SignOut,
    SignOutEverywhere, SignUp, SignUpPolicy, StartCodeApp, Started, accept_invitation, add_passkey,
    change_password, change_sign_up_policy, complete_setup, confirm_code_app, create_invitation,
    end_session, issue_setup_code, passkey_registration_options, passkey_second_step,
    passkey_second_step_options, passkey_sign_in, passkey_sign_in_options, remove_code_app,
    remove_passkey, renew_backup_codes, revoke_invitation, second_step, sessions, sign_in,
    sign_out, sign_out_everywhere, sign_up, start_code_app,
};
use crate::auth::{SecondStepStore as _, create_access_key, delete_access_key};
use crate::editing::{
    AskToFinish, EditAnyway, ReleaseStuckLock, TakeLockQuery, ask_to_finish, edit_anyway,
    release_stuck_lock, take_lock,
};
use crate::people::{
    AddGroupMember, CreateGroup, DeleteGroup, RemoveGroupMember, RenameGroup, add_group_member,
    create_group, delete_group, remove_group_member, rename_group,
};
use crate::projects::{
    ChangeProject, CreateProject, MovePreviewQuery, MoveProject, change_project, create_project,
    move_project, preview_move,
};
use crate::roles::{
    AssignRole, ChangeRole, CopyRole, CreateRole, DeleteRole, Given, RemoveAssignment, ResetRole,
    SetFieldLevel, SetPeopleLevel, Subject, assign_role, change_role, copy_role, create_role,
    delete_role, remove_assignment, reset_role, set_level,
};
use crate::settings::{
    ChangeSetting, LockSetting, ResetSetting, SettingsScope, change_setting, lock_setting,
    reset_setting,
};

mod invitations;
mod keys_and_editing;
mod people;
mod settings;
mod sign_in;

pub(crate) use invitations::*;
pub(crate) use keys_and_editing::*;
pub(crate) use people::*;
pub(crate) use settings::*;
pub(crate) use sign_in::*;

/// The events one call added.
pub(crate) type Added = Result<usize, String>;

pub(crate) fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Runs the call that is under test and says how many events it appended.
pub(crate) async fn measured<T, E: std::fmt::Display>(
    store: &FakeStore,
    call: impl Future<Output = Result<T, E>>,
) -> Added {
    let before = store.event_count();
    call.await.map_err(text)?;
    Ok(store.event_count() - before)
}

fn origin() -> Result<Url, String> {
    Url::parse(PUBLIC_URL).map_err(text)
}

// ---- Signing in ----

// ---- Invitations, signing up and setup ----

// ---- Groups, roles, rows, levels and projects ----

pub(crate) fn token(world: &World) -> String {
    world.admin.session_token.clone()
}

async fn a_group(world: &World, name: &str) -> Result<GroupId, String> {
    Ok(create_group(
        &world.ctx(),
        CreateGroup {
            session_token: token(world),
            name: name.to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id)
}

async fn a_role(world: &World, name: &str) -> Result<RoleId, String> {
    Ok(create_role(
        &world.ctx(),
        CreateRole {
            session_token: token(world),
            name: name.to_owned(),
            permissions: vec![Permission::SeeProjects],
            two_step: false,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id)
}

pub(crate) async fn a_project(world: &World, short_name: &str) -> Result<ProjectId, String> {
    Ok(create_project(
        &world.ctx(),
        CreateProject {
            session_token: token(world),
            parent: None,
            short_name: short_name.to_owned(),
            name: "A project".to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id)
}

async fn a_person(world: &World) -> Result<UserId, String> {
    Ok(world
        .person("bob@example.test")
        .await
        .map_err(text)?
        .person
        .id)
}

// ---- Access keys ----
