//! The share of a person in a project over HTTP (PLAN-15, ADR-0030): who sets it, what a project
//! lead and a reader read of it, and the one rule that adds shares up. Every flow runs on SQLite and
//! PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{PROJECTS, Site, ok};
use super::work_times_http::{MINE, RANGE, project};

#[tokio::test]
async fn plan_15_a_share_is_set_only_with_decide_who_has_which_role_in_the_project() -> TestResult {
    each_database("plan_15_share_needs_decide_roles", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (mut bea, _) = site
            .person_in_acme(StartingRole::ProjectLead, "bea")
            .await?;
        let (mut cleo, cleo_id) = site
            .person_in_acme(StartingRole::TeamMember, "cleo")
            .await?;
        let (mut bob, bob_id) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        let path = format!("{PROJECTS}/{}/shares/{bob_id}", site.acme);
        // A Team member does not decide who has which role.
        let refused = cleo
            .send("PUT", &path, Some(json!({"percent": 60})))
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        let own = bob.send("PUT", &path, Some(json!({"percent": 10}))).await?;
        assert_eq!(
            own.status,
            StatusCode::FORBIDDEN,
            "nobody sets their own without the permission"
        );
        // A Project lead does.
        let set = bea.send("PUT", &path, Some(json!({"percent": 60}))).await?;
        assert_eq!(ok(&set)?["percent"], 60);
        let over = bea
            .send("PUT", &path, Some(json!({"percent": 101})))
            .await?;
        assert_eq!(over.code(), "work-times.share-invalid", "{}", over.body);
        let nobody = bea
            .send(
                "PUT",
                &format!(
                    "{PROJECTS}/{}/shares/01a3185c-5000-73b2-8f37-20757db0778e",
                    site.acme
                ),
                Some(json!({"percent": 60})),
            )
            .await?;
        assert_eq!(nobody.status, StatusCode::NOT_FOUND);
        // The list of the project shows it, to those who see people of the kind: 100 for nobody who
        // has none set.
        let listed = cleo
            .send("GET", &format!("{PROJECTS}/{}/shares", site.acme), None)
            .await?;
        let items = ok(&listed)?["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let share_of = |person: &str| -> Option<Value> {
            items
                .iter()
                .find(|item| item["person"]["id"] == person)
                .map(|item| item["percent"].clone())
        };
        assert_eq!(share_of(&bob_id.to_string()), Some(json!(60)), "{items:?}");
        assert_eq!(
            share_of(&cleo_id.to_string()),
            Some(json!(100)),
            "none set is 100"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_15_a_project_lead_sees_over_100_percent_without_other_projects_names() -> TestResult {
    each_database("plan_15_over_100_percent", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let secret = project(&mut ada, "SECRET").await?;
        let open = project(&mut ada, "OPEN").await?;
        let roles = site.org.starting_roles().await?;
        let lead = roles[StartingRole::ProjectLead.code()];
        let member = roles[StartingRole::TeamMember.code()];
        // Bea leads ACME and OPEN, and has no role in SECRET; Bob works in all three.
        let (mut bea, bea_member) = site.person_holding(lead, "bea", &site.acme).await?;
        site.org
            .give(
                &bea_member,
                lead,
                open.parse().map_err(|_| "no project ID")?,
            )
            .await?;
        let (mut bob, bob_member) = site.person_holding(member, "bob", &site.acme).await?;
        for project in [&secret, &open] {
            site.org
                .give(
                    &bob_member,
                    member,
                    project.parse().map_err(|_| "no project ID")?,
                )
                .await?;
        }
        for (project, percent) in [(&site.acme, 60), (&secret, 50), (&open, 20)] {
            let set = ada
                .send(
                    "PUT",
                    &format!("{PROJECTS}/{project}/shares/{}", bob_member.id),
                    Some(json!({"percent": percent})),
                )
                .await?;
            ok(&set)?;
        }
        // Bea sees that Bob is over 100 percent, and the projects she may see by name.
        let listed = bea
            .send("GET", &format!("{PROJECTS}/{}/shares", site.acme), None)
            .await?;
        let items = ok(&listed)?["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let bob_text = bob_member.id.to_string();
        let of_bob = items
            .iter()
            .find(|item| item["person"]["id"] == bob_text)
            .ok_or("Bob is not listed")?;
        // The fact, never the sum: it would tell how much the project she may not see holds.
        assert!(of_bob.get("sum").is_none(), "{of_bob}");
        assert_eq!(of_bob["over_100"], true);
        assert_eq!(
            of_bob["others"].as_array().map(Vec::len),
            Some(1),
            "{of_bob}"
        );
        // By name only: no share of the other project.
        assert_eq!(of_bob["others"][0]["short_name"], "OPEN");
        assert!(of_bob["others"][0].get("percent").is_none(), "{of_bob}");
        assert_eq!(of_bob["others_hidden"], true);
        // The project she may not see is counted and never named.
        let text = listed.body.to_string();
        assert!(
            !text.contains("SECRET") && !text.contains(&secret),
            "{text}"
        );
        // Somebody who does not decide who has which role sees the share and no sums.
        let (mut cleo, _) = site
            .person_in_acme(StartingRole::TeamMember, "cleo")
            .await?;
        let seen = cleo
            .send("GET", &format!("{PROJECTS}/{}/shares", site.acme), None)
            .await?;
        let theirs = ok(&seen)?["items"].as_array().cloned().unwrap_or_default();
        let of_bob = theirs
            .iter()
            .find(|item| item["person"]["id"] == bob_text)
            .ok_or("Bob is not listed for Cleo")?;
        assert_eq!(of_bob["percent"], 60);
        assert!(
            of_bob.get("sum").is_none() && of_bob.get("over_100").is_none(),
            "{of_bob}"
        );
        // Bob sees his own shares, the sum and that it is over 100 percent, by the names he may see.
        let mine = bob.send("GET", &format!("{MINE}?{RANGE}"), None).await?;
        assert_eq!(ok(&mine)?["shares"]["sum"], 130);
        assert_eq!(mine.body["shares"]["over_100"], true);
        assert_eq!(
            mine.body["shares"]["items"].as_array().map(Vec::len),
            Some(3)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_15_a_lead_of_one_project_sees_no_share_of_a_project_where_they_are_a_guest()
-> TestResult {
    each_database("plan_15_lead_sees_no_percent", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let other = project(&mut ada, "OTHER").await?;
        let roles = site.org.starting_roles().await?;
        let (lead, guest) = (
            roles[StartingRole::ProjectLead.code()],
            roles[StartingRole::Guest.code()],
        );
        let member = roles[StartingRole::TeamMember.code()];
        let other_id = other.parse().map_err(|_| "no project ID")?;
        // Bea leads ACME and is only a guest in OTHER, which she sees.
        let (mut bea, bea_member) = site.person_holding(lead, "bea", &site.acme).await?;
        site.org.give(&bea_member, guest, other_id).await?;
        let (_bob, bob_member) = site.person_holding(member, "bob", &site.acme).await?;
        site.org.give(&bob_member, member, other_id).await?;
        for (project, percent) in [(&site.acme, 70), (&other, 55)] {
            let set = ada
                .send(
                    "PUT",
                    &format!("{PROJECTS}/{project}/shares/{}", bob_member.id),
                    Some(json!({"percent": percent})),
                )
                .await?;
            ok(&set)?;
        }
        let listed = bea
            .send("GET", &format!("{PROJECTS}/{}/shares", site.acme), None)
            .await?;
        let bob_text = bob_member.id.to_string();
        let items = ok(&listed)?["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let of_bob = items
            .iter()
            .find(|item| item["person"]["id"] == bob_text)
            .ok_or("Bob is not listed")?;
        // She learns that the project exists and Bob works there, never how much.
        assert_eq!(of_bob["others"][0]["short_name"], "OTHER");
        assert!(of_bob["others"][0].get("percent").is_none(), "{of_bob}");
        assert!(of_bob.get("sum").is_none(), "{of_bob}");
        assert_eq!(of_bob["over_100"], true);
        Ok(())
    })
    .await
}

/// What both read, on the same data: the person's own shares and the lead's line for the person.
async fn both(
    bob: &mut Browser,
    bea: &mut Browser,
    acme: &str,
    bob_text: &str,
) -> Result<(Value, Value), Failure> {
    let mine = bob.send("GET", &format!("{MINE}?{RANGE}"), None).await?;
    let listed = bea
        .send("GET", &format!("{PROJECTS}/{acme}/shares"), None)
        .await?;
    let items = ok(&listed)?["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let of_bob = items
        .iter()
        .find(|item| item["person"]["id"] == bob_text)
        .ok_or("Bob is not listed")?
        .clone();
    Ok((ok(&mine)?["shares"].clone(), of_bob))
}

#[tokio::test]
async fn plan_15_one_rule_adds_up_the_shares_of_the_person_and_of_the_lead() -> TestResult {
    each_database("plan_15_one_sum_rule", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let other = project(&mut ada, "OTHER").await?;
        let roles = site.org.starting_roles().await?;
        let (lead, member) = (
            roles[StartingRole::ProjectLead.code()],
            roles[StartingRole::TeamMember.code()],
        );
        let other_id = other.parse().map_err(|_| "no project ID")?;
        let (mut bea, _) = site.person_holding(lead, "bea", &site.acme).await?;
        let (mut bob, bob_member) = site.person_holding(member, "bob", &site.acme).await?;
        site.org.give(&bob_member, member, other_id).await?;
        let bob_text = bob_member.id.to_string();
        // Only the other project has a share (50): the start value of ACME is nobody's choice.
        ok(&ada
            .send(
                "PUT",
                &format!("{PROJECTS}/{other}/shares/{bob_text}"),
                Some(json!({"percent": 50})),
            )
            .await?)?;
        let (own, lead_sees) = both(&mut bob, &mut bea, &site.acme, &bob_text).await?;
        assert_eq!(
            (own["sum"].clone(), own["over_100"].clone()),
            (json!(50), json!(false))
        );
        assert_eq!(lead_sees["over_100"], false, "{lead_sees}");
        // A share in ACME as well: 50 and 60 are over 100 for both.
        ok(&ada
            .send(
                "PUT",
                &format!("{PROJECTS}/{}/shares/{bob_text}", site.acme),
                Some(json!({"percent": 60})),
            )
            .await?)?;
        let (own, lead_sees) = both(&mut bob, &mut bea, &site.acme, &bob_text).await?;
        assert_eq!(
            (own["sum"].clone(), own["over_100"].clone()),
            (json!(110), json!(true))
        );
        assert_eq!(lead_sees["over_100"], true, "{lead_sees}");
        Ok(())
    })
    .await
}
