//! What *Remove a person* leaves of others over HTTP on SQLite and PostgreSQL (PRIV-04, ADR-0044,
//! decision 6): the history of what the removed person did to the records of others keeps its
//! values, a name outside ASCII is scrubbed in any capitals, the agent work of the person stays
//! readable as *Former user*, and a switched-off administrator does not count as somebody who
//! decides roles.

use bb_core::id::UserId;
use bb_policy::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::agent_work_http::take;
use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, key_of, ok, version_of};
use super::removal_http::{events_text, remove, switch_off};
use super::rig::PASSWORD;

/// Gives `person` the starting role `role` for the whole organization, as `browser`.
async fn give(
    site: &Site<'_>,
    browser: &mut Browser,
    person: UserId,
    role: StartingRole,
) -> Result<String, Failure> {
    let given = browser
        .send(
            "POST",
            "/api/v1/assignments",
            Some(json!({"who": {"person": person}, "role": site.role_id(role)})),
        )
        .await?;
    Ok(ok(&given)?["id"]
        .as_str()
        .ok_or("no assignment ID")?
        .to_owned())
}

#[tokio::test]
async fn priv_04_the_roles_a_removed_administrator_gave_keep_their_history() -> TestResult {
    each_database("priv_04_removal_keeps_given_roles", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut max, max_id) = site
            .person_called(StartingRole::Administrator, "max", "Max Mustermann")
            .await?;
        let (_, cy_id) = site
            .person_called(StartingRole::TeamMember, "cy", "Cy Clerk")
            .await?;
        // Max gives Cy a role, then he is switched off and removed.
        give(&site, &mut max, cy_id, StartingRole::Viewer).await?;
        let viewer = site.role_id(StartingRole::Viewer);
        let before = events_text(&rig, cy_id).await?;
        assert!(before.contains(&viewer), "{before}");
        switch_off(&mut ada, max_id).await?;
        let done = remove(&mut ada, max_id, json!({})).await?;
        assert_eq!(done["status"], 204, "{done}");
        // The event of the role he gave Cy keeps its values.
        let after = events_text(&rig, cy_id).await?;
        assert_eq!(after, before);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_a_name_outside_ascii_is_scrubbed_as_typed_and_in_capitals() -> TestResult {
    const NAME: &str = "Özlem Çelik";
    const CAPITALS: &str = "ÖZLEM ÇELIK";
    each_database("priv_04_removal_unicode_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, ozlem) = site
            .person_called(StartingRole::TeamMember, "ozlem", NAME)
            .await?;
        // Ada writes her name into a title, as typed and in capitals, and changes it again: only
        // the history holds it.
        let issue = key_of(&site.create(&mut ada, &format!("Ask {NAME}")).await?)?;
        for title in [format!("Ask {CAPITALS}"), "Ask somebody".to_owned()] {
            let version = version_of(&mut ada, &issue).await?;
            ok(&ada
                .send(
                    "PATCH",
                    &format!("/api/v1/issues/{issue}"),
                    Some(json!({"version": version, "title": title})),
                )
                .await?)?;
        }
        assert!(!rig.storage.columns_holding(&[NAME]).await?.is_empty());
        switch_off(&mut ada, ozlem).await?;
        let done = remove(&mut ada, ozlem, json!({})).await?;
        assert_eq!(done["status"], 204, "{done}");
        rig.run_jobs().await?;
        // Nothing holds her name in any capitals, nor her handle.
        assert_eq!(
            rig.storage
                .columns_holding(&[NAME, CAPITALS, "ozlem.celik"])
                .await?,
            Vec::<String>::new()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_the_agent_work_of_a_removed_person_reads_as_former_user() -> TestResult {
    each_database("priv_04_removal_agent_work_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        rig.turn_ai_apps_and_agents_on().await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        let agent = bob
            .send("POST", "/api/v1/agents", Some(json!({"name": "Fixer"})))
            .await?;
        let agent = ok(&agent)?["id"].as_str().ok_or("no agent ID")?.to_owned();
        let made = bob
            .send(
                "POST",
                &format!("/api/v1/agents/{agent}/keys"),
                Some(json!({"password": PASSWORD, "name": "Key of Fixer"})),
            )
            .await?;
        let key = ok(&made)?["secret"].as_str().ok_or("no secret")?.to_owned();
        let issue = key_of(&site.create(&mut ada, "Hand this over").await?)?;
        ok(&bob
            .send(
                "POST",
                "/api/v1/hand-overs",
                Some(json!({
                    "agent": agent,
                    "issues": [issue],
                    "note": "Fix the title",
                    "powers": ["change-fields"],
                    "fields": ["title"],
                })),
            )
            .await?)?;
        take(&rig, &key, &issue).await?;
        switch_off(&mut ada, bob_id).await?;
        let done = remove(&mut ada, bob_id, json!({})).await?;
        assert_eq!(done["status"], 204, "{done}");
        // The work stays on the issue, without its agent and its hand-over, as Former user.
        let work = ada
            .send("GET", &format!("/api/v1/issues/{issue}/agent-work"), None)
            .await?;
        let body = ok(&work)?;
        let piece = &body["work"][0];
        assert_eq!(piece["agent_person"]["shown"], "former", "{body}");
        assert!(piece["agent_name"].is_null(), "{body}");
        assert!(piece["handed_over_at"].is_null(), "{body}");
        assert_eq!(piece["brief"], json!([]), "{body}");
        assert_eq!(body["worked_on_by"], json!([]), "{body}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_the_last_person_who_decides_roles_keeps_the_role_while_another_is_off() -> TestResult
{
    each_database("sec_08_removal_last_manager_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, max_id) = site
            .person_called(StartingRole::Administrator, "max", "Max Mustermann")
            .await?;
        // Ada may remove people without deciding roles.
        let remover = ok(&ada
            .send(
                "POST",
                "/api/v1/roles",
                Some(
                    json!({"name": "Remover", "permissions": ["remove-people"], "two_step": false}),
                ),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no role ID")?
            .to_owned();
        let admin = site.org.admin.id;
        ok(&ada
            .send(
                "POST",
                "/api/v1/assignments",
                Some(json!({"who": {"person": admin}, "role": remover})),
            )
            .await?)?;
        switch_off(&mut ada, max_id).await?;
        // Ada gives up her administrator role.
        let rows = ok(&ada.send("GET", "/api/v1/assignments", None).await?)?.clone();
        let administrator = site.role_id(StartingRole::Administrator);
        let hers = rows["items"]
            .as_array()
            .ok_or("no rows")?
            .iter()
            .find(|row| {
                row["person"]["id"] == admin.to_string().as_str()
                    && row["role"]["id"] == administrator.as_str()
            })
            .and_then(|row| row["id"].as_str())
            .ok_or("no administrator row of Ada")?
            .to_owned();
        let dropped = ada
            .send("DELETE", &format!("/api/v1/assignments/{hers}"), None)
            .await?;
        // A switched-off administrator counts as nobody, so Ada is the last one who decides roles
        // and keeps her role: the lockout is refused here, before any removal could cause it.
        assert_eq!(dropped.status.as_u16(), 409, "{}", dropped.body);
        assert_eq!(dropped.code(), "access.would-lock-out", "{}", dropped.body);
        // Removing Max takes nobody who decides roles away, so it goes through.
        let done = remove(&mut ada, max_id, json!({})).await?;
        assert_eq!(done["status"], 204, "{done}");
        Ok(())
    })
    .await
}
