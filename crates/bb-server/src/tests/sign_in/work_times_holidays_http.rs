//! The next public holidays in the answer of work times (PLAN-11, ADR-0030, amendment M8): the three
//! that come next for the choice of *Holidays*, counted from the person's own today, which is the
//! day of their time zone, not of the server. Runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_app::context::Clock;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{Site, ok};
use super::work_times_http::{MINE, RANGE};

/// Chooses a value of one of the person's own settings, then reads their work times.
async fn choose(browser: &mut Browser, key: &str, value: &str) -> Result<Reply, Failure> {
    let path = format!("/api/v1/me/settings/{key}");
    ok(&browser
        .send("PUT", &path, Some(json!({"value": value})))
        .await?)?;
    browser.send("GET", &format!("{MINE}?{RANGE}"), None).await
}

/// The dates of the next holidays of an answer of work times.
fn next_dates(reply: &Reply) -> Vec<Value> {
    reply.body["next_holidays"]
        .as_array()
        .map(|days| days.iter().map(|day| day["date"].clone()).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn plan_11_the_work_times_name_the_next_three_holidays_of_the_choice() -> TestResult {
    each_database("plan_11_next_holidays", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        // Bavaria: the three days that come next from today, with what each is called.
        let read = choose(&mut bob, "work-times.holidays", "de-by").await?;
        let today = bb_core::time::day::date_of(rig.clock.now(), &jiff::tz::TimeZone::UTC);
        let expected: Vec<Value> = bb_core::holidays::Region::Bavaria
            .next_holidays(today, bb_app::work_times::NEXT_HOLIDAYS)
            .iter()
            .map(|day| json!({"date": day.date.to_string(), "holiday": day.holiday.code()}))
            .collect();
        assert_eq!(expected.len(), 3);
        assert_eq!(ok(&read)?["next_holidays"], json!(expected));
        // *None* names none.
        let none = choose(&mut bob, "work-times.holidays", "none").await?;
        assert_eq!(ok(&none)?["next_holidays"], json!([]));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_11_the_next_holidays_count_from_the_persons_own_today() -> TestResult {
    each_database("plan_11_next_holidays_far_zone", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        // Noon of Christmas Day in UTC: in Kiritimati (UTC+14) it is already Boxing Day.
        let noon: jiff::Timestamp = "2026-12-25T12:00:00Z".parse()?;
        rig.clock
            .advance(noon.as_second() - rig.clock.now().as_second());
        let in_utc = choose(&mut bob, "work-times.holidays", "de").await?;
        assert_eq!(next_dates(&in_utc)[0], "2026-12-25", "{}", in_utc.body);
        let far = choose(&mut bob, "region.time-zone", "Pacific/Kiritimati").await?;
        let dates = next_dates(&far);
        assert_eq!(dates[0], "2026-12-26", "{}", far.body);
        assert_eq!(dates[1], "2027-01-01", "{}", far.body);
        Ok(())
    })
    .await
}
