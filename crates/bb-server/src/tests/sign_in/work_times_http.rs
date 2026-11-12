//! Work times over HTTP (PLAN-11, PLAN-15, PRIV-09, ADR-0030): what others see of an away time, who
//! reads another person's work times, who enters them for others, the note that only its person
//! reads, the person told of a change by somebody else, the deletion of old away times and of
//! everything of a person who leaves, and the share of a person in a project. Every flow runs on
//! SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_app::auth::RequestInfo;
use bb_app::work_times::{
    DeleteWorkTimesOfPerson, WorkTimesStore as _, delete_work_times_of_person,
};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{PROJECTS, Site, ok};
use super::working_hours_http::{TODAY, YESTERDAY};

const MINE: &str = "/api/v1/me/work-times";
/// A stretch that holds every date of these tests.
const RANGE: &str = "from=2027-01-01&to=2027-12-31";

fn work_times_of(person: &str) -> String {
    format!("/api/v1/people/{person}/work-times?{RANGE}")
}

/// Seven usual work days: the first `works` ticked, `minutes` each.
fn week(works: usize, minutes: u16) -> Value {
    json!({"days": (0..7).map(|at| json!({"works": at < works, "minutes": minutes})).collect::<Vec<_>>()})
}

/// The days of an away time.
fn away(first: &str, last: &str) -> Value {
    json!({"first": first, "last": last})
}

/// The away time `body` as `browser` enters it for the person with this ID (`None`: their own).
async fn enter(browser: &mut Browser, person: Option<&str>, body: Value) -> Result<Reply, Failure> {
    let path = person.map_or_else(
        || "/api/v1/me/away-times".to_owned(),
        |person| format!("/api/v1/people/{person}/away-times"),
    );
    browser.send("POST", &path, Some(body)).await
}

/// The away times of an answer of work times.
fn away_of(reply: &Reply) -> Vec<Value> {
    reply.body["away"].as_array().cloned().unwrap_or_default()
}

/// The items of the notifications of the browser's person of this kind.
async fn told(browser: &mut Browser, kind: &str) -> Result<Vec<Value>, Failure> {
    let list = browser.send("GET", "/api/v1/notifications", None).await?;
    Ok(ok(&list)?["items"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|item| item["kind"] == kind)
        .collect())
}

#[tokio::test]
async fn plan_11_others_see_only_away() -> TestResult {
    each_database("plan_11_others_see_only_away", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let (mut cleo, _) = site
            .person_called(StartingRole::TeamMember, "cleo", "Cleo Cook")
            .await?;
        let mut body = away("2027-01-25", "2027-01-29");
        body["first_afternoon_only"] = json!(true);
        body["note"] = json!("Doctor on Tuesday");
        let made = enter(&mut bob, None, body).await?;
        ok(&made)?;
        // Bob sees it in full: the note, and who entered it.
        let own = bob.send("GET", &format!("{MINE}?{RANGE}"), None).await?;
        let mine = away_of(&own);
        assert_eq!(mine.len(), 1, "{}", own.body);
        assert_eq!(mine[0]["note"], "Doctor on Tuesday");
        assert_eq!(mine[0]["entered_by"]["name"], "Bob Baker");
        // Cleo sees the days and nothing else: no note, nobody who entered it, no reason.
        let seen = cleo
            .send("GET", &work_times_of(&bob_id.to_string()), None)
            .await?;
        ok(&seen)?;
        let theirs = away_of(&seen);
        assert_eq!(theirs.len(), 1, "{}", seen.body);
        assert_eq!(theirs[0]["first"], "2027-01-25");
        assert_eq!(theirs[0]["last"], "2027-01-29");
        assert_eq!(theirs[0]["first_afternoon_only"], true);
        assert_eq!(theirs[0]["note"], Value::Null);
        assert_eq!(theirs[0]["entered_by"], Value::Null);
        let text = seen.body.to_string();
        assert!(!text.contains("Doctor"), "{text}");
        assert!(!text.to_lowercase().contains("reason"), "{text}");
        assert_eq!(seen.body["own"], false);
        assert_eq!(seen.body["shares"], Value::Null);
        // No answer of work times, in any shape, has a reason.
        assert!(!own.body.to_string().to_lowercase().contains("reason"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_11_work_times_follow_who_sees_people() -> TestResult {
    each_database("plan_11_follow_who_sees_people", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        ok(&enter(&mut bob, None, away("2027-01-25", "2027-01-29")).await?)?;
        let (mut dan, _) = site
            .person_called(StartingRole::TeamMember, "dan", "Dan Day")
            .await?;
        let (mut cleo, _) = site
            .person_called(StartingRole::Contributor, "cleo", "Cleo Cook")
            .await?;
        let (mut gus, _) = site
            .person_called(StartingRole::Guest, "gus", "Gus Guest")
            .await?;
        let path = work_times_of(&bob_id.to_string());
        // Names for a Team member.
        let by_name = dan.send("GET", &path, None).await?;
        assert_eq!(ok(&by_name)?["person"]["shown"], "name");
        assert_eq!(by_name.body["person"]["name"], "Bob Baker");
        // Role only for a Contributor: the days, and the person as their role, with no ID.
        let by_role = cleo.send("GET", &path, None).await?;
        assert_eq!(ok(&by_role)?["person"]["shown"], "role");
        assert_eq!(by_role.body["person"]["role"], "Team member");
        assert!(by_role.body["person"].get("id").is_none());
        assert_eq!(away_of(&by_role).len(), 1);
        // Hidden for a Guest: the answer of a person that does not exist.
        let hidden = gus.send("GET", &path, None).await?;
        assert_eq!(
            (hidden.status, hidden.code()),
            (StatusCode::NOT_FOUND, "resource.not-found")
        );
        let nobody = gus
            .send(
                "GET",
                &work_times_of("01a3185c-5000-73b2-8f37-20757db0778e"),
                None,
            )
            .await?;
        assert_eq!(nobody.status, StatusCode::NOT_FOUND);
        // The choice of the role counts: Names, Role only and Hidden for the Contributor.
        for (level, shown) in [("names", "name"), ("hidden", "")] {
            site.show_people(&mut ada, StartingRole::Contributor, "work-times", level)
                .await?;
            let read = cleo.send("GET", &path, None).await?;
            if shown.is_empty() {
                assert_eq!(read.status, StatusCode::NOT_FOUND, "{level}");
            } else {
                assert_eq!(ok(&read)?["person"]["shown"], shown, "{level}");
            }
        }
        // Everybody reads their own, whatever the choice.
        let own = gus.send("GET", &format!("{MINE}?{RANGE}"), None).await?;
        assert_eq!(ok(&own)?["own"], true);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_11_only_enter_work_times_for_others_changes_someone_elses_work_times() -> TestResult {
    each_database("plan_11_only_permission_changes_others", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let (mut cleo, _) = site
            .person_called(StartingRole::TeamMember, "cleo", "Cleo Cook")
            .await?;
        let (mut hana, _) = site
            .person_called(StartingRole::Hr, "hana", "Hana Hart")
            .await?;
        let bob_id = bob_id.to_string();
        let own_away = ok(&enter(&mut bob, None, away("2027-01-25", "2027-01-29")).await?)?.clone();
        let (away_id, version) = (
            own_away["id"].as_str().unwrap_or_default().to_owned(),
            own_away["version"].clone(),
        );
        let usual = format!("/api/v1/people/{bob_id}/work-times/usual-days");
        // A Team member who sees Bob is forbidden; HR, who is shown nobody, finds no such person.
        for (browser, status) in [
            (&mut cleo, StatusCode::FORBIDDEN),
            (&mut hana, StatusCode::NOT_FOUND),
        ] {
            let put = browser.send("PUT", &usual, Some(week(4, 360))).await?;
            assert_eq!(put.status, status, "{}", put.body);
            let entered = enter(browser, Some(&bob_id), away("2027-02-01", "2027-02-02")).await?;
            assert_eq!(entered.status, status, "{}", entered.body);
            let changed = browser
                .send(
                    "PATCH",
                    &format!("/api/v1/away-times/{away_id}"),
                    Some(json!({"version": version, "days": away("2027-01-26", "2027-01-29")})),
                )
                .await?;
            assert_eq!(changed.status, status, "{}", changed.body);
            let removed = browser
                .send(
                    "DELETE",
                    &format!("/api/v1/away-times/{away_id}?version=1"),
                    None,
                )
                .await?;
            assert_eq!(removed.status, status, "{}", removed.body);
        }
        // Nothing changed.
        let still = bob.send("GET", &format!("{MINE}?{RANGE}"), None).await?;
        assert_eq!(away_of(&still).len(), 1);
        assert_eq!(still.body["usual"][0]["minutes"], 480);
        // An administrator holds the permission at the start and changes all three.
        let put = ada.send("PUT", &usual, Some(week(4, 360))).await?;
        assert_eq!(ok(&put)?["days"][3]["works"], true);
        assert_eq!(put.body["days"][4]["works"], false);
        let entered = enter(&mut ada, Some(&bob_id), away("2027-02-01", "2027-02-02")).await?;
        ok(&entered)?;
        let changed = ada
            .send(
                "PATCH",
                &format!("/api/v1/away-times/{away_id}"),
                Some(json!({"version": version, "days": away("2027-01-26", "2027-01-29")})),
            )
            .await?;
        assert_eq!(ok(&changed)?["first"], "2027-01-26");
        let removed = ada
            .send(
                "DELETE",
                &format!("/api/v1/away-times/{away_id}?version=2"),
                None,
            )
            .await?;
        assert_eq!(removed.status, StatusCode::NO_CONTENT, "{}", removed.body);
        // An old version is a conflict, not an overwrite.
        let stale = ada
            .send(
                "PATCH",
                &format!(
                    "/api/v1/away-times/{}",
                    entered.body["id"].as_str().unwrap_or_default()
                ),
                Some(json!({"version": 7, "days": away("2027-02-01", "2027-02-03")})),
            )
            .await?;
        assert_eq!(stale.code(), "edit.version-changed", "{}", stale.body);
        // Bob sees what was done: the new days, the one away time that is left.
        let after = bob.send("GET", &format!("{MINE}?{RANGE}"), None).await?;
        assert_eq!(after.body["usual"][4]["works"], false);
        assert_eq!(away_of(&after).len(), 1, "{}", after.body);
        assert_eq!(away_of(&after)[0]["first"], "2027-02-01");
        // The days of one away time must be days: a last day before the first is refused.
        let backwards = enter(&mut bob, None, away("2027-03-05", "2027-03-01")).await?;
        assert_eq!(backwards.code(), "work-times.away-time-invalid");
        let too_long = enter(&mut bob, None, away("2027-03-01", "2028-06-01")).await?;
        assert_eq!(too_long.code(), "work-times.away-time-invalid");
        let long_day = bob
            .send("PUT", &format!("{MINE}/usual-days"), Some(week(5, 1441)))
            .await?;
        assert_eq!(
            long_day.code(),
            "work-times.hours-invalid",
            "{}",
            long_day.body
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_11_nobody_but_its_person_reads_or_writes_the_note_of_an_away_time() -> TestResult {
    each_database("plan_11_note_is_private", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let bob_id = bob_id.to_string();
        // Somebody who enters work times for Bob cannot write a note.
        let mut with_note = away("2027-02-01", "2027-02-03");
        with_note["note"] = json!("Surgery");
        let refused = enter(&mut ada, Some(&bob_id), with_note).await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        let entered =
            ok(&enter(&mut ada, Some(&bob_id), away("2027-02-01", "2027-02-03")).await?)?.clone();
        // The answer to the one who entered it has no note, though Bob may add one.
        assert_eq!(entered["note"], Value::Null);
        let id = entered["id"].as_str().unwrap_or_default().to_owned();
        let path = format!("/api/v1/away-times/{id}");
        let mut days = away("2027-02-01", "2027-02-03");
        days["note"] = json!("Surgery, ask me on my phone");
        let change = json!({"version": entered["version"], "days": days});
        let by_bob = bob.send("PATCH", &path, Some(change)).await?;
        assert_eq!(ok(&by_bob)?["note"], "Surgery, ask me on my phone");
        assert_eq!(by_bob.body["entered_by"]["name"], "Ada Admin");
        // Ada changes the days of Bob's away time: the note stays as it is, and she cannot write it.
        let days = away("2027-02-01", "2027-02-04");
        let change = json!({"version": by_bob.body["version"], "days": days});
        let changed = ada.send("PATCH", &path, Some(change)).await?;
        assert_eq!(ok(&changed)?["note"], Value::Null);
        assert!(!changed.body.to_string().contains("Surgery"));
        let mut days = days;
        days["note"] = json!("Overwritten");
        let change = json!({"version": changed.body["version"], "days": days});
        let overwritten = ada.send("PATCH", &path, Some(change)).await?;
        assert_eq!(overwritten.status, StatusCode::FORBIDDEN);
        let back = bob.send("GET", &format!("{MINE}?{RANGE}"), None).await?;
        assert_eq!(away_of(&back)[0]["note"], "Surgery, ask me on my phone");
        assert_eq!(away_of(&back)[0]["last"], "2027-02-04");
        // Ada reads Bob's work times: no note anywhere.
        let read = ada.send("GET", &work_times_of(&bob_id), None).await?;
        assert!(!ok(&read)?.to_string().contains("Surgery"));
        // The notification Bob got names the days and never the note.
        let notes = told(&mut bob, "work-times-changed").await?;
        assert!(!notes.is_empty());
        assert!(
            notes
                .iter()
                .all(|item| !item.to_string().contains("Surgery")),
            "{notes:?}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_11_the_person_is_told_when_someone_else_enters_an_away_time() -> TestResult {
    each_database("plan_11_person_is_told", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let bob_id = bob_id.to_string();
        // Bob's own entry tells nobody.
        ok(&enter(&mut bob, None, away("2027-03-01", "2027-03-02")).await?)?;
        assert!(told(&mut bob, "work-times-changed").await?.is_empty());
        // Ada enters one: Bob is told, with the days and who did it.
        let mut body = away("2027-01-25", "2027-01-29");
        body["first_afternoon_only"] = json!(true);
        ok(&enter(&mut ada, Some(&bob_id), body).await?)?;
        let notes = told(&mut bob, "work-times-changed").await?;
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert_eq!(notes[0]["work_times_event"], "away-entered");
        assert_eq!(notes[0]["work_times_first"], "2027-01-25");
        assert_eq!(notes[0]["work_times_last"], "2027-01-29");
        assert_eq!(notes[0]["by"]["name"], "Ada Admin");
        // The usual work days and the work hours too.
        ok(&ada
            .send(
                "PUT",
                &format!("/api/v1/people/{bob_id}/work-times/usual-days"),
                Some(week(4, 360)),
            )
            .await?)?;
        let notes = told(&mut bob, "work-times-changed").await?;
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(
            notes
                .iter()
                .any(|item| item["work_times_event"] == "usual-changed")
        );
        // Nobody else hears of it.
        assert!(told(&mut ada, "work-times-changed").await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_11_an_away_time_is_deleted_one_year_after_its_last_day() -> TestResult {
    each_database("plan_11_old_away_times_deleted", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let bob_id = bob_id.to_string();
        let mut old = away("2027-01-04", "2027-01-06");
        old["note"] = json!("Old note");
        ok(&enter(&mut bob, None, old).await?)?;
        ok(&enter(&mut ada, Some(&bob_id), away("2027-01-11", "2027-01-12")).await?)?;
        ok(&enter(&mut bob, None, away("2027-03-01", "2027-03-02")).await?)?;
        assert_eq!(told(&mut bob, "work-times-changed").await?.len(), 1);
        let before = rig.event_count().await?;
        // A year and two weeks on: the two away times of January are old, the one of March is not.
        rig.clock.advance((365 + 14) * 24 * 3600);
        let info = RequestInfo::default();
        rig.services.ctx(info).start_daily_jobs().await?;
        rig.run_jobs().await?;
        let mut bob = Browser::signed_in_as(&app, "bob@example.test").await?;
        let left = bob.send("GET", &format!("{MINE}?{RANGE}"), None).await?;
        let kept = away_of(&left);
        assert_eq!(kept.len(), 1, "{}", left.body);
        assert_eq!(kept[0]["first"], "2027-03-01");
        // The notification about the away time that went is gone with it.
        assert!(told(&mut bob, "work-times-changed").await?.is_empty());
        // The events of the two went too; the job wrote one event of its own that names nobody.
        let after = rig.event_count().await?;
        assert!(after < before + 8, "{before} -> {after}");
        let months = rig
            .storage
            .away_times_of(
                vec![bob_id.parse()?],
                "2027-01-01".parse()?,
                "2027-12-31".parse()?,
            )
            .await
            .map_err(|error| error.to_string())?;
        assert_eq!(months.len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_11_a_person_who_leaves_loses_all_work_times() -> TestResult {
    each_database("plan_11_leaving_loses_work_times", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let (mut cleo, cleo_id) = site
            .person_called(StartingRole::TeamMember, "cleo", "Cleo Cook")
            .await?;
        let (bob_text, cleo_text) = (bob_id.to_string(), cleo_id.to_string());
        for person in [&bob_text, &cleo_text] {
            ok(&enter(&mut ada, Some(person), away("2027-02-01", "2027-02-05")).await?)?;
            ok(&ada
                .send(
                    "PUT",
                    &format!("/api/v1/people/{person}/work-times/usual-days"),
                    Some(week(4, 360)),
                )
                .await?)?;
            ok(&ada
                .send(
                    "PUT",
                    &format!("{PROJECTS}/{}/shares/{person}", site.acme),
                    Some(json!({"percent": 60})),
                )
                .await?)?;
        }
        let chose = bob
            .send(
                "PUT",
                "/api/v1/me/settings/work-times.holidays",
                Some(json!({"value": "de-by"})),
            )
            .await?;
        assert!(chose.status.is_success(), "{}", chose.body);
        let events_before = rig.event_count().await?;
        let ctx = rig.services.ctx(RequestInfo::default());
        delete_work_times_of_person(&ctx, DeleteWorkTimesOfPerson { person: bob_id })
            .await
            .map_err(|error| error.to_string())?;
        // Bob has no work times of his own any more: the starting days, no away times, no shares,
        // the holidays of the organization, and nothing was told about them.
        let read = bob.send("GET", &format!("{MINE}?{RANGE}"), None).await?;
        assert_eq!(ok(&read)?["usual"][4]["works"], true);
        assert_eq!(read.body["usual"][3]["minutes"], 480);
        assert!(away_of(&read).is_empty());
        assert_eq!(read.body["shares"]["items"], json!([]));
        assert_eq!(read.body["holidays"], "de");
        assert!(told(&mut bob, "work-times-changed").await?.is_empty());
        // Cleo keeps hers.
        let other = cleo.send("GET", &format!("{MINE}?{RANGE}"), None).await?;
        assert_eq!(away_of(&other).len(), 1);
        assert_eq!(other.body["usual"][3]["minutes"], 360);
        assert_eq!(
            other.body["shares"]["items"].as_array().map(Vec::len),
            Some(1)
        );
        assert_eq!(told(&mut cleo, "work-times-changed").await?.len(), 2);
        // The events of Bob's work times are deleted with them: three before, none now, and the one
        // event of the deletion itself.
        let events_after = rig.event_count().await?;
        assert_eq!(
            events_after + 3 - 1,
            events_before,
            "{events_before} -> {events_after}"
        );
        Ok(())
    })
    .await
}

/// The ID of a project made by the administrator.
pub(super) async fn project(ada: &mut Browser, short_name: &str) -> Result<String, Failure> {
    let made = ada
        .send(
            "POST",
            PROJECTS,
            Some(json!({"short_name": short_name, "name": format!("{short_name} project")})),
        )
        .await?;
    Ok(ok(&made)?["id"].as_str().unwrap_or_default().to_owned())
}

#[tokio::test]
async fn plan_15_a_share_is_set_only_with_decide_who_has_which_role_in_the_project() -> TestResult {
    each_database("plan_15_share_needs_decide_roles", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (mut bea, _) = site
            .person_in_acme(StartingRole::ProjectLead, "bea")
            .await?;
        let (mut cleo, cleo_id) = site
            .person_in_acme(StartingRole::TeamMember, "cleo")
            .await?;
        let (mut bob, bob_id) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        let path = format!("{PROJECTS}/{}/shares/{bob_id}", site.acme);
        // A Team member does not decide who has which role.
        let refused = cleo
            .send("PUT", &path, Some(json!({"percent": 60})))
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        let own = bob.send("PUT", &path, Some(json!({"percent": 10}))).await?;
        assert_eq!(
            own.status,
            StatusCode::FORBIDDEN,
            "nobody sets their own without the permission"
        );
        // A Project lead does.
        let set = bea.send("PUT", &path, Some(json!({"percent": 60}))).await?;
        assert_eq!(ok(&set)?["percent"], 60);
        let over = bea
            .send("PUT", &path, Some(json!({"percent": 101})))
            .await?;
        assert_eq!(over.code(), "work-times.share-invalid", "{}", over.body);
        let nobody = bea
            .send(
                "PUT",
                &format!(
                    "{PROJECTS}/{}/shares/01a3185c-5000-73b2-8f37-20757db0778e",
                    site.acme
                ),
                Some(json!({"percent": 60})),
            )
            .await?;
        assert_eq!(nobody.status, StatusCode::NOT_FOUND);
        // The list of the project shows it, to those who see people of the kind: 100 for nobody who
        // has none set.
        let listed = cleo
            .send("GET", &format!("{PROJECTS}/{}/shares", site.acme), None)
            .await?;
        let items = ok(&listed)?["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let share_of = |person: &str| -> Option<Value> {
            items
                .iter()
                .find(|item| item["person"]["id"] == person)
                .map(|item| item["percent"].clone())
        };
        assert_eq!(share_of(&bob_id.to_string()), Some(json!(60)), "{items:?}");
        assert_eq!(
            share_of(&cleo_id.to_string()),
            Some(json!(100)),
            "none set is 100"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_15_a_project_lead_sees_over_100_percent_without_other_projects_names() -> TestResult {
    each_database("plan_15_over_100_percent", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let secret = project(&mut ada, "SECRET").await?;
        let open = project(&mut ada, "OPEN").await?;
        let roles = site.org.starting_roles().await?;
        let lead = roles[StartingRole::ProjectLead.code()];
        let member = roles[StartingRole::TeamMember.code()];
        // Bea leads ACME and OPEN, and has no role in SECRET; Bob works in all three.
        let (mut bea, bea_member) = site.person_holding(lead, "bea", &site.acme).await?;
        site.org
            .give(
                &bea_member,
                lead,
                open.parse().map_err(|_| "no project ID")?,
            )
            .await?;
        let (mut bob, bob_member) = site.person_holding(member, "bob", &site.acme).await?;
        for project in [&secret, &open] {
            site.org
                .give(
                    &bob_member,
                    member,
                    project.parse().map_err(|_| "no project ID")?,
                )
                .await?;
        }
        for (project, percent) in [(&site.acme, 60), (&secret, 50), (&open, 20)] {
            let set = ada
                .send(
                    "PUT",
                    &format!("{PROJECTS}/{project}/shares/{}", bob_member.id),
                    Some(json!({"percent": percent})),
                )
                .await?;
            ok(&set)?;
        }
        // Bea sees that Bob is over 100 percent, and the projects she may see by name.
        let listed = bea
            .send("GET", &format!("{PROJECTS}/{}/shares", site.acme), None)
            .await?;
        let items = ok(&listed)?["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let bob_text = bob_member.id.to_string();
        let of_bob = items
            .iter()
            .find(|item| item["person"]["id"] == bob_text)
            .ok_or("Bob is not listed")?;
        // The fact, never the sum: it would tell how much the project she may not see holds.
        assert!(of_bob.get("sum").is_none(), "{of_bob}");
        assert_eq!(of_bob["over_100"], true);
        assert_eq!(
            of_bob["others"].as_array().map(Vec::len),
            Some(1),
            "{of_bob}"
        );
        assert_eq!(of_bob["others"][0]["project"]["short_name"], "OPEN");
        assert_eq!(of_bob["others"][0]["percent"], 20);
        assert_eq!(of_bob["others_hidden"], true);
        // The project she may not see is counted and never named.
        let text = listed.body.to_string();
        assert!(
            !text.contains("SECRET") && !text.contains(&secret),
            "{text}"
        );
        // Somebody who does not decide who has which role sees the share and no sums.
        let (mut cleo, _) = site
            .person_in_acme(StartingRole::TeamMember, "cleo")
            .await?;
        let seen = cleo
            .send("GET", &format!("{PROJECTS}/{}/shares", site.acme), None)
            .await?;
        let theirs = ok(&seen)?["items"].as_array().cloned().unwrap_or_default();
        let of_bob = theirs
            .iter()
            .find(|item| item["person"]["id"] == bob_text)
            .ok_or("Bob is not listed for Cleo")?;
        assert_eq!(of_bob["percent"], 60);
        assert!(
            of_bob.get("sum").is_none() && of_bob.get("over_100").is_none(),
            "{of_bob}"
        );
        // Bob sees his own shares, the sum and that it is over 100 percent, by the names he may see.
        let mine = bob.send("GET", &format!("{MINE}?{RANGE}"), None).await?;
        assert_eq!(ok(&mine)?["shares"]["sum"], 130);
        assert_eq!(mine.body["shares"]["over_100"], true);
        assert_eq!(
            mine.body["shares"]["items"].as_array().map(Vec::len),
            Some(3)
        );
        let _ = (TODAY, YESTERDAY);
        Ok(())
    })
    .await
}
