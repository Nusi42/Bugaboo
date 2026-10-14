//! No N+1 queries (PERF-05, M3 WP-11): the reads of the issue page and the list of issues run the
//! same number of statements for an issue with nothing besides it and for an issue with many
//! comments, relations, sub-issues, tags, labels and followers, and for a page of five issues
//! and a page of forty. Every flow runs on SQLite and PostgreSQL.
//!
//! The statements are counted by the database itself (`Database::statements_run`): the number
//! before and after a request.

use bb_store::Database;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, ok};

/// The most statements any read of the issue page or the list may run, however much the issue has
/// (a ceiling that a change to the code has to notice; the exact numbers are compared apart).
const AT_MOST: usize = 45;

/// Counts the statements a database runs between one look and the next.
struct Counter<'a> {
    database: &'a Database,
    before: u64,
}

impl<'a> Counter<'a> {
    fn on(database: &'a Database) -> Self {
        Self {
            database,
            before: database.statements_run(),
        }
    }

    /// The statements run since the last time this was asked.
    fn take(&mut self) -> usize {
        let now = self.database.statements_run();
        let ran = now - self.before;
        self.before = now;
        usize::try_from(ran).unwrap_or(usize::MAX)
    }
}

/// The reads the issue page makes, by what they read.
fn page_reads(issue: &str) -> Vec<(&'static str, String)> {
    [
        "",
        "/relations",
        "/sub-issues",
        "/related-issues",
        "/history",
        "/sorting",
        "/follow",
        "/followers",
    ]
    .into_iter()
    .map(|part| (part, format!("{ISSUES}/{issue}{part}")))
    .collect()
}

/// How many statements each read of the page ran.
async fn counted_page(
    ada: &mut Browser,
    counter: &mut Counter<'_>,
    issue: &str,
) -> Result<Vec<(&'static str, usize)>, Failure> {
    let mut counts = Vec::new();
    for (part, path) in page_reads(issue) {
        counter.take();
        let reply = ada.send("GET", &path, None).await?;
        ok(&reply)?;
        counts.push((part, counter.take()));
    }
    Ok(counts)
}

/// The ID in an answer that made something.
fn id_in(reply: &Value, member: &str) -> Result<String, Failure> {
    reply[member]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("no {member} in {reply}").into())
}

/// How much an issue has besides itself.
#[derive(Clone, Copy)]
struct Size {
    relations: usize,
    parts: usize,
    comments: usize,
    tags: usize,
}

/// Gives the issue `size` of everything the page reads: related issues, sub-issues, comments that
/// mention a person and name an issue, tags, and a follower.
async fn fill(
    site: &Site<'_>,
    ada: &mut Browser,
    issue: &str,
    named_in_comments: &str,
    size: Size,
) -> Result<(), Failure> {
    let kinds = ada.send("GET", "/api/v1/relation-kinds", None).await?;
    let blocks = id_in(&kinds.body["items"][0], "id")?;
    for number in 0..size.relations {
        let other = id_in(
            ok(&site
                .make(ada, &format!("Other {number} of {issue}"))
                .await?)?,
            "key",
        )?;
        let body = json!({"other": other, "kind": blocks, "direction": "forward"});
        ok(&ada
            .send("POST", &format!("{ISSUES}/{issue}/relations"), Some(body))
            .await?)?;
    }
    for number in 0..size.parts {
        let part = id_in(
            ok(&site.make(ada, &format!("Part {number} of {issue}")).await?)?,
            "key",
        )?;
        let body = json!({"version": 1, "parent": issue});
        ok(&ada
            .send("PUT", &format!("{ISSUES}/{part}/parent"), Some(body))
            .await?)?;
    }
    for number in 0..size.comments {
        // The same person and the same issue in every comment: a page names the same few things
        // over and over, and looks each of them up once.
        let text = format!("Comment {number} for @ada.admin about {named_in_comments}");
        ok(&ada
            .send(
                "POST",
                &format!("{ISSUES}/{issue}/comments"),
                Some(json!({"text": text})),
            )
            .await?)?;
    }
    for number in 0..size.tags {
        let body = json!({"name": format!("tag{number}")});
        ok(&ada
            .send("POST", &format!("{ISSUES}/{issue}/tags"), Some(body))
            .await?)?;
    }
    ada.send("PUT", &format!("{ISSUES}/{issue}/follow"), None)
        .await?;
    Ok(())
}

#[tokio::test]
async fn perf_05_the_reads_of_an_issue_page_run_as_many_statements_for_a_busy_issue_as_for_a_quiet_one()
-> TestResult {
    each_database("perf_05_issue_page", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let named = id_in(ok(&site.make(&mut ada, "Named").await?)?, "key")?;
        let quiet = id_in(ok(&site.make(&mut ada, "Quiet").await?)?, "key")?;
        let busy = id_in(ok(&site.make(&mut ada, "Busy").await?)?, "key")?;
        let two = Size {
            relations: 2,
            parts: 2,
            comments: 2,
            tags: 2,
        };
        let many = Size {
            relations: 8,
            parts: 6,
            comments: 12,
            tags: 5,
        };
        fill(&site, &mut ada, &quiet, &named, two).await?;
        fill(&site, &mut ada, &busy, &named, many).await?;
        let mut counter = Counter::on(&rig.database);
        // The first read makes the program's own caches: not counted.
        counted_page(&mut ada, &mut counter, &quiet).await?;
        let on_quiet = counted_page(&mut ada, &mut counter, &quiet).await?;
        let on_busy = counted_page(&mut ada, &mut counter, &busy).await?;
        assert_eq!(
            on_quiet, on_busy,
            "an issue with many comments, relations and parts must not run more statements than one with two"
        );
        for (part, statements) in &on_busy {
            assert!(
                *statements > 0,
                "{part}: no statement was counted, the counter sees nothing"
            );
            assert!(*statements <= AT_MOST, "{part}: {statements} statements");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_a_page_of_forty_issues_runs_as_many_statements_as_a_page_of_five() -> TestResult {
    each_database("perf_05_list", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        for number in 0..40 {
            let key = id_in(
                ok(&site.make(&mut ada, &format!("Issue {number}")).await?)?,
                "key",
            )?;
            ok(&ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{key}/tags"),
                    Some(json!({"name": format!("tag{}", number % 7)})),
                )
                .await?)?;
        }
        let mut counter = Counter::on(&rig.database);
        let path = |limit: u32| format!("{PROJECTS}/{}/issues?limit={limit}", site.acme);
        // Warm, then count.
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
