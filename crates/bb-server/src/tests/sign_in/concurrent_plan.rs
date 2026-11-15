//! Ten people planning at once, through the real wiring, on both databases (TEST-CONCURRENT,
//! architecture §11 "Ten people plan at once", PLAN-01, PLAN-11, PLAN-13, ADR-0043).
//!
//! Ten Team members move bars of five issues at the same moments, two people each. A person accepts
//! a warning while another moves its bar. A person enters an away time while the plan is read. A
//! *Move it after* is applied after somebody changed an issue it moves, and once more on a fresh
//! preview. The people write the book of what they did; what must hold afterwards is not defined
//! here but in `bb-load-check`, as for the other runs of ten people.

use std::sync::{Arc, Mutex};

use bb_app::access::StartingRole;
use bb_load_check::{
    Accepted, Afterwards, AwayEntered, Book, Kept, Moved, MovedAfter, NamedByMove, Work, check,
};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};
use tokio::task::JoinSet;

use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, ok};
use super::plan_site::{
    MONDAY, TIMELINE, ask, blocks, change, id_of_key, issue, overlapping, page, planned,
    warnings_on,
};

/// Ten people, and the bars they move.
const PEOPLE: usize = 10;
const BARS: usize = 5;
/// The start and the due date each person moves a bar to.
const DAYS: [(&str, &str); PEOPLE] = [
    ("2027-03-02", "2027-03-04"),
    ("2027-03-03", "2027-03-05"),
    ("2027-03-04", "2027-03-06"),
    ("2027-03-05", "2027-03-07"),
    ("2027-03-02", "2027-03-06"),
    ("2027-03-09", "2027-03-11"),
    ("2027-03-10", "2027-03-12"),
    ("2027-03-11", "2027-03-13"),
    ("2027-03-12", "2027-03-14"),
    ("2027-03-15", "2027-03-17"),
];
const MOVE_DATES: &str = "/api/v1/issues/move-dates";

type Shared = Arc<Mutex<Book>>;

fn book(shared: &Shared) -> Result<std::sync::MutexGuard<'_, Book>, Failure> {
    shared.lock().map_err(|_| "the book is poisoned".into())
}

/// Puts the issue with this readable ID in the book, with the changes it has had until now (a
/// relation counts a version), which nobody races for.
async fn register(ada: &mut Browser, shared: &Shared, key: &str) -> Result<(), Failure> {
    let work = Work {
        id: id_of_key(ada, key).await?,
        key: key.to_owned(),
        reporter: None,
    };
    let version = issue(ada, key).await?["version"]
        .as_i64()
        .ok_or("no version")?;
    let mut book = book(shared)?;
    book.issues.push(work);
    book.kept.extend((2..=version).map(|version| Kept {
        key: key.to_owned(),
        version,
        ..Kept::default()
    }));
    Ok(())
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

/// Moves the bar of an issue so it starts on `day` and ends two days later, on the version it has;
/// a move that came second is sent again on the newer version (*Use mine*). Writes what the
/// server kept in the book and says how many sends were refused as stale.
async fn move_bar(
    browser: &mut Browser,
    shared: &Shared,
    key: &str,
    (start, due): (&str, &str),
) -> Result<usize, Failure> {
    let mut refused = 0;
    loop {
        let reply = change(browser, key, json!({"start_date": start, "due_date": due})).await?;
        if reply.status.is_success() {
            let version = reply.body["version"].as_i64().ok_or("no version")?;
            let mut book = book(shared)?;
            book.kept.push(Kept {
                key: key.to_owned(),
                version,
                ..Kept::default()
            });
            book.moved.push(Moved {
                key: key.to_owned(),
                version,
                start: Some(start.to_owned()),
                due: Some(due.to_owned()),
            });
            return Ok(refused);
        }
        assert_eq!(reply.code(), "edit.version-changed", "{}", reply.body);
        refused += 1;
    }
}

/// Ten people move the bars of five issues at the same moments.
async fn ten_people_move_bars(
    site: &Site<'_>,
    ada: &mut Browser,
    shared: &Shared,
    people: Vec<Browser>,
) -> Result<Vec<Browser>, Failure> {
    let mut keys = Vec::new();
    for at in 0..BARS {
        let key = planned(
            site,
            ada,
            &format!("Bar {at}"),
            json!({"start_date": MONDAY, "due_date": "2027-03-03"}),
        )
        .await?;
        register(ada, shared, &key).await?;
        keys.push(key);
    }
    let mut moving = JoinSet::new();
    for (number, mut browser) in people.into_iter().enumerate() {
        let key = keys[number % BARS].clone();
        let shared = Arc::clone(shared);
        moving.spawn(async move {
            move_bar(&mut browser, &shared, &key, DAYS[number]).await?;
            Ok::<(usize, Browser), Failure>((number, browser))
        });
    }
    let mut back = Vec::new();
    while let Some(done) = moving.join_next().await {
        back.push(done??);
    }
    back.sort_by_key(|(number, _)| *number);
    Ok(back.into_iter().map(|(_, browser)| browser).collect())
}

/// A person accepts a warning while another moves its bar.
async fn accept_while_the_bar_moves(
    site: &Site<'_>,
    ada: &mut Browser,
    shared: &Shared,
    (mut accepting, mut moving): (Browser, Browser),
) -> Result<String, Failure> {
    let (first, then) = overlapping(site, ada).await?;
    blocks(ada, &first, &then).await?;
    for key in [&first, &then] {
        register(ada, shared, key).await?;
    }
    let seen = issue(&mut accepting, &then).await?;
    let (start, due) = (text(&seen["start_date"]), text(&seen["due_date"]));
    let shared_move = Arc::clone(shared);
    let key = then.clone();
    let mover = tokio::spawn(async move {
        move_bar(
            &mut moving,
            &shared_move,
            &key,
            ("2027-03-02", "2027-03-06"),
        )
        .await
    });
    let accepted = accepting
        .send(
            "POST",
            &format!("{TIMELINE}/accepted-warnings"),
            Some(json!({
                "issue": then,
                "version": seen["version"],
                "kind": "delay",
                "timeline": {"zoom": "week"},
            })),
        )
        .await?;
    mover.await??;
    if accepted.status.is_success() {
        book(shared)?.accepted.push(Accepted {
            key: then.clone(),
            start,
            due,
        });
    } else {
        assert_eq!(accepted.code(), "edit.version-changed", "{}", accepted.body);
    }
    Ok(then)
}

/// A person enters an away time while the plan is read; the first answer after it shows the days.
async fn away_while_the_plan_is_read(
    site: &Site<'_>,
    ada: &mut Browser,
    shared: &Shared,
    (mut away, mut reader): (Browser, Browser),
    person: bb_core::id::UserId,
) -> Result<(), Failure> {
    let key = planned(
        site,
        ada,
        "Work of the away person",
        json!({
            "start_date": "2027-03-22",
            "due_date": "2027-03-26",
            "assigned_to": [person.to_string()],
        }),
    )
    .await?;
    register(ada, shared, &key).await?;
    let id = id_of_key(ada, &key).await?;
    let body = ask(site, "2027-03-22", "2027-03-31", json!({}));
    let polling = tokio::spawn({
        let body = body.clone();
        async move {
            for _ in 0..5 {
                page(&mut reader, body.clone()).await?;
            }
            Ok::<(), Failure>(())
        }
    });
    let entered = away
        .send(
            "POST",
            "/api/v1/me/away-times",
            Some(json!({"first": "2027-03-23", "last": "2027-03-24"})),
        )
        .await?;
    ok(&entered)?;
    let next = page(ada, body).await?;
    polling.await??;
    let shown = warnings_on(&next, &id)
        .iter()
        .any(|warning| warning["kind"] == "away");
    book(shared)?.away.push(AwayEntered {
        person: person.to_string(),
        first: "2027-03-23".to_owned(),
        last: "2027-03-24".to_owned(),
        shown_on_next_read: shown,
    });
    Ok(())
}

/// The issues a preview of *Move it after* names, with the versions it read.
fn named_by(preview: &Value) -> Vec<(String, Value)> {
    preview["issues"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(|each| {
                    (
                        each["readable_id"].as_str().unwrap_or_default().to_owned(),
                        each.clone(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Applies a *Move it after* of the issue `then` and writes what became of it in the book.
async fn apply_move_after(
    ada: &mut Browser,
    shared: &Shared,
    then: &str,
    changed: &str,
) -> Result<bool, Failure> {
    let then_id = id_of_key(ada, then).await?;
    let found = page(ada, ask_all()).await?;
    let after = warnings_on(&found, &then_id)
        .first()
        .map(|warning| warning["move_after"].clone())
        .ok_or("the issue has no warning to move after")?;
    let preview = ada
        .send(
            "POST",
            &format!("{TIMELINE}/move-after"),
            Some(json!({"issue": then, "after": after, "timeline": {"zoom": "week"}})),
        )
        .await?;
    let named = named_by(ok(&preview)?);
    finish_move_after(ada, shared, &named, changed).await
}

fn ask_all() -> Value {
    json!({
        "search_text": "",
        "timeline": {"zoom": "week"},
        "first": MONDAY,
        "last": "2027-04-30",
    })
}

/// Sends the move and writes the book.
async fn finish_move_after(
    ada: &mut Browser,
    shared: &Shared,
    named: &[(String, Value)],
    changed: &str,
) -> Result<bool, Failure> {
    let issues: Vec<Value> = named
        .iter()
        .map(|(_, each)| {
            json!({
                "issue": each["id"],
                "version": each["version"],
                "start_date": each["start_date"],
                "due_date": each["due_date"],
            })
        })
        .collect();
    let reply = ada
        .send("POST", MOVE_DATES, Some(json!({"issues": issues})))
        .await?;
    let kept = reply.status.is_success();
    let mut book = book(shared)?;
    book.moved_after.push(MovedAfter {
        named: named
            .iter()
            .map(|(key, each)| NamedByMove {
                key: key.clone(),
                version: each["version"].as_i64().unwrap_or_default(),
            })
            .collect(),
        changed: changed.to_owned(),
        kept,
    });
    if kept {
        for (key, each) in named {
            let version = each["version"].as_i64().unwrap_or_default() + 1;
            book.kept.push(Kept {
                key: key.clone(),
                version,
                ..Kept::default()
            });
            book.moved.push(Moved {
                key: key.clone(),
                version,
                start: text(&each["start_date"]),
                due: text(&each["due_date"]),
            });
        }
    } else {
        assert_eq!(reply.code(), "edit.version-changed", "{}", reply.body);
    }
    Ok(kept)
}

/// A *Move it after* is refused whole after somebody changed an issue it moves, and kept whole on a
/// fresh preview.
async fn move_after_while_an_issue_changes(
    site: &Site<'_>,
    ada: &mut Browser,
    shared: &Shared,
    mut changing: Browser,
) -> Result<(), Failure> {
    let (first, then) = overlapping(site, ada).await?;
    let after = planned(
        site,
        ada,
        "Tests of the move",
        json!({"start_date": "2027-03-06", "due_date": "2027-03-06"}),
    )
    .await?;
    blocks(ada, &first, &then).await?;
    blocks(ada, &then, &after).await?;
    for key in [&first, &then, &after] {
        register(ada, shared, key).await?;
    }
    // The preview is read, then somebody changes the last issue it moves, then the move is sent.
    let then_id = id_of_key(ada, &then).await?;
    let found = page(ada, ask_all()).await?;
    let move_after = warnings_on(&found, &then_id)
        .first()
        .map(|warning| warning["move_after"].clone())
        .ok_or("no warning to move after")?;
    let preview = ada
        .send(
            "POST",
            &format!("{TIMELINE}/move-after"),
            Some(json!({"issue": then, "after": move_after, "timeline": {"zoom": "week"}})),
        )
        .await?;
    let named = named_by(ok(&preview)?);
    move_bar(&mut changing, shared, &after, ("2027-03-12", "2027-03-13")).await?;
    let kept = finish_move_after(ada, shared, &named, &after).await?;
    assert!(!kept, "a move after a changed issue must be refused");
    // On a fresh preview it goes through, whole.
    assert!(apply_move_after(ada, shared, &then, &after).await?);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn plan_03_ten_people_plan_at_the_same_moments_and_the_rules_hold() -> TestResult {
    each_database("plan_03_ten_people_plan", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // The clock of the test moves by leases while the people work: the sessions must not run out.
        rig.set_organization_setting("security.session-idle-minutes", 1440)
            .await?;
        rig.set_organization_setting("security.session-absolute-hours", 720)
            .await?;
        let shared: Shared = Arc::default();
        let mut people = Vec::new();
        let mut ids = Vec::new();
        for number in 0..PEOPLE {
            let (browser, id) = site
                .person(StartingRole::TeamMember, &format!("planner{number}"))
                .await?;
            people.push(browser);
            ids.push(id);
        }
        // All ten move bars; then four of them take the other cases, one each.
        let mut people = ten_people_move_bars(&site, &mut ada, &shared, people).await?;
        let reader = people.pop().ok_or("no reader")?;
        let changing = people.pop().ok_or("nobody changes")?;
        let moving = people.pop().ok_or("nobody moves")?;
        let accepting = people.pop().ok_or("nobody accepts")?;
        let away = people.pop().ok_or("nobody is away")?;
        let then =
            accept_while_the_bar_moves(&site, &mut ada, &shared, (accepting, moving)).await?;
        away_while_the_plan_is_read(&site, &mut ada, &shared, (away, reader), ids[PEOPLE - 5])
            .await?;
        move_after_while_an_issue_changes(&site, &mut ada, &shared, changing).await?;
        let book = std::mem::take(&mut *book(&shared)?);
        let afterwards = Afterwards {
            now: rig.clock_now().as_second(),
            lease_seconds: bb_core::edit::LEASE_SECONDS,
            purged: false,
        };
        let found = check(&rig.database, &book, &afterwards).await?;
        assert!(found.is_empty(), "{found:#?}");
        // The rules were put to the test, and the warning accepted while its bar moved is back.
        assert!(book.moved.len() > BARS, "{} moves", book.moved.len());
        assert_eq!(book.away.len(), 1);
        assert_eq!(book.moved_after.len(), 2);
        let back = page(&mut ada, ask_all()).await?;
        let then_id = id_of_key(&mut ada, &then).await?;
        assert_eq!(warnings_on(&back, &then_id)[0]["accepted"], Value::Null);
        Ok(())
    })
    .await
}
