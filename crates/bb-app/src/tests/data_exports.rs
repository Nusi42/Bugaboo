//! *Download my data* (PRIV-03, ADR-0044, decisions 2 to 4), run against the in-memory store: one
//! export at a time, the administrator's export with its permission, reason and security-log
//! entry, the signed link that is for one person and runs out, and other people in the file only as
//! the person may see them.

use std::io::{Cursor, Read as _};

use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::comment::Comment;
use bb_core::data_export::{EXPORT_ASKS_PER_DAY, EXPORT_ASKS_WINDOW, ExportState};
use bb_core::files::LINK_LIFETIME;
use bb_core::id::{DataExportId, UserId};
use bb_core::notification::NotificationKind;
use bb_core::people_kinds::PeopleKind;
use bb_core::personal::{Entity as _, Link};
use bb_core::sign_in::SecurityAction;
use bb_core::technical::TechnicalName;
use bb_core::text::REASON_MAX;
use bb_policy::Permission;
use jiff::SignedDuration;
use zip::ZipArchive;

use super::fake_exports::WalkedRow;
use super::notifications::Crew;
use super::scenarios::{Added, measured, text, token};
use super::world::{OTHER_PASSWORD, Outcome, PASSWORD, World};
use crate::auth::AppError;
use crate::jobs::{JOB_ATTEMPTS_MAX, JobStore as _};
use crate::privacy::export::{
    AskMyData, AskSomeonesData, DownloadExportQuery, ExportLinkQuery, ExportView,
    MyDataExportQuery, ask_for_my_data, ask_for_someones_data, download_export, export_link,
    my_data_export,
};
use crate::privacy::walk::{Cell, FoundRow, Value};
use crate::roles::{ChangeRole, SetPeopleLevel, change_role, set_level};

pub(super) async fn run_jobs(world: &World) -> Result<(), String> {
    while world.ctx().run_due_jobs().await.map_err(text)? > 0 {}
    Ok(())
}

pub(super) async fn ask_mine(world: &World, session: &str) -> Result<ExportView, AppError> {
    ask_for_my_data(
        &world.ctx(),
        AskMyData {
            session_token: session.to_owned(),
        },
    )
    .await
    .map(crate::access::Authorized::into_value)
}

pub(super) async fn ask_for(
    world: &World,
    session: &str,
    person: UserId,
    reason: &str,
) -> Result<ExportView, AppError> {
    ask_typing(world, session, person, reason, PASSWORD).await
}

/// *Download someone's data*, with `password` typed again.
async fn ask_typing(
    world: &World,
    session: &str,
    person: UserId,
    reason: &str,
    password: &str,
) -> Result<ExportView, AppError> {
    ask_for_someones_data(
        &world.ctx(),
        AskSomeonesData {
            session_token: session.to_owned(),
            person: Some(person),
            reason: Some(reason.to_owned()),
            password: password.to_owned(),
        },
    )
    .await
    .map(crate::access::Authorized::into_value)
}

#[tokio::test]
async fn sec_12_a_stolen_session_cannot_download_someones_data_without_the_password() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    let session = token(&world);
    for typed in ["", OTHER_PASSWORD] {
        let refused = ask_typing(&world, &session, bob, "Bob asked by letter", typed).await;
        assert_eq!(
            refused.err().and_then(|error| error.code()),
            Some(ErrorCode::WrongCredentials),
            "{typed:?}"
        );
    }
    ask_typing(&world, &session, bob, "Bob asked by letter", PASSWORD).await?;
    Ok(())
}

async fn state_of(world: &World, session: &str) -> Result<Option<ExportState>, String> {
    Ok(my_data_export(
        &world.ctx(),
        MyDataExportQuery {
            session_token: session.to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .map(|view| view.export.state))
}

async fn link(world: &World, session: &str, export: DataExportId) -> Result<String, AppError> {
    export_link(
        &world.ctx(),
        ExportLinkQuery {
            session_token: session.to_owned(),
            export: Some(export),
        },
    )
    .await
    .map(|link| link.into_value().token)
}

async fn download(world: &World, session: &str, link: &str) -> Result<Vec<u8>, AppError> {
    let mut file = download_export(
        &world.ctx(),
        DownloadExportQuery {
            session_token: session.to_owned(),
            link: link.to_owned(),
        },
    )
    .await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = file
        .body
        .reader
        .next_chunk()
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
    {
        bytes.extend(chunk);
    }
    Ok(bytes)
}

/// The text of one file inside the ZIP of an export.
fn entry(zip: &[u8], name: &str) -> Result<String, String> {
    let mut archive = ZipArchive::new(Cursor::new(zip)).map_err(text)?;
    let mut found = archive.by_name(name).map_err(text)?;
    let mut read = String::new();
    found.read_to_string(&mut read).map_err(text)?;
    Ok(read)
}

fn refused(result: &Result<ExportView, AppError>) -> Option<ErrorCode> {
    match result {
        Err(AppError::Refused(code)) => Some(*code),
        _ => None,
    }
}

pub(crate) async fn ask_for_my_data_scenario(world: &World) -> Added {
    measured(&world.store, ask_mine(world, &token(world))).await
}

pub(crate) async fn ask_for_someones_data_scenario(world: &World) -> Added {
    let bob = world.person("bob@example.test").await.map_err(text)?;
    measured(
        &world.store,
        ask_for(world, &token(world), bob.person.id, "Bob asked by letter"),
    )
    .await
}

#[tokio::test]
async fn priv_03_one_export_at_a_time_per_person() -> Outcome {
    let crew = Crew::new().await?;
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    let first = ask_mine(world, bob).await?;
    assert_eq!(first.export.state, ExportState::Waiting);
    assert_eq!(
        refused(&ask_mine(world, bob).await),
        Some(ErrorCode::ExportWaiting)
    );
    // An administrator cannot start a second one beside it either.
    assert_eq!(
        refused(&ask_for(world, &token(world), crew.bob.person.id, "asked by mail").await),
        Some(ErrorCode::ExportWaiting)
    );
    // Somebody else's export does not wait for Bob's.
    ask_mine(world, &crew.carol.session_token).await?;
    run_jobs(world).await?;
    assert_eq!(state_of(world, bob).await?, Some(ExportState::Ready));
    assert!(world.exports.bytes(first.export.id).is_some());
    // Once it is ready, Bob may ask again.
    ask_mine(world, bob).await?;
    Ok(())
}

#[tokio::test]
async fn priv_03_an_administrator_needs_download_someones_data_and_the_person_is_told() -> Outcome {
    let crew = Crew::new().await?;
    let world = &crew.world;
    let carol = crew.carol.person.id;
    // Bob holds no *Download someone's data*.
    assert_eq!(
        refused(&ask_for(world, &crew.bob.session_token, carol, "curious").await),
        Some(ErrorCode::Forbidden)
    );
    // Ada does, but never without a reason.
    assert_eq!(
        refused(&ask_for(world, &token(world), carol, "  ").await),
        Some(ErrorCode::ExportReasonNeeded)
    );
    let made = ask_for(world, &token(world), carol, "Carol asked by letter").await?;
    assert_eq!(made.export.person, carol);
    assert_eq!(made.export.reason.as_deref(), Some("Carol asked by letter"));
    // Carol is told at once (the notices of exports are tested in `data_export_notices`).
    assert_eq!(
        super::notifications::kinds(&crew.heard(&crew.carol.session_token).await?),
        vec![NotificationKind::ExportByAdministrator]
    );
    run_jobs(world).await?;
    // The file is Ada's to download, not Carol's.
    let ada_link = link(world, &token(world), made.export.id).await?;
    assert!(!download(world, &token(world), &ada_link).await?.is_empty());
    assert!(matches!(
        link(world, &crew.carol.session_token, made.export.id).await,
        Err(AppError::Refused(ErrorCode::NotFound))
    ));
    Ok(())
}

#[tokio::test]
async fn priv_03_the_administrators_export_is_in_the_security_log_with_its_reason() -> Outcome {
    let crew = Crew::new().await?;
    let world = &crew.world;
    let made = ask_for(
        world,
        &token(world),
        crew.carol.person.id,
        "Carol asked by letter",
    )
    .await?;
    let ada = world.admin.person.id;
    let noted = |action: SecurityAction| {
        world
            .store
            .lock()
            .log
            .iter()
            .filter(|entry| {
                entry.action == action
                    && entry.person == Some(ada)
                    && entry.subject.as_ref().is_some_and(|subject| {
                        subject.id == bb_core::id::EntityId::from(made.export.id)
                    })
            })
            .count()
    };
    assert_eq!(noted(SecurityAction::DataExportForSomebodyElse), 1);
    // The entry names the export, and the export holds for whom and why.
    let stored = world
        .store
        .lock()
        .state
        .data_exports
        .iter()
        .find(|export| export.id == made.export.id)
        .cloned()
        .ok_or("the export is stored")?;
    assert_eq!(stored.person, crew.carol.person.id);
    assert_eq!(stored.reason.as_deref(), Some("Carol asked by letter"));
    // Every download is written down too.
    run_jobs(world).await?;
    let link = link(world, &token(world), made.export.id).await?;
    download(world, &token(world), &link).await?;
    assert_eq!(noted(SecurityAction::DataExportDownloaded), 1);
    // A person's own export is no administrator's export.
    ask_mine(world, &crew.bob.session_token).await?;
    assert_eq!(noted(SecurityAction::DataExportForSomebodyElse), 1);
    // Seven days on, the file and its record are deleted. The entry still points to the export,
    // and its history still says whose data it was and why.
    world
        .clock
        .advance(bb_core::data_export::EXPORT_KEPT.as_secs() + 1);
    assert!(world.ctx().delete_old_exports().await? >= 1);
    assert!(world.store.lock().state.data_exports.is_empty());
    assert_eq!(noted(SecurityAction::DataExportForSomebodyElse), 1);
    let asked = world
        .store
        .lock()
        .events
        .iter()
        .find(|event| {
            event.entity.id == bb_core::id::EntityId::from(made.export.id)
                && event.changes.iter().any(|change| change.field == "reason")
        })
        .cloned()
        .ok_or("the history of the export is kept")?;
    let after = |field: &str| {
        asked
            .changes
            .iter()
            .find(|change| change.field == field)
            .and_then(|change| change.after.clone())
    };
    assert_eq!(
        after("person"),
        Some(serde_json::to_value(crew.carol.person.id).map_err(text)?)
    );
    assert_eq!(
        after("reason"),
        Some(serde_json::json!("Carol asked by letter"))
    );
    Ok(())
}

#[tokio::test]
async fn priv_03_the_download_link_is_for_the_person_and_runs_out() -> Outcome {
    let crew = Crew::new().await?;
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    let made = ask_mine(world, bob).await?;
    // No link before the file is ready.
    assert!(link(world, bob, made.export.id).await.is_err());
    run_jobs(world).await?;
    let token_of_bob = link(world, bob, made.export.id).await?;
    let zip = download(world, bob, &token_of_bob).await?;
    assert!(entry(&zip, "README.txt")?.contains("GDPR"));
    // Carol cannot use Bob's link, nor ask for one.
    assert!(matches!(
        download(world, &crew.carol.session_token, &token_of_bob).await,
        Err(AppError::Refused(ErrorCode::NotFound))
    ));
    assert!(
        link(world, &crew.carol.session_token, made.export.id)
            .await
            .is_err()
    );
    // Five minutes later the link opens nothing.
    world.clock.advance(LINK_LIFETIME.as_secs());
    assert!(matches!(
        download(world, bob, &token_of_bob).await,
        Err(AppError::Refused(ErrorCode::NotFound))
    ));
    Ok(())
}

#[tokio::test]
async fn priv_03_an_export_is_deleted_with_its_file_after_seven_days() -> Outcome {
    let crew = Crew::new().await?;
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    ask_mine(world, bob).await?;
    run_jobs(world).await?;
    assert_eq!(world.exports.count(), 1);
    world
        .clock
        .advance(bb_core::data_export::EXPORT_KEPT.as_secs() - 1);
    assert_eq!(world.ctx().delete_old_exports().await?, 0);
    world.clock.advance(2);
    assert_eq!(world.ctx().delete_old_exports().await?, 1);
    assert_eq!(world.exports.count(), 0);
    assert!(world.store.lock().state.data_exports.is_empty());
    // A second round finds nothing more.
    assert_eq!(world.ctx().delete_old_exports().await?, 0);
    Ok(())
}

/// A comment Carol wrote that names Bob, as the walk of Bob's data hands it over.
fn carols_comment(bob: UserId, carol: UserId) -> WalkedRow {
    WalkedRow {
        person: bob,
        kind: "comment".to_owned(),
        marks: Comment::MARKS,
        row: FoundRow {
            links: vec![Link::Subject],
            cells: vec![
                Cell {
                    column: "author".to_owned(),
                    field: "author",
                    value: Value::Person(carol),
                },
                Cell {
                    column: "text".to_owned(),
                    field: "text",
                    value: Value::Text("Thanks, @bob".to_owned()),
                },
            ],
        },
    }
}

/// Sets how the role of Bob and Carol sees people of `kind`.
pub(super) async fn see(crew: &Crew, kind: PeopleKind, level: PeopleLevel) -> Result<(), String> {
    set_level(
        &crew.world.ctx(),
        SetPeopleLevel {
            session_token: token(&crew.world),
            role: crew.role,
            project: None,
            kind,
            level: Some(level),
        },
    )
    .await
    .map(|_| ())
    .map_err(text)
}

/// The JSON of the comments in Bob's file, when every kind but `strict` is seen by names.
async fn bobs_comments(strict: Option<PeopleLevel>) -> Result<String, String> {
    let crew = Crew::new().await?;
    for kind in PeopleKind::ALL {
        see(&crew, *kind, PeopleLevel::Names).await?;
    }
    if let Some(level) = strict {
        see(&crew, PeopleKind::Followers, level).await?;
    }
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    world
        .store
        .lock()
        .state
        .walk
        .push(carols_comment(crew.bob.person.id, crew.carol.person.id));
    let made = ask_mine(world, bob).await.map_err(text)?;
    run_jobs(world).await?;
    let zip = world
        .exports
        .bytes(made.export.id)
        .ok_or("the file is written")?;
    entry(&zip, "data/comment.json")
}

#[tokio::test]
async fn priv_03_others_appear_only_as_the_person_may_see_them() -> Outcome {
    let names = bobs_comments(None).await?;
    assert!(names.contains("Carol Crane"), "{names}");
    // Carol's text is not copied, only that her comment names Bob.
    assert!(!names.contains("Thanks"), "{names}");
    // One kind of people information seen by role only, or not at all, and the file names nobody.
    for level in [PeopleLevel::RoleOnly, PeopleLevel::Hidden] {
        let hidden = bobs_comments(Some(level)).await?;
        assert!(!hidden.contains("Carol"), "{level:?}: {hidden}");
        assert!(!hidden.contains(&"Crane".to_owned()), "{level:?}: {hidden}");
    }
    Ok(())
}

/// A worker of the queue, by name.
fn worker(name: &str) -> Result<TechnicalName, String> {
    TechnicalName::new(name).map_err(text)
}

#[tokio::test]
async fn priv_03_an_export_whose_job_runs_twice_is_finished_and_told_once() -> Outcome {
    let crew = Crew::new().await?;
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    let made = ask_mine(world, bob).await?;
    // A worker takes the job; its lease runs out while it still works, and a second worker takes
    // the same job, so both write the file at once.
    let lease = SignedDuration::from_secs(1);
    let mut first = world
        .store
        .lease_jobs(
            &worker("worker.first")?,
            world.ctx().now(),
            lease,
            5,
            &crate::jobs::Lane::Long.kinds()?,
        )
        .await
        .map_err(text)?;
    world.clock.advance(2);
    let mut second = world
        .store
        .lease_jobs(
            &worker("worker.second")?,
            world.ctx().now(),
            lease,
            5,
            &crate::jobs::Lane::Long.kinds()?,
        )
        .await
        .map_err(text)?;
    let (first, second) = (
        first.pop().ok_or("the first worker took the job")?,
        second.pop().ok_or("the second worker took it again")?,
    );
    assert_eq!(first.id, second.id);
    let (one, other) = (world.ctx(), world.ctx());
    let (one, other) = tokio::join!(one.run_export(&first), other.run_export(&second));
    one?;
    other?;
    // One file was made the export's, and it is whole.
    assert_eq!(world.exports.kept(), 1);
    let zip = world
        .exports
        .bytes(made.export.id)
        .ok_or("the file is there")?;
    assert!(entry(&zip, "README.txt")?.contains("GDPR"));
    assert_eq!(state_of(world, bob).await?, Some(ExportState::Ready));
    // Bob hears once that it is ready.
    run_jobs(world).await?;
    let ready = super::notifications::kinds(&crew.heard(bob).await?)
        .into_iter()
        .filter(|kind| *kind == NotificationKind::ExportReady)
        .count();
    assert_eq!(ready, 1);
    Ok(())
}

#[tokio::test]
async fn priv_03_an_export_whose_job_gives_up_fails_and_may_be_asked_again() -> Outcome {
    let crew = Crew::new().await?;
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    let made = ask_mine(world, bob).await?;
    // The disk is full: every try stops half way and leaves its half-written file.
    world.exports.break_writing();
    run_jobs(world).await?;
    assert_eq!(world.exports.half_written(made.export.id), 1);
    for _ in 1..JOB_ATTEMPTS_MAX {
        world.clock.advance(RETRY_WAIT);
        run_jobs(world).await?;
    }
    assert_eq!(state_of(world, bob).await?, Some(ExportState::Failed));
    assert_eq!(world.exports.half_written(made.export.id), 0);
    assert!(world.exports.bytes(made.export.id).is_none());
    assert!(
        !super::notifications::kinds(&crew.heard(bob).await?)
            .contains(&NotificationKind::ExportReady)
    );
    // Bob may ask again.
    ask_mine(world, bob).await?;
    Ok(())
}

/// Longer than the in-memory queue waits before a job is tried again (thirty seconds), and far
/// shorter than a session lasts.
const RETRY_WAIT: i64 = 60;

#[tokio::test]
async fn priv_03_an_administrator_cannot_ask_for_the_data_of_somebody_who_is_not_there() -> Outcome
{
    let crew = Crew::new().await?;
    let world = &crew.world;
    let nobody = UserId::generate(&mut world.ctx().ids());
    assert_eq!(
        refused(&ask_for(world, &token(world), nobody, "asked by letter").await),
        Some(ErrorCode::NotFound)
    );
    Ok(())
}

#[tokio::test]
async fn priv_03_the_reason_of_an_administrators_export_has_a_length_it_may_not_pass() -> Outcome {
    let crew = Crew::new().await?;
    let world = &crew.world;
    let carol = crew.carol.person.id;
    let longest = "x".repeat(REASON_MAX);
    assert_eq!(
        refused(&ask_for(world, &token(world), carol, &format!("{longest}x")).await),
        Some(ErrorCode::InvalidRequest)
    );
    ask_for(world, &token(world), carol, &longest).await?;
    Ok(())
}

/// Gives the role of Bob and Carol *Download someone's data*, or takes it away again.
pub(super) async fn may_download_someones_data(crew: &Crew, may: bool) -> Result<(), String> {
    let known = crew
        .world
        .store
        .lock()
        .state
        .roles
        .iter()
        .find(|known| known.role.id == crew.role)
        .map(|known| (known.role.version, known.permissions.clone()))
        .ok_or("the role is there")?;
    let (version, mut permissions) = known;
    permissions.remove(&Permission::DownloadSomeonesData);
    if may {
        permissions.insert(Permission::DownloadSomeonesData);
    }
    change_role(
        &crew.world.ctx(),
        ChangeRole {
            session_token: token(&crew.world),
            role: crew.role,
            version,
            name: None,
            permissions: Some(permissions.into_iter().collect()),
            two_step: None,
        },
    )
    .await
    .map(|_| ())
    .map_err(text)
}

#[tokio::test]
async fn priv_03_an_administrator_who_lost_download_someones_data_cannot_download_it() -> Outcome {
    let crew = Crew::new().await?;
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    may_download_someones_data(&crew, true).await?;
    let made = ask_for(world, bob, crew.carol.person.id, "Carol asked by letter").await?;
    run_jobs(world).await?;
    let token_of_bob = link(world, bob, made.export.id).await?;
    may_download_someones_data(&crew, false).await?;
    // Neither a new link nor the one Bob holds opens the file any more.
    assert!(matches!(
        link(world, bob, made.export.id).await,
        Err(AppError::Refused(ErrorCode::NotFound))
    ));
    assert!(matches!(
        download(world, bob, &token_of_bob).await,
        Err(AppError::Refused(ErrorCode::NotFound))
    ));
    // With the permission back, the file is Bob's again.
    may_download_someones_data(&crew, true).await?;
    assert!(!download(world, bob, &token_of_bob).await?.is_empty());
    Ok(())
}

/// Moves every export of the world back by `length`, as if it had been asked for so much earlier.
fn asked_earlier(world: &World, length: SignedDuration) -> Result<(), String> {
    for export in &mut world.store.lock().state.data_exports {
        export.asked_at = export.asked_at.checked_sub(length).map_err(text)?;
    }
    Ok(())
}

#[tokio::test]
async fn priv_03_a_new_file_of_a_persons_data_replaces_the_older_ones() -> Outcome {
    let crew = Crew::new().await?;
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    let first = ask_mine(world, bob).await?;
    run_jobs(world).await?;
    // An administrator's file of Bob's data is the administrator's, and stays.
    let theirs = ask_for(world, &token(world), crew.bob.person.id, "asked by mail").await?;
    run_jobs(world).await?;
    // Within the day the older file stays: its record is one of the asks of the day.
    let second = ask_mine(world, bob).await?;
    run_jobs(world).await?;
    assert!(world.exports.bytes(first.export.id).is_some());
    // A day later the next file replaces the older ones of the same asker.
    asked_earlier(world, EXPORT_ASKS_WINDOW)?;
    let third = ask_mine(world, bob).await?;
    run_jobs(world).await?;
    assert!(world.exports.bytes(first.export.id).is_none());
    assert!(world.exports.bytes(second.export.id).is_none());
    assert!(world.exports.bytes(third.export.id).is_some());
    assert!(world.exports.bytes(theirs.export.id).is_some());
    let ids: Vec<DataExportId> = world
        .store
        .lock()
        .state
        .data_exports
        .iter()
        .map(|export| export.id)
        .collect();
    assert!(!ids.contains(&first.export.id) && !ids.contains(&second.export.id));
    assert!(ids.contains(&third.export.id) && ids.contains(&theirs.export.id));
    Ok(())
}

#[tokio::test]
async fn priv_03_the_asks_of_a_day_are_counted_from_the_records_whoever_asks() -> Outcome {
    // The world counts nothing in memory (its limits let everything through), so what stops the
    // fourth ask is the count in the records: it holds across restarts and for every server.
    let crew = Crew::new().await?;
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    for ask in 0..EXPORT_ASKS_PER_DAY {
        if ask == 1 {
            ask_for(world, &token(world), crew.bob.person.id, "asked by mail").await?;
        } else {
            ask_mine(world, bob).await?;
        }
        if ask == 0 {
            // An ask that is refused while one waits leaves no record and counts for nothing.
            assert_eq!(
                refused(&ask_mine(world, bob).await),
                Some(ErrorCode::ExportWaiting)
            );
        }
        run_jobs(world).await?;
    }
    let wait = match ask_mine(world, bob).await {
        Err(AppError::Limited { retry_after }) => retry_after,
        other => return Err(format!("not limited, answered: {}", other.is_ok()).into()),
    };
    let day = u64::try_from(EXPORT_ASKS_WINDOW.as_secs()).map_err(text)?;
    assert!(wait > 0 && wait <= day, "{wait}");
    // The administrator's ask is refused the same way: the limit is the person's.
    assert!(matches!(
        ask_for(world, &token(world), crew.bob.person.id, "asked again").await,
        Err(AppError::Limited { .. })
    ));
    // Once the asks are a day old, the person may ask again.
    asked_earlier(world, EXPORT_ASKS_WINDOW)?;
    ask_mine(world, bob).await?;
    Ok(())
}

#[tokio::test]
async fn priv_03_a_file_past_its_keep_period_is_not_handed_out_before_it_is_deleted() -> Outcome {
    let crew = Crew::new().await?;
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    let made = ask_mine(world, bob).await?;
    run_jobs(world).await?;
    let token_of_bob = link(world, bob, made.export.id).await?;
    // The job that deletes old files has not run yet, but the file was asked for long ago.
    asked_earlier(world, bb_core::data_export::EXPORT_KEPT)?;
    assert!(matches!(
        download(world, bob, &token_of_bob).await,
        Err(AppError::Refused(ErrorCode::NotFound))
    ));
    assert!(link(world, bob, made.export.id).await.is_err());
    Ok(())
}

#[tokio::test]
async fn priv_03_no_file_is_written_when_the_disk_has_no_room() -> Outcome {
    let crew = Crew::new().await?;
    let (world, bob) = (&crew.world, crew.bob.session_token.as_str());
    let made = ask_mine(world, bob).await?;
    world.files.fill_up();
    run_jobs(world).await?;
    assert_ne!(state_of(world, bob).await?, Some(ExportState::Ready));
    assert!(world.exports.bytes(made.export.id).is_none());
    Ok(())
}
