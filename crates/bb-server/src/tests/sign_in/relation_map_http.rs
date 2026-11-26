//! The relation map over HTTP (ISS-16, UX design §4.15, PERF-N1): the map of an issue by steps
//! away and kinds shown, *Everything* as the limit of the search language, the cap of 200 issues
//! with `more`, fields the reader may not read left out, loops of *blocks* named, and the map of a
//! search. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_app::relations::MAP_ISSUES_MOST;
use bb_app::search::MAX_STEPS;
use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::exit_masks::person_with_levels;
use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, key_of, ok};
use super::perf_queries::{Counter, assert_as_many_statements};
use super::relations_http::{kinds, put_under};

/// The map of a search.
const SEARCH_MAP: &str = "/api/v1/map";

/// The most statements a map of an issue may run, whatever its size: who asks, the issue, the
/// sight and the fields of the reader, the kinds of relation, one statement for each step away,
/// the issues, their types, projects and workflows, the relations and the parents among them (52
/// measured on both databases).
const MAP_AT_MOST: usize = 55;

/// Adds a relation of `kind` from `from` to `to`, as `browser`.
async fn relate(browser: &mut Browser, kind: &str, from: &str, to: &str) -> Result<(), Failure> {
    ok(&browser
        .send(
            "POST",
            &format!("{ISSUES}/{from}/relations"),
            Some(json!({"other": to, "relation_kind": kind, "direction": "forward"})),
        )
        .await?)?;
    Ok(())
}

/// Issues of ACME with these titles, as `browser`; their keys.
async fn issues(
    site: &Site<'_>,
    browser: &mut Browser,
    count: usize,
) -> Result<Vec<String>, Failure> {
    let mut made = Vec::with_capacity(count);
    for at in 0..count {
        made.push(key_of(&site.create(browser, &format!("Step {at}")).await?)?);
    }
    Ok(made)
}

/// The map of `issue` with the query `query`, which has to be answered.
async fn map(browser: &mut Browser, issue: &str, query: &str) -> Result<Value, Failure> {
    Ok(ok(&browser
        .send("GET", &format!("{ISSUES}/{issue}/map{query}"), None)
        .await?)?
    .clone())
}

/// The issues of a map by their keys, with their steps away, in the order of the map.
fn on_map(answer: &Value) -> Vec<(String, Option<u64>)> {
    answer["issues"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    (
                        item["issue"]["readable_id"]
                            .as_str()
                            .unwrap_or_default()
                            .to_owned(),
                        item["steps"].as_u64(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn keys(answer: &Value) -> Vec<String> {
    on_map(answer).into_iter().map(|(key, _)| key).collect()
}

#[tokio::test]
async fn iss_16_the_map_of_an_issue_holds_every_issue_within_the_steps_chosen() -> TestResult {
    each_database("iss_16_map_steps", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (blocks, related) = kinds(&mut ada).await?;
        let chain = issues(&site, &mut ada, 4).await?;
        // 0 blocks 1, 2 is related to 1, 2 blocks 3: a chain in both directions.
        relate(&mut ada, &blocks, &chain[0], &chain[1]).await?;
        relate(&mut ada, &related, &chain[2], &chain[1]).await?;
        relate(&mut ada, &blocks, &chain[2], &chain[3]).await?;
        let one = map(&mut ada, &chain[0], "?steps=1").await?;
        assert_eq!(
            on_map(&one),
            [(chain[0].clone(), Some(0)), (chain[1].clone(), Some(1))]
        );
        assert_eq!(one["lines"].as_array().map(Vec::len), Some(1));
        assert_eq!(one["lines"][0]["forward"], "blocks");
        assert_eq!(one["truncated"], false);
        let all = map(&mut ada, &chain[1], "").await?;
        assert_eq!(
            all["start"],
            ok(&ada
                .send("GET", &format!("{ISSUES}/{}", chain[1]), None)
                .await?)?["issue"]["id"]
        );
        // From the middle: both neighbours one step away, in the order of the kinds (*blocks*
        // before *related to*), then the last one.
        assert_eq!(
            on_map(&all),
            [
                (chain[1].clone(), Some(0)),
                (chain[0].clone(), Some(1)),
                (chain[2].clone(), Some(1)),
                (chain[3].clone(), Some(2)),
            ]
        );
        assert_eq!(all["lines"].as_array().map(Vec::len), Some(3));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_16_kinds_shown_narrow_the_walk_and_part_of_is_one_of_them() -> TestResult {
    each_database("iss_16_map_kinds", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (blocks, related) = kinds(&mut ada).await?;
        let made = issues(&site, &mut ada, 4).await?;
        relate(&mut ada, &blocks, &made[0], &made[1]).await?;
        relate(&mut ada, &related, &made[0], &made[2]).await?;
        ok(&put_under(&mut ada, &made[3], 1, Some(&made[0])).await?)?;
        let only_blocks = map(
            &mut ada,
            &made[0],
            &format!("?kinds={blocks}&part_of=false"),
        )
        .await?;
        assert_eq!(keys(&only_blocks), [made[0].clone(), made[1].clone()]);
        let only_parts = map(&mut ada, &made[0], "?kinds=").await?;
        assert_eq!(keys(&only_parts), [made[0].clone(), made[3].clone()]);
        assert_eq!(only_parts["lines"][0]["forward"], "is part of");
        assert!(only_parts["lines"][0].get("relation_kind").is_none());
        let every = map(&mut ada, &made[0], "").await?;
        assert_eq!(keys(&every).len(), 4);
        // A kind that is no ID is no request.
        let bad = ada
            .send(
                "GET",
                &format!("{ISSUES}/{}/map?kinds=nothing", made[0]),
                None,
            )
            .await?;
        assert_eq!(bad.code(), "request.invalid");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_16_everything_means_five_steps_the_limit_of_the_search_language() -> TestResult {
    each_database("iss_16_map_everything", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (blocks, _) = kinds(&mut ada).await?;
        let steps = usize::try_from(MAX_STEPS)?;
        let chain = issues(&site, &mut ada, steps + 2).await?;
        for pair in chain.windows(2) {
            relate(&mut ada, &blocks, &pair[0], &pair[1]).await?;
        }
        let everything = map(&mut ada, &chain[0], "").await?;
        assert_eq!(keys(&everything), chain[..=steps]);
        assert_eq!(
            on_map(&everything).last().and_then(|(_, away)| *away),
            Some(u64::from(MAX_STEPS))
        );
        let five = map(&mut ada, &chain[0], &format!("?steps={MAX_STEPS}")).await?;
        assert_eq!(keys(&five), keys(&everything));
        for wrong in [0, MAX_STEPS + 1] {
            let refused = ada
                .send(
                    "GET",
                    &format!("{ISSUES}/{}/map?steps={wrong}", chain[0]),
                    None,
                )
                .await?;
            assert_eq!(refused.code(), "request.invalid", "{wrong}");
        }
        Ok(())
    })
    .await
}

/// A start with ten issues one step away, each with twenty more behind it: more than a map holds
/// two steps away, all in reach of one step. The start's key.
async fn a_crowd(site: &Site<'_>, ada: &mut Browser, blocks: &str) -> Result<String, Failure> {
    let start = issues(site, ada, 1).await?.remove(0);
    for _ in 0..10 {
        let hub = issues(site, ada, 1).await?.remove(0);
        relate(ada, blocks, &start, &hub).await?;
        for leaf in issues(site, ada, 20).await? {
            relate(ada, blocks, &hub, &leaf).await?;
        }
    }
    Ok(start)
}

#[tokio::test]
async fn iss_16_the_walk_stops_at_200_and_says_there_are_more() -> TestResult {
    each_database("iss_16_map_cap", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (blocks, _) = kinds(&mut ada).await?;
        let start = a_crowd(&site, &mut ada, &blocks).await?;
        let near = map(&mut ada, &start, "?steps=1").await?;
        assert_eq!(keys(&near).len(), 11);
        assert_eq!(near["truncated"], false);
        let far = map(&mut ada, &start, "?steps=2").await?;
        assert_eq!(keys(&far).len(), MAP_ISSUES_MOST);
        assert_eq!(far["truncated"], true);
        // The ten one step away are all there: the cap cuts the last step, never a nearer one.
        assert_eq!(
            on_map(&far)
                .iter()
                .filter(|(_, away)| *away == Some(1))
                .count(),
            10
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_n1_a_map_of_200_issues_makes_a_fixed_number_of_statements() -> TestResult {
    each_database("iss_16_map_statements", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (blocks, _) = kinds(&mut ada).await?;
        let big = a_crowd(&site, &mut ada, &blocks).await?;
        let small = issues(&site, &mut ada, 3).await?;
        relate(&mut ada, &blocks, &small[0], &small[1]).await?;
        relate(&mut ada, &blocks, &small[1], &small[2]).await?;
        // Warm the caches, then count.
        map(&mut ada, &small[0], "?steps=2").await?;
        let mut counter = Counter::on(&rig.database);
        let few = map(&mut ada, &small[0], "?steps=2").await?;
        let on_few = counter.take();
        let many = map(&mut ada, &big, "?steps=2").await?;
        let on_many = counter.take();
        assert_eq!(keys(&few).len(), 3);
        assert_eq!(keys(&many).len(), MAP_ISSUES_MOST);
        assert_as_many_statements(on_few, on_many, MAP_AT_MOST, "a map of 200 issues");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_16_a_field_the_reader_may_not_read_is_not_on_the_map() -> TestResult {
    each_database("iss_16_map_fields", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (blocks, _) = kinds(&mut ada).await?;
        let made = issues(&site, &mut ada, 2).await?;
        relate(&mut ada, &blocks, &made[0], &made[1]).await?;
        // A reader who sees only the title: status and priority are hidden from them.
        let (mut reader, _) = person_with_levels(
            &site,
            &rig,
            "title-only",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[(FieldKey::BuiltIn(Field::Title), None, FieldLevel::CanSee)],
        )
        .await?;
        let theirs = map(&mut reader, &made[0], "").await?;
        assert_eq!(keys(&theirs), made);
        for item in theirs["issues"].as_array().ok_or("no issues")? {
            assert!(item["issue"].get("title").is_some(), "{item}");
            assert!(item["issue"].get("status").is_none(), "{item}");
            assert!(item["issue"].get("priority").is_none(), "{item}");
        }
        let full = map(&mut ada, &made[0], "").await?;
        assert!(full["issues"][0]["issue"].get("status").is_some());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_16_a_loop_of_blocks_is_named_as_the_issues_on_it() -> TestResult {
    each_database("iss_16_map_loops", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (blocks, related) = kinds(&mut ada).await?;
        let made = issues(&site, &mut ada, 3).await?;
        // A loop of *blocks* is refused today; one stored before the effect stays. *Blocks* loses
        // its date effect, the loop is closed, and giving the effect back is refused for the loop.
        let kind = ok(&ada.send("GET", "/api/v1/relation-kinds", None).await?)?["items"][0].clone();
        let version = kind["version"].as_i64().ok_or("no version")?;
        let path = format!("/api/v1/relation-kinds/{blocks}");
        ok(&ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "date_effect": "none"})),
            )
            .await?)?;
        relate(&mut ada, &blocks, &made[0], &made[1]).await?;
        relate(&mut ada, &blocks, &made[1], &made[2]).await?;
        relate(&mut ada, &blocks, &made[2], &made[0]).await?;
        relate(&mut ada, &related, &made[0], &made[2]).await?;
        let back = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version + 1, "date_effect": "must-finish-first"})),
            )
            .await?;
        assert_eq!(back.code(), "relation.dependency-loop");
        let answer = map(&mut ada, &made[1], "").await?;
        let id_of = |key: &str| {
            answer["issues"]
                .as_array()
                .and_then(|items| {
                    items
                        .iter()
                        .find(|item| item["issue"]["readable_id"] == key)
                })
                .and_then(|item| item["issue"]["id"].as_str())
                .map(str::to_owned)
        };
        // The loop is named once, in order from the issue whose relation comes first.
        let named: Vec<Option<String>> = answer["loops"]
            .as_array()
            .ok_or("no loops")?
            .iter()
            .flat_map(|each| each.as_array().cloned().unwrap_or_default())
            .map(|id| id.as_str().map(str::to_owned))
            .collect();
        assert_eq!(named.len(), 3, "{answer}");
        let mut on_it: Vec<Option<String>> = made.iter().map(|key| id_of(key)).collect();
        on_it.sort();
        let mut sorted = named.clone();
        sorted.sort();
        assert_eq!(sorted, on_it);
        // *Related to* has no direction and closes no loop.
        assert_eq!(answer["lines"].as_array().map(Vec::len), Some(4));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_16_the_map_of_a_search_holds_its_results_and_the_relations_between_them() -> TestResult
{
    each_database("iss_16_map_search", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (blocks, _) = kinds(&mut ada).await?;
        // Two issues of ACME and one of another project, in a chain.
        let other = ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "OTHER", "name": "Other"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project ID")?
            .to_owned();
        let mut made = issues(&site, &mut ada, 2).await?;
        made.push(key_of(
            &ada.send(
                "POST",
                &format!("{PROJECTS}/{other}/issues"),
                Some(site.new_issue("Elsewhere")),
            )
            .await?,
        )?);
        relate(&mut ada, &blocks, &made[0], &made[1]).await?;
        relate(&mut ada, &blocks, &made[1], &made[2]).await?;
        // The search finds the two of ACME: the third is not walked to.
        let text = "project is ACME";
        let answer = ok(&ada
            .send("POST", SEARCH_MAP, Some(json!({"search_text": text})))
            .await?)?
        .clone();
        let mut found = keys(&answer);
        found.sort();
        let mut of_acme = made[..2].to_vec();
        of_acme.sort();
        assert_eq!(found, of_acme);
        assert!(answer.get("start").is_none());
        assert!(on_map(&answer).iter().all(|(_, away)| away.is_none()));
        assert_eq!(answer["lines"].as_array().map(Vec::len), Some(1));
        assert_eq!(answer["truncated"], false);
        // The map of a project's search holds the results of that project only (PRJ-06).
        let in_other = ok(&ada
            .send(
                "POST",
                SEARCH_MAP,
                Some(json!({"search_text": "", "project": "OTHER"})),
            )
            .await?)?
        .clone();
        assert_eq!(keys(&in_other), made[2..].to_vec());
        assert_eq!(in_other["lines"].as_array().map(Vec::len), Some(0));
        // A project the person may not see is not found.
        let unknown = ada
            .send(
                "POST",
                SEARCH_MAP,
                Some(json!({"search_text": "", "project": "NOWHERE"})),
            )
            .await?;
        assert_eq!(unknown.status, StatusCode::NOT_FOUND);
        // A reader of ACME only sees no issue of another project on the map of everything, and
        // no line to one.
        let secret = ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "SECRET", "name": "Secret"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project ID")?
            .to_owned();
        let hidden = key_of(
            &ada.send(
                "POST",
                &format!("{PROJECTS}/{secret}/issues"),
                Some(site.new_issue("Hidden step")),
            )
            .await?,
        )?;
        relate(&mut ada, &blocks, &made[2], &hidden).await?;
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        let theirs = ok(&bob.send("POST", SEARCH_MAP, Some(json!({}))).await?)?.clone();
        let mut seen = keys(&theirs);
        seen.sort();
        assert_eq!(seen, of_acme);
        assert_eq!(theirs["lines"].as_array().map(Vec::len), Some(1));
        let walked = map(&mut bob, &made[0], "").await?;
        assert_eq!(keys(&walked), made[..2]);
        Ok(())
    })
    .await
}
