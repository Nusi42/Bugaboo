//! When something fails or changes, against the in-memory store: a security log that cannot be
//! written (M1-F52), a store that breaks while a change is committed, and the evaluations that
//! live for one request and no longer (ADR-0025, decision 3).

use std::io::Write;
use std::sync::{Arc, Mutex};

use bb_core::sign_in::SecurityAction;
use bb_policy::{Permission, Place};
use tracing_subscriber::fmt::MakeWriter;

use super::world::{Outcome, PASSWORD, World};
use crate::auth::{AppError, CreateInvitation, SignIn, create_invitation, sign_in};
use crate::roles::{
    AssignRole, Given, MakeRole, RemoveAssignment, Subject, assign_role, make_role,
    remove_assignment,
};
use bb_core::access::AppliesTo;

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

#[tokio::test]
async fn sec_03_evaluations_are_kept_for_one_request_and_a_role_taken_away_counts_from_the_next()
-> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    let token = world.admin.session_token.clone();
    let role = make_role(
        &world.ctx(),
        MakeRole {
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
