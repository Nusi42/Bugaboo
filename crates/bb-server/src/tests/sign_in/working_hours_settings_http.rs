//! The settings of working hours through the routes of settings (TIME-09, TIME-10, TIME-12,
//! ADR-0035, decisions 7 and 8): the longest day that the next change of a day is checked against,
//! *Totals only* in the security log and in what others see, and the keep period that stays within
//! the law. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::http::rig_and_app;
use super::issues_site::{Site, ok};
use super::working_hours_http::{MINE, TODAY, YESTERDAY, hana_and_bob, long_sessions, times};

#[tokio::test]
async fn time_09_an_administrator_changes_the_longest_day_and_the_next_change_of_a_day_warns_by_it()
-> TestResult {
    each_database("time_09_longest_day", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        // A day of nine and a half hours: within the 10 hours of the preset, so no warning.
        let path = format!("{MINE}/{YESTERDAY}");
        let long = times(YESTERDAY, "06:00", "15:30", 0, Some("Typed in afterwards"));
        let first = ada.send("PATCH", &path, Some(long.clone())).await?;
        let first = ok(&first)?.clone();
        let longest = |day: &Value| {
            day["warnings"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|warning| warning["warning"]["rule"] == "long-day")
        };
        assert!(!longest(&first), "{first}");
        // The administrator lowers the longest day to nine hours, in lengths of minutes.
        let changed = ada
            .send(
                "PUT",
                "/api/v1/settings/working-hours.day-minutes-max",
                Some(json!({"value": 540})),
            )
            .await?;
        assert_eq!(ok(&changed)?["value"], 540);
        assert_eq!(ok(&changed)?["unit"], "minutes");
        // The next change of the day is checked against the new number.
        rig.clock.advance(1);
        let again = ada
            .send(
                "PATCH",
                &path,
                Some(times(
                    YESTERDAY,
                    "06:00",
                    "15:30",
                    first["version"].as_i64().unwrap_or(0),
                    Some("Added the late call"),
                )),
            )
            .await?;
        let again = ok(&again)?;
        assert!(longest(again), "{again}");
        // A warning never blocks what really happened: the day is saved.
        assert_eq!(again["totals"]["worked"], 570);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_12_turning_on_totals_only_is_in_the_security_log_and_others_see_totals_only()
-> TestResult {
    each_database("time_12_totals_only_setting", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        long_sessions(&rig).await?;
        let (mut hana, _bob, bob_id) = hana_and_bob(&rig, &site).await?;
        let base = format!("/api/v1/people/{bob_id}/work-days?from={YESTERDAY}&to={TODAY}");
        let before = ok(&hana.send("GET", &base, None).await?)?.clone();
        assert_eq!(before["totals_only"], false);
        assert_eq!(before["days"].as_array().map(Vec::len), Some(1));
        let log = rig.database.security_log_entries(0, 10_000).await?.len();
        let on = ada
            .send(
                "PUT",
                "/api/v1/settings/working-hours.totals-only",
                Some(json!({"value": true})),
            )
            .await?;
        ok(&on)?;
        let actions: Vec<String> = rig
            .database
            .security_log_entries(0, 10_000)
            .await?
            .into_iter()
            .skip(log)
            .map(|entry| entry.action.as_str().to_owned())
            .collect();
        assert_eq!(actions, ["setting.changed"]);
        // Others with the permission now read the week total and nothing else.
        let after = ok(&hana.send("GET", &base, None).await?)?.clone();
        assert_eq!(after["totals_only"], true);
        assert_eq!(after["days"], json!([]));
        // The week that runs shows no total, even for a day that has ended.
        assert_eq!(after["weeks"], json!([]));
        // Turning it off again is in the log too.
        let off = ada
            .send("DELETE", "/api/v1/settings/working-hours.totals-only", None)
            .await?;
        ok(&off)?;
        let entries = rig.database.security_log_entries(0, 10_000).await?.len();
        assert_eq!(entries, log + 2);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_10_the_keep_period_is_refused_below_two_years_through_the_route() -> TestResult {
    each_database("time_10_keep_route", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let keep = "/api/v1/settings/working-hours.keep-years";
        let refused = ada.send("PUT", keep, Some(json!({"value": 1}))).await?;
        assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{}", refused.body);
        assert_eq!(refused.code(), "working-hours.keep-years-too-short");
        let before = rig.database.security_log_entries(0, 10_000).await?.len();
        // Without rules the organization chooses; a refused change leaves no entry.
        ok(&ada
            .send(
                "PUT",
                "/api/v1/settings/working-hours.preset",
                Some(json!({"value": "no-rules"})),
            )
            .await?)?;
        let short = ada.send("PUT", keep, Some(json!({"value": 1}))).await?;
        assert_eq!(ok(&short)?["value"], 1);
        assert_eq!(
            rig.database.security_log_entries(0, 10_000).await?.len(),
            before + 2
        );
        Ok(())
    })
    .await
}
