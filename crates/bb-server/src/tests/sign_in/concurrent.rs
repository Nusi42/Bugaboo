//! Ten people working at once, through the real wiring and at the speed of the API (TEST-CONCURRENT,
//! architecture §11, ADR-0019), on both databases.
//!
//! Ten people, each with a browser of their own, change the same few issues together: short values,
//! descriptions under their locks, comments and reads. Some editors vanish without letting go. The
//! clock of the test runs on while they work, so leases really run out. When they are done the
//! invariants of the architecture hold: every change has its event, versions only go up, no lock
//! outlives its lease, and no text that was typed is lost.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_core::edit::{EditWhat, LEASE_SECONDS};
use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::EntityId;
use bb_core::technical::TechnicalName;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};
use tokio::task::JoinSet;

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::rig::Rig;

const LOCKS: &str = "/api/v1/edit-locks";
const PEOPLE: usize = 10;
const ISSUE_COUNT: usize = 4;
const ROUNDS: usize = 14;
/// How many seconds of the clock of the test pass at every tick of the run.
const SECONDS_A_TICK: i64 = 7;
const TICK: Duration = Duration::from_millis(15);
/// Long enough for the clock of the test to pass the lease.
const SLOW_TYPIST: Duration = Duration::from_millis(220);

/// An issue the people work on.
#[derive(Clone)]
struct Work {
    id: String,
    key: String,
}

/// Something that was kept by the server and that the invariants are about.
struct Kept {
    key: String,
    /// The version the issue had after the change.
    version: i64,
    /// The description, when the change was a save of the description.
    text: Option<String>,
}

/// Everything the ten people did that counts, and what went wrong.
#[derive(Default)]
struct Book {
    kept: Vec<Kept>,
    comments: BTreeMap<String, usize>,
    /// The versions each person read of each issue, in the order they read them.
    seen: Vec<(usize, String, i64)>,
    /// Text that was typed, refused and not found as the draft of the person.
    lost: Vec<String>,
    /// How many saves were refused because the lock was lost or taken over, and the text was kept.
    refused: usize,
    /// How many editors closed the laptop and never let go.
    abandoned: usize,
    /// Anything that broke a rule, with the moment.
    broken: Vec<String>,
}

type Shared = Arc<Mutex<Book>>;

fn book(shared: &Shared) -> Result<std::sync::MutexGuard<'_, Book>, Failure> {
    shared.lock().map_err(|_| "the book is poisoned".into())
}

fn what_of(id: &str) -> Value {
    json!({"record": {"kind": "issue", "id": id}, "part": "description"})
}

/// The version of an issue that an answer holds (a change answers with the issue itself).
fn version_in(body: &Value) -> Result<i64, Failure> {
    body["version"]
        .as_i64()
        .or_else(|| body["issue"]["version"].as_i64())
        .ok_or_else(|| format!("no version in {body}").into())
}

/// One person: what they do in each round is fixed by who they are, so a failure repeats.
struct Person {
    number: usize,
    browser: Browser,
    issues: Vec<Work>,
    book: Shared,
}

impl Person {
    fn issue(&self, round: usize) -> &Work {
        &self.issues[(self.number * 5 + round * 7 + self.number * round) % self.issues.len()]
    }

    /// Reads the issue and remembers the version that was read.
    async fn read(&mut self, work: &Work) -> Result<i64, Failure> {
        let reply = self
            .browser
            .send("GET", &format!("{ISSUES}/{}", work.key), None)
            .await?;
        let version = version_in(ok(&reply)?)?;
        book(&self.book)?
            .seen
            .push((self.number, work.key.clone(), version));
        Ok(version)
    }

    /// Changes the title on the version that was read; when somebody was first, says *Use mine*.
    async fn change(&mut self, work: &Work, round: usize) -> Result<(), Failure> {
        let title = format!("Title {}-{round}", self.number);
        let mut version = self.read(work).await?;
        for _ in 0..PEOPLE * 2 {
            let reply = self
                .browser
                .send(
                    "PATCH",
                    &format!("{ISSUES}/{}", work.key),
                    Some(json!({"version": version, "title": title})),
                )
                .await?;
            if reply.status == StatusCode::OK {
                book(&self.book)?.kept.push(Kept {
                    key: work.key.clone(),
                    version: version_in(&reply.body)?,
                    text: None,
                });
                return Ok(());
            }
            if reply.code() != "edit.version_changed" {
                return Err(format!("{}: {}", reply.status, reply.body).into());
            }
            version = reply.body["conflict"]["version"]
                .as_i64()
                .ok_or("a conflict without the version")?;
        }
        Err("a change did not land in twenty tries".into())
    }

    /// Writes a comment.
    async fn comment(&mut self, work: &Work, round: usize) -> Result<(), Failure> {
        let reply = self
            .browser
            .send(
                "POST",
                &format!("{ISSUES}/{}/comments", work.key),
                Some(json!({"text": format!("Comment {}-{round}", self.number)})),
            )
            .await?;
        ok(&reply)?;
        *book(&self.book)?
            .comments
            .entry(work.key.clone())
            .or_default() += 1;
        Ok(())
    }

    /// Asks for the lock; when somebody else holds it and this person may, takes it over.
    async fn lock(&mut self, work: &Work) -> Result<Option<String>, Failure> {
        let editor = format!("00000000-0000-4000-8000-{:012}", self.number);
        let body = json!({"what": what_of(&work.id), "editor_id": editor});
        let taken = self.browser.send("POST", LOCKS, Some(body.clone())).await?;
        if ok(&taken)?["status"] == "taken" {
            return Ok(token_of(&taken.body));
        }
        if taken.body["can_edit_anyway"] != true {
            return Ok(None);
        }
        let over = self
            .browser
            .send("POST", &format!("{LOCKS}/edit-anyway"), Some(body))
            .await?;
        Ok(token_of(ok(&over)?))
    }

    /// Edits the description under the lock. A person whose time ran out or who was taken over
    /// keeps their text as a draft: that is looked for at once.
    async fn describe(&mut self, work: &Work, round: usize) -> Result<(), Failure> {
        let Some(token) = self.lock(work).await? else {
            return Ok(());
        };
        let text = format!("Text {}-{round} on {}", self.number, work.key);
        let version = self.read(work).await?;
        let beat = json!({"what": what_of(&work.id), "lock_token": token,
            "draft": {"text": text, "base_version": version}});
        let beat = self
            .browser
            .send("POST", &format!("{LOCKS}/heartbeat"), Some(beat))
            .await?;
        ok(&beat)?;
        // Every fifth editor closes the laptop and never lets go: the lease has to end it.
        if (self.number + round) % 5 == 0 {
            book(&self.book)?.abandoned += 1;
            return Ok(());
        }
        // Every third one types for longer than the lease lasts.
        if (self.number + round) % 3 == 0 {
            tokio::time::sleep(SLOW_TYPIST).await;
        }
        let saved = self
            .browser
            .send(
                "PUT",
                &format!("{ISSUES}/{}/description", work.key),
                Some(json!({"text": text, "version": version, "lock_token": token})),
            )
            .await?;
        if saved.status == StatusCode::OK {
            book(&self.book)?.kept.push(Kept {
                key: work.key.clone(),
                version: version_in(&saved.body)?,
                text: Some(text),
            });
            return Ok(());
        }
        if !matches!(
            saved.code(),
            "edit.lock_lost" | "edit.taken_over" | "edit.version_changed"
        ) {
            return Err(format!("{}: {}", saved.status, saved.body).into());
        }
        book(&self.book)?.refused += 1;
        self.expect_draft(work, &text).await
    }

    /// The text that a refused save was sent with is the draft of the person.
    async fn expect_draft(&mut self, work: &Work, text: &str) -> Result<(), Failure> {
        let draft = self
            .browser
            .send(
                "GET",
                &format!("/api/v1/drafts?kind=issue&id={}&part=description", work.id),
                None,
            )
            .await?;
        if ok(&draft)?["draft"]["text"] != *text {
            book(&self.book)?.lost.push(text.to_owned());
        }
        Ok(())
    }

    async fn rounds(mut self) -> Result<(), Failure> {
        for round in 0..ROUNDS {
            let work = self.issue(round).clone();
            match (self.number + round) % 4 {
                0 => self.change(&work, round).await?,
                1 | 2 => self.describe(&work, round).await?,
                _ => {
                    self.comment(&work, round).await?;
                    self.read(&work).await?;
                }
            }
        }
        Ok(())
    }
}

fn token_of(body: &Value) -> Option<String> {
    body["lock_token"].as_str().map(str::to_owned)
}

/// The people, each with a browser, and the issues they share.
async fn the_crowd(
    site: &Site<'_>,
    admin: &mut Browser,
    shared: &Shared,
) -> Result<(Vec<Person>, Vec<Work>), Failure> {
    let mut issues = Vec::new();
    for at in 0..ISSUE_COUNT {
        let made = site.make(admin, &format!("Shared issue {at}")).await?;
        let body = ok(&made)?;
        issues.push(Work {
            id: body["id"].as_str().ok_or("no id")?.to_owned(),
            key: body["key"].as_str().ok_or("no key")?.to_owned(),
        });
    }
    let mut people = Vec::new();
    for number in 0..PEOPLE {
        let (browser, _) = site
            .person(StartingRole::TeamMember, &format!("crowd{number}"))
            .await?;
        people.push(Person {
            number,
            browser,
            issues: issues.clone(),
            book: Arc::clone(shared),
        });
    }
    Ok((people, issues))
}

/// The lock of an issue as the database holds it.
async fn stored_lock(rig: &Rig, work: &Work) -> Result<Option<bb_core::edit::EditLock>, Failure> {
    let what = EditWhat {
        record: EntityRef {
            kind: RecordKind::Issue.into(),
            id: work
                .id
                .parse::<EntityId>()
                .map_err(|error| error.to_string())?,
        },
        part: TechnicalName::new("description")?,
    };
    Ok(bb_app::editing::EditStore::edit_lock(&rig.storage, what).await?)
}

/// While the people work, the clock runs on and no lock is allowed to be further away than its lease.
async fn run_the_clock(rig: &Rig, issues: &[Work], shared: &Shared) -> Result<(), Failure> {
    for _ in 0..60 {
        tokio::time::sleep(TICK).await;
        rig.clock.advance(SECONDS_A_TICK);
        for work in issues {
            if let Some(lock) = stored_lock(rig, work).await? {
                let furthest = rig.clock_now().as_second() + LEASE_SECONDS;
                if lock.expires_at.as_second() > furthest {
                    book(shared)?
                        .broken
                        .push(format!("the lock on {} outlives its lease", work.key));
                }
            }
        }
    }
    Ok(())
}

/// All the people work until each has done their rounds, and the clock runs beside them.
async fn work_together(
    rig: &Rig,
    people: Vec<Person>,
    issues: &[Work],
    shared: &Shared,
) -> Result<(), Failure> {
    let mut working = JoinSet::new();
    for person in people {
        working.spawn(person.rounds());
    }
    run_the_clock(rig, issues, shared).await?;
    while let Some(done) = working.join_next().await {
        done??;
    }
    Ok(())
}

/// The distinct events of one kind in the history of an issue.
async fn events_of(admin: &mut Browser, work: &Work, tab: &str) -> Result<usize, Failure> {
    let mut events = std::collections::BTreeSet::new();
    let mut cursor = String::new();
    loop {
        let page = admin
            .send(
                "GET",
                &format!("{ISSUES}/{}/history?tab={tab}&limit=100{cursor}", work.key),
                None,
            )
            .await?;
        let body = ok(&page)?;
        for item in body["items"].as_array().ok_or("no items")? {
            events.insert(item["event"].as_str().ok_or("no event")?.to_owned());
        }
        match body["next_cursor"].as_str() {
            Some(next) => cursor = format!("&cursor={next}"),
            None => return Ok(events.len()),
        }
    }
}

/// Every change has an event, and versions only go up by one at a time.
async fn check_events_and_versions(
    admin: &mut Browser,
    issues: &[Work],
    shared: &Shared,
) -> Result<(), Failure> {
    for work in issues {
        let read = admin
            .send("GET", &format!("{ISSUES}/{}", work.key), None)
            .await?;
        let now = version_in(ok(&read)?)?;
        let book = book(shared)?;
        let mut versions: Vec<i64> = book
            .kept
            .iter()
            .filter(|each| each.key == work.key)
            .map(|each| each.version)
            .collect();
        versions.sort_unstable();
        let expected: Vec<i64> = (2..=now).collect();
        assert_eq!(
            versions, expected,
            "{}: versions go up one by one",
            work.key
        );
        let comments = book.comments.get(&work.key).copied().unwrap_or_default();
        drop(book);
        // The first event is the one that made the issue.
        assert_eq!(
            events_of(admin, work, "changes").await?,
            versions.len() + 1,
            "{}: one event for every change",
            work.key
        );
        assert_eq!(
            events_of(admin, work, "comments").await?,
            comments,
            "{}: one event for every comment",
            work.key
        );
    }
    Ok(())
}

/// What each person read only goes up; the description is the last text that was kept.
fn check_reads_and_texts(book: &Book, issues: &[Work]) {
    let mut last: BTreeMap<(usize, &str), i64> = BTreeMap::new();
    for (person, key, version) in &book.seen {
        let before = last.insert((*person, key.as_str()), *version);
        assert!(
            before.is_none_or(|before| before <= *version),
            "person {person} read {key} at {version} after {before:?}"
        );
    }
    assert!(book.lost.is_empty(), "text that was lost: {:?}", book.lost);
    assert!(book.broken.is_empty(), "{:?}", book.broken);
    assert_eq!(issues.len(), ISSUE_COUNT);
    // The run did what it is for: texts were kept, some were refused and kept as drafts, some
    // editors vanished.
    assert!(book.kept.iter().any(|each| each.text.is_some()));
    assert!(
        book.refused > 0,
        "no save was refused, so no draft was looked for"
    );
    assert!(book.abandoned > 0);
}

/// The description of an issue is the text of the last save that was kept.
async fn check_descriptions(
    admin: &mut Browser,
    issues: &[Work],
    shared: &Shared,
) -> Result<(), Failure> {
    for work in issues {
        let read = admin
            .send("GET", &format!("{ISSUES}/{}", work.key), None)
            .await?;
        let last = book(shared)?
            .kept
            .iter()
            .filter(|each| each.key == work.key && each.text.is_some())
            .max_by_key(|each| each.version)
            .and_then(|each| each.text.clone())
            .unwrap_or_default();
        assert_eq!(
            ok(&read)?["issue"]["description"],
            last,
            "{}: the description is the text that was kept last",
            work.key
        );
    }
    Ok(())
}

/// Once the leases are over nobody holds a lock, and what is left of the rows goes with the purge.
async fn check_locks_are_gone(rig: &Rig, app: &Router, issues: &[Work]) -> Result<(), Failure> {
    rig.clock.advance(LEASE_SECONDS + 1);
    let mut asker = Browser::signed_in_as(app, "crowd0@example.test").await?;
    for work in issues {
        let reply = asker
            .send(
                "GET",
                &format!("{LOCKS}?kind=issue&id={}&part=description", work.id),
                None,
            )
            .await?;
        assert!(
            ok(&reply)?["held"].is_null(),
            "{}: a lock outlived its lease",
            work.key
        );
    }
    let ctx = rig.services.ctx(Rig::info());
    bb_app::editing::purge_drafts(&ctx, bb_app::editing::PurgeDraftsQuery).await?;
    for work in issues {
        assert!(
            stored_lock(rig, work).await?.is_none(),
            "{}: the row of the lock is gone",
            work.key
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_invariants_hold_after_a_run() -> TestResult {
    each_database("test_concurrent_invariants", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut admin) = Site::start(&rig, &app).await?;
        let shared: Shared = Arc::default();
        let (people, issues) = the_crowd(&site, &mut admin, &shared).await?;
        work_together(&rig, people, &issues, &shared).await?;
        check_events_and_versions(&mut admin, &issues, &shared).await?;
        let guard = book(&shared)?;
        check_reads_and_texts(&guard, &issues);
        drop(guard);
        check_descriptions(&mut admin, &issues, &shared).await?;
        check_locks_are_gone(&rig, &app, &issues).await
    })
    .await
}
