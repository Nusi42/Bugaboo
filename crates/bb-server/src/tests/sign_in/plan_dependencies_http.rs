//! The date effect of a kind of relation over HTTP (PLAN-02, ADR-0043, decision 4): *blocks* starts
//! with *Must finish first*, at most two kinds carry it, and a relation of such a kind (a
//! dependency) never closes a loop; a loop is named only by issues the person may see. Every flow
//! runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{ADA_EMAIL, Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, Site, key_of, ok};

/// The kinds of relation, as `browser` reads them.
async fn kinds(browser: &mut Browser) -> Result<Vec<Value>, Failure> {
    let list = browser.send("GET", "/api/v1/relation-kinds", None).await?;
    Ok(ok(&list)?["items"].as_array().cloned().unwrap_or_default())
}

/// The kind with this forward name.
async fn kind_called(browser: &mut Browser, forward: &str) -> Result<Value, Failure> {
    kinds(browser)
        .await?
        .into_iter()
        .find(|kind| kind["forward"] == forward)
        .ok_or_else(|| format!("no kind called {forward}").into())
}

/// A new kind of relation, with no date effect.
async fn new_kind(browser: &mut Browser, forward: &str, backward: &str) -> Result<Value, Failure> {
    let made = browser
        .send(
            "POST",
            "/api/v1/relation-kinds",
            Some(json!({"forward": forward, "backward": backward})),
        )
        .await?;
    Ok(ok(&made)?.clone())
}

/// Gives a kind a date effect, or takes it away: the answer.
async fn set_effect(browser: &mut Browser, kind: &Value, effect: &str) -> Result<Reply, Failure> {
    browser
        .send(
            "PATCH",
            &format!(
                "/api/v1/relation-kinds/{}",
                kind["id"].as_str().ok_or("no ID")?
            ),
            Some(json!({"version": kind["version"], "date_effect": effect})),
        )
        .await
}

/// Relates `first` to `second` with `kind`, reading *first … kind … second*: the answer.
async fn relate(
    browser: &mut Browser,
    first: &str,
    kind: &Value,
    second: &str,
) -> Result<Reply, Failure> {
    browser
        .send(
            "POST",
            &format!("{ISSUES}/{first}/relations"),
            Some(json!({
                "other": second,
                "relation_kind": kind["id"],
                "direction": "forward",
            })),
        )
        .await
}

#[tokio::test]
async fn plan_02_blocks_starts_with_the_date_effect_and_the_other_kinds_with_none() -> TestResult {
    each_database("plan_02_starting_effect", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let effects: Vec<(String, String)> = kinds(&mut ada)
            .await?
            .iter()
            .map(|kind| {
                (
                    kind["forward"].as_str().unwrap_or_default().to_owned(),
                    kind["date_effect"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        assert_eq!(
            effects,
            [
                ("blocks", "must-finish-first"),
                ("duplicate of", "none"),
                ("causes", "none"),
                ("related to", "none"),
            ]
            .map(|(forward, effect)| (forward.to_owned(), effect.to_owned()))
        );
        // A kind that someone makes has none, and the effect is changed on the version looked at.
        let mine = new_kind(&mut ada, "leads to", "comes from").await?;
        assert_eq!(mine["date_effect"], "none");
        let given = set_effect(&mut ada, &mine, "must-finish-first").await?;
        assert_eq!(ok(&given)?["date_effect"], "must-finish-first");
        let stale = set_effect(&mut ada, &mine, "none").await?;
        assert_eq!(stale.code(), "edit.version-changed", "{}", stale.body);
        // A word that is no effect is no request.
        let unknown = set_effect(&mut ada, &given.body, "start-together").await?;
        assert_eq!(unknown.status, StatusCode::BAD_REQUEST, "{}", unknown.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_02_at_most_two_kinds_carry_must_finish_first() -> TestResult {
    each_database("plan_02_two_kinds", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let second = new_kind(&mut ada, "leads to", "comes from").await?;
        let third = new_kind(&mut ada, "feeds", "is fed by").await?;
        // *blocks* has it already; a second kind may.
        ok(&set_effect(&mut ada, &second, "must-finish-first").await?)?;
        // The third is refused with a sentence that names the two.
        let refused = set_effect(&mut ada, &third, "must-finish-first").await?;
        assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);
        assert_eq!(refused.code(), "relation-kind.date-effect-most");
        assert_eq!(refused.body["params"]["most"], 2, "{}", refused.body);
        assert_eq!(
            refused.body["date_effect_kinds"],
            json!(["blocks", "leads to"]),
            "{}",
            refused.body
        );
        assert_eq!(kind_called(&mut ada, "feeds").await?["date_effect"], "none");
        // Taking the effect from one makes room for the other.
        let second = kind_called(&mut ada, "leads to").await?;
        ok(&set_effect(&mut ada, &second, "none").await?)?;
        let third = kind_called(&mut ada, "feeds").await?;
        ok(&set_effect(&mut ada, &third, "must-finish-first").await?)?;
        // A kind that has the effect already may be given it again, and changing its name is no
        // new effect.
        let third = kind_called(&mut ada, "feeds").await?;
        ok(&ada
            .send(
                "PATCH",
                &format!(
                    "/api/v1/relation-kinds/{}",
                    third["id"].as_str().ok_or("no ID")?
                ),
                Some(json!({"version": third["version"], "forward": "supplies"})),
            )
            .await?)?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_02_a_relation_that_closes_a_loop_of_dependencies_is_refused() -> TestResult {
    each_database("plan_02_loop", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let a = key_of(&site.create(&mut ada, "Design").await?)?;
        let b = key_of(&site.create(&mut ada, "Build").await?)?;
        let c = key_of(&site.create(&mut ada, "Test").await?)?;
        let blocks = kind_called(&mut ada, "blocks").await?;
        let related = kind_called(&mut ada, "related to").await?;
        // Design blocks Build, Build blocks Test.
        ok(&relate(&mut ada, &a, &blocks, &b).await?)?;
        ok(&relate(&mut ada, &b, &blocks, &c).await?)?;
        // Test blocks Design would close the loop, and the answer names it in order.
        let refused = relate(&mut ada, &c, &blocks, &a).await?;
        assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);
        assert_eq!(refused.code(), "relation.dependency-loop");
        assert_eq!(
            refused.body["dependency_loop"],
            json!([c, a, b]),
            "{}",
            refused.body
        );
        // Read from the other end, *Design is blocked by Test* is the same loop.
        let backwards = ada
            .send(
                "POST",
                &format!("{ISSUES}/{a}/relations"),
                Some(json!({"other": c, "relation_kind": blocks["id"], "direction": "backward"})),
            )
            .await?;
        assert_eq!(
            backwards.code(),
            "relation.dependency-loop",
            "{}",
            backwards.body
        );
        // A kind with no effect says nothing about dates, so the same pair is fine.
        ok(&relate(&mut ada, &c, &related, &a).await?)?;
        // The other way round, Test after Design is no loop at all.
        ok(&relate(&mut ada, &a, &blocks, &c).await?)?;
        // Giving the effect to a kind is refused when its relations close a loop with the others:
        // *leads to* links Test to Design, and Design already waits for nothing but blocks Test.
        let leads = new_kind(&mut ada, "leads to", "comes from").await?;
        ok(&relate(&mut ada, &c, &leads, &a).await?)?;
        let loop_made = set_effect(&mut ada, &leads, "must-finish-first").await?;
        assert_eq!(
            loop_made.code(),
            "relation.dependency-loop",
            "{}",
            loop_made.body
        );
        assert_eq!(
            loop_made.body["dependency_loop"].as_array().map(Vec::len),
            Some(2),
            "{}",
            loop_made.body
        );
        assert_eq!(
            kind_called(&mut ada, "leads to").await?["date_effect"],
            "none"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_02_a_loop_names_a_hidden_issue_only_as_an_issue_you_cannot_see() -> TestResult {
    each_database("plan_02_loop_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // A Guest sees only what they reported.
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let first = key_of(&site.create(&mut gus, "Gus's first").await?)?;
        let second = key_of(&site.create(&mut gus, "Gus's second").await?)?;
        let secret = key_of(&site.create(&mut ada, "Ada's secret").await?)?;
        let blocks = kind_called(&mut ada, "blocks").await?;
        // Gus's second blocks Ada's secret, which blocks Gus's first: Ada may relate them all.
        ok(&relate(&mut ada, &second, &blocks, &secret).await?)?;
        ok(&relate(&mut ada, &secret, &blocks, &first).await?)?;
        // Gus now says that his first blocks his second: that closes a loop through the secret.
        let refused = relate(&mut gus, &first, &blocks, &second).await?;
        assert_eq!(
            refused.code(),
            "relation.dependency-loop",
            "{}",
            refused.body
        );
        assert_eq!(
            refused.body["dependency_loop"],
            json!([first, second, null]),
            "{}",
            refused.body
        );
        // The answer holds nothing of the secret issue: not its ID, not a count.
        let text = refused.body.to_string();
        assert!(!text.contains(&secret), "{text}");
        assert!(!text.contains("Ada's secret"), "{text}");
        // Ada sees all three, so she is told which they are.
        let to_ada = relate(&mut ada, &first, &blocks, &second).await?;
        assert_eq!(
            to_ada.body["dependency_loop"],
            json!([first, second, secret]),
            "{}",
            to_ada.body
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_02_two_kinds_given_the_effect_at_the_same_moment_are_one_too_many() -> TestResult {
    each_database("plan_02_two_at_once", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let mut also_ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        // *blocks* has the effect; one more kind may, and two administrators give it to two kinds.
        let first = new_kind(&mut ada, "leads to", "comes from").await?;
        let second = new_kind(&mut ada, "feeds", "is fed by").await?;
        let (one, other) = tokio::join!(
            set_effect(&mut ada, &first, "must-finish-first"),
            set_effect(&mut also_ada, &second, "must-finish-first"),
        );
        let statuses = [one?.status, other?.status];
        assert_eq!(
            statuses
                .iter()
                .filter(|status| **status == StatusCode::OK)
                .count(),
            1,
            "{statuses:?}"
        );
        let with_effect = kinds(&mut ada)
            .await?
            .iter()
            .filter(|kind| kind["date_effect"] == "must-finish-first")
            .count();
        assert_eq!(with_effect, 2);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_02_a_kind_that_reads_the_same_from_both_ends_has_no_direction() -> TestResult {
    each_database("plan_02_no_direction", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        // *related to* reads alike from both ends: nothing can finish first.
        let related = kind_called(&mut ada, "related to").await?;
        let refused = set_effect(&mut ada, &related, "must-finish-first").await?;
        assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);
        assert_eq!(refused.code(), "relation-kind.no-direction");
        // A kind that is given the effect cannot be renamed to read alike afterwards either.
        let leads = new_kind(&mut ada, "leads to", "comes from").await?;
        let leads = ok(&set_effect(&mut ada, &leads, "must-finish-first").await?)?.clone();
        let same = ada
            .send(
                "PATCH",
                &format!(
                    "/api/v1/relation-kinds/{}",
                    leads["id"].as_str().ok_or("no ID")?
                ),
                Some(json!({"version": leads["version"], "backward": "leads to"})),
            )
            .await?;
        assert_eq!(same.code(), "relation-kind.no-direction", "{}", same.body);
        Ok(())
    })
    .await
}
