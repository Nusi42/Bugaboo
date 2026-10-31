//! No N+1 queries in the lists of time entries (PERF-05, PERF-N1): a person's own entries and the
//! entries on an issue run as many statements for a page of forty as for a page of five, on SQLite
//! and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::rig_and_app;
use super::issues_site::{ISSUES, Site, key_of, ok};
use super::perf_queries::{LIST_AT_MOST, assert_as_many_statements, five_and_forty};
use super::time_http::MY_ENTRIES;

/// How many entries the lists of the test hold: more than the longer page.
const ENTRIES: usize = 41;

#[tokio::test]
async fn perf_05_a_page_of_forty_time_entries_runs_as_many_statements_as_a_page_of_five()
-> TestResult {
    each_database("perf_05_time_entries", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let shared = key_of(&site.create(&mut ada, "Printer jams").await?)?;
        for number in 0..ENTRIES {
            // Ada books on an issue of its own each time; two people book on the shared issue.
            let own = key_of(&site.create(&mut ada, &format!("Issue {number}")).await?)?;
            let booking =
                |issue: &str| json!({"issue": issue, "date": "2027-01-14", "minutes": 10 + number});
            ok(&ada.send("POST", MY_ENTRIES, Some(booking(&own))).await?)?;
            let booker = if number % 2 == 0 { &mut ada } else { &mut bob };
            ok(&booker
                .send("POST", MY_ENTRIES, Some(booking(&shared)))
                .await?)?;
        }
        let (on_five, on_forty) = five_and_forty(&mut ada, &rig.database, |limit| {
            format!("{MY_ENTRIES}?from=2027-01-11&to=2027-01-17&limit={limit}")
        })
        .await?;
        assert_as_many_statements(on_five, on_forty, LIST_AT_MOST, "a person's own entries");
        let (on_five, on_forty) = five_and_forty(&mut ada, &rig.database, |limit| {
            format!("{ISSUES}/{shared}/time-entries?limit={limit}")
        })
        .await?;
        assert_as_many_statements(on_five, on_forty, LIST_AT_MOST, "the entries on an issue");
        Ok(())
    })
    .await
}
