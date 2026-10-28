//! No N+1 queries in the notifications (PERF-05, M4 WP-10): a page of forty notifications, and the
//! number on the menu for forty unread ones, run the same number of statements as five. The sample
//! showed it: the page of a person with thousands of notifications took longer than the issue page
//! (the issue, its workflow and the comment of every row were read one row after the other).
//! Every flow runs on SQLite and PostgreSQL, and the statements are counted by the database itself.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestDatabase, TestResult, each_database};
use serde_json::json;

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok, version_of};
use super::perf_queries::{
    Counter, assert_as_many_statements, assert_same_statements, five_and_forty,
};

const NOTIFICATIONS: &str = "/api/v1/notifications";
/// The most statements the page or the number may run, however many rows there are (a ceiling that a
/// change to the code has to notice; the exact numbers are compared apart). Most of them are the
/// session and the rights of the person, which every read pays (31 measured, 42 before the rights
/// of the person were read once, M4-F137).
const AT_MOST: usize = 31;

/// The same for the number over many projects: the rights in each project of the page are decided
/// from the tree (36 measured, 39 on main).
const PROJECTS_AT_MOST: usize = 36;

/// The most statements the reader runs to catch up with one change of an issue, with two followers (the
/// kinds of news a person wants are decided before their rights are looked at).
const READER_AT_MOST: usize = 9;

/// The most statements the reader runs to catch up with one change when its follower wants the news.
const CHANGE_AT_MOST: usize = 23;

/// The most statements one more change of an issue costs the reader when its follower wants the
/// news: the issue and its followers are read, the rights of the follower on it are not read again.
const PER_CHANGE_AT_MOST: usize = 3;

/// What Bob does to each new issue of Ada, which is what Ada hears of.
#[derive(Clone, Copy)]
enum News {
    /// A comment under the issue (a row that shows the comment).
    Comment,
    /// A change of the status (a row that shows what changed).
    StatusChange,
}

/// Bob does one thing to each of `count` new issues of Ada, so Ada, who follows what she reported,
/// hears of `count` different issues, one notification each. Every notification is of one kind: the
/// reader makes them all in one request, in one moment, and IDs sort only by the millisecond, so the
/// order of the newest few is chance; a page of five that happened to hold no comment read a
/// statement and a setting fewer than a page of forty (M5-F65, 42 against 40). Both kinds are
/// counted, each on its own.
async fn news_for_ada(
    site: &Site<'_>,
    ada: &mut Browser,
    bob: &mut Browser,
    news: News,
    count: usize,
    first: usize,
) -> Result<(), Failure> {
    for number in first..first + count {
        let issue = key_of(&site.create(ada, &format!("Issue {number}")).await?)?;
        match news {
            News::StatusChange => {
                let version = version_of(bob, &issue).await?;
                let changed = bob
                    .send(
                        "PATCH",
                        &format!("{ISSUES}/{issue}"),
                        Some(json!({"version": version, "status": "done"})),
                    )
                    .await?;
                assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
            }
            News::Comment => {
                // The same words in every comment: a page names the same few things over and over and
                // looks each of them up once; what is different in every row must cost nothing.
                let written = bob
                    .send(
                        "POST",
                        &format!("{ISSUES}/{issue}/comments"),
                        Some(json!({"text": "A word about it."})),
                    )
                    .await?;
                ok(&written)?;
            }
        }
    }
    Ok(())
}

/// A page of forty rows of one kind runs as many statements as a page of five.
async fn forty_run_as_many_as_five(db: &TestDatabase, news: News) -> TestResult {
    let (rig, app) = rig_and_app(db).await?;
    let (site, mut ada) = Site::start(&rig, &app).await?;
    let (mut bob, _) = site
        .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
        .await?;
    news_for_ada(&site, &mut ada, &mut bob, news, 40, 0).await?;
    let path = |limit: u32| format!("{NOTIFICATIONS}?limit={limit}");
    let (on_five, on_forty) = five_and_forty(&mut ada, &rig.database, path).await?;
    assert_as_many_statements(on_five, on_forty, AT_MOST, "a longer page");
    Ok(())
}

#[tokio::test]
async fn perf_05_a_page_of_forty_notifications_runs_as_many_statements_as_a_page_of_five()
-> TestResult {
    each_database("perf_05_notifications_page", async |db| {
        forty_run_as_many_as_five(db, News::Comment).await
    })
    .await
}

#[tokio::test]
async fn perf_05_a_page_of_forty_status_changes_runs_as_many_statements_as_a_page_of_five()
-> TestResult {
    each_database("perf_05_notifications_page_changes", async |db| {
        forty_run_as_many_as_five(db, News::StatusChange).await
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
        news_for_ada(&site, &mut ada, &mut bob, News::Comment, 5, 0).await?;
        ok(&ada.send("GET", &unread, None).await?)?;
        counter.take();
        let few = ada.send("GET", &unread, None).await?;
        assert_eq!(ok(&few)?["unread"], 5);
        let (on_five, five_log) = counter.take_logged();
        news_for_ada(&site, &mut ada, &mut bob, News::Comment, 35, 5).await?;
        ok(&ada.send("GET", &unread, None).await?)?;
        counter.take();
        let many = ada.send("GET", &unread, None).await?;
        assert_eq!(ok(&many)?["unread"], 40);
        let (on_forty, forty_log) = counter.take_logged();
        assert_same_statements(&five_log, &forty_log);
        assert_as_many_statements(on_five, on_forty, AT_MOST, "more unread notifications");
        Ok(())
    })
    .await
}

/// Bob finishes one new issue of Ada in each of the projects `from..to` (made as `P<number>`), so
/// Ada hears of one change in each of them.
async fn news_in_projects(
    site: &Site<'_>,
    ada: &mut Browser,
    bob: &mut Browser,
    from: usize,
    to: usize,
) -> Result<(), Failure> {
    for number in from..to {
        let short = format!("P{number}");
        ok(&ada
            .send(
                "POST",
                super::issues_site::PROJECTS,
                Some(json!({"short_name": short, "name": format!("Project {number}")})),
            )
            .await?)?;
        let made = ada
            .send(
                "POST",
                &format!("{}/{short}/issues", super::issues_site::PROJECTS),
                Some(site.new_issue("An issue")),
            )
            .await?;
        let issue = key_of(&made)?;
        let version = version_of(bob, &issue).await?;
        let changed = bob
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": version, "status": "done"})),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
    }
    Ok(())
}

#[tokio::test]
async fn perf_05_the_number_on_the_menu_runs_as_many_statements_for_twelve_projects_as_for_two()
-> TestResult {
    each_database("perf_05_notifications_projects", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let unread = format!("{NOTIFICATIONS}/unread");
        let mut counter = Counter::on(&rig.database);
        news_in_projects(&site, &mut ada, &mut bob, 0, 2).await?;
        ok(&ada.send("GET", &unread, None).await?)?;
        counter.take();
        let few = ada.send("GET", &unread, None).await?;
        assert_eq!(ok(&few)?["unread"], 2);
        let on_two = counter.take();
        news_in_projects(&site, &mut ada, &mut bob, 2, 12).await?;
        ok(&ada.send("GET", &unread, None).await?)?;
        counter.take();
        let many = ada.send("GET", &unread, None).await?;
        assert_eq!(ok(&many)?["unread"], 12);
        let on_twelve = counter.take();
        // The rights in each project are read together, not one project after the other.
        assert_as_many_statements(on_two, on_twelve, PROJECTS_AT_MOST, "more projects");
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
        let patched = ada
            .send("PATCH", &format!("{ISSUES}/{issue}"), Some(body))
            .await?;
        assert_eq!(patched.status, StatusCode::OK, "{}", patched.body);
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
