//! Ten people working at once, through the real wiring and at the speed of the API (TEST-CONCURRENT,
//! architecture §11, ADR-0019), on both databases.
//!
//! Ten people, each with a browser of their own, change the same few issues together: short values,
//! descriptions under their locks, comments and reads, status changes under a workflow that asks for
//! who does the work, files, and notifications. One of the ten is a Guest, who works on the issue they
//! reported and hears of nothing else. Some editors vanish without letting go, and
//! some type for longer than the lease lasts. The clock of the test moves with the work, not with
//! the wall clock: every request a person sends moves it on by a second, and one who types for long
//! moves it past the lease, so leases really run out, and only through what the people do, however
//! fast or slow the computer is. Three cases are played out first, alone and by construction (an
//! editor who vanishes, one who is too slow, one who takes long by the wall clock and saves in
//! time), so that what the run is for never depends on how fast the computer is. The people write a book of what they did; what must hold
//! afterwards is not defined here but in `bb-load-check`, which `cargo xtask load` calls with the
//! book of its own run.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_core::edit::{DRAFT_KEPT_BY, LEASE_SECONDS};
use bb_load_check::{
    Afterwards, Book, Dayed, DraftEnd, Drafted, Kept, Seen, Work, check, lock_rows,
    locks_beyond_lease,
};

mod work;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};
use tokio::task::JoinSet;

use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::rig::{Rig, TestClock};

const LOCKS: &str = "/api/v1/edit-locks";
/// Ten people: nine Team members and a Guest.
const PEOPLE: usize = 10;
const ISSUE_COUNT: usize = 4;
const ROUNDS: usize = 14;
/// How many seconds of the clock of the test pass with every request a person sends: a person's
/// own save under the lock takes a few requests, far less than the lease, while the requests of
/// the whole crowd move the clock past the lease of an editor who vanished.
const SECONDS_A_STEP: i64 = 1;
/// How often the run looks at the locks while the people work (the wall clock only paces the looks).
const TICK: Duration = Duration::from_millis(15);
/// How long the editor who takes their time waits by the wall clock before saving: more than the
/// old run gave a whole lease (its clock moved 7 seconds every 15 ms, a lease in about 130 ms).
const WALL_WHILE: Duration = Duration::from_secs(1);

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
    /// Types for longer than the lease lasts: moves the clock past the lease.
    TypesForLong,
    /// Takes long by the wall clock but sends nothing meanwhile, so the clock of the app does not
    /// move and the save is in time.
    TakesItsTime,
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
    /// The date and the version of the person's day, once they started it.
    day: Option<(String, i64)>,
    /// How many taps of the day were made, which tap comes next.
    taps: usize,
}

impl Person {
    /// Sends one request as this person; the clock of the test moves on with it.
    async fn send(
        &mut self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Reply, Failure> {
        let reply = self.browser.send(method, path, body).await;
        self.clock.advance(SECONDS_A_STEP);
        reply
    }

    fn issue(&self, round: usize) -> &Work {
        &self.issues[(self.number * 5 + round * 7 + self.number * round) % self.issues.len()]
    }

    /// Reads the issue and remembers the version that was read.
    async fn read(&mut self, work: &Work) -> Result<i64, Failure> {
        let reply = self
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
    /// Starts the person's timer on the issue, or stops the one that runs, while the others do the
    /// same: a stop that saved time is written down, and the database must hold it once.
    async fn time(&mut self, work: &Work) -> Result<(), Failure> {
        let timer = self.send("GET", "/api/v1/me/timer", None).await?;
        let timer = ok(&timer)?.clone();
        if timer.get("running").is_none() {
            let started = self
                .send("POST", "/api/v1/me/timer", Some(json!({"issue": work.key})))
                .await?;
            // A start within a second of the last one waits for the next round.
            return match started.code() {
                "" | "timer.too-fast" => Ok(()),
                other => Err(format!("a start: {other}: {}", started.body).into()),
            };
        }
        let stopped = self
            .send(
                "POST",
                "/api/v1/me/timer/stop",
                Some(json!({"version": timer["version"]})),
            )
            .await?;
        let saved = ok(&stopped)?;
        if let Some(key) = saved["entry"]["readable_id"].as_str() {
            book(&self.book)?.timed.push(key.to_owned());
        }
        Ok(())
    }

    /// Taps the person's day while the others work: starts it, then breaks and ends it over and
    /// over. A change the server kept is written down as a version of that day with its event; what
    /// the rules refuse (a change within a second of the last one, a tap the day does not allow) is
    /// not.
    async fn day(&mut self) -> Result<(), Failure> {
        self.taps += 1;
        let reply = match &self.day {
            Some((_, version)) if !self.taps.is_multiple_of(4) => {
                let path = ["break/start", "break/end", "end"][self.taps % 3];
                self.send(
                    "POST",
                    &format!("/api/v1/me/work-days/{path}"),
                    Some(json!({"version": version})),
                )
                .await?
            }
            _ => {
                self.send("POST", "/api/v1/me/work-days/start", Some(json!({})))
                    .await?
            }
        };
        if reply.status != StatusCode::OK {
            // The rules at work: the next tap starts again.
            let code = reply.code();
            if code == "limit.reached"
                || code == "edit.version-changed"
                || code.starts_with("working-hours.")
            {
                self.day = None;
                return Ok(());
            }
            return Err(format!("a tap of the day: {}: {}", reply.status, reply.body).into());
        }
        let date = reply.body["date"].as_str().ok_or("a day without a date")?;
        self.day = Some((date.to_owned(), version_in(&reply.body)?));
        book(&self.book)?.days.push(Dayed {
            person: self.holder.clone(),
            date: date.to_owned(),
        });
        Ok(())
    }

    async fn comment(&mut self, work: &Work, round: usize) -> Result<(), Failure> {
        let reply = self
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
        let body = json!({"target": what_of(&work.id), "editor_id": editor});
        let taken = self.send("POST", LOCKS, Some(body.clone())).await?;
        if ok(&taken)?["kind"] == "taken" {
            return Ok(token_of(&taken.body));
        }
        if taken.body["can_edit_anyway"] != true {
            return Ok(None);
        }
        let over = self
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
        let beat = json!({"target": what_of(&work.id), "lock_token": token,
            "draft": {"text": text, "base_version": version}});
        let beat = self
            .send("POST", &format!("{LOCKS}/renew"), Some(beat))
            .await?;
        ok(&beat)?;
        // Every fifth editor closes the laptop and never lets go: the lease has to end it.
        if behaviour == Behaviour::Vanishes || (by_round && (self.number + round).is_multiple_of(5))
        {
            return self.drafted(work, &text, DraftEnd::Vanished, None);
        }
        // Every third one types for longer than the lease lasts, by the clock of the test: the
        // lease has run out by then, however fast or slow the computer is.
        if behaviour == Behaviour::TypesForLong
            || (by_round && (self.number + round).is_multiple_of(3))
        {
            self.clock.advance(LEASE_SECONDS + 1);
        }
        if behaviour == Behaviour::TakesItsTime {
            tokio::time::sleep(WALL_WHILE).await;
        }
        let saved = self
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
            .send(
                "GET",
                &format!(
                    "/api/v1/drafts?record_kind=issue&record_id={}&part=description",
                    work.id
                ),
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
                    self.time(&work).await?;
                    self.day().await?;
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
            key: body["readable_id"].as_str().ok_or("no key")?.to_owned(),
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
        day: None,
        taps: 0,
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

/// The three cases the run is for, played out alone before the crowd starts: one editor types for
/// longer than the lease lasts and is refused with the text kept as a draft, one vanishes without
/// letting go, and one takes long by the wall clock and still saves, because only the clock of the
/// app counts. They are in the book like everything the crowd does.
async fn the_three_cases(people: &mut [Person]) -> Result<(), Failure> {
    let [slow, vanishing, unhurried, ..] = people else {
        return Err("fewer than three people".into());
    };
    let work = slow.issue(0).clone();
    slow.describe_as(&work, 0, Behaviour::TypesForLong).await?;
    let work = vanishing.issue(1).clone();
    vanishing.describe_as(&work, 0, Behaviour::Vanishes).await?;
    // The issue of the slow editor, whose lease has run out (the vanishing one still holds theirs).
    let work = unhurried.issue(0).clone();
    unhurried
        .describe_as(&work, 0, Behaviour::TakesItsTime)
        .await
}

/// All the people work until each has done their rounds. Their requests move the clock, and no
/// lock is allowed to be further away than its lease.
async fn work_together(rig: &Rig, people: Vec<Person>, shared: &Shared) -> Result<(), Failure> {
    let mut working = JoinSet::new();
    for person in people {
        working.spawn(person.rounds());
    }
    while !working.is_empty() {
        tokio::time::sleep(TICK).await;
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
        // The test is about edit locks, not about sessions: the clock of the test moves by leases
        // while the people work, so a person could wait out the idle time of a session between two
        // requests. The limits are the largest the settings allow,
        // set the way a deployment sets them.
        rig.set_organization_setting("security.session-idle-minutes", 1440)
            .await?;
        rig.set_organization_setting("security.session-absolute-hours", 720)
            .await?;
        let mut people = the_crowd(&rig, &site, &mut admin, &shared).await?;
        the_three_cases(&mut people).await?;
        {
            let alone = book(&shared)?;
            let ended = |ending| alone.drafted.iter().any(|each| each.ending == ending);
            assert!(ended(DraftEnd::Refused), "the slow editor was not refused");
            assert!(
                ended(DraftEnd::Vanished),
                "the vanishing editor did not vanish"
            );
            // Only the clock of the app counts: a second of the computer is no time of a lease.
            assert!(ended(DraftEnd::Saved), "the unhurried editor was refused");
        }
        work_together(&rig, people, &shared).await?;
        // The run did what it is for: texts were kept, some saves were refused because the clock
        // went past their lease and the text was kept as a draft, some editors vanished (the three
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
        assert!(!book.timed.is_empty(), "no stop of a timer saved time");
        assert!(!book.days.is_empty(), "no change of a work day was kept");
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
