//! Deadline drivers, *Loop* and releases on the timeline as far as the reader sees (PLAN-05,
//! PLAN-08, ISS-08, ADR-0043 amendment of 2026-11-24), on SQLite and PostgreSQL: the walk along
//! dependencies reaches past the page and past its search, but never through an issue the reader
//! may not see, and a release the reader may not know is no release.

use bb_app::access::StartingRole;
use bb_core::id::{IssueId, RelationId, RelationKindId};
use bb_core::relation::Relation;
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{PROJECTS, Site, key_of, ok, text_in};
use super::plan_privacy_http::field_level;
use super::plan_site::{MONDAY, blocks, change, id_of_key, page, planned, warnings_on};

/// A page of the timeline of ACME with *Deadline drivers* on, of the issues `search` finds.
fn drivers_of(site: &Site<'_>, search: &str) -> Value {
    json!({
        "search_text": search,
        "project": site.acme,
        "timeline": {"zoom": "week", "deadline_drivers": true},
        "first": MONDAY,
        "last": "2027-03-31",
    })
}

/// The internal IDs of the drivers of a page, sorted.
fn driver_ids(page: &Value) -> Vec<String> {
    let mut ids: Vec<String> = page["drivers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|driver| driver["issue"].as_str().map(str::to_owned))
        .collect();
    ids.sort();
    ids
}

/// Makes a project the reader of a test has no role in; its ID.
async fn secret_project(ada: &mut Browser) -> Result<String, Failure> {
    let made = ada
        .send(
            "POST",
            PROJECTS,
            Some(json!({"short_name": "SEC", "name": "Secret"})),
        )
        .await?;
    text_in(ok(&made)?, "id")
}

/// An issue in the project `project` with these dates; its readable ID.
async fn dated_in(
    site: &Site<'_>,
    ada: &mut Browser,
    project: &str,
    title: &str,
    (first, last): (&str, &str),
) -> Result<String, Failure> {
    key_of(
        &ada.send(
            "POST",
            &format!("{PROJECTS}/{project}/issues"),
            Some(json!({
                "issue_type": site.kind,
                "title": title,
                "start_date": first,
                "due_date": last,
            })),
        )
        .await?,
    )
}

/// The page of the search *priority is high*: *Pick X* (Wed–Thu) waits for *Two before*
/// (Mon–Tue), which waits for *Far before* (Mon–Fri); *Pick Y* runs Monday to Saturday. Only
/// the two *Pick* issues are on the page. Along the chain *Pick X* ends on 9 March, after *Pick Y*;
/// without its far end it would end on the 4th. `middle` is the project of *Two before*.
async fn chain_past_the_page(
    site: &Site<'_>,
    ada: &mut Browser,
    middle: &str,
) -> Result<(String, String), Failure> {
    let high = |first: &str, last: &str| json!({"start_date": first, "due_date": last, "priority": "high"});
    let pick_x = planned(site, ada, "Pick X", high("2027-03-03", "2027-03-04")).await?;
    let pick_y = planned(site, ada, "Pick Y", high(MONDAY, "2027-03-06")).await?;
    let two = dated_in(site, ada, middle, "Two before", (MONDAY, "2027-03-02")).await?;
    let far = dated_in(site, ada, &site.acme, "Far before", (MONDAY, "2027-03-05")).await?;
    blocks(ada, &far, &two).await?;
    blocks(ada, &two, &pick_x).await?;
    Ok((
        id_of_key(ada, &pick_x).await?,
        id_of_key(ada, &pick_y).await?,
    ))
}

#[tokio::test]
async fn plan_05_a_chain_two_steps_past_the_page_and_its_search_sets_the_end() -> TestResult {
    each_database("plan_05_chain_past_page", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let acme = site.acme.clone();
        let (pick_x, _) = chain_past_the_page(&site, &mut ada, &acme).await?;
        let found = page(&mut ada, drivers_of(&site, "priority is high")).await?;
        assert_eq!(found["rows"].as_array().map(Vec::len), Some(2), "{found}");
        assert_eq!(driver_ids(&found), [pick_x], "{found}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_05_a_visible_issue_behind_a_hidden_one_off_the_page_counts_for_nothing() -> TestResult
{
    each_database("plan_05_behind_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let secret = secret_project(&mut ada).await?;
        let (pick_x, pick_y) = chain_past_the_page(&site, &mut ada, &secret).await?;
        let body = drivers_of(&site, "priority is high");
        // Ada sees the whole chain: *Pick X* ends last.
        assert_eq!(driver_ids(&page(&mut ada, body.clone()).await?), [pick_x]);
        // A Team member of ACME only: the walk stops at the hidden issue, so the visible one behind
        // it moves nothing, and *Pick Y* ends last.
        let (mut team, _) = site
            .person_in_acme(StartingRole::TeamMember, "team")
            .await?;
        let theirs = page(&mut team, body).await?;
        assert_eq!(driver_ids(&theirs), [pick_y], "{theirs}");
        assert!(!theirs.to_string().contains("Two before"), "{theirs}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_05_a_loop_through_a_hidden_issue_shows_no_warning() -> TestResult {
    each_database("plan_05_loop_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let secret = secret_project(&mut ada).await?;
        let shown = planned(
            &site,
            &mut ada,
            "Shown",
            json!({"start_date": MONDAY, "due_date": "2027-03-02"}),
        )
        .await?;
        let hidden = dated_in(
            &site,
            &mut ada,
            &secret,
            "Hidden",
            ("2027-03-03", "2027-03-04"),
        )
        .await?;
        blocks(&mut ada, &shown, &hidden).await?;
        let kinds = ok(&ada.send("GET", "/api/v1/relation-kinds", None).await?)?.clone();
        let blocks_kind = kinds["items"]
            .as_array()
            .and_then(|items| items.iter().find(|kind| kind["forward"] == "blocks"))
            .and_then(|kind| kind["id"].as_str())
            .ok_or("no kind blocks")?;
        let (shown_id, hidden_id) = (
            id_of_key(&mut ada, &shown).await?,
            id_of_key(&mut ada, &hidden).await?,
        );
        rig.database
            .store_relation_as_it_is(&Relation {
                id: RelationId::from_parts(Timestamp::UNIX_EPOCH, [8; 10])?,
                kind: blocks_kind.parse::<RelationKindId>()?,
                from: hidden_id.parse::<IssueId>()?,
                to: shown_id.parse::<IssueId>()?,
                added_by: site.org.admin.id,
                added_at: Timestamp::UNIX_EPOCH,
            })
            .await?;
        let everywhere = json!({
            "search_text": "",
            "timeline": {"zoom": "week"},
            "first": MONDAY,
            "last": "2027-03-31",
        });
        let is_loop = |page: &Value, id: &str| {
            warnings_on(page, id)
                .iter()
                .any(|warning| warning["kind"] == "loop")
        };
        let all = page(&mut ada, everywhere.clone()).await?;
        assert!(is_loop(&all, &shown_id), "{all}");
        // The loop passes through an issue the Team member of ACME may not see: no warning tells
        // of it.
        let (mut team, _) = site
            .person_in_acme(StartingRole::TeamMember, "team")
            .await?;
        let theirs = page(&mut team, everywhere).await?;
        assert!(!is_loop(&theirs, &shown_id), "{theirs}");
        Ok(())
    })
    .await
}

/// What a reader is told of a release on a page grouped by release: its mark, the groups of the
/// rows and the warnings that name it.
fn release_told(page: &Value) -> (usize, Vec<String>, usize) {
    let marks = page["releases"].as_array().map_or(0, Vec::len);
    let groups = page["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row["group"]["type"].as_str().map(str::to_owned))
        .collect();
    let delays = page["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|warning| !warning["release"].is_null())
        .count();
    (marks, groups, delays)
}

#[tokio::test]
async fn iss_08_a_release_the_reader_may_not_know_has_no_mark_no_group_and_no_delay() -> TestResult
{
    each_database("iss_08_release_unknown", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // WEB inside ACME; the release is made in ACME, and the late issue is in WEB.
        let web = text_in(
            ok(&ada
                .send(
                    "POST",
                    &format!("{PROJECTS}/{}/sub-projects", site.acme),
                    Some(json!({"short_name": "WEB", "name": "Web"})),
                )
                .await?)?,
            "id",
        )?;
        let release = text_in(
            ok(&ada
                .send(
                    "POST",
                    &format!("{PROJECTS}/{}/releases", site.acme),
                    Some(json!({"name": "Moonbeam", "release_date": "2027-03-03"})),
                )
                .await?)?,
            "id",
        )?;
        let late = dated_in(&site, &mut ada, &web, "Late", (MONDAY, "2027-03-05")).await?;
        ok(&change(&mut ada, &late, json!({"planned_for_release": release})).await?)?;
        let grouped = |project: &str| {
            json!({
                "search_text": "",
                "project": project,
                "timeline": {"zoom": "week"},
                "group_by": "planned for release",
                "first": MONDAY,
                "last": "2027-03-31",
            })
        };
        // Ada knows it: the mark, its group, the delay.
        let all = page(&mut ada, grouped(&web)).await?;
        assert_eq!(
            release_told(&all),
            (1, vec!["release".to_owned()], 1),
            "{all}"
        );
        // A Team member of WEB only does not see ACME: no mark, no group of its own, no delay.
        let (mut tim, _) = site
            .person_in(StartingRole::TeamMember, "tim", &web)
            .await?;
        let theirs = page(&mut tim, grouped(&web)).await?;
        assert_eq!(
            release_told(&theirs),
            (0, vec!["empty".to_owned()], 0),
            "{theirs}"
        );
        assert!(!theirs.to_string().contains("Moonbeam"), "{theirs}");
        // A Team member of ACME for whom *Planned for release* is hidden: no mark and no delay.
        field_level(
            &site,
            &mut ada,
            StartingRole::TeamMember,
            "planned-for-release",
            "hidden",
        )
        .await?;
        let (mut team, _) = site
            .person_in_acme(StartingRole::TeamMember, "team")
            .await?;
        // The field is no field of their search, so the rows are not grouped by it at all.
        let mut ungrouped = grouped(&site.acme);
        if let Some(body) = ungrouped.as_object_mut() {
            body.remove("group_by");
        }
        let hidden = page(&mut team, ungrouped).await?;
        assert_eq!(release_told(&hidden), (0, Vec::new(), 0), "{hidden}");
        Ok(())
    })
    .await
}
