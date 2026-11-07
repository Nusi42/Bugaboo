//! The place of a time entry in a list tells nothing about when it was saved (PRIV-09, ADR-0035,
//! decision 8): for a reader who may not see that moment the history puts the rows of time by their
//! date, the list of the entries of an issue is by date, and the cursors hold no moment. Both
//! databases.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok};
use super::time_http::MY_ENTRIES;
use super::working_hours_http::long_sessions;

/// The rows of a history page that are comments or time, as `kind event`, and the cursor of the next
/// page.
fn rows_of(page: &Value) -> Vec<String> {
    page["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|item| item["kind"] == "comment" || item["kind"] == "time")
                .map(|item| format!("{} {}", item["kind"], item["event"]))
                .collect()
        })
        .unwrap_or_default()
}

/// The address of a page of a list: `limit` rows after the cursor, if there is one.
fn paged(path: &str, limit: u32, cursor: Option<&str>) -> String {
    match cursor {
        Some(text) => format!("{path}?limit={limit}&cursor={text}"),
        None => format!("{path}?limit={limit}"),
    }
}

/// Every row of the history of an issue, `limit` rows a page, in the order the pages give them.
async fn walked(browser: &mut Browser, issue: &str, limit: u32) -> Result<Vec<String>, Failure> {
    let mut all = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..50 {
        let path = paged(
            &format!("{ISSUES}/{issue}/history"),
            limit,
            cursor.as_deref(),
        );
        let reply = browser.send("GET", &path, None).await?;
        let page = ok(&reply)?;
        all.extend(rows_of(page));
        cursor = page["next_cursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            return Ok(all);
        }
    }
    Err("the history never ends".into())
}

async fn comment(browser: &mut Browser, issue: &str, text: &str) -> TestResult {
    let reply = browser
        .send(
            "POST",
            &format!("{ISSUES}/{issue}/comments"),
            Some(json!({"text": text})),
        )
        .await?;
    ok(&reply)?;
    Ok(())
}

async fn book_on(browser: &mut Browser, issue: &str, date: &str) -> TestResult {
    let reply = browser
        .send(
            "POST",
            MY_ENTRIES,
            Some(json!({"issue": issue, "date": date, "minutes": 30})),
        )
        .await?;
    ok(&reply)?;
    Ok(())
}

#[tokio::test]
async fn priv_09_a_time_row_is_placed_by_its_date_and_paging_loses_and_repeats_no_row() -> TestResult
{
    each_database("priv_09_history_order", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        long_sessions(&rig).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        // An event at 09:00, a time entry at 10:00 for yesterday, and an event at 11:00.
        rig.clock.advance(3600);
        comment(&mut ada, &issue, "first").await?;
        rig.clock.advance(3600);
        book_on(&mut ada, &issue, "2027-01-14").await?;
        rig.clock.advance(3600);
        comment(&mut ada, &issue, "second").await?;
        let (mut tom, _) = site.person(StartingRole::TeamMember, "tom").await?;
        // The booker sees the order things happened in.
        let own = walked(&mut ada, &issue, 100).await?;
        let kinds_of = |rows: &[String]| -> Vec<String> {
            rows.iter()
                .map(|row| row.split(' ').next().unwrap_or_default().to_owned())
                .collect()
        };
        assert_eq!(
            kinds_of(&own),
            [r#""comment""#, r#""time""#, r#""comment""#],
            "{own:?}"
        );
        // Another reader gets the time row where its date puts it, after both events of the later
        // day, so its place says nothing of the hour.
        let other = walked(&mut tom, &issue, 100).await?;
        assert_eq!(
            kinds_of(&other),
            [r#""comment""#, r#""comment""#, r#""time""#],
            "{other:?}"
        );
        // Pages of any length give the same rows once each, whatever the boundary falls on.
        for limit in 1..=3 {
            assert_eq!(
                walked(&mut tom, &issue, limit).await?,
                other,
                "limit {limit}"
            );
            assert_eq!(walked(&mut ada, &issue, limit).await?, own, "limit {limit}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_09_the_entries_of_an_issue_are_listed_by_date_with_a_cursor_of_no_moment()
-> TestResult {
    each_database("priv_09_entries_order", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        long_sessions(&rig).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        // The later date is saved first.
        book_on(&mut ada, &issue, "2027-01-14").await?;
        rig.clock.advance(3600);
        book_on(&mut ada, &issue, "2027-01-12").await?;
        rig.clock.advance(3600);
        book_on(&mut ada, &issue, "2027-01-13").await?;
        let (mut tom, _) = site.person(StartingRole::TeamMember, "tom").await?;
        let path = format!("{ISSUES}/{issue}/time-entries");
        let all = tom.send("GET", &path, None).await?;
        let dates: Vec<Value> = ok(&all)?["items"]
            .as_array()
            .map(|items| items.iter().map(|item| item["date"].clone()).collect())
            .unwrap_or_default();
        assert_eq!(
            dates,
            [
                json!("2027-01-14"),
                json!("2027-01-13"),
                json!("2027-01-12")
            ]
        );
        // Paged one at a time: each entry once, in the same order (the cursor is a count, sealed by the API).
        let mut seen = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..5 {
            let reply = tom
                .send("GET", &paged(&path, 1, cursor.as_deref()), None)
                .await?;
            let page = ok(&reply)?;
            seen.extend(page["items"].as_array().cloned().unwrap_or_default());
            cursor = page["next_cursor"].as_str().map(str::to_owned);
            if cursor.is_none() {
                break;
            }
        }
        let paged: Vec<Value> = seen.iter().map(|item| item["date"].clone()).collect();
        assert_eq!(paged, dates);
        Ok(())
    })
    .await
}
