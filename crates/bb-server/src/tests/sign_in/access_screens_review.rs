//! What the review of WP-12 asked to be pinned down: where a level comes from when the person who
//! reads may not know the project, the level a role has when no project says anything, what a
//! person may set (their own level, in the project and in what is inside it), and whom a checker
//! without a management permission can choose (SEC-03, SEC-04, PRIV-12, M2-Q3).

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_app::roles::{Given, Subject};
use bb_core::access::AppliesTo;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::access_screens::{CHECK, FIELDS, PEOPLE, at, browser_of, cell, named, role};
use super::policy_rig::Org;
use super::rig::Rig;
use super::status::app_with_probe;

#[tokio::test]
async fn sec_03_a_level_from_a_project_the_lead_cannot_see_is_not_named_in_the_tables() -> TestResult
{
    each_database("sec_03_hidden_origin", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let acme = org.project(None, "ACME").await?.id;
        let web = org.project(Some(acme), "WEB").await?.id;
        let lead = role(&org, StartingRole::ProjectLead).await?;
        let contributor = role(&org, StartingRole::Contributor).await?;
        let lea = org.person().await?;
        org.give(&lea, lead, web).await?;
        let mut ada = browser_of(&app, "ada@example.test").await?;
        for (table, subject, level) in [
            (FIELDS, "cost-center", "hidden"),
            (PEOPLE, "followers", "hidden"),
        ] {
            let set = ada
                .send(
                    "PUT",
                    &format!("{table}/{contributor}/{subject}{}", at(acme)),
                    Some(json!({"level": level})),
                )
                .await?;
            assert_eq!(set.status, StatusCode::NO_CONTENT, "{}", set.body);
        }
        let mut lea_browser = browser_of(&app, "person1@example.test").await?;
        for (table, subject) in [(FIELDS, "cost-center"), (PEOPLE, "followers")] {
            let reply = lea_browser
                .send("GET", &format!("{table}{}", at(web)), None)
                .await?;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
            let cell = cell(&reply.body, subject, contributor)?;
            assert_eq!(cell["level"], "hidden");
            assert_eq!(cell["origin"]["kind"], "inherited");
            assert_eq!(cell["origin"]["from"], json!({"shown": "hidden"}));
            let text = reply.body.to_string();
            assert!(!text.contains("ACME"), "{text}");
            assert!(!text.contains(&acme.to_string()), "{text}");
        }
        // The administrator, who sees ACME, reads its name.
        let all = ada
            .send("GET", &format!("{FIELDS}{}", at(web)), None)
            .await?;
        assert_eq!(
            cell(&all.body, "cost-center", contributor)?["origin"]["from"]["short_name"],
            "ACME"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_04_a_project_without_a_setting_shows_the_role_own_level_and_yours_is_the_persons_own()
-> TestResult {
    each_database("sec_04_role_own_and_yours", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let acme = org.project(None, "ACME").await?.id;
        let contributor = role(&org, StartingRole::Contributor).await?;
        let mut ada = browser_of(&app, "ada@example.test").await?;
        // Nothing is set for ACME: a Contributor has the role's own level, *Can see*.
        let fields = ada
            .send("GET", &format!("{FIELDS}{}", at(acme)), None)
            .await?;
        let own = cell(&fields.body, "cost-center", contributor)?;
        assert_eq!(own["level"], "can-see");
        assert_eq!(own["origin"], json!({"kind": "role_own"}));
        // A role that was given no level at all is closed: nothing set, so hidden.
        let blank = org.role("Blank", &[Permission::SeeProjects]).await?;
        let again = ada
            .send("GET", &format!("{FIELDS}{}", at(acme)), None)
            .await?;
        let closed = cell(&again.body, "title", blank.id)?;
        assert_eq!(closed["level"], "hidden");
        assert_eq!(closed["origin"], json!({"kind": "unset"}));

        // *Yours*, in the table of kinds, is what the person who asks holds themselves.
        let decider = org
            .role(
                "Decider",
                &[Permission::SeeProjects, Permission::DecideWhatRolesSee],
            )
            .await?;
        for (kind, level) in [("followers", "role-only"), ("assigned-to", "names")] {
            ada.send(
                "PUT",
                &format!("{PEOPLE}/{}/{kind}", decider.id),
                Some(json!({"level": level})),
            )
            .await?;
        }
        let dee = org.person().await?;
        org.give(&dee, decider.id, acme).await?;
        let mut dee_browser = browser_of(&app, "person1@example.test").await?;
        let kinds = dee_browser
            .send("GET", &format!("{PEOPLE}{}", at(acme)), None)
            .await?;
        let yours = |kind: &str| {
            kinds.body["rows"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|row| row["subject"] == kind)
                .map(|row| row["yours"].clone())
        };
        assert_eq!(yours("followers"), Some(json!("role-only")));
        assert_eq!(yours("assigned-to"), Some(json!("names")));
        assert_eq!(yours("mentions"), Some(json!("hidden")));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_04_a_level_set_on_a_project_never_gives_more_than_the_person_holds_in_its_sub_projects()
-> TestResult {
    each_database("sec_04_levels_reach_below", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let acme = org.project(None, "ACME").await?.id;
        let web = org.project(Some(acme), "WEB").await?.id;
        org.project(Some(web), "API").await?;
        let contributor = role(&org, StartingRole::Contributor).await?;
        let decider = org
            .role(
                "Decider",
                &[Permission::SeeProjects, Permission::DecideWhatRolesSee],
            )
            .await?;
        let editor = org.role("Editor", &[Permission::SeeProjects]).await?;
        let mut ada = browser_of(&app, "ada@example.test").await?;
        // The Decider holds *Can see* and *Role only*; the Editor *Can change* and *Names*.
        for (table, subject, low, high, who) in [
            (FIELDS, "title", "can-see", "can-change", decider.id),
            (PEOPLE, "assigned-to", "role-only", "names", decider.id),
        ] {
            ada.send(
                "PUT",
                &format!("{table}/{who}/{subject}"),
                Some(json!({"level": low})),
            )
            .await?;
            ada.send(
                "PUT",
                &format!("{table}/{}/{subject}", editor.id),
                Some(json!({"level": high})),
            )
            .await?;
        }
        // Lea decides in WEB and everything inside it, and is an Editor in WEB alone.
        let lea = org.person().await?;
        org.give(&lea, decider.id, web).await?;
        org.assign(
            &org.admin,
            Subject::Person(lea.id),
            Given::Role(editor.id),
            Some(web),
            AppliesTo::Project,
        )
        .await?;
        let mut lea_browser = browser_of(&app, "person1@example.test").await?;
        for (table, subject, low, high) in [
            (FIELDS, "title", "can-see", "can-change"),
            (PEOPLE, "assigned-to", "role-only", "names"),
        ] {
            let path = format!("{table}/{contributor}/{subject}{}", at(web));
            // In WEB she holds the higher level, but a level set there reaches API, where she holds less.
            let too_much = lea_browser
                .send("PUT", &path, Some(json!({"level": high})))
                .await?;
            assert_eq!(
                (too_much.status, too_much.code()),
                (StatusCode::FORBIDDEN, "access.too_much"),
                "{subject}"
            );
            let fine = lea_browser
                .send("PUT", &path, Some(json!({"level": low})))
                .await?;
            assert_eq!(fine.status, StatusCode::NO_CONTENT, "{subject}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_a_checker_without_a_management_permission_chooses_only_people_with_a_role_there()
-> TestResult {
    each_database("sec_03_picker_roles_only", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let acme = org.project(None, "ACME").await?.id;
        let web = org.project(Some(acme), "WEB").await?.id;
        let checker = org
            .role(
                "Checker",
                &[Permission::SeeProjects, Permission::CheckAccess],
            )
            .await?;
        let looker = org.role("Looker", &[Permission::SeeProjects]).await?;
        let mut ada = browser_of(&app, "ada@example.test").await?;
        // Project members are shown by name to the Checker, so only the rule of the list is at work.
        let set = ada
            .send(
                "PUT",
                &format!("{PEOPLE}/{}/project-members", checker.id),
                Some(json!({"level": "names"})),
            )
            .await?;
        assert_eq!(set.status, StatusCode::NO_CONTENT);
        let lea = named(&org, "Lea Looker", "lea@example.test").await?;
        let bea = named(&org, "Bea Baker", "bea@example.test").await?;
        let cleo = named(&org, "Cleo Coder", "cleo@example.test").await?;
        org.give(&lea, checker.id, web).await?;
        org.give(&bea, looker.id, acme).await?;
        let mut lea_browser = browser_of(&app, "lea@example.test").await?;
        let names = |reply: &super::http::Reply| -> Vec<String> {
            reply.body["items"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item["name"].as_str())
                .map(str::to_owned)
                .collect()
        };
        let listed = lea_browser
            .send("GET", &format!("{CHECK}{}", at(web)), None)
            .await?;
        assert_eq!(listed.status, StatusCode::OK, "{}", listed.body);
        let chosen = names(&listed);
        assert!(chosen.contains(&"Bea Baker".to_owned()), "{chosen:?}");
        assert!(chosen.contains(&"Lea Looker".to_owned()), "{chosen:?}");
        assert!(!chosen.contains(&"Cleo Coder".to_owned()), "{chosen:?}");
        // The answer follows the list: Bea (a role from ACME reaches WEB) can be checked, Cleo,
        // who has no role there, is as good as not there.
        let bea_path = format!("/api/v1/check-access/people/{}{}", bea.id, at(web));
        let cleo_path = format!("/api/v1/check-access/people/{}{}", cleo.id, at(web));
        assert_eq!(
            lea_browser.send("GET", &bea_path, None).await?.status,
            StatusCode::OK
        );
        let missing = lea_browser.send("GET", &cleo_path, None).await?;
        assert_eq!(
            (missing.status, missing.code()),
            (StatusCode::NOT_FOUND, "resource.not_found")
        );
        // Whoever manages sees everybody, Cleo too.
        let everybody = ada
            .send("GET", &format!("{CHECK}{}", at(web)), None)
            .await?;
        assert!(names(&everybody).contains(&"Cleo Coder".to_owned()));
        assert_eq!(
            ada.send("GET", &cleo_path, None).await?.status,
            StatusCode::OK
        );
        Ok(())
    })
    .await
}
