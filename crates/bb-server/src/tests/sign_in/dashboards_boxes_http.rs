//! The boxes of dashboards over HTTP on SQLite and PostgreSQL (CFG-03, ADR-0050, decisions 3 to 5):
//! each box runs as the viewer through the read that exists for it, a box that does not check for
//! the viewer is left out and the grid closes up, no chart ranks people, *Recent changes* shows
//! people as *Who sees people* says, every ready-made box answers, and a dashboard costs a fixed
//! number of statements for each box (PERF-N1).

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_core::dashboard::{BoxKind, READY_MADE_BOXES, TimeSpan};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::dashboards_http::{DASHBOARDS, MISSING, a_box, create, project, results, share, timer};
use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, id_of, ok, text_in, version_of};
use super::perf_queries::{Counter, LIST_AT_MOST, assert_as_many_statements, five_and_forty};

/// *Numbers and chart* of `text`, as bars by `by` or as one number.
pub(super) fn chart(text: &str, by: Option<&str>) -> Value {
    let options = match by {
        Some(by) => json!({"search_text": text, "by": by, "shape": "bars"}),
        None => json!({"search_text": text, "shape": "number"}),
    };
    a_box("numbers-and-chart", "Numbers", (0, 0, 6, 4), &options)
}

/// Creates an issue titled `title` in `project` with the members of `more`; its readable ID.
async fn issue_in(
    site: &Site<'_>,
    browser: &mut Browser,
    project: &str,
    (title, more): (&str, Value),
) -> Result<String, Failure> {
    let mut body = site.new_issue(title);
    if let (Some(body), Some(more)) = (body.as_object_mut(), more.as_object()) {
        body.extend(more.clone());
    }
    let made = browser
        .send("POST", &format!("{PROJECTS}/{project}/issues"), Some(body))
        .await?;
    text_in(ok(&made)?, "readable_id")
}

/// What one box of a dashboard answers for `browser`.
async fn answer_of(
    browser: &mut Browser,
    dashboard: &str,
    number: usize,
) -> Result<Value, Failure> {
    let frame = browser
        .send("GET", &format!("{DASHBOARDS}/{dashboard}"), None)
        .await?;
    let box_id = text_in(&ok(&frame)?["boxes"][number], "id")?;
    let read = browser
        .send(
            "GET",
            &format!("{DASHBOARDS}/{dashboard}/boxes/{box_id}"),
            None,
        )
        .await?;
    Ok(ok(&read)?.clone())
}

/// What a box would answer for `browser`.
async fn preview(browser: &mut Browser, body: Value) -> Result<Value, Failure> {
    let read = browser
        .send("POST", &format!("{DASHBOARDS}/box-preview"), Some(body))
        .await?;
    Ok(ok(&read)?.clone())
}

/// The titles of the issues a box of search results answers.
fn titles(answer: &Value) -> Vec<String> {
    answer["issues"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|issue| issue["title"].as_str().map(str::to_owned))
        .collect()
}

/// The kind, the title and the place of each box of a dashboard: what a viewer is shown of it.
fn frame_of(answer: &Value) -> Vec<Value> {
    answer["boxes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|each| {
            json!([
                each["kind"],
                each["title"],
                each["column"],
                each["row"],
                each["width"],
                each["height"]
            ])
        })
        .collect()
}

#[tokio::test]
async fn cfg_03_each_box_runs_as_the_viewer() -> TestResult {
    each_database("cfg_03_as_viewer", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let ops = project(&mut ada, "OPS").await?;
        issue_in(&site, &mut ada, &site.acme, ("In ACME", json!({}))).await?;
        issue_in(&site, &mut ada, &ops, ("In OPS", json!({}))).await?;
        let made = create(
            &mut ada,
            "Team",
            vec![results("Everything", "", (0, 0, 6, 4)), {
                let mut number = chart("", None);
                number["column"] = json!(6);
                number
            }],
        )
        .await?;
        let id = id_of(&made)?;
        ok(&share(&mut ada, &id, ("project", &site.acme)).await?)?;
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        let mut adas = titles(&answer_of(&mut ada, &id, 0).await?);
        adas.sort();
        assert_eq!(adas, ["In ACME", "In OPS"]);
        assert_eq!(titles(&answer_of(&mut bob, &id, 0).await?), ["In ACME"]);
        let adas_number = answer_of(&mut ada, &id, 1).await?;
        assert_eq!(adas_number["answer"], "totals", "{adas_number}");
        assert_eq!(adas_number["totals"]["all"]["issues"], 2);
        // Bob's number is the total of a search of his own: the read that exists, run as him.
        let bobs_number = answer_of(&mut bob, &id, 1).await?;
        let bobs_search = bob
            .send("POST", "/api/v1/search/totals", Some(json!({"q": ""})))
            .await?;
        assert_eq!(bobs_number["totals"], *ok(&bobs_search)?);
        assert_eq!(bobs_number["totals"]["all"]["issues"], 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_a_box_that_does_not_check_for_the_viewer_is_left_out_without_a_gap_or_a_title()
-> TestResult {
    each_database("cfg_03_left_out", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        project(&mut ada, "OPS").await?;
        let top = results("Everything", "", (0, 0, 6, 2));
        let hidden = results("Secret plans", "project is OPS", (6, 0, 6, 4));
        let under_hidden = results("Open", "status group is not Done", (6, 4, 6, 2));
        let below = a_box("my-timer", "My timer", (0, 6, 12, 2), &json!({}));
        let made = create(
            &mut ada,
            "Team",
            vec![top.clone(), hidden, under_hidden.clone(), below.clone()],
        )
        .await?;
        let id = id_of(&made)?;
        assert_eq!(made.body["boxes"].as_array().map(Vec::len), Some(4));
        let hidden_box = text_in(&made.body["boxes"][1], "id")?;
        ok(&share(&mut ada, &id, ("project", &site.acme)).await?)?;
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        let seen = bob.send("GET", &format!("{DASHBOARDS}/{id}"), None).await?;
        let seen = ok(&seen)?.clone();
        // Bob sees the dashboard Ada would make of the boxes he may read: packed, no gap.
        let without = create(&mut ada, "Without", vec![top, under_hidden, below]).await?;
        assert_eq!(frame_of(&seen), frame_of(ok(&without)?), "{seen}");
        assert_eq!(
            seen["boxes"][1]["row"], 0,
            "the box under the hidden one moved up"
        );
        assert!(!seen.to_string().contains("Secret plans"), "{seen}");
        assert!(!seen.to_string().contains(&hidden_box), "{seen}");
        // Its address answers like a box that does not exist.
        let asked = bob
            .send(
                "GET",
                &format!("{DASHBOARDS}/{id}/boxes/{hidden_box}"),
                None,
            )
            .await?;
        let missing = bob
            .send("GET", &format!("{DASHBOARDS}/{id}/boxes/{MISSING}"), None)
            .await?;
        assert_eq!(asked.status, StatusCode::NOT_FOUND);
        assert_eq!(
            (asked.status, asked.code()),
            (missing.status, missing.code())
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_a_chart_by_a_person_is_in_the_order_of_names_and_cannot_be_sorted_by_size()
-> TestResult {
    each_database("cfg_03_no_ranking", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, zed) = site
            .person_called(StartingRole::TeamMember, "zed", "Zed Zimmer")
            .await?;
        let (_, bea) = site
            .person_called(StartingRole::TeamMember, "bea", "Bea Berg")
            .await?;
        // Zed has three issues and Bea one: by size, Zed would come first.
        for (title, who) in [("One", zed), ("Two", zed), ("Three", zed), ("Four", bea)] {
            let assigned = json!({"assigned_to": [who.to_string()]});
            issue_in(&site, &mut ada, &site.acme, (title, assigned)).await?;
        }
        let answer = preview(&mut ada, chart("", Some("assigned to"))).await?;
        let groups: Vec<(Value, Value)> = answer["totals"]["groups"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|group| (group["name"].clone(), group["issues"].clone()))
            .collect();
        assert_eq!(
            groups,
            [
                (json!("Bea Berg"), json!(1)),
                (json!("Zed Zimmer"), json!(3))
            ],
            "{answer}"
        );
        // No box has an order of its own, nor a number by a field.
        for asked in [
            {
                let mut by_size = chart("", Some("assigned to"));
                by_size["order"] = json!("count");
                by_size
            },
            {
                let mut ranked = chart("", None);
                ranked["by"] = json!("assigned to");
                ranked
            },
        ] {
            let refused = ada
                .send("POST", &format!("{DASHBOARDS}/box-preview"), Some(asked))
                .await?;
            assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{}", refused.body);
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_03_recent_changes_show_people_as_who_sees_people_says() -> TestResult {
    each_database("cfg_03_recent_people", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let key = issue_in(&site, &mut ada, &site.acme, ("Old title", json!({}))).await?;
        let version = version_of(&mut ada, &key).await?;
        let renamed = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{key}"),
                Some(json!({"version": version, "title": "New title"})),
            )
            .await?;
        ok(&renamed)?;
        site.show_people(&mut ada, StartingRole::Contributor, "authors", "role-only")
            .await?;
        let (mut cy, _) = site.person_in_acme(StartingRole::Contributor, "cy").await?;
        let recent = a_box(
            "recent-changes",
            "Recent changes",
            (0, 0, 6, 4),
            &json!({"search_text": ""}),
        );
        let history = format!("{ISSUES}/{key}/history?tab=changes");
        // Each person reads the changes as the history of the issue shows them to them: Cy sees the
        // author by the role only, as the level of Contributors says.
        for (browser, shown) in [(&mut ada, None), (&mut cy, Some("role"))] {
            let read = preview(browser, recent.clone()).await?;
            assert_eq!(read["answer"], "changes", "{read}");
            assert_eq!(read["changes"][0]["issue"], json!(key), "{read}");
            let by = &read["changes"][0]["entries"][0]["by"];
            let own = browser.send("GET", &history, None).await?;
            assert_eq!(*by, ok(&own)?["items"][0]["by"], "{read}");
            if let Some(shown) = shown {
                assert_eq!(by["shown"], shown, "{read}");
            }
        }
        Ok(())
    })
    .await
}

/// The answer each kind of box gives, by the read it calls.
fn answer_of_kind(kind: BoxKind, span: Option<TimeSpan>) -> &'static str {
    match (kind, span) {
        (BoxKind::SearchResults, _) => "issues",
        (BoxKind::TimeTotals, Some(TimeSpan::MyWeek)) => "week",
        (BoxKind::NumbersAndChart | BoxKind::TimeTotals, _) => "totals",
        (BoxKind::SmallTimeline, _) => "timeline",
        (BoxKind::MyTimer, _) => "timer",
        (BoxKind::RecentChanges, _) => "changes",
    }
}

#[tokio::test]
async fn cfg_03_every_ready_made_box_answers_with_the_reads_that_exist() -> TestResult {
    each_database("cfg_03_ready_made", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        site.create(&mut ada, "Something to show").await?;
        let listed = ada
            .send("GET", &format!("{DASHBOARDS}/ready-made-boxes"), None)
            .await?;
        let items = ok(&listed)?["boxes"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_eq!(items.len(), READY_MADE_BOXES.len(), "{}", listed.body);
        // One case for each row of the list, in its order: the row's box, previewed by the read of
        // its kind.
        for (entry, item) in READY_MADE_BOXES.iter().zip(&items) {
            let which = entry.which.code();
            assert_eq!(item["which"], which);
            let mut body = item["box"].clone();
            if let Some(fields) = body.as_object_mut() {
                fields.remove("id");
                fields.retain(|_, value| !value.is_null());
            }
            let read = ada
                .send("POST", &format!("{DASHBOARDS}/box-preview"), Some(body))
                .await?;
            assert_eq!(read.status, StatusCode::OK, "{which}: {}", read.body);
            assert_eq!(
                read.body["answer"],
                answer_of_kind(entry.kind, entry.span),
                "{which}: {}",
                read.body
            );
            // The text of the row is kept as the language spells it.
            assert_eq!(item["box"]["search_text"].as_str(), entry.text, "{which}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_a_dashboard_runs_a_fixed_number_of_statements_for_each_box() -> TestResult {
    each_database("perf_05_dashboard_boxes", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        site.create(&mut ada, "Something to show").await?;
        let with = |count: u32| {
            (0..count)
                .map(|at| {
                    results(
                        &format!("Box {at}"),
                        "status group is not Done",
                        (0, at * 2, 3, 2),
                    )
                })
                .collect::<Vec<_>>()
        };
        let mut statements = Vec::new();
        for count in [1, 2, 10] {
            let id = id_of(&create(&mut ada, &format!("{count} boxes"), with(count)).await?)?;
            let path = format!("{DASHBOARDS}/{id}");
            ok(&ada.send("GET", &path, None).await?)?;
            let mut counter = Counter::on(&rig.database);
            let read = ada.send("GET", &path, None).await?;
            assert_eq!(
                ok(&read)?["boxes"].as_array().map(Vec::len),
                Some(count as usize)
            );
            statements.push(counter.take());
        }
        let [one, two, ten] = statements[..] else {
            return Err("three counts".into());
        };
        let each = two - one;
        assert_eq!(
            ten - one,
            9 * each,
            "{statements:?}: a fixed number for each box"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_05_a_page_of_forty_dashboards_runs_as_many_statements_as_a_page_of_five() -> TestResult
{
    each_database("perf_05_dashboard_list", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        for number in 0..40 {
            let id = id_of(&create(&mut ada, &format!("Board {number}"), vec![timer(0)]).await?)?;
            ok(&share(&mut ada, &id, ("project", &site.acme)).await?)?;
        }
        let shared = |limit: u32| format!("{DASHBOARDS}?scope=shared&limit={limit}");
        let (on_five, on_forty) = five_and_forty(&mut bob, &rig.database, shared).await?;
        assert_as_many_statements(
            on_five,
            on_forty,
            LIST_AT_MOST,
            "a longer list of dashboards",
        );
        Ok(())
    })
    .await
}
