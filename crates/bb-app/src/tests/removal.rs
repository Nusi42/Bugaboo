//! *Remove a person* against the in-memory store (PRIV-04, ADR-0044, decisions 6 to 8): only a
//! switched-off person is removed, and only once; the change records the decision and queues the
//! job that finishes it, step by step and again after a restart; the person becomes a tombstone
//! with a label and, once the job is done, no name or address; the changes are scrubbed of their
//! records and values; the comments go only when asked; and the line of the replay list waits
//! after the change. A replay needs no switch-off and leaves a removed person as they are.

use bb_core::ErrorCode;
use bb_core::backup::replay::{self, ReplayKind};
use bb_core::comment::Comment;
use bb_core::event::RecordKind;
use bb_core::id::{CommentId, IdSource, IssueId, SequentialIds, UserId};
use bb_core::person::Standing;
use bb_core::settings::privacy::RemovedTexts;

use super::scenarios::{Added, measured, text, token};
use super::switch_off::switched_off;
use super::world::{OTHER_PASSWORD, Outcome, PASSWORD, World};
use crate::auth::AppError;
use crate::people::{SwitchOn, switch_on};
use crate::ports::ChangeStore as _;
use crate::privacy::removal::{
    FinishRemoval, FinishStep, Removal, RemovePerson, Removed, remove_in, remove_person,
};
use crate::privacy::replay::Asked;

/// The administrator removes `person`, with `comments` or the setting's choice.
pub(super) async fn removed(
    world: &World,
    person: UserId,
    comments: Option<RemovedTexts>,
) -> Result<Removed, AppError> {
    removed_typing(world, person, comments, PASSWORD).await
}

/// The administrator removes `person`, with `password` typed again.
async fn removed_typing(
    world: &World,
    person: UserId,
    comments: Option<RemovedTexts>,
    password: &str,
) -> Result<Removed, AppError> {
    remove_person(
        &world.ctx(),
        RemovePerson {
            session_token: token(world),
            person,
            comments,
            password: password.to_owned(),
        },
    )
    .await
}

#[tokio::test]
async fn sec_12_a_stolen_session_cannot_remove_a_person_without_the_password() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    switched_off(&world, bob).await?;
    for typed in ["", OTHER_PASSWORD] {
        let refused = removed_typing(&world, bob, None, typed).await;
        assert_eq!(
            refused.err().and_then(|error| error.code()),
            Some(ErrorCode::WrongCredentials),
            "{typed:?}"
        );
    }
    removed_typing(&world, bob, None, PASSWORD).await?;
    Ok(())
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
pub(super) fn two_comments(
    world: &World,
    ids: &mut SequentialIds,
    bob: UserId,
) -> Result<(), String> {
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
        Some(ErrorCode::PersonNotSwitchedOff)
    );
    Ok(())
}

/// Whether Bugaboo still removes what was `person`'s, as the page of the person says it.
pub(super) async fn removing(world: &World, person: UserId) -> Result<bool, AppError> {
    crate::people::person(
        &world.ctx(),
        crate::people::PersonQuery {
            session_token: token(world),
            person,
        },
    )
    .await
    .map(|found| found.into_value().removing)
}

#[tokio::test]
async fn priv_04_a_removed_person_is_a_tombstone_with_no_name_or_address() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    switched_off(&world, bob).await?;
    removed(&world, bob, None).await?;
    // The change answers at once: the account is closed, and the job goes on with the rest.
    assert!(
        removing(&world, bob).await?,
        "removing… until the job is done"
    );
    let scrubs = world.store.read(|state| state.scrubs.clone());
    // The change scrubs the changes about the person's records, through them, before they go.
    let first = &scrubs.first().ok_or("the changes are scrubbed")?.1;
    assert!(first.kinds.contains(&RecordKind::Person));
    assert!(first.kinds.contains(&RecordKind::AccessKey));
    assert!(!first.kinds.contains(&RecordKind::SavedSearch));
    assert!(!first.kinds.contains(&RecordKind::WorkDay));
    assert!(!first.records.is_empty());
    assert!(first.names.is_empty(), "the words are the job's");
    super::data_exports::run_jobs(&world).await?;
    assert!(!removing(&world, bob).await?);
    let (person, standing, words) = world.store.read(|state| {
        (
            state.people.iter().find(|(p, _)| p.id == bob).cloned(),
            state.standings.iter().find(|s| s.person == bob).cloned(),
            state.word_scrubs.clone(),
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
    // The job scrubbed every table of his name and address, a table a step here.
    let tables: Vec<_> = words.iter().map(|(_, place)| place.table).collect();
    assert_eq!(
        tables,
        crate::privacy::removal::ScrubbedTable::ALL.to_vec(),
        "each table once"
    );
    let (scrub, _) = words.first().ok_or("the words are scrubbed")?;
    assert!(scrub.names.contains(&"Bob Builder".to_owned()));
    assert!(scrub.addresses.contains(&"bob@example.test".to_owned()));
    Ok(())
}

/// The row of the removal of `person` that is not finished.
pub(super) fn pending_of(
    world: &World,
    person: UserId,
) -> Option<bb_core::removal::PendingRemoval> {
    world.store.read(|state| {
        state
            .pending_removals
            .iter()
            .find(|(pending, _)| pending.person == person)
            .map(|(pending, _)| pending.clone())
    })
}

/// The jobs that finish a removal and wait now, with their payloads.
pub(super) fn finishing(world: &World) -> Vec<serde_json::Value> {
    world.store.read(|state| {
        state
            .jobs
            .iter()
            .filter(|job| job.kind.as_str() == "privacy.finish-removal")
            .map(|job| job.payload.clone())
            .collect()
    })
}

#[tokio::test]
async fn priv_04_a_removal_goes_on_after_a_restart_from_its_last_step() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    switched_off(&world, bob).await?;
    removed(&world, bob, None).await?;
    // A round of the queue runs the one step that is due; the next waits with its place.
    world.ctx().run_due_jobs().await.map_err(text)?;
    world.ctx().run_due_jobs().await.map_err(text)?;
    let [waiting] = finishing(&world)
        .try_into()
        .map_err(|left| format!("{left:?}"))?;
    let left: FinishRemoval = serde_json::from_value(waiting.clone()).map_err(text)?;
    assert!(matches!(left.step, FinishStep::Scrub(_)), "{left:?}");
    assert!(removing(&world, bob).await?);
    // The server stopped after it queued the next step but before it was done with this one: the
    // step runs twice. Nothing is lost and nothing breaks; the person ends removed once.
    {
        let mut shared = world.store.lock();
        let copy = shared
            .state
            .jobs
            .iter()
            .find(|job| job.kind.as_str() == "privacy.finish-removal")
            .cloned()
            .ok_or("the step waits")?;
        let mut again = copy.clone();
        again.id = bb_core::id::JobId::generate(&mut SequentialIds::default()).map_err(text)?;
        shared.state.jobs.push(again);
    }
    super::data_exports::run_jobs(&world).await?;
    assert!(!removing(&world, bob).await?);
    assert!(finishing(&world).is_empty(), "nothing is left to do");
    let compactions = world.store.read(|state| state.compactions);
    assert!(
        compactions >= 1,
        "the old text is let go after the last step"
    );
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
    super::data_exports::run_jobs(&world).await?;
    assert_eq!(comments_by(&world, carol), 1);
    removed(&world, bob, Some(RemovedTexts::Delete)).await?;
    // The comments go in the job, which then lets the index forget them.
    assert_eq!(comments_by(&world, bob), 1, "the change only queues it");
    super::data_exports::run_jobs(&world).await?;
    assert_eq!(comments_by(&world, bob), 0);
    assert_eq!(comments_by(&world, carol), 1);
    assert_eq!(comments_by(&world, ada), 2);
    let reindexed = world.store.read(|state| state.reindexed.clone());
    assert!(
        reindexed
            .iter()
            .any(|(issues, comments)| issues.len() == 1 && comments.len() == 1),
        "{reindexed:?}"
    );
    Ok(())
}

#[tokio::test]
async fn priv_04_a_replayed_removal_needs_no_switch_off_and_runs_once() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    let ada = world.admin.person.id;
    let removal = std::sync::Arc::new(
        crate::privacy::removal::prepare_removal(
            &world.store,
            &mut SequentialIds::default(),
            bob,
            world.ctx().now(),
            Some(RemovedTexts::Keep),
            Asked::ByReplay,
        )
        .await
        .map_err(|failure| format!("{failure:?}"))?,
    );
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
async fn priv_04_the_job_after_a_removal_forgets_the_texts_and_deletes_the_export_files() -> Outcome
{
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    switched_off(&world, bob).await?;
    let export = super::data_exports::ask_for(&world, &token(&world), bob, "Bob asked by letter")
        .await?
        .export
        .id;
    super::data_exports::run_jobs(&world).await?;
    assert!(world.exports.bytes(export).is_some(), "the file is written");
    let mut ids = SequentialIds::default();
    two_comments(&world, &mut ids, bob)?;
    removed(&world, bob, Some(RemovedTexts::Delete)).await?;
    let waiting = finishing(&world);
    let [payload] = waiting.as_slice() else {
        return Err(format!("one job finishes the removal: {waiting:?}").into());
    };
    let left: FinishRemoval = serde_json::from_value(payload.clone()).map_err(text)?;
    assert_eq!(left.person, bob);
    assert_eq!(left.step, FinishStep::Files(vec![export]));
    let pending = pending_of(&world, bob).ok_or("the removal is recorded as not finished")?;
    assert!(pending.comments_go);
    super::data_exports::run_jobs(&world).await?;
    assert!(world.exports.bytes(export).is_none(), "the file is gone");
    let (reindexed, compactions) = world
        .store
        .read(|state| (state.reindexed.clone(), state.compactions));
    let forgotten: usize = reindexed.iter().map(|(_, comments)| comments.len()).sum();
    assert_eq!(forgotten, 1, "the index forgets his comment");
    assert_eq!(compactions, 1, "the old text is let go after the last step");
    assert!(finishing(&world).is_empty());
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
            world.ctx().now(),
            None,
            Asked::ByReplay,
        )
        .await
        .map_err(|failure| format!("{failure:?}"))?,
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
            world.ctx().now(),
            None,
            Asked::ByReplay,
        )
        .await
        .map_err(|failure| format!("{failure:?}"))?;
        let removal = std::sync::Arc::new(Removal {
            person: prepared.person,
            at: prepared.at,
            label: prepared.label,
            comments: prepared.comments,
            keep: FixedEnd(until),
            today,
            asked: prepared.asked,
            job: prepared.job,
            words: prepared.words,
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

#[tokio::test]
async fn priv_04_a_replayed_switch_off_runs_once() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    let ada = world.admin.person.id;
    let at = world.ctx().now();
    let event = world.ctx().event(
        crate::auth::by(ada),
        crate::auth::entity(RecordKind::Person, bob),
        Vec::new(),
    )?;
    let mut done = Vec::new();
    for _ in 0..2 {
        let switched = world
            .store
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    crate::people::switch_off_in::<super::fake::FakeStore>(
                        transaction,
                        bob,
                        at,
                        Asked::ByReplay,
                    )
                    .await
                    .map(|switched| switched.is_some())
                })
            })
            .await
            .map_err(text)?;
        done.push(switched);
    }
    assert_eq!(done, [true, false], "the second run leaves him as he is");
    let (standing, lines) = world.store.read(|state| {
        (
            state
                .standings
                .iter()
                .find(|s| s.person == bob)
                .map(|s| s.standing),
            state
                .replay_waiting
                .iter()
                .filter(|line| line.contains("switch-off"))
                .count(),
        )
    });
    assert_eq!(standing, Some(Standing::SwitchedOff));
    assert_eq!(lines, 1);
    Ok(())
}
