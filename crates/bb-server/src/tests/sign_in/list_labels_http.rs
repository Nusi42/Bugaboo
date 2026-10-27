//! The labels of the issues in a list (M3-F171, WP-08): the list of a project and the search carry
//! them when asked for, read once for the page; a reader whose level for *Labels* is *Hidden*
//! cannot ask for them. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::Permission;
use bb_core::access::FieldLevel;
use bb_core::fields::Field;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::exit_masks::{issue_of_type, person_with_levels};
use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, ok, text_in, version_of};
use super::perf_queries::{Counter, LIST_AT_MOST, assert_as_many_statements, five_and_forty};

const LABELS: &str = "/api/v1/labels";
const SEARCH: &str = "/api/v1/search/issues";

/// The reads the labels cost a page: the links of the page and the catalogue.
const LABEL_READS: usize = 2;

/// The most statements a page of a list or a search with labels may run: those of a page without
/// them, and the reads of the labels.
const AT_MOST: usize = LIST_AT_MOST + LABEL_READS;

/// Makes a label and returns its ID.
async fn label(browser: &mut Browser, name: &str) -> Result<String, Failure> {
    let made = browser
        .send(
            "POST",
            LABELS,
            Some(json!({"name": name, "label_group": null})),
        )
        .await?;
    text_in(ok(&made)?, "id")
}

/// Puts a label on an issue.
async fn put(browser: &mut Browser, issue: &str, label: &str) -> Result<(), Failure> {
    ok(&browser
        .send(
            "POST",
            &format!("{ISSUES}/{issue}/labels"),
            Some(json!({"label": label})),
        )
        .await?)?;
    Ok(())
}

/// The names of the labels of an item of a list.
fn label_names(item: &Value) -> Vec<String> {
    item["labels"]
        .as_array()
        .map(|labels| {
            labels
                .iter()
                .filter_map(|each| each["name"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn cfg_02_the_list_and_the_search_carry_the_labels_when_asked_for() -> TestResult {
    each_database("cfg_02_list_labels", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (first, second) = (
            text_in(ok(&site.create(&mut ada, "Has two").await?)?, "readable_id")?,
            text_in(
                ok(&site.create(&mut ada, "Has none").await?)?,
                "readable_id",
            )?,
        );
        let urgent = label(&mut ada, "urgent").await?;
        let crash = label(&mut ada, "crash").await?;
        // In this order, a moment apart; the order of the answer is the order they were put on, not
        // that of the names or of the IDs.
        put(&mut ada, &first, &urgent).await?;
        rig.clock.advance(1);
        put(&mut ada, &first, &crash).await?;
        for path in [
            format!("{PROJECTS}/{}/issues?fields=readable_id,labels", site.acme),
            format!("{SEARCH}?fields=readable_id,labels"),
        ] {
            let answer = ada.send("GET", &path, None).await?;
            let items = ok(&answer)?["items"].as_array().ok_or("no items")?.clone();
            let of = |key: &str| {
                items
                    .iter()
                    .find(|item| item["readable_id"] == key)
                    .map(label_names)
            };
            // Both labels, in the order they were put on; an issue without labels has an empty
            // list.
            assert_eq!(
                of(&first),
                Some(vec!["urgent".to_owned(), "crash".to_owned()]),
                "{path}"
            );
            assert_eq!(of(&second), Some(Vec::new()), "{path}");
        }
        // Not asked for, not in the answer.
        let plain = ada
            .send("GET", &format!("{PROJECTS}/{}/issues", site.acme), None)
            .await?;
        assert!(ok(&plain)?["items"][0].get("labels").is_none());
        Ok(())
    })
    .await
}

/// An answer to a change does not read the labels, so it does not say the issue has none: the
/// member is left out of it (the API is frozen; a client takes the labels from their own route).
#[tokio::test]
async fn cfg_02_the_answers_to_a_change_do_not_claim_that_the_issue_has_no_labels() -> TestResult {
    each_database("cfg_02_change_answers", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let key = text_in(
            ok(&site.create(&mut ada, "Has a label").await?)?,
            "readable_id",
        )?;
        let urgent = label(&mut ada, "urgent").await?;
        put(&mut ada, &key, &urgent).await?;
        let path = format!("{ISSUES}/{key}");
        let version = version_of(&mut ada, &key).await?;
        let changed = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "title": "Renamed"})),
            )
            .await?;
        assert!(ok(&changed)?.get("labels").is_none(), "{}", changed.body);
        let other = ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?)?
        .clone();
        let version = version_of(&mut ada, &key).await?;
        let moved = ada
            .send(
                "POST",
                &format!("{path}/move"),
                Some(json!({"version": version, "project": text_in(&other, "id")?})),
            )
            .await?;
        assert!(ok(&moved)?.get("labels").is_none(), "{}", moved.body);
        ok(&ada.send("DELETE", &path, None).await?).ok();
        let restored = ada.send("POST", &format!("{path}/restore"), None).await?;
        assert!(ok(&restored)?.get("labels").is_none(), "{}", restored.body);
        // The labels are still on the issue, and a list that asks for them says so.
        let list = ada
            .send("GET", &format!("{SEARCH}?fields=readable_id,labels"), None)
            .await?;
        let labelled = ok(&list)?["items"]
            .as_array()
            .ok_or("no items")?
            .iter()
            .any(|item| label_names(item) == ["urgent"]);
        assert!(labelled, "{}", list.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_a_page_of_forty_issues_with_labels_runs_as_many_statements_as_a_page_of_five()
-> TestResult {
    each_database("perf_05_list_labels", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let labels = [
            label(&mut ada, "one").await?,
            label(&mut ada, "two").await?,
            label(&mut ada, "three").await?,
        ];
        for number in 0..40 {
            let key = text_in(
                ok(&site.create(&mut ada, &format!("Issue {number}")).await?)?,
                "readable_id",
            )?;
            put(&mut ada, &key, &labels[number % 3]).await?;
        }
        let project = |limit: u32| {
            format!(
                "{PROJECTS}/{}/issues?limit={limit}&fields=readable_id,labels",
                site.acme
            )
        };
        let (on_five, on_forty) = five_and_forty(&mut ada, &rig.database, project).await?;
        assert_as_many_statements(on_five, on_forty, AT_MOST, "a longer page of a list");
        let search = |limit: u32| format!("{SEARCH}?limit={limit}&fields=readable_id,labels");
        let (on_five, on_forty) = five_and_forty(&mut ada, &rig.database, search).await?;
        assert_as_many_statements(on_five, on_forty, AT_MOST, "a longer page of a search");
        // The labels cost the page their reads, and nothing when they are not asked for.
        let mut counter = Counter::on(&rig.database);
        let with = format!("{SEARCH}?limit=5&fields=readable_id,labels");
        let without = format!("{SEARCH}?limit=5&fields=readable_id");
        ok(&ada.send("GET", &with, None).await?)?;
        counter.take();
        ok(&ada.send("GET", &with, None).await?)?;
        let asked = counter.take();
        ok(&ada.send("GET", &without, None).await?)?;
        let not_asked = counter.take();
        assert_eq!(
            asked,
            not_asked + LABEL_READS,
            "the labels are read once for the page"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_the_labels_of_a_list_are_asked_for_only_where_the_role_sees_them() -> TestResult {
    each_database("sec_06_list_labels", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut hal, _) = person_with_levels(
            &site,
            &rig,
            "hal",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[
                (Field::Title.into(), None, FieldLevel::CanSee),
                (Field::Labels.into(), None, FieldLevel::CanSee),
            ],
        )
        .await?;
        let (mut ivy, _) = person_with_levels(
            &site,
            &rig,
            "ivy",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[(Field::Title.into(), None, FieldLevel::CanSee)],
        )
        .await?;
        let (bug, _) = issue_of_type(&site, &mut ada, &site.kind, "A bug").await?;
        let urgent = label(&mut ada, "urgent").await?;
        put(&mut ada, &bug, &urgent).await?;
        let path = format!("{PROJECTS}/ACME/issues?fields=readable_id,labels");
        // Where the role sees the field, the labels are on the answer.
        let seen = hal.send("GET", &path, None).await?;
        assert_eq!(label_names(&ok(&seen)?["items"][0]), ["urgent"]);
        // Where it does not, asking for the field fails like asking for one that does not exist,
        // in the list of a project and in a search, and the default fields never name it.
        for path in [path.as_str(), &format!("{SEARCH}?fields=labels")] {
            let hidden = ivy.send("GET", path, None).await?;
            assert_eq!(
                (hidden.status, hidden.code()),
                (StatusCode::BAD_REQUEST, "request.unknown-field"),
                "{path}: {}",
                hidden.body
            );
        }
        let plain = ivy
            .send("GET", &format!("{PROJECTS}/ACME/issues"), None)
            .await?;
        assert!(ok(&plain)?["items"][0].get("labels").is_none());
        Ok(())
    })
    .await
}
