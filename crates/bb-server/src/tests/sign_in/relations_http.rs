//! Sub-issues and related issues over HTTP (ISS-03, ISS-04, PRIV-13): no circles, progress that
//! counts only what the viewer sees, relations read from both sides in their own words, and
//! relations to hidden issues that are not shown. Every flow runs on SQLite and PostgreSQL.

use std::sync::Arc;

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_app::auth::{AppError, AuthApi as _};
use bb_app::relations::{AddRelation, SetParent};
use bb_core::ErrorCode;
use bb_core::relation::Direction;
use bb_core::relation::RELATIONS_MAX;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok};
use super::rig::Rig;

/// The keys of the issues in a list.
fn keys(list: &Reply) -> Vec<String> {
    list.body["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["readable_id"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Puts `child` under `parent`; the answer.
async fn put_under(
    browser: &mut Browser,
    child: &str,
    version: i64,
    parent: Option<&str>,
) -> Result<Reply, Failure> {
    browser
        .send(
            "PUT",
            &format!("{ISSUES}/{child}/parent"),
            Some(json!({"version": version, "parent": parent})),
        )
        .await
}

/// The ID of the first kind of relation (*blocks*), and of the last (*related to*).
async fn kinds(browser: &mut Browser) -> Result<(String, String), Failure> {
    let list = browser.send("GET", "/api/v1/relation-kinds", None).await?;
    let items = list.body["items"].as_array().ok_or("no kinds")?;
    let id = |item: Option<&Value>| -> Result<String, Failure> {
        Ok(item
            .and_then(|item| item["id"].as_str())
            .ok_or("no kind of relation on a started server")?
            .to_owned())
    };
    Ok((id(items.first())?, id(items.last())?))
}

#[tokio::test]
async fn iss_03_a_parent_cannot_become_its_own_sub_issue() -> TestResult {
    each_database("iss_03_no_circles", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let a = key_of(&site.create(&mut ada, "Goal").await?)?;
        let b = key_of(&site.create(&mut ada, "Step").await?)?;
        let c = key_of(&site.create(&mut ada, "Part").await?)?;
        // An issue under itself.
        let itself = put_under(&mut ada, &a, 1, Some(&a)).await?;
        assert_eq!(
            (itself.status, itself.code()),
            (StatusCode::CONFLICT, "issue.parent-circle")
        );
        // B under A, C under B: a chain.
        let first = put_under(&mut ada, &b, 1, Some(&a)).await?;
        assert_eq!(ok(&first)?["part_of"]["readable_id"], a);
        let second = put_under(&mut ada, &c, 1, Some(&b)).await?;
        assert_eq!(ok(&second)?["part_of"]["readable_id"], b);
        // A under B or under C would close the circle, however deep.
        for parent in [&b, &c] {
            let circle = put_under(&mut ada, &a, 1, Some(parent)).await?;
            assert_eq!(circle.code(), "issue.parent-circle", "{parent}");
        }
        // The same version twice is stale (B's version counted up with its parent).
        let stale = put_under(&mut ada, &b, 1, None).await?;
        assert_eq!(stale.code(), "edit.version-changed");
        // Taking B out from under A works and is in the sub-issues of nobody.
        let seen = ada.send("GET", &format!("{ISSUES}/{b}"), None).await?;
        let version = ok(&seen)?["issue"]["version"]
            .as_i64()
            .ok_or("no version")?;
        let out = put_under(&mut ada, &b, version, None).await?;
        assert!(ok(&out)?.get("part_of").is_none());
        let below = ada
            .send("GET", &format!("{ISSUES}/{a}/sub-issues"), None)
            .await?;
        assert!(keys(&below).is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_03_progress_counts_only_the_sub_issues_you_see() -> TestResult {
    each_database("iss_03_progress", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // A Guest sees only the issues they reported: the goal and one part of it.
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let goal = key_of(&site.create(&mut gus, "Goal of Gus").await?)?;
        let mine = key_of(&site.create(&mut gus, "Part of Gus").await?)?;
        let first = key_of(&site.create(&mut ada, "Part of Ada").await?)?;
        let second = key_of(&site.create(&mut ada, "Another part of Ada").await?)?;
        for part in [&mine, &first, &second] {
            let put = put_under(&mut ada, part, 1, Some(&goal)).await?;
            ok(&put)?;
        }
        // One of Ada's parts is done.
        let done = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{first}"),
                Some(json!({"version": 2, "status": "done"})),
            )
            .await?;
        ok(&done)?;
        let ada_sees = ada
            .send("GET", &format!("{ISSUES}/{goal}/relations"), None)
            .await?;
        assert_eq!(ok(&ada_sees)?["progress"], json!({"done": 1, "total": 3}));
        let gus_sees = gus
            .send("GET", &format!("{ISSUES}/{goal}/relations"), None)
            .await?;
        assert_eq!(ok(&gus_sees)?["progress"], json!({"done": 0, "total": 1}));
        let list = gus
            .send("GET", &format!("{ISSUES}/{goal}/sub-issues"), None)
            .await?;
        assert_eq!(keys(&list), std::slice::from_ref(&mine));
        // Gus sees the goal under no parent and the part under the goal; Ada's parts are not
        // there for him, not even as a gap.
        let own = gus
            .send("GET", &format!("{ISSUES}/{mine}/relations"), None)
            .await?;
        assert_eq!(ok(&own)?["part_of"]["readable_id"], goal);
        let hidden = gus
            .send("GET", &format!("{ISSUES}/{first}/relations"), None)
            .await?;
        assert_eq!(hidden.code(), "resource.not-found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_04_a_relation_shows_from_both_sides_in_their_own_words() -> TestResult {
    each_database("iss_04_both_sides", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let blocker = key_of(&site.create(&mut ada, "Release notes").await?)?;
        let victim = key_of(&site.create(&mut ada, "Release").await?)?;
        let (blocks, related_to) = kinds(&mut ada).await?;
        let added = ada
            .send(
                "POST",
                &format!("{ISSUES}/{blocker}/relations"),
                Some(json!({"other": victim, "kind": blocks, "direction": "forward"})),
            )
            .await?;
        assert_eq!(ok(&added)?["name"], "blocks");
        assert_eq!(added.body["issue"]["readable_id"], victim);
        // The same relation again is refused.
        let again = ada
            .send(
                "POST",
                &format!("{ISSUES}/{blocker}/relations"),
                Some(json!({"other": victim, "kind": blocks, "direction": "forward"})),
            )
            .await?;
        assert_eq!(again.code(), "relation.exists");
        // Each issue reads it in its own words.
        let from = ada
            .send("GET", &format!("{ISSUES}/{blocker}/related-issues"), None)
            .await?;
        assert_eq!(from.body["items"][0]["name"], "blocks");
        assert_eq!(from.body["items"][0]["issue"]["readable_id"], victim);
        let to = ada
            .send("GET", &format!("{ISSUES}/{victim}/related-issues"), None)
            .await?;
        assert_eq!(to.body["items"][0]["name"], "is blocked by");
        assert_eq!(to.body["items"][0]["issue"]["readable_id"], blocker);
        assert_eq!(to.body["items"][0]["direction"], "backward");
        // Started from the other end, the same words come out the same way round.
        let backward = ada
            .send(
                "POST",
                &format!("{ISSUES}/{victim}/relations"),
                Some(json!({"other": blocker, "kind": related_to, "direction": "backward"})),
            )
            .await?;
        assert_eq!(ok(&backward)?["name"], "related to");
        // A kind that reads alike from both ends is one relation whichever issue starts it.
        let alike = ada
            .send(
                "POST",
                &format!("{ISSUES}/{blocker}/relations"),
                Some(json!({"other": victim, "kind": related_to, "direction": "forward"})),
            )
            .await?;
        assert_eq!(alike.code(), "relation.exists");
        // An issue is not related to itself.
        let itself = ada
            .send(
                "POST",
                &format!("{ISSUES}/{blocker}/relations"),
                Some(json!({"other": blocker, "kind": blocks, "direction": "forward"})),
            )
            .await?;
        assert_eq!(itself.code(), "request.invalid");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_04_a_relation_to_a_hidden_issue_is_not_shown() -> TestResult {
    each_database("iss_04_hidden_end", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let theirs = key_of(&site.create(&mut gus, "Gus's problem").await?)?;
        let secret = key_of(&site.create(&mut ada, "Ada's secret").await?)?;
        let (blocks, _) = kinds(&mut ada).await?;
        ok(&ada
            .send(
                "POST",
                &format!("{ISSUES}/{secret}/relations"),
                Some(json!({"other": theirs, "kind": blocks, "direction": "forward"})),
            )
            .await?)?;
        // Ada sees it on both; Gus sees his own issue with nothing related to it, and no sign
        // that something is hidden.
        let ada_list = ada
            .send("GET", &format!("{ISSUES}/{theirs}/related-issues"), None)
            .await?;
        assert_eq!(ada_list.body["items"].as_array().map(Vec::len), Some(1));
        let gus_list = gus
            .send("GET", &format!("{ISSUES}/{theirs}/related-issues"), None)
            .await?;
        assert_eq!(ok(&gus_list)?["items"], json!([]));
        // Gus cannot relate his issue to one he cannot see, and cannot take the relation away.
        let (_, related_to) = kinds(&mut gus).await?;
        let refused = gus
            .send(
                "POST",
                &format!("{ISSUES}/{theirs}/relations"),
                Some(json!({"other": secret, "kind": related_to, "direction": "forward"})),
            )
            .await?;
        assert_eq!(refused.code(), "resource.not-found");
        let relation = ada_list.body["items"][0]["id"]
            .as_str()
            .ok_or("no relation ID")?
            .to_owned();
        let removal = gus
            .send("DELETE", &format!("/api/v1/relations/{relation}"), None)
            .await?;
        assert_eq!(removal.code(), "resource.not-found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_04_adding_and_removing_is_in_both_histories() -> TestResult {
    each_database("iss_04_histories", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let first = key_of(&site.create(&mut ada, "First").await?)?;
        let second = key_of(&site.create(&mut ada, "Second").await?)?;
        let (blocks, _) = kinds(&mut ada).await?;
        let added = ada
            .send(
                "POST",
                &format!("{ISSUES}/{first}/relations"),
                Some(json!({"other": second, "kind": blocks, "direction": "forward"})),
            )
            .await?;
        let relation = ok(&added)?["id"].as_str().ok_or("no ID")?.to_owned();
        let removal = ada
            .send("DELETE", &format!("/api/v1/relations/{relation}"), None)
            .await?;
        assert_eq!(removal.status, StatusCode::NO_CONTENT);
        for (issue, name, other) in [
            (&first, "blocks", &second),
            (&second, "is blocked by", &first),
        ] {
            let history = ada
                .send(
                    "GET",
                    &format!("{ISSUES}/{issue}/history?tab=changes"),
                    None,
                )
                .await?;
            let rows: Vec<(&str, &str, &str)> = ok(&history)?["items"]
                .as_array()
                .ok_or("no items")?
                .iter()
                .filter(|item| item["kind"] == "relation")
                .map(|item| {
                    (
                        item["relation_event"].as_str().unwrap_or_default(),
                        item["relation_name"].as_str().unwrap_or_default(),
                        item["issue"]["readable_id"].as_str().unwrap_or_default(),
                    )
                })
                .collect();
            // Newest first: removed, then added.
            assert_eq!(
                rows,
                [
                    ("removed", name, other.as_str()),
                    ("added", name, other.as_str())
                ]
            );
        }
        Ok(())
    })
    .await
}

/// How many issues want the one place that is left on an issue that is full but for one relation.
const WANTING: usize = 80;

/// Runs all the requests at the same moment on their own threads; what each one answered, in the
/// order of the requests.
async fn at_the_same_moment(
    requests: Vec<impl Future<Output = Result<(), AppError>> + Send + 'static>,
) -> Result<Vec<Result<(), AppError>>, Failure> {
    let tasks: Vec<_> = requests.into_iter().map(tokio::spawn).collect();
    let mut answers = Vec::new();
    for task in tasks {
        answers.push(task.await?);
    }
    Ok(answers)
}

/// How many of the answers are `Ok`, and how many are the refusal `code`.
fn count(answers: &[Result<(), AppError>], code: ErrorCode) -> (usize, usize) {
    let worked = answers.iter().filter(|answer| answer.is_ok()).count();
    let refused = answers
        .iter()
        .filter(|answer| matches!(answer, Err(AppError::Refused(found)) if *found == code))
        .count();
    (worked, refused)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn iss_04_relations_made_at_the_same_moment_do_not_pass_the_limit() -> TestResult {
    each_database("iss_04_limit_race", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (blocks, _) = kinds(&mut ada).await?;
        let hub = key_of(&site.create(&mut ada, "Hub").await?)?;
        // The hub holds one relation less than the limit.
        for number in 1..RELATIONS_MAX {
            let other = key_of(&site.create(&mut ada, &format!("Other {number}")).await?)?;
            let added = ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{hub}/relations"),
                    Some(json!({"other": other, "kind": blocks, "direction": "forward"})),
                )
                .await?;
            ok(&added)?;
        }
        // More issues than one each want the one place that is left.
        let mut late = Vec::new();
        for number in 0..WANTING {
            late.push(key_of(
                &site.create(&mut ada, &format!("Late {number}")).await?,
            )?);
        }
        let requests = late
            .iter()
            .map(|other| {
                let services = Arc::clone(&rig.services);
                let input = AddRelation {
                    session_token: site.org.admin.session.clone(),
                    issue: hub.clone(),
                    other: other.clone(),
                    kind: blocks.parse().expect("an ID"),
                    direction: Direction::Forward,
                };
                async move { services.add_relation(Rig::info(), input).await.map(|_| ()) }
            })
            .collect();
        let answers = at_the_same_moment(requests).await?;
        assert_eq!(
            count(&answers, ErrorCode::LimitReached),
            (1, WANTING - 1),
            "{answers:?}"
        );
        // Exactly one of them is related to the hub.
        let mut related = 0;
        for key in &late {
            let list = ada
                .send("GET", &format!("{ISSUES}/{key}/related-issues"), None)
                .await?;
            related += list.body["items"].as_array().map_or(0, Vec::len);
        }
        assert_eq!(related, 1);
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn iss_03_parents_set_at_the_same_moment_do_not_create_a_circle() -> TestResult {
    each_database("iss_03_circle_race", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // Ten pairs: in each, A goes under B and B under A, both from version 1.
        let mut pairs = Vec::new();
        for number in 0..10 {
            let a = key_of(&site.create(&mut ada, &format!("A {number}")).await?)?;
            let b = key_of(&site.create(&mut ada, &format!("B {number}")).await?)?;
            pairs.push((a, b));
        }
        let put = |child: &str, parent: &str| {
            let services = Arc::clone(&rig.services);
            let input = SetParent {
                session_token: site.org.admin.session.clone(),
                issue: child.to_owned(),
                version: 1,
                parent: Some(parent.to_owned()),
            };
            async move { services.set_parent(Rig::info(), input).await.map(|_| ()) }
        };
        let requests = pairs
            .iter()
            .flat_map(|(a, b)| [put(a, b), put(b, a)])
            .collect();
        let answers = at_the_same_moment(requests).await?;
        for (number, two) in answers.chunks(2).enumerate() {
            assert_eq!(
                count(two, ErrorCode::ParentCircle),
                (1, 1),
                "pair {number}: {two:?}"
            );
        }
        // In each pair one issue is under the other, not both.
        for (a, b) in &pairs {
            let mut under = 0;
            for parent in [a, b] {
                let list = ada
                    .send("GET", &format!("{ISSUES}/{parent}/sub-issues"), None)
                    .await?;
                under += keys(&list).len();
            }
            assert_eq!(under, 1, "{a} and {b}");
        }
        Ok(())
    })
    .await
}
