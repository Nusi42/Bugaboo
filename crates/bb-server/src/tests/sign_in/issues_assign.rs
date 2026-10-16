//! Who can be assigned to an issue (ISS-01, M3-F28, ADR-0022): the people who may see the issue,
//! listed by name only where *Assigned to* and *Project members* show names, and only to somebody
//! who may set the field.

use std::str::FromStr as _;

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_app::roles::SetFieldLevel;
use bb_core::access::FieldLevel;
use bb_core::fields::Field;
use bb_core::id::ProjectId;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::policy_rig::Member;
use super::rig::Rig;

/// A person who does not exist.
const NOBODY: &str = "01900000-0000-4000-8000-000000000000";

fn assignable_path(site: &Site<'_>) -> String {
    format!("/api/v1/projects/{}/assignable-people", site.acme)
}

fn ids_of(listed: &Value) -> Result<Vec<String>, Failure> {
    Ok(listed["items"]
        .as_array()
        .ok_or("no list")?
        .iter()
        .filter_map(|person| person["id"].as_str().map(str::to_owned))
        .collect())
}

/// Tries to assign an issue to a person, on this version; the status and the code of the answer.
async fn assign(
    admin: &mut Browser,
    issue: &str,
    version: i64,
    person: &str,
) -> Result<(StatusCode, String), Failure> {
    let reply = admin
        .send(
            "PATCH",
            &format!("{ISSUES}/{issue}"),
            Some(json!({"version": version, "assigned_to": [person]})),
        )
        .await?;
    Ok((reply.status, reply.code().to_owned()))
}

#[tokio::test]
async fn m3_f28_an_issue_can_be_assigned_to_those_who_may_see_it_only() -> TestResult {
    each_database("m3_f28_assignable_people", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, cora) = site.person(StartingRole::Contributor, "cora").await?;
        let (_, vera) = site.person(StartingRole::Viewer, "vera").await?;
        let (mut gus, gus_id) = site.person(StartingRole::Guest, "gus").await?;
        let outsider = site.outsider("olga").await?;
        let path = assignable_path(&site);
        let listed = ada.send("GET", &path, None).await?;
        let ids = ids_of(ok(&listed)?)?;
        assert!(ids.contains(&cora.to_string()), "{ids:?}");
        assert!(ids.contains(&vera.to_string()), "{ids:?}");
        // A Guest cannot see the issues of others and an outsider has no role: neither is offered.
        assert!(!ids.contains(&gus_id.to_string()), "{ids:?}");
        assert!(!ids.contains(&outsider.to_string()), "{ids:?}");
        // Assigning is refused alike for a person who does not exist, one with no role and a
        // Guest who did not report the issue.
        let made = site.create(&mut ada, "Who does it").await?;
        let issue = ok(&made)?["key"].as_str().ok_or("no key")?.to_owned();
        for person in [outsider.to_string(), gus_id.to_string(), NOBODY.to_owned()] {
            assert_eq!(
                assign(&mut ada, &issue, 1, &person).await?,
                (StatusCode::BAD_REQUEST, "request.invalid".to_owned())
            );
        }
        // A Guest can be assigned to the issue they reported themselves.
        let own = site.create(&mut gus, "Gus found it").await?;
        let own_key = ok(&own)?["key"].as_str().ok_or("no key")?.to_owned();
        let on_gus = assign(&mut ada, &own_key, 1, &gus_id.to_string()).await?;
        assert_eq!(on_gus.0, StatusCode::OK);
        let on_cora = assign(&mut ada, &issue, 1, &cora.to_string()).await?;
        assert_eq!(on_cora.0, StatusCode::OK);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0022_the_list_of_people_to_assign_follows_project_members_and_the_right_to_assign()
-> TestResult {
    each_database("adr_0022_assignable_names", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tim, tim_id) = site.person(StartingRole::TeamMember, "tim").await?;
        let (mut cora, _) = site.person(StartingRole::Contributor, "cora").await?;
        let (mut vera, _) = site.person(StartingRole::Viewer, "vera").await?;
        let path = assignable_path(&site);
        // A Team member sets *Assigned to* and sees the names of the project members.
        let team = tim.send("GET", &path, None).await?;
        assert!(ids_of(ok(&team)?)?.len() > 1);
        // With *Project members* as a role only, the Team member is offered only themselves.
        site.show_people(
            &mut ada,
            StartingRole::TeamMember,
            "project-members",
            "role-only",
        )
        .await?;
        let narrowed = tim.send("GET", &path, None).await?;
        assert_eq!(ids_of(ok(&narrowed)?)?, [tim_id.to_string()]);
        // A Contributor and a Viewer cannot set *Assigned to*: seeing issues is not enough.
        for (who, name) in [(&mut cora, "contributor"), (&mut vera, "viewer")] {
            let refused = who.send("GET", &path, None).await?;
            assert_eq!(
                (refused.status, refused.code()),
                (StatusCode::FORBIDDEN, "access.forbidden"),
                "{name}"
            );
        }
        Ok(())
    })
    .await
}

/// A person whose role may see and create issues and set *Assigned to*, but not change issues: the
/// path of the new-issue form.
async fn maker_who_assigns(site: &Site<'_>, rig: &Rig) -> Result<Browser, Failure> {
    let email = "maya@example.test";
    let role = site
        .org
        .role(
            "Maker",
            &[
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::CreateIssues,
            ],
        )
        .await?;
    for field in [Field::Title, Field::AssignedTo] {
        rig.api()
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: site.org.admin.session.clone(),
                    role: role.id,
                    project: None,
                    issue_type: None,
                    field: field.into(),
                    level: Some(FieldLevel::CanChange),
                },
            )
            .await?;
    }
    let started = rig.person(&site.org.admin.session, email).await?;
    let member = Member {
        id: started.person.id,
        session: started.session_token,
    };
    site.org.give_everywhere(&member, role.id).await?;
    Browser::signed_in_as(site.app, email).await
}

#[tokio::test]
async fn m3_f28_who_may_create_issues_and_set_assigned_to_gets_the_list_for_the_new_issue_form()
-> TestResult {
    each_database("m3_f28_new_issue_form", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let mut maya = maker_who_assigns(&site, &rig).await?;
        let listed = maya.send("GET", &assignable_path(&site), None).await?;
        assert_eq!(listed.status, StatusCode::OK, "{}", listed.body);
        assert!(!ids_of(&listed.body)?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn m3_f28_an_assigned_person_who_lost_access_does_not_stop_a_change() -> TestResult {
    each_database("m3_f28_assigned_person_lost_access", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, cora) = site.person(StartingRole::Contributor, "cora").await?;
        let made = site.create(&mut ada, "Cora has it").await?;
        let issue = ok(&made)?["key"].as_str().ok_or("no key")?.to_owned();
        assert_eq!(
            assign(&mut ada, &issue, 1, &cora.to_string()).await?.0,
            StatusCode::OK
        );
        // Cora's access goes away. The issue can still be changed, with her still on it.
        let member = Member {
            id: cora,
            session: String::new(),
        };
        let project = ProjectId::from_str(&site.acme)?;
        site.org.forbid(&member, project).await?;
        let changed = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({
                    "version": 2,
                    "assigned_to": [cora.to_string()],
                    "priority": "high"
                })),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        // But somebody else without access cannot be added.
        let outsider = site.outsider("olga").await?;
        assert_eq!(
            assign(&mut ada, &issue, 3, &outsider.to_string()).await?.0,
            StatusCode::BAD_REQUEST
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0027_who_is_shown_only_themselves_can_assign_only_themselves() -> TestResult {
    each_database("adr_0027_assign_names", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tim, tim_id) = site.person(StartingRole::TeamMember, "tim").await?;
        let (_, cora) = site.person(StartingRole::Contributor, "cora").await?;
        site.show_people(
            &mut ada,
            StartingRole::TeamMember,
            "project-members",
            "role-only",
        )
        .await?;
        // The list offers Tim only himself, and sending the ID of somebody else is no way round it.
        let made = site.create(&mut ada, "Who does it").await?;
        let issue = ok(&made)?["key"].as_str().ok_or("no key")?.to_owned();
        assert_eq!(
            assign(&mut tim, &issue, 1, &cora.to_string()).await?,
            (StatusCode::BAD_REQUEST, "request.invalid".to_owned())
        );
        let own = assign(&mut tim, &issue, 1, &tim_id.to_string()).await?;
        assert_eq!(own.0, StatusCode::OK);
        // Where names are shown again, anybody who may see the issue can be assigned.
        site.show_people(
            &mut ada,
            StartingRole::TeamMember,
            "project-members",
            "names",
        )
        .await?;
        let named = assign(&mut tim, &issue, 2, &cora.to_string()).await?;
        assert_eq!(named.0, StatusCode::OK);
        Ok(())
    })
    .await
}
