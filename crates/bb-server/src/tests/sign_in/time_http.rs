//! Time on issues over HTTP on both databases (TIME-01, TIME-02, TIME-13, ADR-0035): one timer per
//! person on every device, an entry saved from the timer or booked by hand with the defaults of its
//! issue and its projects, the starter fields found by their mark, and a long timer told once.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_app::time::LONG_TIMER_HOURS;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{ADA_EMAIL, Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, id_in, key_of, ok, version_of};

pub(super) const TIMER: &str = "/api/v1/me/timer";
pub(super) const MY_ENTRIES: &str = "/api/v1/me/time-entries";
const FIELDS: &str = "/api/v1/fields";

/// The seconds in a minute, for moving the clock of the tests.
const MINUTE: i64 = 60;

/// The person's timer.
pub(super) async fn timer_of(browser: &mut Browser) -> Result<Value, Failure> {
    Ok(ok(&browser.send("GET", TIMER, None).await?)?.clone())
}

/// Starts the timer on an issue; the answer.
pub(super) async fn start(browser: &mut Browser, issue: &str) -> Result<Reply, Failure> {
    browser
        .send("POST", TIMER, Some(json!({"issue": issue})))
        .await
}

/// The own field a starter field is, by its mark (`activity`, `cost-center`), as the timer says.
pub(super) async fn starter(browser: &mut Browser, mark: &str) -> Result<String, Failure> {
    Ok(timer_of(browser).await?["starters"][mark]
        .as_str()
        .ok_or_else(|| format!("no starter field {mark}"))?
        .to_owned())
}

/// The field as its page shows it.
async fn field(browser: &mut Browser, key: &str) -> Result<Value, Failure> {
    Ok(ok(&browser
        .send("GET", &format!("{FIELDS}/{key}"), None)
        .await?)?
    .clone())
}

/// The ID of the choice of a field that has this name.
fn choice_named(field: &Value, name: &str) -> Result<String, Failure> {
    field["choices"]
        .as_array()
        .and_then(|choices| choices.iter().find(|choice| choice["name"] == name))
        .and_then(|choice| choice["id"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("no choice {name}").into())
}

/// Adds a choice to a field; its ID.
async fn add_choice(browser: &mut Browser, key: &str, name: &str) -> Result<String, Failure> {
    let version = field(browser, key).await?["version"].clone();
    let added = browser
        .send(
            "POST",
            &format!("{FIELDS}/{key}/choices"),
            Some(json!({"version": version, "name": name})),
        )
        .await?;
    choice_named(ok(&added)?, name)
}

/// Sets the value of an own field on an issue.
async fn set_on_issue(
    browser: &mut Browser,
    issue: &str,
    key: &str,
    value: &str,
) -> Result<(), Failure> {
    let version = version_of(browser, issue).await?;
    let changed = browser
        .send(
            "PATCH",
            &format!("{ISSUES}/{issue}"),
            Some(json!({"version": version, "fields": {key: value}})),
        )
        .await?;
    ok(&changed)?;
    Ok(())
}

/// Sets the value of an own field on a project (by its ID).
async fn set_on_project(
    browser: &mut Browser,
    project: &str,
    key: &str,
    value: &str,
) -> Result<(), Failure> {
    let shown = browser
        .send("GET", &format!("{PROJECTS}/{project}"), None)
        .await?;
    let version = ok(&shown)?["project"]["version"].clone();
    let changed = browser
        .send(
            "PATCH",
            &format!("{PROJECTS}/{project}"),
            Some(json!({"version": version, "fields": {key: value}})),
        )
        .await?;
    ok(&changed)?;
    Ok(())
}

/// Books time by hand on an issue; the answer.
async fn book(browser: &mut Browser, issue: &str, minutes: u32) -> Result<Reply, Failure> {
    browser
        .send(
            "POST",
            MY_ENTRIES,
            Some(json!({"issue": issue, "date": "2027-01-14", "minutes": minutes})),
        )
        .await
}

#[tokio::test]
async fn time_01_one_person_has_one_running_timer_on_every_device() -> TestResult {
    each_database("time_01_one_timer", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut laptop) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut laptop, "Printer jams").await?)?;
        let mut phone = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        // Two devices start at the same moment: one timer, and one refusal with the conflict.
        let (first, second) = tokio::join!(start(&mut laptop, &issue), start(&mut phone, &issue));
        let (first, second) = (first?, second?);
        let mut answers = [first.status, second.status];
        answers.sort();
        assert_eq!(answers, [StatusCode::OK, StatusCode::CONFLICT]);
        let refused = if first.status == StatusCode::CONFLICT {
            &first
        } else {
            &second
        };
        assert_eq!(refused.code(), "timer.running", "{}", refused.body);
        // Every device shows the same timer.
        let on_laptop = timer_of(&mut laptop).await?;
        let on_phone = timer_of(&mut phone).await?;
        assert_eq!(on_laptop, on_phone);
        assert_eq!(on_laptop["running"]["readable_id"], issue.as_str());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_01_switch_issue_saves_the_time_so_far_with_the_defaults_of_its_issue() -> TestResult {
    each_database("time_01_switch", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let first = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let second = key_of(&site.create(&mut ada, "Scanner is slow").await?)?;
        let activity = starter(&mut ada, "activity").await?;
        let testing = choice_named(&field(&mut ada, &activity).await?, "Testing")?;
        set_on_issue(&mut ada, &first, &activity, &testing).await?;
        let started = start(&mut ada, &first).await?;
        let version = ok(&started)?["version"].clone();
        rig.clock.advance(25 * MINUTE + 30);
        let switched = ada
            .send(
                "POST",
                &format!("{TIMER}/switch"),
                Some(json!({"version": version, "issue": second})),
            )
            .await?;
        let saved = ok(&switched)?;
        // The time so far, in whole minutes, on the first issue with its Activity.
        assert_eq!(saved["entry"]["minutes"], 25);
        assert_eq!(saved["entry"]["readable_id"], first.as_str());
        assert_eq!(saved["entry"]["own_fields"][&activity], json!(testing));
        assert_eq!(saved["entry"]["date"], "2027-01-15");
        // The timer goes on with the second issue.
        assert_eq!(saved["timer"]["running"]["readable_id"], second.as_str());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_01_a_timer_under_one_minute_saves_nothing() -> TestResult {
    each_database("time_01_under_a_minute", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let version = ok(&start(&mut ada, &issue).await?)?["version"].clone();
        rig.clock.advance(MINUTE - 1);
        let stopped = ada
            .send(
                "POST",
                &format!("{TIMER}/stop"),
                Some(json!({"version": version})),
            )
            .await?;
        let answer = ok(&stopped)?;
        assert!(answer.get("entry").is_none(), "{answer}");
        assert!(answer["timer"].get("running").is_none(), "{answer}");
        let week = ada
            .send(
                "GET",
                &format!("{MY_ENTRIES}?from=2027-01-11&to=2027-01-17"),
                None,
            )
            .await?;
        assert_eq!(ok(&week)?["items"], json!([]));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_01_a_long_timer_sends_one_notification() -> TestResult {
    each_database("time_01_long_timer", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        ok(&start(&mut ada, &issue).await?)?;
        // Before the longest day has passed, the job waits.
        rig.clock.advance(LONG_TIMER_HOURS * 60 * MINUTE - MINUTE);
        rig.run_jobs().await?;
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let none_yet = ada.send("GET", "/api/v1/notifications", None).await?;
        let long = |list: &Reply| {
            list.body["items"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter(|item| item["kind"] == "long-timer")
                        .count()
                })
                .unwrap_or_default()
        };
        assert_eq!(long(&none_yet), 0, "{}", none_yet.body);
        // Then the person is told once, and the timer says it runs long.
        rig.clock.advance(2 * MINUTE);
        rig.run_jobs().await?;
        rig.run_jobs().await?;
        let told = ada.send("GET", "/api/v1/notifications", None).await?;
        assert_eq!(long(&told), 1, "{}", told.body);
        assert_eq!(timer_of(&mut ada).await?["running"]["runs_long"], true);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_02_a_time_entry_by_hand_takes_a_date_a_length_and_the_defaults() -> TestResult {
    each_database("time_02_by_hand", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let cost_center = starter(&mut ada, "cost-center").await?;
        let product = add_choice(&mut ada, &cost_center, "4100 Product development").await?;
        set_on_project(&mut ada, &site.acme, &cost_center, &product).await?;
        let booked = book(&mut ada, &issue, 90).await?;
        let entry = ok(&booked)?;
        assert_eq!(entry["date"], "2027-01-14");
        assert_eq!(entry["minutes"], 90);
        assert!(entry.get("started_at").is_none());
        assert_eq!(entry["own_fields"][&cost_center], json!(product));
        // A length that is no entry is refused in plain words.
        let none = book(&mut ada, &issue, 0).await?;
        assert_eq!(none.code(), "time.entry-too-short");
        let too_long = book(&mut ada, &issue, 1441).await?;
        assert_eq!(too_long.code(), "time.entry-too-long");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_13_activity_and_cost_center_are_found_by_their_mark_after_a_rename() -> TestResult {
    each_database("time_13_rename", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let activity = starter(&mut ada, "activity").await?;
        let shown = field(&mut ada, &activity).await?;
        assert_eq!(shown["name"], "Activity");
        let renamed = ada
            .send(
                "PATCH",
                &format!("{FIELDS}/{activity}"),
                Some(json!({"version": shown["version"], "name": "Kind of work"})),
            )
            .await?;
        ok(&renamed)?;
        // The mark still finds the field, and the default of the issue still fills it in.
        assert_eq!(starter(&mut ada, "activity").await?, activity);
        let support = choice_named(&field(&mut ada, &activity).await?, "Support")?;
        set_on_issue(&mut ada, &issue, &activity, &support).await?;
        let booked = book(&mut ada, &issue, 15).await?;
        assert_eq!(ok(&booked)?["own_fields"][&activity], json!(support));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_13_the_default_comes_from_the_issue_then_the_project_then_the_projects_above()
-> TestResult {
    each_database("time_13_defaults", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let activity = starter(&mut ada, "activity").await?;
        let choices = field(&mut ada, &activity).await?;
        let (meetings, support, testing) = (
            choice_named(&choices, "Meetings")?,
            choice_named(&choices, "Support")?,
            choice_named(&choices, "Testing")?,
        );
        let sub = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{}/sub-projects", site.acme),
                Some(json!({"short_name": "SUB", "name": "Sub"})),
            )
            .await?;
        let sub = id_in(ok(&sub)?)?;
        let made = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{sub}/issues"),
                Some(site.new_issue("Paper runs out")),
            )
            .await?;
        let issue = key_of(&made)?;
        let booked_activity = async |ada: &mut Browser| -> Result<Value, Failure> {
            let booked = book(ada, &issue, 10).await?;
            Ok(ok(&booked)?["own_fields"][&activity].clone())
        };
        // Nothing anywhere: no default.
        assert_eq!(booked_activity(&mut ada).await?, Value::Null);
        // The project above.
        set_on_project(&mut ada, &site.acme, &activity, &meetings).await?;
        assert_eq!(booked_activity(&mut ada).await?, json!(meetings));
        // The project itself is nearer.
        set_on_project(&mut ada, &sub, &activity, &support).await?;
        assert_eq!(booked_activity(&mut ada).await?, json!(support));
        // The issue is nearest.
        set_on_issue(&mut ada, &issue, &activity, &testing).await?;
        assert_eq!(booked_activity(&mut ada).await?, json!(testing));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_13_a_required_cost_center_refuses_an_entry_without_one() -> TestResult {
    each_database("time_13_required", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let cost_center = starter(&mut ada, "cost-center").await?;
        let product = add_choice(&mut ada, &cost_center, "4100 Product development").await?;
        let version = field(&mut ada, &cost_center).await?["version"].clone();
        let required = ada
            .send(
                "PATCH",
                &format!("{FIELDS}/{cost_center}"),
                Some(json!({"version": version, "rules": {"required": true}})),
            )
            .await?;
        ok(&required)?;
        let refused = book(&mut ada, &issue, 30).await?;
        assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{}", refused.body);
        assert_eq!(refused.code(), "field.required");
        assert_eq!(
            refused.body["field"],
            cost_center.as_str(),
            "{}",
            refused.body
        );
        // With a value it is booked.
        let booked = ada
            .send(
                "POST",
                MY_ENTRIES,
                Some(json!({
                    "issue": issue,
                    "date": "2027-01-14",
                    "minutes": 30,
                    "entry": {"fields": {&cost_center: product}},
                })),
            )
            .await?;
        ok(&booked)?;
        // An issue that has the value as its default needs nothing more.
        set_on_issue(&mut ada, &issue, &cost_center, &product).await?;
        ok(&book(&mut ada, &issue, 30).await?)?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_13_a_person_with_a_role_in_one_project_sees_the_fields_of_their_own_entries()
-> TestResult {
    each_database("time_13_project_role", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let activity = starter(&mut ada, "activity").await?;
        let testing = choice_named(&field(&mut ada, &activity).await?, "Testing")?;
        // Tom holds his role in ACME only: the organization gives him nothing.
        let (mut tom, _) = site.person_in_acme(StartingRole::TeamMember, "tom").await?;
        let booked = tom
            .send(
                "POST",
                MY_ENTRIES,
                Some(json!({
                    "issue": issue,
                    "date": "2027-01-14",
                    "minutes": 20,
                    "entry": {"fields": {&activity: testing}},
                })),
            )
            .await?;
        assert_eq!(ok(&booked)?["own_fields"][&activity], json!(testing));
        let week = tom
            .send(
                "GET",
                &format!("{MY_ENTRIES}?from=2027-01-11&to=2027-01-17&fields=id,own_fields"),
                None,
            )
            .await?;
        assert_eq!(
            ok(&week)?["items"][0]["own_fields"][&activity],
            json!(testing),
            "{}",
            week.body
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_02_two_bookings_at_once_never_pass_the_most_entries_of_a_date() -> TestResult {
    each_database("time_02_most_entries_at_once", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        let most = usize::try_from(bb_app::time::TIME_ENTRIES_PER_DAY_MAX)?;
        for _ in 1..most {
            ok(&book(&mut ada, &issue, 1).await?)?;
        }
        let mut other = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let (first, second) = tokio::join!(book(&mut ada, &issue, 1), book(&mut other, &issue, 1));
        let (first, second) = (first?, second?);
        let mut codes = [first.code().to_owned(), second.code().to_owned()];
        codes.sort();
        assert_eq!(
            codes,
            ["", "time.too-many-entries"],
            "{} {}",
            first.body,
            second.body
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_01_a_stop_on_two_devices_or_a_stop_and_a_switch_save_the_time_once() -> TestResult {
    each_database("time_01_stop_twice", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut laptop) = Site::start(&rig, &app).await?;
        let first = key_of(&site.create(&mut laptop, "Printer jams").await?)?;
        let second = key_of(&site.create(&mut laptop, "Scanner is slow").await?)?;
        let mut phone = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        for other_action in ["stop", "switch"] {
            let version = ok(&start(&mut laptop, &first).await?)?["version"].clone();
            rig.clock.advance(10 * MINUTE);
            let stop = json!({"version": version});
            let other = if other_action == "stop" {
                stop.clone()
            } else {
                json!({"version": version, "issue": second})
            };
            let (stop_path, other_path) =
                (format!("{TIMER}/stop"), format!("{TIMER}/{other_action}"));
            let (a, b) = tokio::join!(
                laptop.send("POST", &stop_path, Some(stop)),
                phone.send("POST", &other_path, Some(other))
            );
            let (a, b) = (a?, b?);
            let saved = [&a, &b]
                .iter()
                .filter(|reply| reply.status == StatusCode::OK)
                .count();
            assert_eq!(saved, 1, "{other_action}: {} {}", a.body, b.body);
            // Whatever won, the issue has the ten minutes once more, never twice.
            let entries = laptop
                .send("GET", &format!("{ISSUES}/{first}/time-entries"), None)
                .await?;
            let count = ok(&entries)?["items"].as_array().map_or(0, Vec::len);
            let expected = if other_action == "stop" { 1 } else { 2 };
            assert_eq!(count, expected, "{}", entries.body);
            // A switch that won leaves the timer running; stop it for the next round.
            let timer = timer_of(&mut laptop).await?;
            if timer.get("running").is_some() {
                ok(&laptop
                    .send(
                        "POST",
                        &format!("{TIMER}/throw-away"),
                        Some(json!({"version": timer["version"]})),
                    )
                    .await?)?;
            }
            rig.clock.advance(MINUTE);
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_13_the_retired_words_of_activity_and_cost_center_are_accepted_and_change_nothing()
-> TestResult {
    each_database("time_13_retired_words", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let roles = ada.send("GET", "/api/v1/roles", None).await?;
        let role = ok(&roles)?["items"][0]["id"]
            .as_str()
            .ok_or("no role")?
            .to_owned();
        for word in bb_core::fields::RETIRED_FIELD_WORDS {
            let set = ada
                .send(
                    "PUT",
                    &format!("/api/v1/field-permissions/{role}/{word}"),
                    Some(json!({"level": "hidden"})),
                )
                .await?;
            assert_eq!(set.status, StatusCode::NO_CONTENT, "{word}: {}", set.body);
            // A role that does not exist is not found, as for a field that is in the catalogue.
            let unknown = bb_core::id::RoleId::from_parts(rig.clock_now(), [9; 10])?;
            let nowhere = format!("/api/v1/field-permissions/{unknown}/{word}");
            let missing = ada
                .send("PUT", &nowhere, Some(json!({"level": "hidden"})))
                .await?;
            assert_eq!(
                missing.status,
                StatusCode::NOT_FOUND,
                "{word}: {}",
                missing.body
            );
        }
        // No answer names them.
        let table = ada.send("GET", "/api/v1/field-permissions", None).await?;
        let text = ok(&table)?.to_string();
        assert!(
            !text.contains("\"cost-center\"") && !text.contains("\"activity\""),
            "{text}"
        );
        // The schema of the frozen API still holds them.
        let description: Value = serde_json::from_str(bb_api::openapi::json())?;
        let words = &description["components"]["schemas"]["Field"]["enum"];
        assert!(
            words
                .as_array()
                .is_some_and(|all| all.contains(&json!("cost-center")))
        );
        Ok(())
    })
    .await
}
