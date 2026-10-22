//! No N+1 queries (PERF-05, M3 WP-11): the reads of the issue page and the list of issues run the
//! same number of statements for an issue with nothing besides it and for an issue with many
//! comments, relations, sub-issues, tags, labels and followers, and for a page of five issues
//! and a page of forty. Every flow runs on SQLite and PostgreSQL.
//!
//! The statements are counted by the database itself (`Database::statements_run`): the number
//! before and after a request.

use bb_app::access::StartingRole;
use bb_store::Database;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::files_http::upload;
use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, ok, text_in};

/// The most statements any read of the issue page or the list may run, however much the issue has
/// (a ceiling that a change to the code has to notice; the exact numbers are compared apart). The
/// workflows that count (ADR-0031) are two statements for a request, and one when no place uses a
/// stored workflow; with them the ceiling went from 45 to 46. The files that a page of history
/// names are read in one statement, which made it 47.
const AT_MOST: usize = 47;

/// The most statements the history of an issue may run: the history names an issue type and a label
/// only where the reader may know it (LM1 of the M4 exit review, `offers/known.rs`), so it also
/// reads the types, the label groups and what the reader sees of the projects, once for a page
/// and only when a row names a type or a label, and the tags the reader may know the same way
/// (55 measured: four statements for the tags, once for a page; the types come from the one read of
/// the request).
const HISTORY_AT_MOST: usize = 55;

/// Counts the statements a database runs between one look and the next.
pub(super) struct Counter<'a> {
    database: &'a Database,
    before: u64,
}

impl<'a> Counter<'a> {
    pub(super) fn on(database: &'a Database) -> Self {
        Self {
            database,
            before: database.statements_run(),
        }
    }

    /// The statements run since the last time this was asked.
    pub(super) fn take(&mut self) -> usize {
        let now = self.database.statements_run();
        let ran = now - self.before;
        self.before = now;
        usize::try_from(ran).unwrap_or(usize::MAX)
    }
}

/// A longer page, or more rows behind a number, runs as many statements as a short one and no more
/// than `at_most` (`what` says which, for the message of a failure).
pub(super) fn assert_as_many_statements(on_few: usize, on_many: usize, at_most: usize, what: &str) {
    assert!(on_few > 0, "the counter sees nothing");
    assert_eq!(on_few, on_many, "{what} must not run more statements");
    assert!(on_many <= at_most, "{on_many} statements");
}

/// The statements that a page of five and a page of forty run, after a read that warms the caches
/// (not counted); each page has as many items as were asked for.
pub(super) async fn five_and_forty(
    browser: &mut Browser,
    database: &Database,
    path: impl Fn(u32) -> String,
) -> Result<(usize, usize), Failure> {
    let mut counter = Counter::on(database);
    ok(&browser.send("GET", &path(5), None).await?)?;
    counter.take();
    let five = browser.send("GET", &path(5), None).await?;
    assert_eq!(ok(&five)?["items"].as_array().map(Vec::len), Some(5));
    let on_five = counter.take();
    let forty = browser.send("GET", &path(40), None).await?;
    assert_eq!(ok(&forty)?["items"].as_array().map(Vec::len), Some(40));
    Ok((on_five, counter.take()))
}

/// The reads the issue page makes, by what they read.
fn page_reads(issue: &str) -> Vec<(&'static str, String)> {
    [
        "",
        "/relations",
        "/sub-issues",
        "/related-issues",
        "/history",
        "/labels-and-tags",
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

/// How much an issue has besides itself.
#[derive(Clone, Copy)]
struct Size {
    relations: usize,
    parts: usize,
    comments: usize,
    tags: usize,
    labels: usize,
    followers: usize,
    /// Files of the issue: each is a row in its history.
    files: usize,
}

/// Gives the issue `amount` of everything the page reads: related issues, sub-issues, comments that
/// mention a person and name an issue, tags, labels, files and followers.
async fn fill(
    site: &Site<'_>,
    ada: &mut Browser,
    issue: &str,
    named_in_comments: &str,
    amount: Size,
) -> Result<(), Failure> {
    let kinds = ada.send("GET", "/api/v1/relation-kinds", None).await?;
    let blocks = text_in(&kinds.body["items"][0], "id")?;
    for number in 0..amount.relations {
        let other = text_in(
            ok(&site
                .create(ada, &format!("Other {number} of {issue}"))
                .await?)?,
            "key",
        )?;
        let body = json!({"other": other, "kind": blocks, "direction": "forward"});
        ok(&ada
            .send("POST", &format!("{ISSUES}/{issue}/relations"), Some(body))
            .await?)?;
    }
    for number in 0..amount.parts {
        let part = text_in(
            ok(&site
                .create(ada, &format!("Part {number} of {issue}"))
                .await?)?,
            "key",
        )?;
        let body = json!({"version": 1, "parent": issue});
        ok(&ada
            .send("PUT", &format!("{ISSUES}/{part}/parent"), Some(body))
            .await?)?;
    }
    for number in 0..amount.comments {
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
    for number in 0..amount.tags {
        let body = json!({"name": format!("tag{number}")});
        ok(&ada
            .send("POST", &format!("{ISSUES}/{issue}/tags"), Some(body))
            .await?)?;
    }
    for number in 0..amount.labels {
        let made = ada
            .send(
                "POST",
                "/api/v1/labels",
                Some(json!({"name": format!("label {number} of {issue}")})),
            )
            .await?;
        let label = text_in(ok(&made)?, "id")?;
        ok(&ada
            .send(
                "POST",
                &format!("{ISSUES}/{issue}/labels"),
                Some(json!({"label": label})),
            )
            .await?)?;
    }
    for number in 0..amount.files {
        upload(ada, issue, &format!("file{number}.txt"), b"the same bytes").await?;
    }
    // The followers are people of their own: each follows the issue from their own session. Both
    // issues of the test have as many, because every different person on a page costs the
    // statements of their own access (M3-F113).
    for number in 0..amount.followers {
        let name = format!("fan{number}-{}", issue.to_lowercase());
        let (mut fan, _) = site.person(StartingRole::TeamMember, &name).await?;
        fan.send("PUT", &format!("{ISSUES}/{issue}/follow"), None)
            .await?;
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
        let named = text_in(ok(&site.create(&mut ada, "Named").await?)?, "key")?;
        let quiet = text_in(ok(&site.create(&mut ada, "Quiet").await?)?, "key")?;
        let busy = text_in(ok(&site.create(&mut ada, "Busy").await?)?, "key")?;
        let two = Size {
            relations: 2,
            parts: 2,
            comments: 2,
            tags: 2,
            labels: 2,
            followers: 1,
            files: 2,
        };
        let many = Size {
            relations: 8,
            parts: 6,
            comments: 12,
            tags: 5,
            labels: 4,
            followers: 1,
            files: 7,
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
            let ceiling = if part.ends_with("/history") {
                HISTORY_AT_MOST
            } else {
                AT_MOST
            };
            assert!(*statements <= ceiling, "{part}: {statements} statements");
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
            let key = text_in(
                ok(&site.create(&mut ada, &format!("Issue {number}")).await?)?,
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
        let path = |limit: u32| format!("{PROJECTS}/{}/issues?limit={limit}", site.acme);
        let (on_five, on_forty) = five_and_forty(&mut ada, &rig.database, path).await?;
        assert_as_many_statements(on_five, on_forty, AT_MOST, "a longer page");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_tag_suggestions_run_as_many_statements_for_twelve_projects_as_for_two()
-> TestResult {
    each_database("perf_05_suggestions", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = text_in(ok(&site.create(&mut ada, "One").await?)?, "key")?;
        ok(&ada
            .send(
                "POST",
                &format!("{ISSUES}/{issue}/tags"),
                Some(json!({"name": "plan"})),
            )
            .await?)?;
        let path = format!("{PROJECTS}/{}/tag-suggestions?text=pl", site.acme);
        let mut counter = Counter::on(&rig.database);
        ok(&ada.send("GET", &path, None).await?)?;
        counter.take();
        ok(&ada.send("GET", &path, None).await?)?;
        let on_two = counter.take();
        for number in 0..10 {
            ok(&ada
                .send(
                    "POST",
                    PROJECTS,
                    Some(
                        json!({"short_name": format!("P{number}"), "name": format!("P {number}")}),
                    ),
                )
                .await?)?;
        }
        counter.take();
        let found = ada.send("GET", &path, None).await?;
        assert_eq!(ok(&found)?["tags"][0]["name"], "plan");
        let on_twelve = counter.take();
        assert!(on_two > 0, "the counter sees nothing");
        assert_eq!(
            on_two, on_twelve,
            "more projects must not run more statements"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_a_hidden_comment_or_relation_and_a_missing_one_run_the_same_statements()
-> TestResult {
    each_database("sec_06_equal_statements", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // A Guest sees the project ACME and only the issues they reported.
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let first = text_in(ok(&site.create(&mut ada, "Ada's first").await?)?, "key")?;
        let second = text_in(ok(&site.create(&mut ada, "Ada's second").await?)?, "key")?;
        let comment = ok(&ada
            .send(
                "POST",
                &format!("{ISSUES}/{first}/comments"),
                Some(json!({"text": "Hidden."})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no comment")?
            .to_owned();
        let kinds = ada.send("GET", "/api/v1/relation-kinds", None).await?;
        let kind = text_in(&kinds.body["items"][0], "id")?;
        let relation = text_in(
            ok(&ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{first}/relations"),
                    Some(json!({"other": second, "kind": kind, "direction": "forward"})),
                )
                .await?)?,
            "id",
        )?;
        let nobody = "01890a5d-ac96-774b-bcce-b302099a8057";
        let mut counter = Counter::on(&rig.database);
        let mut count = async |path: String| -> Result<(String, usize), Failure> {
            counter.take();
            let reply = gus.send("DELETE", &path, None).await?;
            Ok((reply.code().to_owned(), counter.take()))
        };
        for (hidden, missing) in [
            (
                format!("/api/v1/comments/{comment}"),
                format!("/api/v1/comments/{nobody}"),
            ),
            (
                format!("/api/v1/relations/{relation}"),
                format!("/api/v1/relations/{nobody}"),
            ),
        ] {
            let hidden = count(hidden).await?;
            let missing = count(missing).await?;
            assert_eq!(hidden.0, "resource.not_found");
            assert_eq!(missing.0, "resource.not_found");
            assert!(hidden.1 > 0, "the counter sees nothing");
            assert_eq!(hidden.1, missing.1, "the same statements");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_types_and_label_groups_of_a_project_run_as_many_statements_for_forty_as_for_five()
-> TestResult {
    each_database("perf_05_types_and_groups", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // A guest of another project knows none of the types and groups of ACME, so the lists
        // look for what they use on the issues they read.
        let web = ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "WEB", "name": "Website"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project ID")?
            .to_owned();
        let (mut gus, _) = site.person_in(StartingRole::Guest, "gus", &web).await?;
        let create = async |ada: &mut Browser, range: std::ops::Range<u32>| -> TestResult {
            for number in range {
                ok(&ada
                    .send(
                        "POST",
                        "/api/v1/issue-types",
                        Some(json!({
                            "name": format!("Type {number}"),
                            "icon": "circle-help",
                            "project": "ACME",
                        })),
                    )
                    .await?)?;
                ok(&ada
                    .send(
                        "POST",
                        "/api/v1/label-groups",
                        Some(json!({
                            "name": format!("Group {number}"),
                            "pick_one": false,
                            "project": "ACME",
                        })),
                    )
                    .await?)?;
            }
            Ok(())
        };
        let types = "/api/v1/issue-types?project=ACME&limit=200";
        let labels = "/api/v1/label-groups?project=ACME";
        let mut counter = Counter::on(&rig.database);
        create(&mut ada, 0..5).await?;
        // Warm, then count.
        ok(&ada.send("GET", types, None).await?)?;
        ok(&ada.send("GET", labels, None).await?)?;
        ok(&gus
            .send("GET", "/api/v1/issue-types?limit=200", None)
            .await?)?;
        ok(&gus.send("GET", "/api/v1/label-groups", None).await?)?;
        counter.take();
        ok(&ada.send("GET", types, None).await?)?;
        let types_on_five = counter.take();
        ok(&ada.send("GET", labels, None).await?)?;
        let labels_on_five = counter.take();
        ok(&gus
            .send("GET", "/api/v1/issue-types?limit=200", None)
            .await?)?;
        ok(&gus.send("GET", "/api/v1/label-groups", None).await?)?;
        let guest_on_five = counter.take();
        create(&mut ada, 5..40).await?;
        counter.take();
        let listed = ada.send("GET", types, None).await?;
        assert!(ok(&listed)?["items"].as_array().map_or(0, Vec::len) >= 40);
        let types_on_forty = counter.take();
        let listed = ada.send("GET", labels, None).await?;
        assert!(ok(&listed)?["items"].as_array().map_or(0, Vec::len) >= 40);
        let labels_on_forty = counter.take();
        ok(&gus
            .send("GET", "/api/v1/issue-types?limit=200", None)
            .await?)?;
        ok(&gus.send("GET", "/api/v1/label-groups", None).await?)?;
        let guest_on_forty = counter.take();
        assert!(
            types_on_five > 0 && labels_on_five > 0,
            "the counter sees nothing"
        );
        assert_eq!(
            types_on_five, types_on_forty,
            "more types must not run more statements"
        );
        assert_eq!(
            labels_on_five, labels_on_forty,
            "more groups must not run more statements"
        );
        assert_eq!(
            guest_on_five, guest_on_forty,
            "more subjects the reader does not know must not run more statements"
        );
        Ok(())
    })
    .await
}
