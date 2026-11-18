//! *Keep and delete rules* over HTTP on both databases (PRIV-05, ADR-0045): the rules with their
//! periods, and time entries past their keep period, which only an administrator deletes, with the
//! events that carry their note.

use axum::Router;
use bb_app::auth::RequestInfo;
use bb_core::event::{EntityKind, RecordKind};
use bb_core::time::SECONDS_PER_DAY;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{ADA_EMAIL, Browser, Reply, rig_and_app};
use super::issues_site::{Site, key_of, ok};
use super::rig::Rig;
use super::time_http::MY_ENTRIES;

const RULES: &str = "/api/v1/admin/keep-rules";
const PAST: &str = "/api/v1/admin/keep-rules/time-entries/past";
const DELETE_PAST: &str = "/api/v1/admin/keep-rules/time-entries/delete-past";

/// *Delete them* for the time entries an administrator counted: the count says before which date
/// they are past their period, and the deletion names that date.
pub(super) async fn delete_counted(browser: &mut Browser) -> Result<Reply, Failure> {
    let counted = ok(&browser.send("GET", PAST, None).await?)?.clone();
    let before = counted["before"].as_str().ok_or("no date in the count")?;
    browser
        .send("POST", &format!("{DELETE_PAST}?before={before}"), None)
        .await
}

/// Ada, with a time entry of 2027-01-14 on a new issue: the answer of the entry.
async fn an_entry(rig: &Rig, app: &Router) -> Result<(Browser, Value), Failure> {
    let (site, mut ada) = Site::start(rig, app).await?;
    let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
    let booked = ada
        .send(
            "POST",
            MY_ENTRIES,
            Some(json!({"issue": issue, "date": "2027-01-14", "minutes": 30})),
        )
        .await?;
    let entry = ok(&booked)?.clone();
    Ok((ada, entry))
}

/// A note a time entry carries, to find in the events.
const NOTE: &str = "Call with the customer about the invoice";

#[tokio::test]
async fn priv_05_the_rules_list_every_kind_with_its_period_and_group() -> TestResult {
    each_database("priv_05_rules", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let rules = ok(&ada.send("GET", RULES, None).await?)?.clone();
        let rows = rules["rules"].as_array().cloned().unwrap_or_default();
        assert_eq!(rows.len(), bb_core::keeping::KEEP_KINDS.len());
        let time = rows
            .iter()
            .find(|row| row["kind"] == "time-entries")
            .ok_or("no row of time entries")?;
        assert_eq!(time["group"], "by-an-administrator");
        assert_eq!(time["period"], json!({"count": 10, "unit": "years"}));
        assert_eq!(time["setting"], "keeping.time-entry-years");
        // A kind Bugaboo deletes by itself has no count of what is past its period.
        let unknown = ada
            .send("GET", &format!("{RULES}/trash/past"), None)
            .await?;
        assert_eq!(unknown.code(), "resource.not-found");
        let nothing = ada
            .send("GET", &format!("{RULES}/nothing/past"), None)
            .await?;
        assert_eq!(nothing.code(), "resource.not-found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_05_time_entries_are_deleted_only_when_an_administrator_says_so() -> TestResult {
    each_database("priv_05_old_time_entries", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (mut ada, entry) = an_entry(&rig, &app).await?;
        let id = entry["id"].as_str().ok_or("no ID of the entry")?;
        // A change of the note puts the note into an event.
        let changed = ada
            .send(
                "PATCH",
                &format!("{MY_ENTRIES}/{id}"),
                Some(json!({"version": entry["version"], "entry": {"note": NOTE}})),
            )
            .await?;
        ok(&changed)?;
        let kind = EntityKind::from(RecordKind::TimeEntry);
        let carries_the_note = async || -> Result<bool, Failure> {
            Ok(rig
                .database
                .events_of_kind(&kind, 1000)
                .await?
                .iter()
                .any(|event| {
                    serde_json::to_string(&event.changes)
                        .unwrap_or_default()
                        .contains(NOTE)
                }))
        };
        assert!(
            carries_the_note().await?,
            "the event of the entry holds its note"
        );
        // Eleven years later the entry is past its period, and no job deletes it.
        rig.clock.advance(11 * 366 * SECONDS_PER_DAY);
        rig.services
            .ctx(RequestInfo::default())
            .start_daily_jobs()
            .await
            .map_err(|error| error.to_string())?;
        rig.run_jobs().await?;
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let counted = ok(&ada.send("GET", PAST, None).await?)?.clone();
        assert_eq!(counted["total"], 1, "{counted}");
        assert_eq!(counted["period"], json!({"count": 10, "unit": "years"}));
        let log_before = rig.database.security_log_entries(0, 10_000).await?.len();
        let deleted = delete_counted(&mut ada).await?;
        assert_eq!(ok(&deleted)?["deleted"], 1);
        // The entry went with the events that carry its note, and the deletion is in the log.
        assert!(!carries_the_note().await?, "an event still holds the note");
        assert_eq!(ok(&ada.send("GET", PAST, None).await?)?["total"], 0);
        let actions: Vec<String> = rig
            .database
            .security_log_entries(0, 10_000)
            .await?
            .into_iter()
            .skip(log_before)
            .map(|entry| entry.action.as_str().to_owned())
            .collect();
        assert_eq!(actions, ["time_entries.deleted"]);
        let again = delete_counted(&mut ada).await?;
        assert_eq!(ok(&again)?["deleted"], 0, "safe to say again");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_05_delete_them_deletes_only_what_the_administrator_counted() -> TestResult {
    each_database("priv_05_delete_what_was_counted", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        an_entry(&rig, &app).await?;
        rig.clock.advance(11 * 366 * SECONDS_PER_DAY);
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let counted = |reply: &Reply| -> Result<String, Failure> {
            Ok(ok(reply)?["before"]
                .as_str()
                .ok_or("no date in the count")?
                .to_owned())
        };
        let delete = async |browser: &mut Browser, before: &str| {
            browser
                .send("POST", &format!("{DELETE_PAST}?before={before}"), None)
                .await
        };
        // The administrator counts; then the period is made shorter before they say *Delete them*.
        let seen = counted(&ada.send("GET", PAST, None).await?)?;
        let shorter = ada
            .send(
                "PUT",
                "/api/v1/settings/keeping.time-entry-years",
                Some(json!({"value": 5})),
            )
            .await?;
        ok(&shorter)?;
        let refused = delete(&mut ada, &seen).await?;
        assert_eq!(refused.code(), "keeping.past-changed", "{}", refused.body);
        assert_eq!(ok(&ada.send("GET", PAST, None).await?)?["total"], 1);
        // Counted again, and midnight passes before *Delete them*.
        let seen = counted(&ada.send("GET", PAST, None).await?)?;
        rig.clock.advance(SECONDS_PER_DAY);
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let refused = delete(&mut ada, &seen).await?;
        assert_eq!(refused.code(), "keeping.past-changed", "{}", refused.body);
        // Counted once more and deleted at once: it goes.
        assert_eq!(ok(&delete_counted(&mut ada).await?)?["deleted"], 1);
        // A date that is no date is refused.
        let wrong = delete(&mut ada, "soon").await?;
        assert_eq!(wrong.code(), "request.invalid", "{}", wrong.body);
        Ok(())
    })
    .await
}
