//! No N+1 queries in the notifications (PERF-05, M4 WP-10): a page of forty notifications, and the
//! number on the menu for forty unread ones, run the same number of statements as five. The sample
//! showed it: the page of a person with thousands of notifications took longer than the issue page
//! (the issue, its workflow and the comment of every row were read one row after the other).
//! Every flow runs on SQLite and PostgreSQL, and the statements are counted by the database itself.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_store::Database;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok, version_of};

const NOTIFICATIONS: &str = "/api/v1/notifications";
/// The most statements the page or the number may run, however many rows there are (a ceiling that a
/// change to the code has to notice; the exact numbers are compared apart). Most of them are the
/// session and the rights of the person, which every read pays.
const AT_MOST: usize = 60;

/// The statements the database ran since the last time this was asked.
struct Counter<'a> {
    database: &'a Database,
    before: u64,
}

impl Counter<'_> {
    fn take(&mut self) -> usize {
        let now = self.database.statements_run();
        let ran = now - self.before;
        self.before = now;
        usize::try_from(ran).unwrap_or(usize::MAX)
    }
}

/// Bob finishes `count` new issues of Ada, one comment under each, so Ada, who follows what she
/// reported, hears of `count` different issues.
async fn news_for_ada(
    site: &Site<'_>,
    ada: &mut Browser,
    bob: &mut Browser,
    count: usize,
    first: usize,
) -> Result<(), Failure> {
    for number in first..first + count {
        let issue = key_of(&site.create(ada, &format!("Issue {number}")).await?)?;
        let version = version_of(bob, &issue).await?;
        let changed = bob
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": version, "status": "done"})),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        // The same words in every comment: a page names the same few things over and over and looks
        // each of them up once; what is different in every row must cost nothing.
        let written = bob
            .send(
                "POST",
                &format!("{ISSUES}/{issue}/comments"),
                Some(json!({"text": "A word about it."})),
            )
            .await?;
        ok(&written)?;
    }
    Ok(())
}

#[tokio::test]
async fn perf_05_a_page_of_forty_notifications_runs_as_many_statements_as_a_page_of_five()
-> TestResult {
    each_database("perf_05_notifications_page", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        news_for_ada(&site, &mut ada, &mut bob, 40, 0).await?;
        let mut counter = Counter {
            database: &rig.database,
            before: rig.database.statements_run(),
        };
        let path = |limit: u32| format!("{NOTIFICATIONS}?limit={limit}");
        // The first read catches the reader up with the events and warms the caches: not counted.
        ok(&ada.send("GET", &path(5), None).await?)?;
        counter.take();
        let five = ada.send("GET", &path(5), None).await?;
        assert_eq!(ok(&five)?["items"].as_array().map(Vec::len), Some(5));
        let on_five = counter.take();
        let forty = ada.send("GET", &path(40), None).await?;
        assert_eq!(ok(&forty)?["items"].as_array().map(Vec::len), Some(40));
        let on_forty = counter.take();
        assert!(on_five > 0, "the counter sees nothing");
        assert_eq!(
            on_five, on_forty,
            "a longer page must not run more statements"
        );
        assert!(on_forty <= AT_MOST, "{on_forty} statements");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_the_number_on_the_menu_runs_as_many_statements_for_forty_unread_as_for_five()
-> TestResult {
    each_database("perf_05_notifications_unread", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let unread = format!("{NOTIFICATIONS}/unread");
        let mut counter = Counter {
            database: &rig.database,
            before: rig.database.statements_run(),
        };
        news_for_ada(&site, &mut ada, &mut bob, 5, 0).await?;
        ok(&ada.send("GET", &unread, None).await?)?;
        counter.take();
        let few = ada.send("GET", &unread, None).await?;
        assert_eq!(ok(&few)?["unread"], 10);
        let on_five = counter.take();
        news_for_ada(&site, &mut ada, &mut bob, 35, 5).await?;
        ok(&ada.send("GET", &unread, None).await?)?;
        counter.take();
        let many = ada.send("GET", &unread, None).await?;
        assert_eq!(ok(&many)?["unread"], 80);
        let on_forty = counter.take();
        assert!(on_five > 0, "the counter sees nothing");
        assert_eq!(
            on_five, on_forty,
            "more unread notifications must not run more statements"
        );
        assert!(on_forty <= AT_MOST, "{on_forty} statements");
        Ok(())
    })
    .await
}
