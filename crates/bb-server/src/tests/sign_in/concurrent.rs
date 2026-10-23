//! Ten people working at once, through the real wiring and at the speed of the API (TEST-CONCURRENT,
//! architecture §11, ADR-0019), on both databases.
//!
//! Ten people, each with a browser of their own, change the same few issues together: short values,
//! descriptions under their locks, comments and reads, status changes under a workflow that asks for
//! who does the work, files, and notifications. One of the ten is a Guest, who works on the issue they
//! reported and hears of nothing else. Some editors vanish without letting go, and
//! some type for longer than the lease lasts. The clock of the test runs on until the last person
//! is done, so leases really run out. Two cases are played out first, alone and by construction
//! (an editor who vanishes, one who is too slow), so that what the run is for never depends on how
//! fast the computer is. The people write a book of what they did; what must hold
//! afterwards is not defined here but in `bb-load-check`, which `cargo xtask load` calls with the
//! book of its own run.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_app::context::Clock as _;
use bb_core::edit::{DRAFT_KEPT_BY, LEASE_SECONDS};
use bb_load_check::{
    Afterwards, Book, DraftEnd, Drafted, Kept, Seen, Work, check, lock_rows, locks_beyond_lease,
};

mod work;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};
use tokio::task::JoinSet;

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::rig::{Rig, TestClock};

const LOCKS: &str = "/api/v1/edit-locks";
/// Ten people: nine Team members and a Guest.
const PEOPLE: usize = 10;
const ISSUE_COUNT: usize = 4;
const ROUNDS: usize = 14;
/// How many seconds of the clock of the test pass at every tick of the run.
const SECONDS_A_TICK: i64 = 7;
const TICK: Duration = Duration::from_millis(15);

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

/// How an editor behaves under the lock: by who they are and the round (the crowd), or one of the two
/// cases that are played out alone first.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Behaviour {
    /// Fixed by the number of the person and the round.
    ByRound,
    /// Closes the laptop and never lets go.
    Vanishes,
    /// Types for longer than the lease lasts: nobody else moves the clock, so this one does.
    TypesForLongAlone,
}

/// One person: what they do in each round is fixed by who they are, so a failure repeats.
struct Person {
    number: usize,
    /// The ID of the person, as the drafts name them.
    holder: String,
    browser: Browser,
    issues: Vec<Work>,
    book: Shared,
    clock: Arc<TestClock>,
    /// The own field that Guests may not see, which the Team members write.
    hidden_field: String,
    /// A Guest works on their own issue only; the Team members work on all of them.
    guest: bool,
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
        book(&self.book)?.seen.push(Seen {
            person: u32::try_from(self.number)?,
            key: work.key.clone(),
            version,
        });
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
                    title: Some(title),
                    ..Kept::default()
                });
                return Ok(());
            }
            if reply.code() != "edit.version-changed" {
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
        book(&self.book)?.comments.push(work.key.clone());
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

    /// Writes down what became of a text that was sent as a draft.
    fn drafted(
        &self,
        work: &Work,
        text: &str,
        ending: DraftEnd,
        found_as_draft: Option<bool>,
    ) -> Result<(), Failure> {
        book(&self.book)?.drafted.push(Drafted {
            holder: self.holder.clone(),
            key: work.key.clone(),
            text: text.to_owned(),
            ending,
            found_as_draft,
        });
        Ok(())
    }

    /// Edits the description under the lock. A person whose time ran out or who was taken over
    /// keeps their text as a draft: that is looked for at once.
    async fn describe(&mut self, work: &Work, round: usize) -> Result<(), Failure> {
        self.describe_as(work, round, Behaviour::ByRound).await
    }

    /// [`Self::describe`] with the way the editor behaves.
    async fn describe_as(
        &mut self,
        work: &Work,
        round: usize,
        behaviour: Behaviour,
    ) -> Result<(), Failure> {
        let by_round = behaviour == Behaviour::ByRound;
        let Some(token) = self.lock(work).await? else {
            return Ok(());
        };
        let text = format!("Text {}-{round} on {}", self.number, work.key);
        let version = self.read(work).await?;
        let started = self.clock.now().as_second();
        let beat = json!({"what": what_of(&work.id), "lock_token": token,
            "draft": {"text": text, "base_version": version}});
        let beat = self
            .browser
            .send("POST", &format!("{LOCKS}/heartbeat"), Some(beat))
            .await?;
        ok(&beat)?;
        // Every fifth editor closes the laptop and never lets go: the lease has to end it.
        if behaviour == Behaviour::Vanishes || (by_round && (self.number + round).is_multiple_of(5))
        {
            return self.drafted(work, &text, DraftEnd::Vanished, None);
        }
        // Every third one types for longer than the lease lasts, by the clock of the test. The
        // clock runs on as long as anybody works, so the wait ends, and the lease has run out by
        // then, however slow the computer is.
        if behaviour == Behaviour::TypesForLongAlone {
            self.clock.advance(LEASE_SECONDS + 1);
        } else if by_round && (self.number + round).is_multiple_of(3) {
            while self.clock.now().as_second() <= started + LEASE_SECONDS {
                tokio::time::sleep(TICK).await;
            }
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
                text: Some(text.clone()),
                ..Kept::default()
            });
            return self.drafted(work, &text, DraftEnd::Saved, None);
        }
        if !DRAFT_KEPT_BY.iter().any(|code| code.code() == saved.code()) {
            return Err(format!("{}: {}", saved.status, saved.body).into());
        }
        let found = self.is_draft(work, &text).await?;
        self.drafted(work, &text, DraftEnd::Refused, Some(found))
    }

    /// Whether the text is the draft of the person now.
    async fn is_draft(&mut self, work: &Work, text: &str) -> Result<bool, Failure> {
        let draft = self
            .browser
            .send(
                "GET",
                &format!("/api/v1/drafts?kind=issue&id={}&part=description", work.id),
                None,
            )
            .await?;
        Ok(ok(&draft)?["draft"]["text"] == *text)
    }

    async fn rounds(mut self) -> Result<(), Failure> {
        for round in 0..ROUNDS {
            let work = self.issue(round).clone();
            if self.guest {
                self.guest_round(&work, round).await?;
                continue;
            }
            match (self.number + round) % 7 {
                0 => self.change(&work, round).await?,
                1 | 2 => self.describe(&work, round).await?,
                3 => {
                    self.comment(&work, round).await?;
                    self.read(&work).await?;
                }
                4 | 5 => {
                    let hidden = self.hidden_field.clone();
                    self.move_on(&work, round, &hidden).await?;
                }
                _ => {
                    self.add_file(&work, round).await?;
                    self.hear().await?;
                }
            }
        }
        Ok(())
    }

    /// A Guest comments on, adds a file to and reads their own issue, and reads their notifications.
    async fn guest_round(&mut self, work: &Work, round: usize) -> Result<(), Failure> {
        match round % 3 {
            0 => self.comment(work, round).await?,
            1 => self.add_file(work, round).await?,
            _ => {
                self.read(work).await?;
                self.hear().await?;
            }
        }
        Ok(())
    }
}

fn token_of(body: &Value) -> Option<String> {
    body["lock_token"].as_str().map(str::to_owned)
}

/// The people, each with a browser, and the issues they share (which the book names): nine Team
/// members who share the issues, and a Guest who has one issue of their own. The project has a
/// workflow of its own that asks for who does the work, and an own field that Guests may not see.
async fn the_crowd(
    rig: &Rig,
    site: &Site<'_>,
    admin: &mut Browser,
    shared: &Shared,
) -> Result<Vec<Person>, Failure> {
    work::workflow_that_asks(site, admin).await?;
    let hidden_field = work::field_hidden_from_guests(site, rig, admin).await?;
    let mut issues = Vec::new();
    for at in 0..ISSUE_COUNT {
        let made = site.create(admin, &format!("Shared issue {at}")).await?;
        let body = ok(&made)?;
        issues.push(Work {
            id: body["id"].as_str().ok_or("no id")?.to_owned(),
            key: body["key"].as_str().ok_or("no key")?.to_owned(),
            reporter: None,
        });
    }
    let (mut guest, guest_id) = site.person(StartingRole::Guest, "crowdguest").await?;
    let guests_issue = work::guests_issue(site, &mut guest, &guest_id.to_string()).await?;
    let mut everything = issues.clone();
    everything.push(guests_issue.clone());
    book(shared)?.issues.clone_from(&everything);
    work::note_who_is_judged(shared, &guest_id.to_string(), &hidden_field)?;
    let person = |number, holder: String, browser, issues, guest| Person {
        number,
        holder,
        browser,
        issues,
        book: Arc::clone(shared),
        clock: Arc::clone(&rig.clock),
        hidden_field: hidden_field.clone(),
        guest,
    };
    let mut people = Vec::new();
    for number in 0..PEOPLE - 1 {
        let (browser, holder) = site
            .person(StartingRole::TeamMember, &format!("crowd{number}"))
            .await?;
        people.push(person(
            number,
            holder.to_string(),
            browser,
            everything.clone(),
            false,
        ));
    }
    people.push(person(
        PEOPLE - 1,
        guest_id.to_string(),
        guest,
        vec![guests_issue],
        true,
    ));
    Ok(people)
}

/// The two cases the run is for, played out alone before the crowd starts: one editor types for
/// longer than the lease lasts and is refused with the text kept as a draft, and one vanishes
/// without letting go. They are in the book like everything the crowd does.
async fn the_two_cases(people: &mut [Person]) -> Result<(), Failure> {
    let [slow, vanishing, ..] = people else {
        return Err("fewer than two people".into());
    };
    let work = slow.issue(0).clone();
    slow.describe_as(&work, 0, Behaviour::TypesForLongAlone)
        .await?;
    let work = vanishing.issue(1).clone();
    vanishing.describe_as(&work, 0, Behaviour::Vanishes).await
}

/// All the people work until each has done their rounds. The clock runs beside them for as long as
/// any of them is at work, and no lock is allowed to be further away than its lease.
async fn work_together(rig: &Rig, people: Vec<Person>, shared: &Shared) -> Result<(), Failure> {
    let mut working = JoinSet::new();
    for person in people {
        working.spawn(person.rounds());
    }
    while !working.is_empty() {
        tokio::time::sleep(TICK).await;
        rig.clock.advance(SECONDS_A_TICK);
        let rows = lock_rows(&rig.database).await?;
        let now = rig.clock_now().as_second();
        let mut guard = book(shared)?;
        let beyond = locks_beyond_lease(&guard, &rows, now, LEASE_SECONDS);
        guard.broken.extend(beyond);
        drop(guard);
        while let Some(done) = working.try_join_next() {
            done??;
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_invariants_hold_after_a_run() -> TestResult {
    each_database("test_concurrent_invariants", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut admin) = Site::start(&rig, &app).await?;
        let shared: Shared = Arc::default();
        // The test is about edit locks, not about sessions: the clock of the test runs about 470
        // times as fast as the real one, so on a slow computer a person could wait out the idle
        // time of a session between two requests. The limits are the largest the settings allow,
        // set the way a deployment sets them.
        rig.set_organization_setting("security.session-idle-minutes", 1440)
            .await?;
        rig.set_organization_setting("security.session-absolute-hours", 720)
            .await?;
        let mut people = the_crowd(&rig, &site, &mut admin, &shared).await?;
        the_two_cases(&mut people).await?;
        {
            let alone = book(&shared)?;
            let ended = |ending| alone.drafted.iter().any(|each| each.ending == ending);
            assert!(ended(DraftEnd::Refused), "the slow editor was not refused");
            assert!(
                ended(DraftEnd::Vanished),
                "the vanishing editor did not vanish"
            );
        }
        work_together(&rig, people, &shared).await?;
        // The run did what it is for: texts were kept, some saves were refused because the clock
        // went past their lease and the text was kept as a draft, some editors vanished (the two
        // cases played out first are there by construction, the crowd adds more).
        let book = std::mem::take(&mut *book(&shared)?);
        let ended = |ending| book.drafted.iter().any(|each| each.ending == ending);
        assert!(book.kept.iter().any(|each| each.text.is_some()));
        assert!(
            book.kept.iter().any(|each| each.title.is_some()),
            "no title was kept"
        );
        assert!(
            book.kept.iter().any(|each| each.status.is_some()),
            "no status was changed under the workflow"
        );
        assert!(!book.files.is_empty(), "no file was added");
        assert!(ended(DraftEnd::Refused), "no save was refused");
        assert!(ended(DraftEnd::Vanished), "no editor vanished");
        // Once the leases are over nobody holds a lock, and the purge takes the rows.
        rig.clock.advance(LEASE_SECONDS + 1);
        let ctx = rig.services.ctx(Rig::info());
        bb_app::editing::purge_drafts(&ctx, bb_app::editing::PurgeDraftsQuery).await?;
        let afterwards = Afterwards {
            now: rig.clock_now().as_second(),
            lease_seconds: LEASE_SECONDS,
            purged: true,
        };
        let found = check(&rig.database, &book, &afterwards).await?;
        assert!(found.is_empty(), "{found:#?}");
        // The rule about what a Guest is told is not empty: the Guest was told of their issue.
        let guest = book.guests.first().ok_or("no Guest in the book")?.parse()?;
        assert!(
            !rig.database.stored_notifications(guest).await?.is_empty(),
            "the Guest was told of nothing, so the rule was never put to the test"
        );
        Ok(())
    })
    .await
}
