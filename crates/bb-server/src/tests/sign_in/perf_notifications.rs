//! No N+1 queries in the notifications (PERF-05, M4 WP-10): a page of forty notifications, and the
//! number on the menu for forty unread ones, run the same number of statements as five. The sample
//! showed it: the page of a person with thousands of notifications took longer than the issue page
//! (the issue, its workflow and the comment of every row were read one row after the other).
//! Every flow runs on SQLite and PostgreSQL, and the statements are counted by the database itself.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok, version_of};
use super::perf_queries::{Counter, assert_as_many_statements, five_and_forty};

const NOTIFICATIONS: &str = "/api/v1/notifications";
/// The most statements the page or the number may run, however many rows there are (a ceiling that a
/// change to the code has to notice; the exact numbers are compared apart). Most of them are the
/// session and the rights of the person, which every read pays.
const AT_MOST: usize = 42;

/// The most statements the reader runs to catch up with one change of an issue, with two followers (the
/// kinds of news a person wants are decided before their rights are looked at).
const READER_AT_MOST: usize = 9;

/// The most statements the reader runs to catch up with one change when its follower wants the news.
const CHANGE_AT_MOST: usize = 23;

/// The most statements one more change of an issue costs the reader when its follower wants the
/// news: the issue and its followers are read, the rights of the follower on it are not read again.
const PER_CHANGE_AT_MOST: usize = 3;

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
        let path = |limit: u32| format!("{NOTIFICATIONS}?limit={limit}");
        let (on_five, on_forty) = five_and_forty(&mut ada, &rig.database, path).await?;
        assert_as_many_statements(on_five, on_forty, AT_MOST, "a longer page");
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
        let mut counter = Counter::on(&rig.database);
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
        assert_as_many_statements(on_five, on_forty, AT_MOST, "more unread notifications");
        Ok(())
    })
    .await
}

/// Makes a new issue that `followers` more people follow, and changes it `changes` times as Ada
/// (`status` false: the title, which followers do not want to hear of unless they turned it on;
/// true: the status, which they do). The statements the reader runs to catch up with the changes.
async fn statements_to_hear(
    rig: &super::rig::Rig,
    site: &Site<'_>,
    ada: &mut Browser,
    followers: usize,
    changes: usize,
    status: bool,
) -> Result<usize, Failure> {
    let issue = key_of(&site.create(ada, "An issue to follow").await?)?;
    for number in 0..followers {
        let (mut person, _) = site
            .person_called(
                StartingRole::TeamMember,
                &format!("f{followers}x{changes}x{number}"),
                "Some Follower",
            )
            .await?;
        let followed = person
            .send("PUT", &format!("{ISSUES}/{issue}/follow"), None)
            .await?;
        assert!(followed.status.is_success(), "{}", followed.body);
    }
    // Everything before the changes is read, and not counted.
    catch_up(rig).await?;
    for round in 0..changes {
        let version = version_of(ada, &issue).await?;
        let body = if status {
            let to = if round % 2 == 0 {
                "in-progress"
            } else {
                "open"
            };
            json!({"version": version, "status": to})
        } else {
            json!({"version": version, "title": format!("A better title {round}")})
        };
        let changed = ada
            .send("PATCH", &format!("{ISSUES}/{issue}"), Some(body))
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
    }
    let mut counter = Counter::on(&rig.database);
    let read = catch_up(rig).await?;
    assert!(read > 0, "the changes were not read");
    Ok(counter.take())
}

/// Lets the reader of notifications catch up with the events; how many events it read.
async fn catch_up(rig: &super::rig::Rig) -> Result<u64, Failure> {
    Ok(rig
        .services
        .ctx(bb_app::auth::RequestInfo::default())
        .catch_up_notifications()
        .await
        .map_err(|error| error.to_string())?)
}

#[tokio::test]
async fn perf_05_the_reader_runs_as_many_statements_for_many_followers_who_want_nothing_as_for_few()
-> TestResult {
    each_database("perf_05_reader_wants_first", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let on_few = statements_to_hear(&rig, &site, &mut ada, 2, 1, false).await?;
        let on_many = statements_to_hear(&rig, &site, &mut ada, 10, 1, false).await?;
        // Whoever wants none of the kinds of news a change makes costs no look at their rights, and
        // what all of them chose is read in one statement.
        assert_as_many_statements(on_few, on_many, READER_AT_MOST, "more followers");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_the_rights_of_a_follower_on_an_issue_are_read_once_for_all_its_changes()
-> TestResult {
    each_database("perf_05_reader_rights_once", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // One follower who wants the news of a status; one change, then six of one issue.
        let on_one = statements_to_hear(&rig, &site, &mut ada, 1, 1, true).await?;
        let on_six = statements_to_hear(&rig, &site, &mut ada, 1, 6, true).await?;
        assert!(on_one <= CHANGE_AT_MOST, "{on_one} statements");
        let more = (on_six - on_one) / 5;
        assert!(
            more <= PER_CHANGE_AT_MOST,
            "{more} statements for each change ({on_one} for one, {on_six} for six)"
        );
        Ok(())
    })
    .await
}
