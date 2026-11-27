//! *Download my data* holds the parts of a person's records (ADR-0044, amendment 2026-11-24): rows
//! that name nobody themselves but belong to a record of the person, found through the ID of that
//! record (`#[person_for(part_of)]`). Two people each make every kind of part; each file holds the
//! parts of its person's records and nothing of the other's. Every flow runs on SQLite and
//! PostgreSQL.

use std::collections::BTreeSet;

use bb_app::access::StartingRole;
use bb_app::agent_work::GrantStore as _;
use bb_core::id::StandingRuleId;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::ai_mcp_rig::{Mcp, ai_on, sign_in_app_in};
use super::comments_http::{comment, lock};
use super::dashboards_http::{create, share, timer};
use super::data_exports_http::{MyFile, my_file};
use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, id_of, key_of, ok, text_in};
use super::removal_http::{remove, switch_off};
use super::rig::{PASSWORD, Rig};
use super::rules_hand_over_http::{agent_with_limit, rule_over};
use super::working_hours_http::{MINE, YESTERDAY};

/// A record and one kind of its parts: the kind of the record, the kind of the part, and the field
/// of the part that holds the ID of the record.
type Whole = (&'static str, &'static str, &'static str);

/// The parts of the first test: a work day with its times and breaks, a comment with its earlier
/// texts, a dashboard and a saved search with their shares, an access key with its permissions and
/// projects.
const PARTS: &[Whole] = &[
    ("work_day", "work_period", "day"),
    ("work_day", "work_break", "day"),
    ("comment", "comment_version", "comment"),
    ("dashboard", "dashboard_share", "dashboard"),
    ("saved_search", "saved_search_share", "search"),
    ("access_key", "access_key_permission", "key"),
    ("access_key", "access_key_project", "key"),
];

/// The parts of the second test: the projects of a connected app and the issues a standing rule
/// found.
const AGENT_PARTS: &[Whole] = &[
    ("oauth_grant", "oauth_grant_project", "grant"),
    ("standing_rule", "rule_match", "rule"),
];

/// The texts of `field` in the rows of `kind` in the file.
fn texts(file: &mut MyFile, kind: &str, field: &str) -> Result<BTreeSet<String>, Failure> {
    Ok(file
        .rows(kind)?
        .iter()
        .filter_map(|row| row[field].as_str().map(str::to_owned))
        .collect())
}

/// The IDs of the person's records of each kind in `parts`, after checking that the file holds
/// parts of every kind and that each belongs to one of the person's records.
fn own_records(file: &mut MyFile, parts: &[Whole]) -> Result<BTreeSet<String>, Failure> {
    let mut own = BTreeSet::new();
    for (whole, part, field) in parts {
        let records = texts(file, whole, "id")?;
        let wholes = texts(file, part, field)?;
        assert!(!wholes.is_empty(), "no {part} in {:?}", file.names());
        assert!(
            wholes.is_subset(&records),
            "{part} of records that are not the person's: {wholes:?} against {records:?}"
        );
        own.extend(records);
    }
    Ok(own)
}

/// Both files hold the parts of their own person's records, and no record of one is in the other.
async fn each_holds_only_their_own(
    rig: &Rig,
    (first, second): (&mut Browser, &mut Browser),
    parts: &[Whole],
) -> Result<(), Failure> {
    let firsts = own_records(&mut my_file(rig, first).await?, parts)?;
    let seconds = own_records(&mut my_file(rig, second).await?, parts)?;
    assert!(
        firsts.is_disjoint(&seconds),
        "{:?}",
        firsts.intersection(&seconds).collect::<Vec<_>>()
    );
    Ok(())
}

/// A work day of yesterday with a time worked and a break.
async fn work_day(person: &mut Browser) -> Result<(), Failure> {
    let at = |time: &str| format!("{YESTERDAY}T{time}:00Z");
    ok(&person
        .send(
            "PATCH",
            &format!("{MINE}/{YESTERDAY}"),
            Some(json!({
                "version": 0,
                "periods": [{"start": at("07:00"), "end": at("15:00")}],
                "breaks": [{"start": at("11:00"), "end": at("11:30")}],
                "reason": "Typed in afterwards",
            })),
        )
        .await?)?;
    Ok(())
}

/// A comment on `issue`, then its text changed once.
async fn edited_comment(person: &mut Browser, issue: &str, text: &str) -> Result<(), Failure> {
    let id = id_of(&comment(person, issue, text).await?)?;
    let token = lock(person, &id).await?;
    ok(&person
        .send(
            "PUT",
            &format!("/api/v1/comments/{id}"),
            Some(json!({"version": 1, "lock_token": token, "text": format!("{text} Again.")})),
        )
        .await?)?;
    Ok(())
}

/// A dashboard and a saved search, each shared with `project`.
async fn shared_dashboard_and_search(person: &mut Browser, project: &str) -> Result<(), Failure> {
    let dashboard = id_of(&create(person, "Team", vec![timer(0)]).await?)?;
    ok(&share(person, &dashboard, ("project", project)).await?)?;
    let search = text_in(
        ok(&person
            .send(
                "POST",
                "/api/v1/saved-searches",
                Some(json!({"name": "Open", "search_text": ""})),
            )
            .await?)?,
        "id",
    )?;
    ok(&person
        .send(
            "POST",
            &format!("/api/v1/saved-searches/{search}/shares"),
            Some(json!({"kind": "project", "id": project})),
        )
        .await?)?;
    Ok(())
}

/// An access key that may only see projects, and only `project`.
async fn key_for(person: &mut Browser, project: &str) -> Result<(), Failure> {
    ok(&person
        .send(
            "POST",
            "/api/v1/access-keys",
            Some(json!({
                "password": PASSWORD,
                "name": "Reports",
                "choice": "permissions",
                "permissions": ["see-projects"],
                "projects": [project],
            })),
        )
        .await?)?;
    Ok(())
}

/// Every part of the first test, made by `person`.
async fn every_part(site: &Site<'_>, person: &mut Browser) -> Result<(), Failure> {
    work_day(person).await?;
    let issue = key_of(&site.create(person, "Printer jams").await?)?;
    edited_comment(person, &issue, "It jams.").await?;
    shared_dashboard_and_search(person, &site.acme).await?;
    key_for(person, &site.acme).await
}

#[tokio::test]
async fn priv_03_the_file_holds_the_parts_of_the_persons_records_and_none_of_another_person()
-> TestResult {
    each_database("priv_03_parts_of_records", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        every_part(&site, &mut ada).await?;
        every_part(&site, &mut bob).await?;
        // Bob also writes under Ada's issue: his comment is his, not hers.
        let issue = key_of(&site.create(&mut ada, "Scanner").await?)?;
        edited_comment(&mut bob, &issue, "Same here.").await?;
        each_holds_only_their_own(&rig, (&mut ada, &mut bob), PARTS).await
    })
    .await
}

/// A team member with a connected app limited to ACME and an agent whose standing rule finds every
/// issue of ACME.
async fn app_and_rule(site: &Site<'_>, mcp: &Mcp<'_>, name: &str) -> Result<Browser, Failure> {
    let (mut person, _) = site.person(StartingRole::TeamMember, name).await?;
    sign_in_app_in(
        mcp,
        &mut person,
        "Claude Code",
        "see-only",
        "see-only",
        std::slice::from_ref(&site.acme),
    )
    .await?;
    let (agent, _) = agent_with_limit(&mut person, 5).await?;
    rule_over(site, &mut person, &agent, ("", "Everything"), false).await?;
    Ok(person)
}

#[tokio::test]
async fn priv_03_the_file_holds_the_projects_of_a_connected_app_and_the_finds_of_a_rule()
-> TestResult {
    each_database("priv_03_parts_of_agents", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        ai_on(&rig).await?;
        let mcp = Mcp::new(&app);
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut tess = app_and_rule(&site, &mcp, "tess").await?;
        let mut uma = app_and_rule(&site, &mcp, "uma").await?;
        // One issue that both rules find.
        site.create(&mut ada, "Printer jams").await?;
        rig.run_jobs().await?;
        each_holds_only_their_own(&rig, (&mut tess, &mut uma), AGENT_PARTS).await
    })
    .await
}

/// The titles and counts of one list of a removal plan.
pub(super) fn plan_list(plan: &serde_json::Value, list: &str) -> Vec<(String, i64)> {
    plan[list]
        .as_array()
        .into_iter()
        .flatten()
        .map(|kind| {
            (
                kind["title"].as_str().unwrap_or_default().to_owned(),
                kind["rows"].as_i64().unwrap_or_default(),
            )
        })
        .collect()
}

#[tokio::test]
async fn priv_04_a_removal_deletes_the_rules_with_what_they_found_and_the_plan_says_so()
-> TestResult {
    each_database("priv_04_rules_and_finds", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        rig.turn_ai_apps_and_agents_on().await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tess, tess_id) = site.person(StartingRole::TeamMember, "tess").await?;
        let (agent, _) = agent_with_limit(&mut tess, 5).await?;
        let rule = rule_over(&site, &mut tess, &agent, ("", "Everything"), false).await?;
        let rule: StandingRuleId = rule.parse().map_err(|_| "no rule ID")?;
        site.create(&mut ada, "Printer jams").await?;
        rig.run_jobs().await?;
        assert_eq!(rig.storage.matches_of(rule, None, 10).await?.len(), 1);
        switch_off(&mut ada, tess_id).await?;
        let plan = ada
            .send(
                "GET",
                &format!("/api/v1/people/{tess_id}/removal-plan"),
                None,
            )
            .await?;
        let gone = plan_list(&plan.body, "deleted_now");
        let kept = plan_list(&plan.body, "kept_as_former_user");
        // The rule goes, and what it found goes with it: the plan counts the rule, not its finds.
        assert!(
            gone.contains(&("Standing rules".to_owned(), 1)),
            "{}",
            plan.body
        );
        assert!(
            !gone
                .iter()
                .chain(&kept)
                .any(|(title, _)| title == "Issues found by standing rules"),
            "{}",
            plan.body
        );
        let done = remove(&mut ada, tess_id, json!({"comments": "keep"})).await?;
        assert_eq!(done["status"], 204, "{done}");
        rig.run_jobs().await?;
        assert!(rig.storage.matches_of(rule, None, 10).await?.is_empty());
        Ok(())
    })
    .await
}
