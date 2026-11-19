//! *Remove a person* against the in-memory store (PRIV-04, ADR-0044, decisions 6 to 8): only a
//! switched-off person is removed, and only once; the person becomes a tombstone with a label and
//! no name or address; the changes are scrubbed of their records and values; the comments go only
//! when asked; and the line of the replay list waits after the change. A replay needs no
//! switch-off and leaves a removed person as they are.

use bb_core::ErrorCode;
use bb_core::backup::replay::{self, ReplayKind};
use bb_core::comment::Comment;
use bb_core::event::RecordKind;
use bb_core::id::{CommentId, IdSource, IssueId, SequentialIds, UserId};
use bb_core::person::Standing;
use bb_core::settings::privacy::RemovedTexts;

use super::scenarios::{Added, measured, text, token};
use super::switch_off::switched_off;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::people::{SwitchOn, switch_on};
use crate::ports::ChangeStore as _;
use crate::privacy::removal::{
    Removal, RemovalAsked, RemovePerson, Removed, remove_in, remove_person,
};

/// The administrator removes `person`, with `comments` or the setting's choice.
async fn removed(
    world: &World,
    person: UserId,
    comments: Option<RemovedTexts>,
) -> Result<Removed, AppError> {
    remove_person(
        &world.ctx(),
        RemovePerson {
            session_token: token(world),
            person,
            comments,
        },
    )
    .await
}

pub(crate) async fn remove_person_scenario(world: &World) -> Added {
    let bob = world.person("bob@example.test").await.map_err(text)?;
    switched_off(world, bob.person.id).await.map_err(text)?;
    measured(&world.store, removed(world, bob.person.id, None)).await
}

/// A comment by `author` on `issue`.
fn comment(ids: &mut impl IdSource, issue: IssueId, author: UserId) -> Comment {
    Comment {
        id: CommentId::generate(ids).unwrap_or_else(|_| unreachable!()),
        issue,
        author,
        text: "a comment".to_owned(),
        created_at: jiff::Timestamp::UNIX_EPOCH,
        edited_at: None,
        deleted_at: None,
        version: 1,
    }
}

/// Bob and Ada each wrote a comment on one issue.
fn two_comments(world: &World, ids: &mut SequentialIds, bob: UserId) -> Result<(), String> {
    let issue = IssueId::generate(ids).map_err(text)?;
    let ada = world.admin.person.id;
    let mut shared = world.store.lock();
    shared.state.comments.push(comment(ids, issue, bob));
    shared.state.comments.push(comment(ids, issue, ada));
    Ok(())
}

fn comments_by(world: &World, author: UserId) -> usize {
    world
        .store
        .read(|state| state.comments.iter().filter(|c| c.author == author).count())
}

#[tokio::test]
async fn priv_04_only_a_switched_off_person_is_removed_and_only_once() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    let active = removed(&world, bob, None).await;
    assert_eq!(
        active.err().and_then(|error| error.code()),
        Some(ErrorCode::PersonNotSwitchedOff)
    );
    switched_off(&world, bob).await?;
    removed(&world, bob, None).await?;
    // No undo: a second removal finds nobody, and *Switch on* refuses a removed person.
    let again = removed(&world, bob, None).await;
    assert_eq!(
        again.err().and_then(|error| error.code()),
        Some(ErrorCode::NotFound)
    );
    let on = switch_on(
        &world.ctx(),
        SwitchOn {
            session_token: token(&world),
            person: bob,
        },
    )
    .await;
    assert_eq!(
        on.err().and_then(|error| error.code()),
        Some(ErrorCode::VersionChanged)
    );
    Ok(())
}

#[tokio::test]
async fn priv_04_a_removed_person_is_a_tombstone_with_no_name_or_address() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    switched_off(&world, bob).await?;
    removed(&world, bob, None).await?;
    let (person, standing, scrubs) = world.store.read(|state| {
        (
            state.people.iter().find(|(p, _)| p.id == bob).cloned(),
            state.standings.iter().find(|s| s.person == bob).cloned(),
            state.scrubs.clone(),
        )
    });
    let (person, _) = person.ok_or("Bob's record stays")?;
    let standing = standing.ok_or("Bob has a standing")?;
    assert_eq!(standing.standing, Standing::Removed);
    let label = standing.label.ok_or("a removed person has a label")?;
    assert!(person.display_name.is_empty());
    assert_eq!(person.email, bb_core::person::cleared_address(&label));
    // No work days: no legal hold.
    assert_eq!(standing.hold_name, None);
    // The changes are scrubbed of the person's records, their settings, name and address.
    let first = &scrubs.first().ok_or("the changes are scrubbed")?.1;
    assert!(first.kinds.contains(&RecordKind::Person));
    assert!(first.kinds.contains(&RecordKind::AccessKey));
    assert!(!first.kinds.contains(&RecordKind::SavedSearch));
    assert!(!first.kinds.contains(&RecordKind::WorkDay));
    assert!(!first.records.is_empty());
    assert!(first.names.contains(&"Bob Builder".to_owned()));
    assert!(first.addresses.contains(&"bob@example.test".to_owned()));
    Ok(())
}

#[tokio::test]
async fn priv_04_the_replay_line_waits_with_the_comments_choice() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    switched_off(&world, bob).await?;
    removed(&world, bob, Some(RemovedTexts::Delete)).await?;
    // The line moved from the database into the file right after the change.
    assert!(world.store.read(|state| state.replay_waiting.is_empty()));
    let (lines, _) = replay::read(&world.replay.text());
    let removal = lines
        .into_iter()
        .find(|line| line.kind == ReplayKind::Removal)
        .ok_or("a removal line waits")?;
    assert_eq!(removal.id, bob.to_string());
    assert_eq!(removal.choice.as_deref(), Some("delete"));
    Ok(())
}

#[tokio::test]
async fn priv_04_delete_their_comments_deletes_comments_and_nothing_else() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    let carol = world
        .person_named("carol@example.test", "Carol")
        .await?
        .person
        .id;
    let ada = world.admin.person.id;
    let mut ids = SequentialIds::default();
    two_comments(&world, &mut ids, bob)?;
    two_comments(&world, &mut ids, carol)?;
    switched_off(&world, bob).await?;
    switched_off(&world, carol).await?;
    // Kept, shown as *Former user*.
    removed(&world, carol, Some(RemovedTexts::Keep)).await?;
    assert_eq!(comments_by(&world, carol), 1);
    let gone = removed(&world, bob, Some(RemovedTexts::Delete)).await?;
    assert_eq!(gone.comments.len(), 1);
    assert_eq!(gone.issues.len(), 1);
    assert_eq!(comments_by(&world, bob), 0);
    assert_eq!(comments_by(&world, carol), 1);
    assert_eq!(comments_by(&world, ada), 2);
    Ok(())
}

#[tokio::test]
async fn priv_04_a_replayed_removal_needs_no_switch_off_and_runs_once() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    let ada = world.admin.person.id;
    let removal = std::sync::Arc::new(Removal {
        person: bob,
        by: ada,
        at: world.ctx().now(),
        label: "K7Q2".to_owned(),
        comments: RemovedTexts::Keep,
        keep: world.ctx().setting_layers_of(None).await?,
        today: jiff::civil::date(2026, 11, 19),
        asked: RemovalAsked::ByReplay,
    });
    let event = world.ctx().event(
        crate::auth::by(ada),
        crate::auth::entity(RecordKind::Person, bob),
        Vec::new(),
    )?;
    for _ in 0..2 {
        let removal = std::sync::Arc::clone(&removal);
        world
            .store
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    remove_in::<super::fake::FakeStore, _>(transaction, &removal)
                        .await
                        .map(|_| ())
                })
            })
            .await
            .map_err(text)?;
    }
    let (standings, lines) = world.store.read(|state| {
        (
            state.standings.iter().filter(|s| s.person == bob).count(),
            state
                .replay_waiting
                .iter()
                .filter(|line| line.contains("removal"))
                .count(),
        )
    });
    assert_eq!(standings, 1);
    assert_eq!(
        lines, 1,
        "the second run left the removed person as they were"
    );
    Ok(())
}

#[tokio::test]
async fn priv_04_the_legal_hold_name_has_its_reason_and_end() -> Outcome {
    use bb_core::working_time::record::{StoredDay, WorkDay};
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    let today = jiff::civil::date(2026, 11, 19);
    let mut ids = SequentialIds::default();
    world.store.lock().state.work_days.push(StoredDay {
        day: WorkDay {
            id: bb_core::id::WorkDayId::generate(&mut ids).map_err(text)?,
            person: bob,
            date: today,
            zone: "Europe/Berlin".to_owned(),
            warnings: Vec::new(),
            updated_at: jiff::Timestamp::UNIX_EPOCH,
            version: 1,
        },
        times: bb_core::working_time::Times::default(),
    });
    switched_off(&world, bob).await?;
    removed(&world, bob, None).await?;
    let standing = world
        .store
        .read(|state| state.standings.iter().find(|s| s.person == bob).cloned())
        .ok_or("Bob has a standing")?;
    assert_eq!(standing.hold_name.as_deref(), Some("Bob Builder"));
    assert_eq!(
        standing.hold_reason,
        Some(bb_core::person::HoldReason::WorkDays)
    );
    assert!(standing.hold_until.is_some_and(|until| until > today));
    Ok(())
}

#[tokio::test]
async fn priv_04_a_replayed_removal_of_nobody_does_nothing() -> Outcome {
    let world = World::new().await?;
    let mut ids = SequentialIds::default();
    let nobody = UserId::generate(&mut ids);
    let ada = world.admin.person.id;
    let removal = std::sync::Arc::new(
        crate::privacy::removal::prepare_removal(
            &world.store,
            &mut ids,
            nobody,
            ada,
            world.ctx().now(),
            None,
            RemovalAsked::ByReplay,
        )
        .await?,
    );
    let event = world.ctx().event(
        crate::auth::by(ada),
        crate::auth::entity(RecordKind::Person, nobody),
        Vec::new(),
    )?;
    let removed = world
        .store
        .record_change(&event, move |transaction| {
            Box::pin(
                async move { remove_in::<super::fake::FakeStore, _>(transaction, &removal).await },
            )
        })
        .await
        .map_err(text)?;
    assert!(removed.taken.is_none());
    let (standings, jobs) = world
        .store
        .read(|state| (state.standings.len(), state.jobs.len()));
    assert_eq!((standings, jobs), (0, 0), "a backup older than the account");
    Ok(())
}

/// Keep periods that answer one fixed end for every work day.
struct FixedEnd(bb_core::keeping::KeptUntil);

impl crate::ports::KeepPeriods for FixedEnd {
    fn kept_until(
        &self,
        _kind: bb_core::keeping::KeepKind,
        _date: jiff::civil::Date,
    ) -> bb_core::keeping::KeptUntil {
        self.0
    }
}

#[tokio::test]
async fn priv_04_the_legal_hold_ends_with_the_keep_period_of_the_last_work_day() -> Outcome {
    use bb_core::keeping::KeptUntil;
    use bb_core::working_time::record::{StoredDay, WorkDay};
    let today = jiff::civil::date(2026, 11, 19);
    let cases = [
        (
            KeptUntil::Day(jiff::civil::date(2036, 12, 31)),
            Some(Some(jiff::civil::date(2036, 12, 31))),
        ),
        (KeptUntil::NoFixedEnd, Some(None)),
        // The last work day is past its keep period: no name is held.
        (KeptUntil::Day(jiff::civil::date(2026, 11, 18)), None),
    ];
    for (until, held) in cases {
        let world = World::new().await?;
        let bob = world.person("bob@example.test").await?.person.id;
        let mut ids = SequentialIds::default();
        world.store.lock().state.work_days.push(StoredDay {
            day: WorkDay {
                id: bb_core::id::WorkDayId::generate(&mut ids).map_err(text)?,
                person: bob,
                date: jiff::civil::date(2016, 3, 1),
                zone: "Europe/Berlin".to_owned(),
                warnings: Vec::new(),
                updated_at: jiff::Timestamp::UNIX_EPOCH,
                version: 1,
            },
            times: bb_core::working_time::Times::default(),
        });
        let ada = world.admin.person.id;
        let prepared = crate::privacy::removal::prepare_removal(
            &world.store,
            &mut ids,
            bob,
            ada,
            world.ctx().now(),
            None,
            RemovalAsked::ByReplay,
        )
        .await?;
        let removal = std::sync::Arc::new(Removal {
            person: prepared.person,
            by: prepared.by,
            at: prepared.at,
            label: prepared.label,
            comments: prepared.comments,
            keep: FixedEnd(until),
            today,
            asked: prepared.asked,
        });
        let event = world.ctx().event(
            crate::auth::by(ada),
            crate::auth::entity(RecordKind::Person, bob),
            Vec::new(),
        )?;
        world
            .store
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    remove_in::<super::fake::FakeStore, _>(transaction, &removal)
                        .await
                        .map(|_| ())
                })
            })
            .await
            .map_err(text)?;
        let standing = world
            .store
            .read(|state| state.standings.iter().find(|s| s.person == bob).cloned())
            .ok_or("Bob has a standing")?;
        let found = standing
            .hold_name
            .as_ref()
            .map(|name| (name.clone(), standing.hold_until));
        assert_eq!(
            found,
            held.map(|end| ("Bob Builder".to_owned(), end)),
            "{until:?}"
        );
    }
    Ok(())
}
