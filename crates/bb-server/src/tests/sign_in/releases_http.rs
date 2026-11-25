//! Releases over HTTP on SQLite and PostgreSQL (ISS-08, ADR-0049): where a release is offered,
//! the two members of an issue, a search by them, and the statements a list of releases and a list
//! of issues with both columns run (PERF-N1).

use axum::http::StatusCode;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, ok, text_in, version_of};
use super::perf_queries::{Counter, LIST_AT_MOST, assert_as_many_statements, five_and_forty};

const SEARCH: &str = "/api/v1/search/issues";

/// The reads the releases cost a page: the releases the page names and the releases themselves.
const RELEASE_READS: usize = 2;

/// Makes a release in `project`; its ID.
async fn release(browser: &mut Browser, project: &str, name: &str) -> Result<String, Failure> {
    let made = browser
        .send(
            "POST",
            &format!("{PROJECTS}/{project}/releases"),
            Some(json!({"name": name, "release_date": "2026-12-01"})),
        )
        .await?;
    text_in(ok(&made)?, "id")
}

/// Makes a project, below `parent` when one is named; its ID.
async fn project(
    browser: &mut Browser,
    short_name: &str,
    parent: Option<&str>,
) -> Result<String, Failure> {
    let path = match parent {
        Some(parent) => format!("{PROJECTS}/{parent}/sub-projects"),
        None => PROJECTS.to_owned(),
    };
    let made = browser
        .send(
            "POST",
            &path,
            Some(json!({"short_name": short_name, "name": short_name})),
        )
        .await?;
    text_in(ok(&made)?, "id")
}

/// Makes an issue in `project`; its readable ID.
async fn issue_in(
    site: &Site<'_>,
    browser: &mut Browser,
    project: &str,
    title: &str,
) -> Result<String, Failure> {
    let made = browser
        .send(
            "POST",
            &format!("{PROJECTS}/{project}/issues"),
            Some(site.new_issue(title)),
        )
        .await?;
    text_in(ok(&made)?, "readable_id")
}

/// Sets the release an issue is planned for; the answer.
async fn plan_for(
    browser: &mut Browser,
    issue: &str,
    release: &str,
) -> Result<super::http::Reply, Failure> {
    let version = version_of(browser, issue).await?;
    browser
        .send(
            "PATCH",
            &format!("{ISSUES}/{issue}"),
            Some(json!({"version": version, "planned_for_release": release, "found_in_release": [release]})),
        )
        .await
}

#[tokio::test]
async fn iss_08_a_release_is_offered_in_its_project_and_sub_projects_only() -> TestResult {
    each_database("iss_08_offered", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let web = project(&mut ada, "WEB", Some(&site.acme)).await?;
        let ops = project(&mut ada, "OPS", None).await?;
        let made = release(&mut ada, &site.acme, "2.4").await?;
        for listed_in in [&site.acme, &web] {
            let listed = ada
                .send("GET", &format!("{PROJECTS}/{listed_in}/releases"), None)
                .await?;
            assert_eq!(
                ok(&listed)?["items"][0]["id"],
                json!(made),
                "{}",
                listed.body
            );
        }
        let none = ada
            .send("GET", &format!("{PROJECTS}/{ops}/releases"), None)
            .await?;
        assert_eq!(ok(&none)?["items"], json!([]));
        let below = issue_in(&site, &mut ada, &web, "In a part").await?;
        let planned = plan_for(&mut ada, &below, &made).await?;
        let answer = ok(&planned)?;
        assert_eq!(answer["planned_for_release"]["id"], json!(made), "{answer}");
        assert_eq!(answer["found_in_release"][0]["offered"], json!(true));
        let beside = issue_in(&site, &mut ada, &ops, "Elsewhere").await?;
        let refused = plan_for(&mut ada, &beside, &made).await?;
        assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);
        assert_eq!(refused.body["code"], json!("release.not-offered"));
        // A release a person cannot know is refused the same way as one that does not exist.
        let nowhere = plan_for(&mut ada, &beside, "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c").await?;
        assert_eq!(nowhere.status, StatusCode::CONFLICT);
        assert_eq!(nowhere.body["code"], refused.body["code"]);
        // Deleting a release an issue names is refused, and says nothing of the issue.
        let deleted = ada
            .send(
                "DELETE",
                &format!("{PROJECTS}/{}/releases/{made}", site.acme),
                None,
            )
            .await?;
        assert_eq!(deleted.status, StatusCode::CONFLICT);
        assert_eq!(deleted.body["code"], json!("release.in-use"));
        assert!(
            !deleted.body.to_string().contains(&below),
            "{}",
            deleted.body
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_08_search_filters_by_a_release_by_its_name() -> TestResult {
    each_database("iss_08_search", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = release(&mut ada, &site.acme, "2.4").await?;
        let planned = issue_in(&site, &mut ada, &site.acme, "Planned").await?;
        issue_in(&site, &mut ada, &site.acme, "Not planned").await?;
        ok(&plan_for(&mut ada, &planned, &made).await?)?;
        let found = ada
            .send(
                "GET",
                &format!(
                    "{SEARCH}?q={}&fields=title,planned_for_release",
                    "planned%20for%20release%20is%20%222.4%22"
                ),
                None,
            )
            .await?;
        let items = ok(&found)?["items"].as_array().ok_or("no items")?.clone();
        assert_eq!(items.len(), 1, "{}", found.body);
        assert_eq!(items[0]["title"], json!("Planned"));
        assert_eq!(items[0]["planned_for_release"]["name"], json!("2.4"));
        // A name that is no release the person may know is an unknown value.
        let unknown = ada
            .send(
                "GET",
                &format!("{SEARCH}?q={}", "planned%20for%20release%20is%20%229.9%22"),
                None,
            )
            .await?;
        assert_eq!(unknown.status, StatusCode::BAD_REQUEST, "{}", unknown.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_a_list_of_releases_and_a_page_of_issues_with_both_columns_run_as_many_statements_for_forty_as_for_five()
-> TestResult {
    each_database("perf_05_releases", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut made = Vec::new();
        for number in 0..40 {
            made.push(release(&mut ada, &site.acme, &format!("1.{number}")).await?);
        }
        let releases = |limit: u32| format!("{PROJECTS}/{}/releases?limit={limit}", site.acme);
        let (on_five, on_forty) = five_and_forty(&mut ada, &rig.database, releases).await?;
        assert_as_many_statements(on_five, on_forty, LIST_AT_MOST, "a longer list of releases");
        for (number, release) in made.iter().enumerate() {
            let key = text_in(
                ok(&site.create(&mut ada, &format!("Issue {number}")).await?)?,
                "readable_id",
            )?;
            ok(&plan_for(&mut ada, &key, release).await?)?;
        }
        let fields = "readable_id,found_in_release,planned_for_release";
        let list = |limit: u32| {
            format!(
                "{PROJECTS}/{}/issues?limit={limit}&fields={fields}",
                site.acme
            )
        };
        let (on_five, on_forty) = five_and_forty(&mut ada, &rig.database, list).await?;
        assert_as_many_statements(
            on_five,
            on_forty,
            LIST_AT_MOST + RELEASE_READS,
            "a longer page of a list with the releases",
        );
        // The releases cost the page their reads, and nothing when they are not asked for.
        let mut counter = Counter::on(&rig.database);
        let with = format!("{SEARCH}?limit=5&fields={fields}");
        let without = format!("{SEARCH}?limit=5&fields=readable_id");
        ok(&ada.send("GET", &with, None).await?)?;
        counter.take();
        let page: Value = ok(&ada.send("GET", &with, None).await?)?.clone();
        let asked = counter.take();
        ok(&ada.send("GET", &without, None).await?)?;
        let not_asked = counter.take();
        assert_eq!(
            asked,
            not_asked + RELEASE_READS,
            "the releases are read once for the page"
        );
        assert!(
            page["items"][0]["planned_for_release"]["name"].is_string(),
            "{page}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_08_totals_by_planned_for_release_show_only_the_releases_the_reader_may_know()
-> TestResult {
    each_database("iss_08_totals_by_release", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let web = project(&mut ada, "WEB", Some(&site.acme)).await?;
        let top = release(&mut ada, &site.acme, "Moonbeam").await?;
        let part = release(&mut ada, &web, "2.4").await?;
        for (title, planned) in [
            ("Hidden", Some(&top)),
            ("Seen", Some(&part)),
            ("None", None),
        ] {
            let issue = issue_in(&site, &mut ada, &web, title).await?;
            if let Some(planned) = planned {
                ok(&plan_for(&mut ada, &issue, planned).await?)?;
            }
        }
        // Tim sees WEB only: the release of ACME is no value for him.
        let (mut tim, _) = site
            .person_in(bb_app::access::StartingRole::TeamMember, "tim", &web)
            .await?;
        let asked = json!({"by": "planned for release", "sums": ["count"]});
        let answer = super::search_totals_http::totals(&mut tim, asked.clone()).await?;
        let answer = ok(&answer)?;
        assert!(!answer.to_string().contains("Moonbeam"), "{answer}");
        let issues_where = |answer: &Value, part: &str| -> Option<Value> {
            answer["groups"].as_array()?.iter().find_map(|group| {
                group["filter"]
                    .as_str()
                    .is_some_and(|filter| filter.contains(part))
                    .then(|| group["issues"].clone())
            })
        };
        assert_eq!(
            answer["groups"].as_array().map(Vec::len),
            Some(2),
            "{answer}"
        );
        assert_eq!(issues_where(answer, "2.4"), Some(json!(1)), "{answer}");
        assert_eq!(issues_where(answer, "is empty"), Some(json!(2)), "{answer}");
        // Ada knows both releases: a group each, and one issue without.
        let all = super::search_totals_http::totals(&mut ada, asked).await?;
        let all = ok(&all)?;
        assert_eq!(issues_where(all, "Moonbeam"), Some(json!(1)), "{all}");
        assert_eq!(issues_where(all, "is empty"), Some(json!(1)), "{all}");
        Ok(())
    })
    .await
}
