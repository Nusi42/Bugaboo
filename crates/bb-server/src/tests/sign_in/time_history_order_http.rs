//! The place of a time entry in a list tells nothing about when it was saved (PRIV-09, ADR-0035,
//! decision 8): for a reader who may not see that moment the history puts the rows of time by their
//! date, the list of the entries of an issue is by date, and the cursors hold no moment. Both
//! databases.

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_app::access::StartingRole;
use bb_app::history::opaque_id;
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
        let (mut tom, _) = site.person(StartingRole::TeamMember, "tom").await?;
        // Events of Ada at 09:00 and 11:00 with a time entry at 10:00 for yesterday between them,
        // and a comment of Tom's half an hour after the first and the second, so that Tom has rows
        // of his own as well as the rows of others.
        rig.clock.advance(3600);
        comment(&mut ada, &issue, "first").await?;
        rig.clock.advance(1800);
        comment(&mut tom, &issue, "Tom one").await?;
        rig.clock.advance(1800);
        book_on(&mut ada, &issue, "2027-01-14").await?;
        rig.clock.advance(1800);
        comment(&mut tom, &issue, "Tom two").await?;
        rig.clock.advance(1800);
        comment(&mut ada, &issue, "second").await?;
        // The booker sees the order things happened in.
        let own = walked(&mut ada, &issue, 100).await?;
        let kinds_of = |rows: &[String]| -> Vec<String> {
            rows.iter()
                .map(|row| row.split(' ').next().unwrap_or_default().to_owned())
                .collect()
        };
        assert_eq!(
            kinds_of(&own),
            [
                r#""comment""#,
                r#""comment""#,
                r#""time""#,
                r#""comment""#,
                r#""comment""#
            ],
            "{own:?}"
        );
        // Another reader gets the time row where its date puts it, after both events of the later
        // day, so its place says nothing of the hour.
        let other = walked(&mut tom, &issue, 100).await?;
        assert_eq!(
            kinds_of(&other),
            [
                r#""comment""#,
                r#""comment""#,
                r#""comment""#,
                r#""comment""#,
                r#""time""#
            ],
            "{other:?}"
        );
        // Pages of any length give the same rows once each, whatever the boundary falls on, with
        // Tom's own rows and the rows of others both in the pages.
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

/// What a cursor says: the position inside it, after the tag the server puts in front of it.
fn position_in(cursor: &str) -> Result<String, Failure> {
    let bytes = Base64UrlUnpadded::decode_vec(cursor).map_err(|error| error.to_string())?;
    let position = bytes.get(16..).ok_or("a cursor shorter than its tag")?;
    Ok(String::from_utf8(position.to_vec())?)
}

const DAY_MICROS: i64 = 86_400_000_000;

#[tokio::test]
async fn priv_09_a_cursor_of_such_a_reader_holds_a_shown_moment_and_a_hash_and_nothing_else()
-> TestResult {
    each_database("priv_09_cursor_content", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        long_sessions(&rig).await?;
        let issue = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        rig.clock.advance(3600);
        comment(&mut ada, &issue, "first").await?;
        rig.clock.advance(3600);
        book_on(&mut ada, &issue, "2027-01-14").await?;
        rig.clock.advance(3600);
        comment(&mut ada, &issue, "second").await?;
        let (mut tom, _) = site.person(StartingRole::TeamMember, "tom").await?;
        let history = format!("{ISSUES}/{issue}/history");
        let mut cursor: Option<String> = None;
        let mut checked = 0;
        for _ in 0..20 {
            let reply = tom
                .send("GET", &paged(&history, 1, cursor.as_deref()), None)
                .await?;
            let page = ok(&reply)?;
            cursor = page["next_cursor"].as_str().map(str::to_owned);
            let (Some(text), Some(last)) = (
                &cursor,
                page["items"].as_array().and_then(|items| items.last()),
            ) else {
                break;
            };
            // The position is `moment:hash`, the moment the row is shown at (microseconds) and the
            // hash of the ID of its event: no place in the outbox, no other number.
            let position = position_in(text)?;
            let (moment, hash) = position.split_once(':').ok_or("no moment and hash")?;
            let micros: i64 = moment.parse()?;
            let shown = jiff::Timestamp::from_microsecond(micros)?;
            assert_eq!(
                shown.to_string(),
                last["at"].as_str().unwrap_or_default(),
                "{position}"
            );
            let event = last["event"].as_str().unwrap_or_default();
            let expected = if last["kind"] == "time" {
                event.to_owned()
            } else {
                opaque_id(event)
            };
            assert_eq!(hash, expected, "{position}");
            if last["kind"] == "time" {
                // A time row is at the start of its date, not at the moment it was saved.
                assert_eq!(micros % DAY_MICROS, 0, "{position}");
            }
            assert_eq!(position.matches(':').count(), 1, "{position}");
            checked += 1;
        }
        assert!(checked >= 2, "{checked}");
        // The list of the entries of the issue: the date and the hash of the entry.
        let entries = format!("{ISSUES}/{issue}/time-entries");
        book_on(&mut ada, &issue, "2027-01-13").await?;
        let reply = tom.send("GET", &paged(&entries, 1, None), None).await?;
        let page = ok(&reply)?;
        let position = position_in(page["next_cursor"].as_str().ok_or("no next page")?)?;
        let (day, hash) = position.split_once(':').ok_or("no date and hash")?;
        assert_eq!(day, page["items"][0]["date"].as_str().unwrap_or_default());
        assert_eq!(hash, page["items"][0]["id"].as_str().unwrap_or_default());
        Ok(())
    })
    .await
}
