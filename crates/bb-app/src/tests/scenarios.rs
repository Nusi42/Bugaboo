//! One scenario for every command use case (M1-F18): the preparation, the call, and how many events
//! the call added. The table at the end names each use case once; `events.rs` runs them all and
//! checks that the table is complete (against the list of use cases of `AuthApi`, and against the
//! code through ARCH-EVENTS).

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_auth::TotpSecret;
use bb_core::access::AppliesTo;
use bb_core::access_key::KeyChoice;
use bb_core::fields::Field;
use bb_core::id::{GroupId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::settings::SettingValue;
use serde_json::Value;
use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator as _};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;

use super::fake::FakeStore;
use super::world::{OTHER_PASSWORD, PASSWORD, PUBLIC_URL, World};
use crate::access::{MakeStartingRoles, Permission, make_starting_roles};
use crate::auth::{
    AddPasskey, ChangePassword, ChangeSignUpPolicy, CompleteSetup, ConfirmCodeApp,
    CreateInvitation, DeleteAccessKey, EndSession, IssueSetupCode, MakeAccessKey,
    PasskeyRegistrationQuery, PasskeySecondStep, PasskeySecondStepQuery, PasskeySignIn,
    PasskeySignInQuery, RemoveCodeApp, RemovePasskey, RenewBackupCodes, RevokeInvitation,
    SecondStep, SignIn, SignOut, SignOutEverywhere, SignUp, SignUpPolicy, StartCodeApp, Started,
    accept_invitation, add_passkey, change_password, change_sign_up_policy, complete_setup,
    confirm_code_app, create_invitation, end_session, issue_setup_code,
    passkey_registration_options, passkey_second_step, passkey_second_step_options,
    passkey_sign_in, passkey_sign_in_options, remove_code_app, remove_passkey, renew_backup_codes,
    revoke_invitation, second_step, sessions, sign_in, sign_out, sign_out_everywhere, sign_up,
    start_code_app,
};
use crate::auth::{SecondStepStore as _, delete_access_key, make_access_key};
use crate::people::{
    AddGroupMember, DeleteGroup, MakeGroup, RemoveGroupMember, RenameGroup, add_group_member,
    delete_group, make_group, remove_group_member, rename_group,
};
use crate::projects::{
    ChangeProject, MakeProject, MovePreviewQuery, MoveProject, change_project, make_project,
    move_project, preview_move,
};
use crate::roles::{
    AssignRole, ChangeRole, CopyRole, DeleteRole, Given, MakeRole, RemoveAssignment, ResetRole,
    SetFieldLevel, SetPeopleLevel, Subject, assign_role, change_role, copy_role, delete_role,
    make_role, remove_assignment, reset_role, set_level,
};
use crate::settings::{
    ChangeSetting, LockSetting, ResetSetting, SettingsScope, change_setting, lock_setting,
    reset_setting,
};

/// The events one call added.
pub(crate) type Added = Result<usize, String>;

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Runs the call that is under test and says how many events it appended.
async fn measured<T, E: std::fmt::Display>(
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

pub(crate) async fn sign_in_scenario(world: &World) -> Added {
    world.person("bob@example.test").await.map_err(text)?;
    measured(
        &world.store,
        sign_in(
            &world.ctx(),
            SignIn {
                email: "bob@example.test".to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        ),
    )
    .await
}

/// Ada with a code app: set up and confirmed with the first code.
async fn ada_with_a_code_app(world: &World) -> Result<(), String> {
    let token = world.admin.session_token.clone();
    start_code_app(
        &world.ctx(),
        StartCodeApp {
            session_token: token.clone(),
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    confirm_code_app(
        &world.ctx(),
        ConfirmCodeApp {
            session_token: token,
            code: code_now(world).await?,
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    Ok(())
}

/// The code of the code app of Ada at this moment.
async fn code_now(world: &World) -> Result<String, String> {
    let secret = world
        .store
        .totp_secret(world.admin.person.id)
        .await
        .map_err(text)?
        .ok_or("no secret")?;
    TotpSecret::from_bytes(&secret)
        .code_at(crate::context::Clock::now(&*world.clock))
        .map_err(text)
}

/// Ada signs in with her password and still needs the second step.
async fn ada_waits_for_the_second_step(world: &World) -> Result<Started, String> {
    let started = sign_in(
        &world.ctx(),
        SignIn {
            email: "ada@example.test".to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await
    .map_err(text)?;
    if started.second_step_needed.is_none() {
        return Err("a second step was expected".into());
    }
    Ok(started)
}

pub(crate) async fn second_step_scenario(world: &World) -> Added {
    ada_with_a_code_app(world).await?;
    let pending = ada_waits_for_the_second_step(world).await?;
    // A code is good once: the next one is for the next 30 seconds.
    world.clock.advance(30);
    measured(
        &world.store,
        second_step(
            &world.ctx(),
            SecondStep {
                session_token: pending.session_token,
                code: code_now(world).await?,
            },
        ),
    )
    .await
}

/// A device with a passkey of Ada.
async fn ada_with_a_passkey(world: &World) -> Result<SoftPasskey, String> {
    let mut device = SoftPasskey::new(true);
    let challenge = passkey_registration_options(
        &world.ctx(),
        PasskeyRegistrationQuery {
            session_token: world.admin.session_token.clone(),
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    let answer = device
        .do_registration(
            origin()?,
            serde_json::from_value(challenge.options).map_err(text)?,
        )
        .map_err(text)?;
    add_passkey(
        &world.ctx(),
        AddPasskey {
            session_token: world.admin.session_token.clone(),
            challenge_id: challenge.challenge_id,
            credential: serde_json::to_value(answer).map_err(text)?,
            label: "Phone".to_owned(),
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    Ok(device)
}

pub(crate) async fn add_passkey_scenario(world: &World) -> Added {
    let mut device = SoftPasskey::new(true);
    let challenge = passkey_registration_options(
        &world.ctx(),
        PasskeyRegistrationQuery {
            session_token: world.admin.session_token.clone(),
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    let answer = device
        .do_registration(
            origin()?,
            serde_json::from_value(challenge.options).map_err(text)?,
        )
        .map_err(text)?;
    measured(
        &world.store,
        add_passkey(
            &world.ctx(),
            AddPasskey {
                session_token: world.admin.session_token.clone(),
                challenge_id: challenge.challenge_id,
                credential: serde_json::to_value(answer).map_err(text)?,
                label: "Phone".to_owned(),
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn remove_passkey_scenario(world: &World) -> Added {
    ada_with_a_passkey(world).await?;
    let passkey = world
        .store
        .passkeys_of(world.admin.person.id)
        .await
        .map_err(text)?
        .first()
        .map(|passkey| passkey.id)
        .ok_or("no passkey")?;
    measured(
        &world.store,
        remove_passkey(
            &world.ctx(),
            RemovePasskey {
                session_token: world.admin.session_token.clone(),
                passkey_id: passkey,
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn passkey_second_step_scenario(world: &World) -> Added {
    let mut device = ada_with_a_passkey(world).await?;
    let pending = ada_waits_for_the_second_step(world).await?;
    let challenge = passkey_second_step_options(
        &world.ctx(),
        PasskeySecondStepQuery {
            session_token: pending.session_token.clone(),
        },
    )
    .await
    .map_err(text)?;
    let answer = device
        .do_authentication(
            origin()?,
            serde_json::from_value(challenge.options).map_err(text)?,
        )
        .map_err(text)?;
    measured(
        &world.store,
        passkey_second_step(
            &world.ctx(),
            PasskeySecondStep {
                session_token: pending.session_token,
                challenge_id: challenge.challenge_id,
                credential: serde_json::to_value(answer).map_err(text)?,
            },
        ),
    )
    .await
}

pub(crate) async fn passkey_sign_in_scenario(world: &World) -> Added {
    let mut device = ada_with_a_passkey(world).await?;
    let credential = world
        .store
        .passkeys_of(world.admin.person.id)
        .await
        .map_err(text)?
        .first()
        .map(|passkey| passkey.credential_id.clone())
        .ok_or("no passkey")?;
    let challenge = passkey_sign_in_options(&world.ctx(), PasskeySignInQuery)
        .await
        .map_err(text)?;
    // What a phone does that holds a discoverable passkey: it names the passkey in the request and
    // puts the person into the answer.
    let mut options: Value = challenge.options;
    options["publicKey"]["allowCredentials"] = serde_json::json!([{
        "type": "public-key",
        "id": Base64UrlUnpadded::encode_string(&credential),
    }]);
    let answer = device
        .do_authentication(origin()?, serde_json::from_value(options).map_err(text)?)
        .map_err(text)?;
    let mut answer = serde_json::to_value(answer).map_err(text)?;
    answer["response"]["userHandle"] = Value::String(Base64UrlUnpadded::encode_string(
        &world.admin.person.id.to_bytes(),
    ));
    measured(
        &world.store,
        passkey_sign_in(
            &world.ctx(),
            PasskeySignIn {
                challenge_id: challenge.challenge_id,
                credential: answer,
                previous_session: None,
            },
        ),
    )
    .await
}

pub(crate) async fn sign_out_scenario(world: &World) -> Added {
    measured(
        &world.store,
        sign_out(
            &world.ctx(),
            SignOut {
                session_token: world.admin.session_token.clone(),
            },
        ),
    )
    .await
}

pub(crate) async fn sign_out_everywhere_scenario(world: &World) -> Added {
    measured(
        &world.store,
        sign_out_everywhere(
            &world.ctx(),
            SignOutEverywhere {
                session_token: world.admin.session_token.clone(),
            },
        ),
    )
    .await
}

pub(crate) async fn end_session_scenario(world: &World) -> Added {
    // A second place where Ada is signed in, ended from the first.
    let second = sign_in(
        &world.ctx(),
        SignIn {
            email: "ada@example.test".to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await
    .map_err(text)?;
    let listed = sessions(
        &world.ctx(),
        crate::auth::SessionsQuery {
            session_token: second.session_token,
        },
    )
    .await
    .map_err(text)?;
    let other = listed
        .sessions
        .iter()
        .find(|session| !session.current)
        .map(|session| session.id)
        .ok_or("no other session")?;
    measured(
        &world.store,
        end_session(
            &world.ctx(),
            EndSession {
                session_token: world.admin.session_token.clone(),
                session_id: other,
            },
        ),
    )
    .await
}

pub(crate) async fn change_password_scenario(world: &World) -> Added {
    measured(
        &world.store,
        change_password(
            &world.ctx(),
            ChangePassword {
                session_token: world.admin.session_token.clone(),
                current_password: PASSWORD.to_owned(),
                new_password: OTHER_PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn start_code_app_scenario(world: &World) -> Added {
    measured(
        &world.store,
        start_code_app(
            &world.ctx(),
            StartCodeApp {
                session_token: world.admin.session_token.clone(),
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn confirm_code_app_scenario(world: &World) -> Added {
    start_code_app(
        &world.ctx(),
        StartCodeApp {
            session_token: world.admin.session_token.clone(),
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        confirm_code_app(
            &world.ctx(),
            ConfirmCodeApp {
                session_token: world.admin.session_token.clone(),
                code: code_now(world).await?,
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn remove_code_app_scenario(world: &World) -> Added {
    ada_with_a_code_app(world).await?;
    measured(
        &world.store,
        remove_code_app(
            &world.ctx(),
            RemoveCodeApp {
                session_token: world.admin.session_token.clone(),
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn renew_backup_codes_scenario(world: &World) -> Added {
    ada_with_a_code_app(world).await?;
    measured(
        &world.store,
        renew_backup_codes(
            &world.ctx(),
            RenewBackupCodes {
                session_token: world.admin.session_token.clone(),
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

// ---- Invitations, signing up and setup ----

/// An invitation of the administrator: its ID and the secret in its link.
async fn invitation(
    world: &World,
    email: &str,
) -> Result<(bb_core::id::InvitationId, String), String> {
    let created = create_invitation(
        &world.ctx(),
        CreateInvitation {
            session_token: world.admin.session_token.clone(),
            email: email.to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value();
    let secret = created
        .link_path
        .split_once('#')
        .map(|(_, secret)| secret.to_owned())
        .ok_or("no secret in the link")?;
    Ok((created.invitation.id, secret))
}

pub(crate) async fn create_invitation_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_invitation(
            &world.ctx(),
            CreateInvitation {
                session_token: world.admin.session_token.clone(),
                email: "bob@example.test".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn revoke_invitation_scenario(world: &World) -> Added {
    let (id, _) = invitation(world, "bob@example.test").await?;
    measured(
        &world.store,
        revoke_invitation(
            &world.ctx(),
            RevokeInvitation {
                session_token: world.admin.session_token.clone(),
                invitation_id: id,
            },
        ),
    )
    .await
}

pub(crate) async fn accept_invitation_scenario(world: &World) -> Added {
    let (_, secret) = invitation(world, "bob@example.test").await?;
    measured(
        &world.store,
        accept_invitation(
            &world.ctx(),
            crate::auth::AcceptInvitation {
                secret,
                display_name: "Bob Builder".to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        ),
    )
    .await
}

pub(crate) async fn change_sign_up_policy_scenario(world: &World) -> Added {
    measured(
        &world.store,
        change_sign_up_policy(
            &world.ctx(),
            ChangeSignUpPolicy {
                session_token: world.admin.session_token.clone(),
                policy: SignUpPolicy {
                    enabled: true,
                    domains: vec!["example.test".to_owned()],
                },
            },
        ),
    )
    .await
}

pub(crate) async fn sign_up_scenario(world: &World) -> Added {
    change_sign_up_policy(
        &world.ctx(),
        ChangeSignUpPolicy {
            session_token: world.admin.session_token.clone(),
            policy: SignUpPolicy {
                enabled: true,
                domains: vec!["example.test".to_owned()],
            },
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        sign_up(
            &world.ctx(),
            SignUp {
                email: "eve@example.test".to_owned(),
                display_name: "Eve".to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        ),
    )
    .await
}

pub(crate) async fn issue_setup_code_scenario(_world: &World) -> Added {
    // A server that nobody set up: the starting roles are there, nobody can manage roles, and the
    // start makes the setup code.
    let bare = World::bare()?;
    make_starting_roles(&bare.ctx(), MakeStartingRoles)
        .await
        .map_err(text)?;
    measured(&bare.store, issue_setup_code(&bare.ctx(), IssueSetupCode)).await
}

pub(crate) async fn complete_setup_scenario(_world: &World) -> Added {
    let bare = World::bare()?;
    // The start of the program makes the starting roles before the person comes to set up.
    make_starting_roles(&bare.ctx(), MakeStartingRoles)
        .await
        .map_err(text)?;
    let code = issue_setup_code(&bare.ctx(), IssueSetupCode)
        .await
        .map_err(text)?
        .ok_or("no setup code")?;
    measured(
        &bare.store,
        complete_setup(
            &bare.ctx(),
            CompleteSetup {
                code: code.as_str().to_owned(),
                display_name: "Ada Admin".to_owned(),
                email: "ada@example.test".to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        ),
    )
    .await
}

pub(crate) async fn make_starting_roles_scenario(_world: &World) -> Added {
    let bare = World::bare()?;
    measured(
        &bare.store,
        make_starting_roles(&bare.ctx(), MakeStartingRoles),
    )
    .await
}

// ---- Groups, roles, rows, levels and projects ----

fn token(world: &World) -> String {
    world.admin.session_token.clone()
}

async fn a_group(world: &World, name: &str) -> Result<GroupId, String> {
    Ok(make_group(
        &world.ctx(),
        MakeGroup {
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
    Ok(make_role(
        &world.ctx(),
        MakeRole {
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

async fn a_project(world: &World, short_name: &str) -> Result<ProjectId, String> {
    Ok(make_project(
        &world.ctx(),
        MakeProject {
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

pub(crate) async fn make_group_scenario(world: &World) -> Added {
    measured(
        &world.store,
        make_group(
            &world.ctx(),
            MakeGroup {
                session_token: token(world),
                name: "Team".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn rename_group_scenario(world: &World) -> Added {
    let group = a_group(world, "Team").await?;
    measured(
        &world.store,
        rename_group(
            &world.ctx(),
            RenameGroup {
                session_token: token(world),
                group,
                name: "Crew".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn delete_group_scenario(world: &World) -> Added {
    let group = a_group(world, "Team").await?;
    measured(
        &world.store,
        delete_group(
            &world.ctx(),
            DeleteGroup {
                session_token: token(world),
                group,
            },
        ),
    )
    .await
}

pub(crate) async fn add_group_member_scenario(world: &World) -> Added {
    let (group, person) = (a_group(world, "Team").await?, a_person(world).await?);
    measured(
        &world.store,
        add_group_member(
            &world.ctx(),
            AddGroupMember {
                session_token: token(world),
                group,
                person,
            },
        ),
    )
    .await
}

pub(crate) async fn remove_group_member_scenario(world: &World) -> Added {
    let (group, person) = (a_group(world, "Team").await?, a_person(world).await?);
    add_group_member(
        &world.ctx(),
        AddGroupMember {
            session_token: token(world),
            group,
            person,
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        remove_group_member(
            &world.ctx(),
            RemoveGroupMember {
                session_token: token(world),
                group,
                person,
            },
        ),
    )
    .await
}

pub(crate) async fn make_role_scenario(world: &World) -> Added {
    measured(
        &world.store,
        make_role(
            &world.ctx(),
            MakeRole {
                session_token: token(world),
                name: "Own".to_owned(),
                permissions: vec![Permission::SeeProjects],
                two_step: false,
            },
        ),
    )
    .await
}

pub(crate) async fn change_role_scenario(world: &World) -> Added {
    let role = a_role(world, "Own").await?;
    measured(
        &world.store,
        change_role(
            &world.ctx(),
            ChangeRole {
                session_token: token(world),
                role,
                name: Some("Own role".to_owned()),
                permissions: None,
                two_step: Some(true),
            },
        ),
    )
    .await
}

pub(crate) async fn copy_role_scenario(world: &World) -> Added {
    let role = a_role(world, "Own").await?;
    measured(
        &world.store,
        copy_role(
            &world.ctx(),
            CopyRole {
                session_token: token(world),
                role,
                name: "Copy of own".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn delete_role_scenario(world: &World) -> Added {
    let role = a_role(world, "Own").await?;
    measured(
        &world.store,
        delete_role(
            &world.ctx(),
            DeleteRole {
                session_token: token(world),
                role,
                replacement: None,
            },
        ),
    )
    .await
}

pub(crate) async fn reset_role_scenario(world: &World) -> Added {
    let viewer = world
        .store
        .read(|state| {
            state
                .roles
                .iter()
                .find(|record| record.role.name == "Viewer")
                .map(|record| record.role.id)
        })
        .ok_or("no Viewer role")?;
    measured(
        &world.store,
        reset_role(
            &world.ctx(),
            ResetRole {
                session_token: token(world),
                role: viewer,
            },
        ),
    )
    .await
}

pub(crate) async fn assign_role_scenario(world: &World) -> Added {
    let (role, person, project) = (
        a_role(world, "Own").await?,
        a_person(world).await?,
        a_project(world, "ACME").await?,
    );
    measured(
        &world.store,
        assign_role(
            &world.ctx(),
            AssignRole {
                session_token: token(world),
                subject: Subject::Person(person),
                given: Given::Role(role),
                project: Some(project),
                applies_to: AppliesTo::ProjectAndSubProjects,
            },
        ),
    )
    .await
}

pub(crate) async fn remove_assignment_scenario(world: &World) -> Added {
    let (person, project) = (a_person(world).await?, a_project(world, "ACME").await?);
    let row = assign_role(
        &world.ctx(),
        AssignRole {
            session_token: token(world),
            subject: Subject::Person(person),
            given: Given::NoAccess,
            project: Some(project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    measured(
        &world.store,
        remove_assignment(
            &world.ctx(),
            RemoveAssignment {
                session_token: token(world),
                assignment: row.id,
            },
        ),
    )
    .await
}

pub(crate) async fn set_field_level_scenario(world: &World) -> Added {
    let role = a_role(world, "Own").await?;
    measured(
        &world.store,
        set_level(
            &world.ctx(),
            SetFieldLevel {
                session_token: token(world),
                role,
                project: None,
                field: Field::Title,
                level: Some(bb_core::access::FieldLevel::CanSee),
            },
        ),
    )
    .await
}

pub(crate) async fn set_people_level_scenario(world: &World) -> Added {
    let role = a_role(world, "Own").await?;
    measured(
        &world.store,
        set_level(
            &world.ctx(),
            SetPeopleLevel {
                session_token: token(world),
                role,
                project: None,
                kind: PeopleKind::Followers,
                level: Some(bb_core::access::PeopleLevel::Names),
            },
        ),
    )
    .await
}

pub(crate) async fn make_project_scenario(world: &World) -> Added {
    measured(
        &world.store,
        make_project(
            &world.ctx(),
            MakeProject {
                session_token: token(world),
                parent: None,
                short_name: "ACME".to_owned(),
                name: "Acme".to_owned(),
            },
        ),
    )
    .await
}

/// A value of the person's own, for the scenarios that take one away.
async fn a_person_value(world: &World) -> Result<(), String> {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            session_token: token(world),
            scope: SettingsScope::Person,
            key: "appearance.mode".to_owned(),
            value: SettingValue::Text("dark".to_owned()),
        },
    )
    .await
    .map(|_| ())
    .map_err(text)
}

pub(crate) async fn change_setting_scenario(world: &World) -> Added {
    measured(
        &world.store,
        change_setting(
            &world.ctx(),
            ChangeSetting {
                session_token: token(world),
                scope: SettingsScope::Organization,
                key: "security.session-idle-minutes".to_owned(),
                value: SettingValue::Integer(20),
            },
        ),
    )
    .await
}

pub(crate) async fn reset_setting_scenario(world: &World) -> Added {
    a_person_value(world).await?;
    measured(
        &world.store,
        reset_setting(
            &world.ctx(),
            ResetSetting {
                session_token: token(world),
                scope: SettingsScope::Person,
                key: "appearance.mode".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn lock_setting_scenario(world: &World) -> Added {
    measured(
        &world.store,
        lock_setting(
            &world.ctx(),
            LockSetting {
                session_token: token(world),
                key: "appearance.mode".to_owned(),
                locked: true,
            },
        ),
    )
    .await
}

pub(crate) async fn change_project_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    measured(
        &world.store,
        change_project(
            &world.ctx(),
            ChangeProject {
                session_token: token(world),
                project,
                version: 1,
                name: Some("Acme Corporation".to_owned()),
                short_name: Some("ACMECORP".to_owned()),
            },
        ),
    )
    .await
}

pub(crate) async fn move_project_scenario(world: &World) -> Added {
    let acme = a_project(world, "ACME").await?;
    let web = a_project(world, "WEB").await?;
    let preview = preview_move(
        &world.ctx(),
        MovePreviewQuery {
            session_token: token(world),
            project: web,
            to: Some(acme),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .preview;
    measured(
        &world.store,
        move_project(
            &world.ctx(),
            MoveProject {
                session_token: token(world),
                project: web,
                to: Some(acme),
                preview,
            },
        ),
    )
    .await
}

// ---- Access keys ----

/// Ada makes a key that may only see.
fn see_only_key(world: &World) -> MakeAccessKey {
    MakeAccessKey {
        session_token: token(world),
        password: PASSWORD.to_owned(),
        name: "Nightly report".to_owned(),
        choice: KeyChoice::SeeOnly,
        permissions: Vec::new(),
        projects: Vec::new(),
        expires_at: None,
    }
}

pub(crate) async fn make_access_key_scenario(world: &World) -> Added {
    measured(
        &world.store,
        make_access_key(&world.ctx(), see_only_key(world)),
    )
    .await
}

pub(crate) async fn delete_access_key_scenario(world: &World) -> Added {
    let made = make_access_key(&world.ctx(), see_only_key(world))
        .await
        .map_err(text)?
        .into_value();
    measured(
        &world.store,
        delete_access_key(
            &world.ctx(),
            DeleteAccessKey {
                session_token: token(world),
                id: made.key.id,
            },
        ),
    )
    .await
}
