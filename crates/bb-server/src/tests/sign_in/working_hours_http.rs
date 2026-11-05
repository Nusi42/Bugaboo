//! Working hours over HTTP (TIME-08, TIME-10, TIME-12, PRIV-09, ADR-0035, decisions 5 to 9): a work
//! day from its taps, a day left open, a past day that changes only with a reason and keeps the old
//! times, the person told of a change by somebody else, who sees another person's days, *Totals
//! only*, the security log, and the deletion of old days only when an administrator says so. Every
//! flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{ADA_EMAIL, Browser, Reply, rig_and_app};
use super::issues_site::{Site, ok};
use super::rig::{PASSWORD, Rig};

pub(super) const MINE: &str = "/api/v1/me/work-days";
const OLD: &str = "/api/v1/work-days/old";
/// The date of the clock of the tests (2027-01-15 08:00 UTC); people who chose no zone count in UTC.
pub(super) const TODAY: &str = "2027-01-15";
pub(super) const YESTERDAY: &str = "2027-01-14";
const HOUR: i64 = 3600;
const MINUTE: i64 = 60;

pub(super) fn days(reply: &Reply) -> Vec<Value> {
    reply.body["days"].as_array().cloned().unwrap_or_default()
}

/// The person's days from `from` to `to`.
async fn mine(browser: &mut Browser, from: &str, to: &str) -> Result<Reply, Failure> {
    browser
        .send("GET", &format!("{MINE}?from={from}&to={to}"), None)
        .await
}

/// A tap on the day that runs, at the version the person looked at.
async fn tap(browser: &mut Browser, path: &str, version: &Value) -> Result<Reply, Failure> {
    browser
        .send(
            "POST",
            &format!("{MINE}/{path}"),
            Some(json!({ "version": version })),
        )
        .await
}

async fn start(browser: &mut Browser) -> Result<Reply, Failure> {
    browser
        .send("POST", &format!("{MINE}/start"), Some(json!({})))
        .await
}

/// A day that starts and ends at these moments of `date`, with no break.
pub(super) fn times(date: &str, from: &str, to: &str, version: i64, reason: Option<&str>) -> Value {
    json!({
        "version": version,
        "periods": [{"start": format!("{date}T{from}:00Z"), "end": format!("{date}T{to}:00Z")}],
        "breaks": [],
        "reason": reason,
    })
}

/// Sessions that last through a work day: the longest the settings allow, so the clock of a test
/// can move by hours.
pub(super) async fn long_sessions(rig: &Rig) -> Result<(), Failure> {
    rig.set_organization_setting("security.session-idle-minutes", 1440)
        .await?;
    rig.set_organization_setting("security.session-absolute-hours", 720)
        .await
}

/// Lets `seconds` pass for a person who keeps working: the session is used every few hours.
async fn pass(rig: &Rig, browser: &mut Browser, seconds: i64) -> Result<(), Failure> {
    let mut left = seconds;
    while left > 0 {
        let step = left.min(6 * HOUR);
        rig.clock.advance(step);
        left -= step;
        browser.send("GET", "/api/v1/auth/me", None).await?;
    }
    Ok(())
}

/// The ID of the person with this address.
async fn id_of_person(rig: &Rig, email: &str) -> Result<String, Failure> {
    Ok(rig.person_id(email).await?.to_string())
}

/// Ada's own work day today: started, ended after `hours`.
async fn a_day(rig: &Rig, browser: &mut Browser, hours: i64) -> Result<Value, Failure> {
    let started = start(browser).await?;
    let version = ok(&started)?["version"].clone();
    pass(rig, browser, hours * HOUR).await?;
    Ok(ok(&tap(browser, "end", &version).await?)?.clone())
}

#[tokio::test]
async fn time_08_start_break_end_form_one_work_day_with_its_totals() -> TestResult {
    each_database("time_08_work_day_taps", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        long_sessions(&rig).await?;
        let started = start(&mut ada).await?;
        let day = ok(&started)?;
        assert_eq!(day["date"], TODAY);
        assert_eq!(day["runs"], true);
        // A second tap in the same second records one moment.
        let again = tap(&mut ada, "break/start", &day["version"]).await?;
        assert_eq!(again.code(), "limit.reached", "{}", again.body);
        pass(&rig, &mut ada, 3 * HOUR).await?;
        let on_break = tap(&mut ada, "break/start", &day["version"]).await?;
        let on_break = ok(&on_break)?.clone();
        rig.clock.advance(30 * MINUTE);
        let back = ok(&tap(&mut ada, "break/end", &on_break["version"]).await?)?.clone();
        pass(&rig, &mut ada, 4 * HOUR).await?;
        let ended = ok(&tap(&mut ada, "end", &back["version"]).await?)?.clone();
        assert_eq!(ended["runs"], false);
        let listed = mine(&mut ada, TODAY, TODAY).await?;
        let listed_days = days(&listed);
        assert_eq!(listed_days.len(), 1, "{}", listed.body);
        let only = &listed_days[0];
        assert_eq!(only["periods"].as_array().map(Vec::len), Some(1));
        assert_eq!(only["breaks"].as_array().map(Vec::len), Some(1));
        assert_eq!(only["totals"], json!({"worked": 420, "breaks": 30}));
        assert_eq!(listed.body["totals"], json!({"worked": 420, "breaks": 30}));
        assert_eq!(listed.body["person"]["name"], "Ada Admin");
        // A second start on the same date reopens the day with a new period.
        rig.clock.advance(HOUR);
        let reopened = ok(&start(&mut ada).await?)?.clone();
        assert_eq!(reopened["periods"].as_array().map(Vec::len), Some(2));
        assert_eq!(start(&mut ada).await?.code(), "working-hours.day-running");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_08_a_day_left_open_must_be_ended_before_the_next_starts() -> TestResult {
    each_database("time_08_day_left_open", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        long_sessions(&rig).await?;
        let left = ok(&start(&mut ada).await?)?.clone();
        pass(&rig, &mut ada, 25 * HOUR).await?;
        // Nothing ends the day by itself: the next start names it.
        let refused = start(&mut ada).await?;
        assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);
        assert_eq!(refused.code(), "working-hours.day-left-open");
        assert_eq!(refused.body["open_day"], TODAY);
        // Ending it now would make a day longer than 24 hours.
        let too_long = tap(&mut ada, "end", &left["version"]).await?;
        assert_eq!(too_long.code(), "working-hours.not-a-work-day");
        // The person says when it ended: a change of a past day, with a reason.
        let answered = ada
            .send(
                "PATCH",
                &format!("{MINE}/{TODAY}"),
                Some(times(TODAY, "08:00", "16:00", 1, Some("Forgot to end it"))),
            )
            .await?;
        assert_eq!(ok(&answered)?["runs"], false);
        rig.clock.advance(1);
        let next = ok(&start(&mut ada).await?)?.clone();
        assert_eq!(next["date"], "2027-01-16");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_10_a_past_day_changes_only_with_a_reason_and_keeps_the_old_value() -> TestResult {
    each_database("time_10_past_day_reason", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        long_sessions(&rig).await?;
        let day = a_day(&rig, &mut ada, 8).await?;
        // Today a day changes without a reason.
        rig.clock.advance(MINUTE);
        let today = ada
            .send(
                "PATCH",
                &format!("{MINE}/{TODAY}"),
                Some(times(
                    TODAY,
                    "08:00",
                    "15:30",
                    day["version"].as_i64().unwrap_or(0),
                    None,
                )),
            )
            .await?;
        let today = ok(&today)?.clone();
        pass(&rig, &mut ada, 12 * HOUR).await?;
        let path = format!("{MINE}/{TODAY}");
        let version = today["version"].as_i64().unwrap_or(0);
        let without = ada
            .send(
                "PATCH",
                &path,
                Some(times(TODAY, "08:00", "17:00", version, None)),
            )
            .await?;
        assert_eq!(
            without.code(),
            "working-hours.reason-needed",
            "{}",
            without.body
        );
        let with = ada
            .send(
                "PATCH",
                &path,
                Some(times(
                    TODAY,
                    "08:00",
                    "17:00",
                    version,
                    Some("Stayed for the release"),
                )),
            )
            .await?;
        assert_eq!(ok(&with)?["totals"]["worked"], 540);
        let history = ada.send("GET", &format!("{path}/history"), None).await?;
        let changes = ok(&history)?["changes"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let last = changes.last().ok_or("no change in the history")?;
        assert_eq!(last["reason"], "Stayed for the release");
        assert_eq!(
            last["before"]["periods"][0]["end"],
            format!("{TODAY}T15:30:00Z")
        );
        assert_eq!(
            last["after"]["periods"][0]["end"],
            format!("{TODAY}T17:00:00Z")
        );
        assert_eq!(last["by"]["name"], "Ada Admin");
        // The first change, which made the day, has no times before it.
        assert_eq!(changes[0]["before"], Value::Null);
        // Nothing that has not happened yet is recorded.
        let tomorrow = "2027-01-16";
        rig.clock.advance(1);
        let ahead = ada
            .send(
                "PATCH",
                &format!("{MINE}/{tomorrow}"),
                Some(times(tomorrow, "08:00", "16:00", 0, Some("Planned"))),
            )
            .await?;
        assert_eq!(
            ahead.code(),
            "working-hours.in-the-future",
            "{}",
            ahead.body
        );
        Ok(())
    })
    .await
}

/// Hana holds *HR* for the whole organization; Bob is a team member with a day of yesterday.
pub(super) async fn hana_and_bob(
    rig: &Rig,
    site: &Site<'_>,
) -> Result<(Browser, Browser, String), Failure> {
    let (hana, _) = site
        .person_called(StartingRole::Hr, "hana", "Hana Hart")
        .await?;
    let (mut bob, bob_id) = site
        .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
        .await?;
    rig.clock.advance(1);
    let made = bob
        .send(
            "PATCH",
            &format!("{MINE}/{YESTERDAY}"),
            Some(times(
                YESTERDAY,
                "07:00",
                "15:00",
                0,
                Some("Typed in afterwards"),
            )),
        )
        .await?;
    ok(&made)?;
    rig.clock.advance(1);
    Ok((hana, bob, bob_id.to_string()))
}

#[tokio::test]
async fn time_10_a_change_by_someone_else_tells_the_person() -> TestResult {
    each_database("time_10_change_tells_person", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut hana, mut bob, bob_id) = hana_and_bob(&rig, &site).await?;
        let path = format!("/api/v1/people/{bob_id}/work-days/{YESTERDAY}");
        // Somebody else's day changes only with a reason, also today's.
        let without = hana
            .send(
                "PATCH",
                &path,
                Some(times(YESTERDAY, "07:00", "16:00", 1, None)),
            )
            .await?;
        assert_eq!(without.code(), "working-hours.reason-needed");
        let changed = hana
            .send(
                "PATCH",
                &path,
                Some(times(
                    YESTERDAY,
                    "07:00",
                    "16:00",
                    1,
                    Some("Badge log says 16:00"),
                )),
            )
            .await?;
        ok(&changed)?;
        let told = bob.send("GET", "/api/v1/notifications", None).await?;
        let items = ok(&told)?["items"].as_array().cloned().unwrap_or_default();
        let item = items
            .iter()
            .find(|item| item["kind"] == "work-day-changed")
            .ok_or("Bob was not told")?;
        assert_eq!(item["work_day"], YESTERDAY);
        assert_eq!(item["reason"], "Badge log says 16:00");
        assert_eq!(item["by"]["name"], "Hana Hart");
        assert!(item.get("periods").is_none());
        // Nobody else hears of it.
        for browser in [&mut ada, &mut hana] {
            let others = browser.send("GET", "/api/v1/notifications", None).await?;
            let kinds = others.body["items"].as_array().cloned().unwrap_or_default();
            assert!(kinds.iter().all(|item| item["kind"] != "work-day-changed"));
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_12_without_the_permission_another_persons_days_answer_like_a_person_that_does_not_exist()
-> TestResult {
    each_database("time_12_others_not_found", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (_hana, mut bob, _) = hana_and_bob(&rig, &site).await?;
        let ada_id = id_of_person(&rig, ADA_EMAIL).await?;
        let nobody = "01a3185c-5000-73b2-8f37-20757db0778e";
        for person in [ada_id.as_str(), nobody] {
            let base = format!("/api/v1/people/{person}/work-days");
            let read = bob
                .send("GET", &format!("{base}?from={YESTERDAY}&to={TODAY}"), None)
                .await?;
            assert_eq!(
                (read.status, read.code()),
                (StatusCode::NOT_FOUND, "resource.not-found")
            );
            let change = bob
                .send(
                    "PATCH",
                    &format!("{base}/{YESTERDAY}"),
                    Some(times(YESTERDAY, "07:00", "16:00", 0, Some("x"))),
                )
                .await?;
            assert_eq!(change.status, StatusCode::NOT_FOUND);
            let history = bob
                .send("GET", &format!("{base}/{YESTERDAY}/history"), None)
                .await?;
            assert_eq!(history.status, StatusCode::NOT_FOUND);
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_12_the_people_to_choose_from_are_offered_to_hr_by_name_and_to_nobody_else()
-> TestResult {
    each_database("time_12_people_to_choose", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (mut hana, mut bob, bob_id) = hana_and_bob(&rig, &site).await?;
        let found = hana
            .send("GET", "/api/v1/work-days/people?search=bob", None)
            .await?;
        let items = ok(&found)?["items"].as_array().cloned().unwrap_or_default();
        assert_eq!(items.len(), 1, "{}", found.body);
        assert_eq!(items[0]["id"], bob_id);
        assert_eq!(items[0]["name"], "Bob Baker");
        // Names and IDs only: no hours, no counts, no e-mail address.
        assert_eq!(items[0].as_object().map(serde_json::Map::len), Some(2));
        let refused = bob.send("GET", "/api/v1/work-days/people", None).await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        Ok(())
    })
    .await
}

/// A past day nobody has recorded yet.
const EARLIER: &str = "2027-01-05";

#[tokio::test]
async fn time_10_a_day_typed_afterwards_is_marked_changed_takes_the_device_zone_and_totals_the_week()
-> TestResult {
    each_database("time_10_changed_flag", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (_hana, mut bob, _) = hana_and_bob(&rig, &site).await?;
        let path = format!("/api/v1/me/work-days/{EARLIER}");
        let mut body = times(EARLIER, "08:00", "16:00", 0, Some("Typed in afterwards"));
        body["device_zone"] = json!("Europe/Berlin");
        let saved = bob.send("PATCH", &path, Some(body)).await?;
        assert_eq!(ok(&saved)?["zone"], "Europe/Berlin");
        let read = bob
            .send(
                "GET",
                &format!("/api/v1/me/work-days?from={EARLIER}&to={EARLIER}"),
                None,
            )
            .await?;
        assert_eq!(ok(&read)?["days"][0]["changed"], true);
        // The week is one answer with its total: the browser adds nothing up.
        let booked = bob
            .send(
                "GET",
                &format!("/api/v1/me/work-days/booked?date={EARLIER}"),
                None,
            )
            .await?;
        assert_eq!(ok(&booked)?["week"]["worked"], 480);
        assert_eq!(booked.body["week"]["not_booked"], 480);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_12_an_administrator_does_not_see_working_hours_until_given_hr() -> TestResult {
    each_database("time_12_admin_needs_hr", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_hana, _bob, bob_id) = hana_and_bob(&rig, &site).await?;
        let read = format!("/api/v1/people/{bob_id}/work-days?from={YESTERDAY}&to={TODAY}");
        assert_eq!(
            ada.send("GET", &read, None).await?.status,
            StatusCode::NOT_FOUND
        );
        // An administrator gives *HR*, also to themselves, without holding its permissions.
        let roles = site.org.starting_roles().await?;
        let hr = roles[StartingRole::Hr.code()];
        site.org.give_everywhere(&site.org.admin, hr).await?;
        let seen = ada.send("GET", &read, None).await?;
        let seen_days = days(&seen);
        assert_eq!(seen_days.len(), 1, "{}", seen.body);
        assert_eq!(seen.body["person"]["name"], "Bob Baker");
        assert_eq!(seen.body["may_change"], true);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_12_totals_only_answers_week_totals_and_nothing_else() -> TestResult {
    each_database("time_12_totals_only", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        long_sessions(&rig).await?;
        let (mut hana, mut bob, bob_id) = hana_and_bob(&rig, &site).await?;
        rig.set_organization_switch("working-hours.totals-only", true)
            .await?;
        let base = format!("/api/v1/people/{bob_id}/work-days");
        let read = hana
            .send("GET", &format!("{base}?from={YESTERDAY}&to={TODAY}"), None)
            .await?;
        let body = ok(&read)?;
        assert_eq!(body["days"], json!([]));
        assert_eq!(body["totals"], Value::Null);
        assert_eq!(body["totals_only"], true);
        assert_eq!(body["may_change"], false);
        assert_eq!(body["weeks"][0]["worked"], 480);
        assert!(body["weeks"][0].get("breaks").is_none());
        // A stretch of one day reads the whole week: no day is told apart from its week.
        let today_only = hana
            .send("GET", &format!("{base}?from={TODAY}&to={TODAY}"), None)
            .await?;
        assert_eq!(ok(&today_only)?["weeks"], body["weeks"]);
        // A day that runs counts as nothing until it ends, so nobody learns that Bob works now.
        rig.clock.advance(HOUR);
        ok(&start(&mut bob).await?)?;
        rig.clock.advance(2 * HOUR);
        let running = hana
            .send("GET", &format!("{base}?from={TODAY}&to={TODAY}"), None)
            .await?;
        assert_eq!(ok(&running)?["weeks"], body["weeks"]);
        let change = hana
            .send(
                "PATCH",
                &format!("{base}/{YESTERDAY}"),
                Some(times(YESTERDAY, "07:00", "16:00", 1, Some("x"))),
            )
            .await?;
        assert_eq!(change.code(), "working-hours.week-totals-only");
        let history = hana
            .send("GET", &format!("{base}/{YESTERDAY}/history"), None)
            .await?;
        assert_eq!(history.code(), "working-hours.week-totals-only");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_09_every_change_of_who_sees_working_hours_is_in_the_security_log() -> TestResult {
    each_database("priv_09_security_log", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let before = rig.database.security_log_entries(0, 10_000).await?.len();
        site.person_called(StartingRole::Hr, "hana", "Hana Hart")
            .await?;
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let setting = ada
            .send(
                "PUT",
                "/api/v1/settings/working-hours.totals-only",
                Some(json!({"value": true})),
            )
            .await?;
        ok(&setting)?;
        let actions: Vec<String> = rig
            .database
            .security_log_entries(0, 10_000)
            .await?
            .into_iter()
            .skip(before)
            .map(|entry| entry.action.as_str().to_owned())
            .collect();
        assert!(
            actions.iter().any(|action| action == "assignment.created"),
            "{actions:?}"
        );
        assert!(
            actions.iter().any(|action| action == "setting.changed"),
            "{actions:?}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_09_old_working_hours_are_deleted_only_when_an_administrator_says_so() -> TestResult {
    each_database("priv_09_old_days", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_hana, mut bob, bob_id) = hana_and_bob(&rig, &site).await?;
        let old = "2024-01-10";
        let made = bob
            .send(
                "PATCH",
                &format!("{MINE}/{old}"),
                Some(times(old, "07:00", "15:00", 0, Some("From the paper list"))),
            )
            .await?;
        ok(&made)?;
        // Nothing deletes it by itself.
        rig.run_jobs().await?;
        let counted = ok(&ada.send("GET", OLD, None).await?)?.clone();
        assert_eq!(counted["total"], 1);
        assert_eq!(counted["keep_years"], 2);
        assert_eq!(counted["people"][0]["person"]["name"], "Bob Baker");
        // Only who may change organization settings counts or deletes them.
        assert_eq!(
            bob.send("GET", OLD, None).await?.status,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            bob.send("DELETE", OLD, None).await?.status,
            StatusCode::FORBIDDEN
        );
        let log_before = rig.database.security_log_entries(0, 10_000).await?.len();
        let deleted = ada
            .send("DELETE", &format!("{OLD}?person={bob_id}"), None)
            .await?;
        assert_eq!(ok(&deleted)?["deleted"], 1);
        // The day younger than the keep period stays; the old one is gone with its history.
        let kept = mine(&mut bob, YESTERDAY, YESTERDAY).await?;
        assert_eq!(days(&kept).len(), 1);
        let history = bob
            .send("GET", &format!("{MINE}/{old}/history"), None)
            .await?;
        assert_eq!(history.status, StatusCode::NOT_FOUND);
        let actions: Vec<String> = rig
            .database
            .security_log_entries(0, 10_000)
            .await?
            .into_iter()
            .skip(log_before)
            .map(|entry| entry.action.as_str().to_owned())
            .collect();
        assert_eq!(actions, ["work_days.deleted"]);
        assert_eq!(ok(&ada.send("GET", OLD, None).await?)?["total"], 0);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_09_working_hours_are_in_no_search_list_or_order() -> TestResult {
    each_database("priv_09_no_order", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let document = ada.send("GET", "/api/v1/openapi.json", None).await?;
        let paths = document.body["paths"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        let mut seen = 0;
        for (path, item) in paths.iter().filter(|(path, _)| path.contains("work-days")) {
            for operation in item.as_object().into_iter().flat_map(|item| item.values()) {
                seen += 1;
                let parameters = operation["parameters"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                for parameter in parameters {
                    let name = parameter["name"].as_str().unwrap_or_default();
                    assert!(
                        !["order", "sort", "sort_by", "order_by"].contains(&name),
                        "{path} takes an order"
                    );
                }
            }
        }
        assert!(seen >= 10, "{seen}");
        // The search language has no field of work days.
        let fields = ada.send("GET", "/api/v1/search/fields", None).await?;
        let text = fields.body.to_string();
        for word in ["workDay", "work_day", "breaks", "workingHours"] {
            assert!(!text.contains(word), "the search knows {word}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_12_an_access_key_never_reaches_its_owners_working_hours() -> TestResult {
    each_database("time_12_key_and_own_days", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let ada_id = id_of_person(&rig, ADA_EMAIL).await?;
        let made = ada
            .send(
                "PATCH",
                &format!("{MINE}/{YESTERDAY}"),
                Some(times(
                    YESTERDAY,
                    "07:00",
                    "15:00",
                    0,
                    Some("Typed in afterwards"),
                )),
            )
            .await?;
        ok(&made)?;
        let key = ada
            .send(
                "POST",
                "/api/v1/access-keys",
                Some(json!({"password": PASSWORD, "name": "Script", "choice": "everything"})),
            )
            .await?;
        let secret = ok(&key)?["secret"].as_str().ok_or("no secret")?.to_owned();
        // Neither by the address of one's own days nor by the address of the person.
        let own = ada
            .script(
                "GET",
                &format!("{MINE}?from={YESTERDAY}&to={TODAY}"),
                None,
                &secret,
            )
            .await?;
        assert_eq!(own.code(), "auth.access-key-not-allowed", "{}", own.body);
        let base = format!("/api/v1/people/{ada_id}/work-days");
        let read = ada
            .script(
                "GET",
                &format!("{base}?from={YESTERDAY}&to={TODAY}"),
                None,
                &secret,
            )
            .await?;
        assert_eq!(read.status, StatusCode::NOT_FOUND, "{}", read.body);
        let change = ada
            .script(
                "PATCH",
                &format!("{base}/{YESTERDAY}"),
                Some(times(YESTERDAY, "07:00", "18:00", 1, None)),
                &secret,
            )
            .await?;
        assert_eq!(change.status, StatusCode::NOT_FOUND, "{}", change.body);
        let history = ada
            .script("GET", &format!("{base}/{YESTERDAY}/history"), None, &secret)
            .await?;
        assert_eq!(history.status, StatusCode::NOT_FOUND);
        // Not even when the owner holds *HR*: no key holds the permissions of working hours.
        let hr = site.org.starting_roles().await?[StartingRole::Hr.code()];
        site.org.give_everywhere(&site.org.admin, hr).await?;
        let with_hr = ada
            .script(
                "GET",
                &format!("{base}?from={YESTERDAY}&to={TODAY}"),
                None,
                &secret,
            )
            .await?;
        assert_eq!(with_hr.status, StatusCode::NOT_FOUND);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_08_two_starts_at_the_same_moment_keep_one_work_day() -> TestResult {
    each_database("time_08_two_starts", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let mut phone = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let (first, second) = tokio::join!(start(&mut ada), start(&mut phone));
        let (first, second) = (first?, second?);
        let started = [&first, &second]
            .iter()
            .filter(|reply| reply.status == StatusCode::OK)
            .count();
        assert_eq!(started, 1, "{} / {}", first.body, second.body);
        let listed = mine(&mut ada, TODAY, TODAY).await?;
        let listed_days = days(&listed);
        assert_eq!(listed_days.len(), 1, "{}", listed.body);
        assert_eq!(listed_days[0]["periods"].as_array().map(Vec::len), Some(1));
        Ok(())
    })
    .await
}
