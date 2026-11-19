//! The plan of *Remove a person* over HTTP on SQLite and PostgreSQL (PRIV-04, ADR-0044, decision
//! 6): read from the walk and the keep periods, it says what goes now, what stays as *Former user*
//! and what the law keeps until when, and names no table.

use bb_policy::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, key_of, ok, version_of};
use super::rig::Rig;
use super::working_hours_http::{MINE, TODAY};

/// The titles of a list of the plan.
fn titles(plan: &Value, list: &str) -> Vec<String> {
    plan[list]
        .as_array()
        .map(|kinds| {
            kinds
                .iter()
                .filter_map(|kind| kind["title"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// How many rows the kind whose title holds `word` has in a list of the plan; none when the list
/// has no such kind.
fn rows_of(plan: &Value, list: &str, word: &str) -> Option<u64> {
    plan[list].as_array()?.iter().find_map(|kind| {
        kind["title"]
            .as_str()
            .filter(|title| title.to_lowercase().contains(word))
            .and_then(|_| kind["rows"].as_u64())
    })
}

/// Assigns `issue` to `person` as `browser`.
async fn assign(browser: &mut Browser, issue: &str, person: &str) -> Result<(), Failure> {
    let version = version_of(browser, issue).await?;
    ok(&browser
        .send(
            "PATCH",
            &format!("/api/v1/issues/{issue}"),
            Some(json!({"version": version, "assigned_to": [person]})),
        )
        .await?)?;
    Ok(())
}

/// Saves a search as `browser`; its ID.
pub(super) async fn save_search(browser: &mut Browser, name: &str) -> Result<String, Failure> {
    let saved = browser
        .send(
            "POST",
            "/api/v1/saved-searches",
            Some(json!({"name": name, "search_text": ""})),
        )
        .await?;
    ok(&saved)?["id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "a saved search has an ID".into())
}

/// What Bob leaves behind besides his comment and his work day: a notification he gets and one
/// he causes for Ada, a private search, and a search shared with a group he is in. The IDs of the
/// shared search and of the group.
pub(super) async fn bob_leaves_traces(
    site: &Site<'_>,
    rig: &Rig,
    ada: &mut Browser,
    bob: &mut Browser,
    issue: &str,
    bob_id: &str,
) -> Result<(String, String), Failure> {
    // A notification Bob gets, and one he causes for Ada.
    assign(ada, issue, bob_id).await?;
    assign(bob, issue, &site.org.admin.id.to_string()).await?;
    rig.run_jobs().await?;
    // A private search, and one shared with a group Bob is in.
    save_search(bob, "Mine alone").await?;
    let shared = save_search(bob, "For the team").await?;
    let group = ok(&ada
        .send("POST", "/api/v1/groups", Some(json!({"name": "Backend"})))
        .await?)?["id"]
        .as_str()
        .ok_or("a group has an ID")?
        .to_owned();
    for reply in [
        ada.send(
            "PUT",
            &format!("/api/v1/groups/{group}/members/{bob_id}"),
            None,
        )
        .await?,
        bob.send(
            "POST",
            &format!("/api/v1/saved-searches/{shared}/shares"),
            Some(json!({"kind": "group", "id": group})),
        )
        .await?,
    ] {
        assert!(reply.status.is_success(), "{}", reply.body);
    }
    Ok((shared, group))
}

#[tokio::test]
async fn priv_04_the_plan_names_what_goes_what_stays_and_until_when() -> TestResult {
    each_database("priv_04_removal_plan", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        let issue = key_of(&site.create(&mut ada, "Plan of a removal").await?)?;
        ok(&bob
            .send(
                "POST",
                &format!("/api/v1/issues/{issue}/comments"),
                Some(json!({"text": "Bob's comment"})),
            )
            .await?)?;
        ok(&bob
            .send("POST", &format!("{MINE}/start"), Some(json!({})))
            .await?)?;
        bob_leaves_traces(&site, &rig, &mut ada, &mut bob, &issue, &bob_id.to_string()).await?;
        let plan = ada
            .send(
                "GET",
                &format!("/api/v1/people/{bob_id}/removal-plan"),
                None,
            )
            .await?;
        let plan = ok(&plan)?.clone();
        // His session goes with the account; his comment stays as a record of the organization.
        assert!(!titles(&plan, "deleted_now").is_empty(), "{plan}");
        assert!(!titles(&plan, "kept_as_former_user").is_empty(), "{plan}");
        assert_eq!(plan["comments"], "keep");
        // The notifications he got go (of Ada's assignment and of the issue he follows); the ones
        // his comment and his assignment caused for Ada stay, showing Former user.
        assert_eq!(
            rows_of(&plan, "deleted_now", "notification"),
            Some(2),
            "{plan}"
        );
        assert_eq!(
            rows_of(&plan, "kept_as_former_user", "notification"),
            Some(2),
            "{plan}"
        );
        // The private search goes; the shared one stays with Former user as its maker.
        assert_eq!(
            rows_of(&plan, "deleted_now", "saved search"),
            Some(1),
            "{plan}"
        );
        assert_eq!(
            rows_of(&plan, "kept_as_former_user", "saved search"),
            Some(1),
            "{plan}"
        );
        let kept = plan["kept_by_law"].as_array().ok_or("no kept_by_law")?;
        let work_days = kept
            .iter()
            .find(|kept| kept["kind"] == "work-days")
            .ok_or("work days are kept by law")?;
        let until = work_days["until"]
            .as_str()
            .ok_or("work days have a last day")?;
        assert!(until > TODAY, "kept past today: {until}");
        let backups = kept
            .iter()
            .find(|kept| kept["kind"] == "backups")
            .ok_or("backups are named")?;
        assert!(backups.get("until").is_none(), "{backups}");
        assert!(
            backups["no_fixed_end"]
                .as_str()
                .is_some_and(|words| !words.is_empty())
        );
        // No list names a table.
        let text = plan.to_string();
        for table in ["sign_in_session", "work_day", "comment\""] {
            assert!(!text.contains(table), "{table} in {text}");
        }
        // Only a holder of Remove people sees the plan.
        let refused = bob
            .send(
                "GET",
                &format!("/api/v1/people/{bob_id}/removal-plan"),
                None,
            )
            .await?;
        assert_eq!(refused.code(), "access.forbidden", "{}", refused.body);
        Ok(())
    })
    .await
}
