//! Editing against the in-memory store (ISS-17, ADR-0019): the parts that need no database to
//! be seen. The flows of locks, drafts and saving run on both databases in `bb-server`
//! (`sign_in/editing.rs`).

use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::event::{Actor, Channel, DomainEvent, EntityRef, FieldChange, RecordKind};
use bb_core::fields::Field;
use bb_core::id::{EntityId, EventId, UserId};
use bb_policy::{PersonRef, Place};
use std::sync::Arc;

use bb_core::version::counted_up;
use serde_json::json;

use crate::editing::testing::TestNotes;

use super::world::{Outcome, World};
use crate::context::Clock as _;
use crate::editing::Stale;
use crate::projects::{MakeProject, make_project};
use crate::roles::{
    AssignRole, Given, MakeRole, SetFieldLevel, SetPeopleLevel, Subject, assign_role, make_role,
    set_level,
};
use bb_core::access::PeopleLevel;
use bb_core::people_kinds::PeopleKind;

/// A record that was changed by Ada: its status and its cost center.
fn changed_by_ada(world: &World, notes: &TestNotes) -> Result<EntityRef, String> {
    let record = notes.add(EntityId::from(UserId::from_random([9; 16])));
    let event = DomainEvent {
        id: EventId::from_parts(world.clock.now(), [1; 10]).map_err(|error| error.to_string())?,
        time: world.clock.now(),
        actor: Actor::Person(world.admin.person.id),
        channel: Channel::Ui,
        entity: record.clone(),
        changes: vec![
            FieldChange {
                field: "status".to_owned(),
                before: Some(json!("Open")),
                after: Some(json!("Done")),
            },
            FieldChange {
                field: "cost-center".to_owned(),
                before: None,
                after: Some(json!("4711")),
            },
            counted_up(1),
        ],
        reason: None,
    };
    world.store.lock().events.push(event);
    Ok(record)
}

#[tokio::test]
async fn iss_17_the_conflict_leaves_out_what_the_reader_may_not_see() -> Outcome {
    let notes = TestNotes::new()?;
    let world = World::new().await?.with_records(Arc::new(notes.clone()));
    let record = changed_by_ada(&world, &notes)?;
    // Bob may see the status, not the cost center, and sees names in *Last changed by*.
    let bob = world.person("bob@example.test").await?;
    let role = make_role(
        &world.ctx(),
        MakeRole {
            session_token: world.admin.session_token.clone(),
            name: "Reader".to_owned(),
            permissions: vec![bb_policy::Permission::SeeProjects],
            two_step: false,
        },
    )
    .await?
    .into_value()
    .id;
    for field in [Field::Status, Field::CostCenter] {
        set_level(
            &world.ctx(),
            SetFieldLevel {
                session_token: world.admin.session_token.clone(),
                role,
                project: None,
                issue_type: None,
                field: field.into(),
                level: Some(if field == Field::Status {
                    FieldLevel::CanSee
                } else {
                    FieldLevel::Hidden
                }),
            },
        )
        .await?;
    }
    set_level(
        &world.ctx(),
        SetPeopleLevel {
            session_token: world.admin.session_token.clone(),
            role,
            project: None,
            kind: PeopleKind::LastChangedBy,
            level: Some(PeopleLevel::Names),
        },
    )
    .await?;
    let project = make_project(
        &world.ctx(),
        MakeProject {
            session_token: world.admin.session_token.clone(),
            parent: None,
            short_name: "ACME".to_owned(),
            name: "Acme".to_owned(),
        },
    )
    .await?
    .into_value()
    .id;
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: world.admin.session_token.clone(),
            subject: Subject::Person(bob.person.id),
            given: Given::Role(role),
            project: Some(project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await?;
    let stale = Stale {
        record,
        place: Place::Project(project),
        viewer: bob.person.id,
    };
    let crate::auth::AppError::Conflict(conflict) = world.ctx().conflict_of(&stale).await else {
        return Err("no structured answer".into());
    };
    assert_eq!(conflict.fields, ["status"]);
    assert_eq!(conflict.current.len(), 1);
    assert_eq!(conflict.current["status"], json!("Done"));
    assert!(matches!(conflict.changed_by, Some(PersonRef::Name { .. })));
    Ok(())
}

#[tokio::test]
async fn iss_17_a_kind_of_record_decides_which_changes_a_conflict_shows_and_people_become_person_refs()
-> Outcome {
    let notes = TestNotes::new()?;
    let world = World::new().await?.with_records(Arc::new(notes.clone()));
    let note = notes.add(EntityId::from(UserId::from_random([5; 16])));
    let bob = world.person("bob@example.test").await?;
    let event = DomainEvent {
        id: EventId::from_parts(world.clock.now(), [2; 10]).map_err(|error| error.to_string())?,
        time: world.clock.now(),
        actor: Actor::Person(world.admin.person.id),
        channel: Channel::Ui,
        entity: note.clone(),
        changes: vec![
            FieldChange {
                field: "assigned-to".to_owned(),
                before: None,
                after: Some(json!(bob.person.id.to_string())),
            },
            FieldChange {
                field: "title".to_owned(),
                before: None,
                after: Some(json!("A title")),
            },
            // A field nobody said how to show stays out.
            FieldChange {
                field: "internal_note".to_owned(),
                before: None,
                after: Some(json!("secret")),
            },
            counted_up(1),
        ],
        reason: None,
    };
    world.store.lock().events.push(event);
    let stale = Stale {
        record: note,
        place: Place::Organization,
        viewer: world.admin.person.id,
    };
    let crate::auth::AppError::Conflict(conflict) = world.ctx().conflict_of(&stale).await else {
        return Err("no structured answer".into());
    };
    assert_eq!(conflict.fields, ["assigned-to", "title"]);
    assert_eq!(conflict.current["title"], json!("A title"));
    assert_eq!(conflict.current["assigned-to"]["shown"], json!("name"));
    let text = serde_json::to_string(&conflict)?;
    assert!(conflict.current["assigned-to"].is_object(), "{text}");
    assert!(!text.contains("secret"), "{text}");
    Ok(())
}

#[tokio::test]
async fn iss_17_a_conflict_finds_the_version_event_however_many_events_came_after() -> Outcome {
    let notes = TestNotes::new()?;
    let world = World::new().await?.with_records(Arc::new(notes.clone()));
    let note = notes.add(EntityId::from(UserId::from_random([6; 16])));
    let event = |at: u8, changes: Vec<FieldChange>| -> Result<DomainEvent, String> {
        Ok(DomainEvent {
            id: EventId::from_parts(world.clock.now(), [at; 10])
                .map_err(|error| error.to_string())?,
            time: world.clock.now(),
            actor: Actor::Person(world.admin.person.id),
            channel: Channel::Ui,
            entity: note.clone(),
            changes,
            reason: None,
        })
    };
    world.store.lock().events.push(event(
        1,
        vec![
            FieldChange {
                field: "title".to_owned(),
                before: None,
                after: Some(json!("Renamed")),
            },
            counted_up(1),
        ],
    )?);
    // Far more later events than any window, none of which counted the version up.
    for at in 2..=120_u8 {
        world.store.lock().events.push(event(
            at,
            vec![FieldChange {
                field: "edit_lock".to_owned(),
                before: None,
                after: Some(json!({ "asked_to_finish": "description" })),
            }],
        )?);
    }
    let stale = Stale {
        record: note,
        place: Place::Organization,
        viewer: world.admin.person.id,
    };
    let crate::auth::AppError::Conflict(conflict) = world.ctx().conflict_of(&stale).await else {
        return Err("no structured answer".into());
    };
    assert_eq!(conflict.fields, ["title"]);
    assert_eq!(conflict.current["title"], json!("Renamed"));
    Ok(())
}

#[tokio::test]
async fn iss_17_a_conflict_about_a_record_without_events_still_answers() -> Outcome {
    let world = World::new().await?;
    let stale = Stale {
        record: EntityRef {
            kind: RecordKind::Group.into(),
            id: EntityId::from(UserId::from_random([8; 16])),
        },
        place: Place::Organization,
        viewer: world.admin.person.id,
    };
    // Nothing known of who or what, but the answer is still the conflict, not another error.
    let crate::auth::AppError::Conflict(conflict) = world.ctx().conflict_of(&stale).await else {
        return Err("no structured answer".into());
    };
    assert!(conflict.changed_by.is_none() && conflict.fields.is_empty());
    Ok(())
}

#[tokio::test]
async fn iss_17_edit_anyway_tries_again_when_other_writers_get_in_the_way() -> Outcome {
    // On PostgreSQL ten people taking over one lock make a take-over find the row changed under
    // it; the person must not see a failure for that, the change is tried again.
    let world = World::new().await?;
    world
        .store
        .lock()
        .state
        .contended_replaces
        .store(2, std::sync::atomic::Ordering::SeqCst);
    super::scenarios::edit_anyway_scenario(&world).await?;
    assert_eq!(
        world
            .store
            .lock()
            .state
            .contended_replaces
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    Ok(())
}
