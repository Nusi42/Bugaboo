//! When something fails or changes, against the in-memory store: a security log that cannot be
//! written (M1-F52), a store that breaks or stays busy while a change is committed, and the
//! evaluations that live for one request and no longer (ADR-0025, decision 3).

use std::io::Write;
use std::sync::{Arc, Mutex};

use bb_core::ErrorCode;
use bb_core::id::{GroupId, ProjectId, RoleId};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_policy::{PeopleContext, Permission, Place};
use jiff::Timestamp;
use tracing_subscriber::fmt::MakeWriter;

use super::scenarios::a_project;
use super::world::{Outcome, PASSWORD, World};
use crate::access::{PolicyStore as _, ProjectLabel};
use crate::auth::{AppError, CreateInvitation, SignIn, create_invitation, sign_in};
use crate::projects::{
    CreateProject, MovePreviewQuery, MoveProject, create_project, move_project, preview_move,
};
use crate::roles::{
    AssignRole, CreateRole, Given, RemoveAssignment, SetPeopleLevel, Subject, assign_role,
    create_role, remove_assignment, set_level,
};
use bb_core::access::{AppliesTo, PeopleLevel};

/// Collects what the server logs, so a test can read it.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Self;

    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}

impl Captured {
    fn text(&self) -> String {
        String::from_utf8_lossy(
            &self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
        .into_owned()
    }
}

#[tokio::test]
async fn sec_15_a_log_entry_that_cannot_be_written_is_counted_and_logged_without_secrets() -> Outcome
{
    let captured = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(captured.clone())
        .with_ansi(false)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let world = World::new().await?;
    let events = world.store.event_count();
    let written = world.services.log_writes();
    assert_eq!(written.failed(), 0);
    world.store.fail_security_log(true);

    // The change still happens: a log that is full must not lock everybody out (M1-F40).
    let created = create_invitation(
        &world.ctx(),
        CreateInvitation {
            session_token: world.admin.session_token.clone(),
            email: "secret.person@example.test".to_owned(),
        },
    )
    .await?;
    assert_eq!(world.store.event_count(), events + 1);
    assert_eq!(
        written.failed(),
        1,
        "the server counts the entry it could not write"
    );

    // Signing in works too, and every failed entry is counted.
    sign_in(
        &world.ctx(),
        SignIn {
            email: "ada@example.test".to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await?;
    assert_eq!(written.failed(), 2);

    // The server log says who and what it was about, by ID, and holds no secret and no address.
    let log = captured.text();
    assert!(
        log.contains("the security log could not be written"),
        "{log}"
    );
    assert!(
        log.contains(SecurityAction::InvitationCreated.code()),
        "{log}"
    );
    assert!(log.contains(&world.admin.person.id.to_string()), "{log}");
    let link = created.into_value().link_path;
    let secret = link
        .split_once('#')
        .map(|(_, secret)| secret)
        .unwrap_or_default();
    for private in [
        secret,
        "secret.person@example.test",
        "ada@example.test",
        PASSWORD,
        "Ada Admin",
        world.admin.session_token.as_str(),
    ] {
        assert!(!log.contains(private), "{private} is in the server log");
    }

    // The log works again: the next change writes its entry.
    world.store.fail_security_log(false);
    let entries = world.store.log_count();
    create_invitation(
        &world.ctx(),
        CreateInvitation {
            session_token: world.admin.session_token.clone(),
            email: "other@example.test".to_owned(),
        },
    )
    .await?;
    assert_eq!(world.store.log_count(), entries + 1);
    assert_eq!(written.failed(), 2);
    Ok(())
}

#[tokio::test]
async fn adr_0007_a_store_that_breaks_when_a_change_is_committed_leaves_nothing() -> Outcome {
    let world = World::new().await?;
    let (events, entries) = (world.store.event_count(), world.store.log_count());
    world.store.fail_changes(true);
    let result = create_invitation(
        &world.ctx(),
        CreateInvitation {
            session_token: world.admin.session_token.clone(),
            email: "bob@example.test".to_owned(),
        },
    )
    .await;
    assert!(matches!(result, Err(AppError::Internal(_))));
    // No event, no entry of the log, no invitation.
    assert_eq!(world.store.event_count(), events);
    assert_eq!(world.store.log_count(), entries);
    assert!(world.store.read(|state| state.invitations.is_empty()));
    Ok(())
}

/// A change that the store reports as staying busy with its upkeep (ADR-0017) is refused in plain
/// words, *not available right now, try again in a moment*, and leaves nothing.
#[tokio::test]
async fn adr_0017_a_change_that_stays_busy_is_refused_as_not_available_and_leaves_nothing()
-> Outcome {
    let world = World::new().await?;
    let events = world.store.event_count();
    world.store.keep_changes_busy(true);
    let result = create_invitation(
        &world.ctx(),
        CreateInvitation {
            session_token: world.admin.session_token.clone(),
            email: "bob@example.test".to_owned(),
        },
    )
    .await;
    assert_eq!(
        result.err(),
        Some(AppError::Refused(ErrorCode::Unavailable))
    );
    assert_eq!(world.store.event_count(), events);
    assert!(world.store.read(|state| state.invitations.is_empty()));
    Ok(())
}

#[tokio::test]
async fn sec_03_evaluations_are_kept_for_one_request_and_a_role_taken_away_counts_from_the_next()
-> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    let token = world.admin.session_token.clone();
    let role = create_role(
        &world.ctx(),
        CreateRole {
            session_token: token.clone(),
            name: "Watcher".to_owned(),
            permissions: vec![Permission::SeeStatus],
            two_step: false,
        },
    )
    .await?
    .into_value();
    let row = assign_role(
        &world.ctx(),
        AssignRole {
            session_token: token.clone(),
            subject: Subject::Person(bob),
            given: Given::Role(role.id),
            project: None,
            applies_to: AppliesTo::Organization,
        },
    )
    .await?
    .into_value();

    // One request asks as often as it likes: the rows are loaded once.
    let request = world.ctx();
    let loads = world.store.policy_loads();
    assert!(
        request
            .evaluate(bob, Place::Organization)
            .await?
            .allows(Permission::SeeStatus)
    );
    assert!(
        request
            .evaluate(bob, Place::Organization)
            .await?
            .allows(Permission::SeeStatus)
    );
    assert_eq!(world.store.policy_loads(), loads + 1);

    // The role is taken away during that request (by another one): the request that asked
    // before still has its answer, which it keeps until it ends...
    remove_assignment(
        &world.ctx(),
        RemoveAssignment {
            session_token: token,
            assignment: row.id,
        },
    )
    .await?;
    assert!(
        request
            .evaluate(bob, Place::Organization)
            .await?
            .allows(Permission::SeeStatus)
    );
    // ...and the next request knows.
    let next = world.ctx();
    assert!(
        !next
            .evaluate(bob, Place::Organization)
            .await?
            .allows(Permission::SeeStatus)
    );
    Ok(())
}

#[tokio::test]
async fn auth_07_a_key_whose_public_prefix_is_taken_is_made_again_once() -> Outcome {
    use bb_core::access_key::KeyChoice;

    use crate::auth::{CreateAccessKey, create_access_key};
    let world = World::new().await?;
    let ask_for = |name: &str| CreateAccessKey {
        session_token: world.admin.session_token.clone(),
        password: PASSWORD.to_owned(),
        name: name.to_owned(),
        choice: KeyChoice::SeeOnly,
        permissions: Vec::new(),
        projects: Vec::new(),
        expires_at: None,
    };
    // One clash: the key is made with a new secret, and nobody sees that it happened.
    super::fake::clash_the_next_key_prefix();
    let made = create_access_key(&world.ctx(), ask_for("After a clash"))
        .await?
        .into_value();
    assert_eq!(made.key.name, "After a clash");
    assert!(made.secret.starts_with(&format!("bb_{}", made.key.prefix)));
    // The store refusing more than once is not a clash of prefixes: the use case gives up.
    super::fake::clash_the_next_key_prefix();
    super::fake::clash_the_next_key_prefix();
    let again = create_access_key(&world.ctx(), ask_for("Again")).await;
    assert!(again.is_ok(), "a second key is stored after one clash");
    Ok(())
}

/// A store that breaks while a request asks who may do what is a failure of the server. Only a
/// project that is not there, or that the person may not know about, is an answer (hidden, no
/// role label, not allowed): the places where a read used to turn any failure into such an answer.
#[tokio::test]
async fn sec_03_a_store_that_breaks_while_asking_who_may_do_what_is_not_taken_for_an_answer()
-> Outcome {
    let world = World::new().await?;
    let token = world.admin.session_token.clone();
    let made = create_project(
        &world.ctx(),
        CreateProject {
            session_token: token.clone(),
            parent: None,
            short_name: "ACME".to_owned(),
            name: "Acme".to_owned(),
        },
    )
    .await?
    .into_value();
    let ctx = world.ctx();
    let actor = ctx.actor(&token).await?;
    // A project that does not exist is hidden, has no role label and may not be made in: facts.
    let nowhere =
        ProjectId::from_parts(Timestamp::UNIX_EPOCH, [0; 10]).map_err(|e| e.to_string())?;
    let ctx = world.ctx();
    assert_eq!(
        ctx.project_label(&actor, nowhere).await?,
        ProjectLabel::Hidden
    );
    assert!(
        !ctx.may_create_projects_beside(&actor, Some(nowhere))
            .await?
    );
    assert!(matches!(
        ctx.project_label(&actor, made.id).await?,
        ProjectLabel::Visible { .. }
    ));

    let other = world.person("bea@example.test").await?.person.id;
    // Ada sees the members of a project by their role only, so the label of a role is looked for.
    let admin_role = ctx
        .store()
        .roles()
        .await
        .map_err(|error| format!("{error:?}"))?
        .into_iter()
        .find(|record| record.role.starting.is_some())
        .map(|record| record.role.id)
        .ok_or("no starting role")?;
    set_level(
        &ctx,
        SetPeopleLevel {
            session_token: token.clone(),
            role: admin_role,
            project: None,
            kind: PeopleKind::ProjectMembers,
            level: Some(PeopleLevel::RoleOnly),
        },
    )
    .await?;
    let role_only = world
        .ctx()
        .evaluation_of(&actor, Place::Organization)
        .await?;
    // The store breaks. A new request cannot find out, and says so.
    world.store.fail_policy_reads(true);
    let ctx = world.ctx();
    assert!(matches!(
        ctx.project_label(&actor, made.id).await,
        Err(AppError::Internal(_))
    ));
    assert!(matches!(
        ctx.may_create_projects_beside(&actor, Some(made.id)).await,
        Err(AppError::Internal(_))
    ));
    // The role label of another person is looked for only for a viewer who sees people by their
    // role (a person is never shown their own role).
    assert!(matches!(
        ctx.person_ref(
            &role_only,
            PeopleKind::ProjectMembers,
            PeopleContext::Ordinary,
            other,
        )
        .await,
        Err(AppError::Internal(_))
    ));
    // Working again, the same questions have their answers.
    world.store.fail_policy_reads(false);
    let ctx = world.ctx();
    assert!(matches!(
        ctx.project_label(&actor, made.id).await?,
        ProjectLabel::Visible { .. }
    ));
    Ok(())
}

/// A row that names a role or a group the store does not have is told as not found, not shown with
/// a blank name.
#[tokio::test]
async fn sec_03_a_role_or_group_that_is_gone_is_not_found_not_a_blank_name() -> Outcome {
    let world = World::new().await?;
    let ctx = world.ctx();
    let role = RoleId::from_parts(Timestamp::UNIX_EPOCH, [0; 10]).map_err(|e| e.to_string())?;
    let group = GroupId::from_parts(Timestamp::UNIX_EPOCH, [0; 10]).map_err(|e| e.to_string())?;
    assert_eq!(
        ctx.role_ref(role).await.err(),
        Some(AppError::Refused(ErrorCode::NotFound))
    );
    assert_eq!(
        ctx.group_view(group).await.err(),
        Some(AppError::Refused(ErrorCode::NotFound))
    );
    Ok(())
}

/// What a request read of the projects (the chain of a project, the project of an address) is kept
/// for the request (M5-F169) and forgotten when the request changes a project, so the chain read
/// after a move is the new one.
#[tokio::test]
async fn prj_03_a_request_that_moves_a_project_reads_its_chain_again() -> Outcome {
    let world = World::new().await?;
    let (acme, beta) = (
        a_project(&world, "ACME").await?,
        a_project(&world, "BETA").await?,
    );
    let ctx = world.ctx();
    let token = world.admin.session_token.clone();
    assert!(!ctx.chain_of(beta).await?.contains(&acme));
    let preview = preview_move(
        &ctx,
        MovePreviewQuery {
            session_token: token.clone(),
            project: beta,
            to: Some(acme),
        },
    )
    .await?
    .into_value();
    move_project(
        &ctx,
        MoveProject {
            session_token: token,
            project: beta,
            to: Some(acme),
            preview: preview.preview,
        },
    )
    .await?;
    assert!(ctx.chain_of(beta).await?.contains(&acme), "the new chain");
    Ok(())
}
