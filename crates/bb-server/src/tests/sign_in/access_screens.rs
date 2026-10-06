//! The routes behind *Field permissions*, *Who sees people* and *Check access* over HTTP (SEC-04,
//! SEC-06, SEC-03, PRIV-12, PRIV-13, UX §5.2): what the screens read and set, with the refusals a
//! person gets, and what never reaches a browser that may not see it.

use axum::Router;
use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_app::auth::{AcceptInvitation, CreateInvitation};
use bb_app::roles::{ChangeRole, RolesQuery};
use bb_core::id::{ProjectId, RoleId};
use bb_testing::each_database;
use serde_json::{Value, json};

use super::http::{Browser, Failure, called};
use super::policy_rig::{Member, Org};
use super::rig::{Outcome, PASSWORD, Rig, secret_of};
use super::status::app_with_probe;

pub(super) const FIELDS: &str = "/api/v1/field-permissions";
pub(super) const PEOPLE: &str = "/api/v1/people-visibility";
pub(super) const CHECK: &str = "/api/v1/check-access/people";
/// An ID that no person, role or project has (the random kind).
pub(super) const NOBODY: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";

/// The role of the starting kind, as the administrator lists the roles.
pub(super) async fn role(org: &Org<'_>, starting: StartingRole) -> Result<RoleId, Failure> {
    let list = org
        .rig
        .api()
        .roles(
            Rig::info(),
            RolesQuery {
                session_token: org.admin.session.clone(),
                place: None,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    list.roles
        .into_iter()
        .find(|role| role.starting.as_deref() == Some(starting.key()))
        .map(|role| role.id)
        .ok_or_else(|| "the starting role is missing".into())
}

/// A person with a name of their own and no role yet.
pub(super) async fn named(org: &Org<'_>, name: &str, email: &str) -> Result<Member, Failure> {
    let created = org
        .rig
        .api()
        .create_invitation(
            Rig::info(),
            CreateInvitation {
                session_token: org.admin.session.clone(),
                email: email.to_owned(),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    let started = org
        .rig
        .api()
        .accept_invitation(
            Rig::info(),
            AcceptInvitation {
                secret: secret_of(&created.link_path),
                display_name: name.to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(Member {
        id: started.person.id,
        session: started.session_token,
    })
}

/// A browser of the person with this address, signed in.
pub(super) async fn browser_of(app: &Router, email: &str) -> Result<Browser, Failure> {
    let mut browser = Browser::new(app, &called());
    let signed_in = browser
        .send(
            "POST",
            "/api/v1/auth/sign-in",
            Some(json!({"email": email, "password": PASSWORD})),
        )
        .await?;
    assert_eq!(signed_in.status, StatusCode::OK, "{}", signed_in.body);
    Ok(browser)
}

/// The cell of a table: the row of the subject, the cell of the role.
pub(super) fn cell<'a>(
    table: &'a Value,
    subject: &str,
    role: RoleId,
) -> Result<&'a Value, Failure> {
    table["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["subject"] == subject)
        .and_then(|row| row["cells"].as_array())
        .into_iter()
        .flatten()
        .find(|cell| cell["role"] == role.to_string())
        .ok_or_else(|| format!("no cell for {subject}").into())
}

/// The path of a project, as the query of a route.
pub(super) fn at(project: ProjectId) -> String {
    format!("?project={project}")
}

#[tokio::test]
async fn sec_04_a_field_level_set_on_a_project_applies_to_its_sub_projects_and_can_be_changed_below()
-> Outcome {
    each_database("sec_04_levels_over_http", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let acme = org.project(None, "ACME").await?.id;
        let web = org.project(Some(acme), "WEB").await?.id;
        let api = org.project(Some(web), "API").await?.id;
        let contributor = role(&org, StartingRole::Contributor).await?;
        let mut ada = browser_of(&app, "ada@example.test").await?;
        let cell_of =
            async |ada: &mut Browser, place: Option<ProjectId>| -> Result<Value, Failure> {
                let path = place.map_or_else(
                    || FIELDS.to_owned(),
                    |project| format!("{FIELDS}{}", at(project)),
                );
                let table = ada.send("GET", &path, None).await?;
                assert_eq!(table.status, StatusCode::OK, "{}", table.body);
                Ok(cell(&table.body, "cost-center", contributor)?.clone())
            };

        // A Contributor starts with *Can see* on *Cost center*, as the role's own level.
        let own = cell_of(&mut ada, None).await?;
        assert_eq!(
            (own["level"].as_str(), own["origin"]["kind"].as_str()),
            (Some("can-see"), Some("role_own"))
        );

        // Hidden on ACME: set there, inherited by WEB and by API below it.
        let path =
            |project: ProjectId| format!("{FIELDS}/{contributor}/cost-center{}", at(project));
        let hidden = ada
            .send("PUT", &path(acme), Some(json!({"level": "hidden"})))
            .await?;
        assert_eq!(hidden.status, StatusCode::NO_CONTENT, "{}", hidden.body);
        let here = cell_of(&mut ada, Some(acme)).await?;
        assert_eq!(
            (here["level"].as_str(), here["origin"]["kind"].as_str()),
            (Some("hidden"), Some("set_here"))
        );
        for below in [web, api] {
            let inherited = cell_of(&mut ada, Some(below)).await?;
            assert_eq!(inherited["level"], "hidden");
            assert_eq!(inherited["origin"]["kind"], "inherited");
            assert_eq!(inherited["origin"]["from"]["shown"], "visible");
            assert_eq!(inherited["origin"]["from"]["short_name"], "ACME");
        }
        // The organization's own level did not change.
        assert_eq!(cell_of(&mut ada, None).await?["level"], "can-see");

        // Changed below: WEB says *Can see*, API inherits that from WEB, ACME keeps *Hidden*.
        let seen = ada
            .send("PUT", &path(web), Some(json!({"level": "can-see"})))
            .await?;
        assert_eq!(seen.status, StatusCode::NO_CONTENT);
        let web_cell = cell_of(&mut ada, Some(web)).await?;
        assert_eq!(
            (
                web_cell["level"].as_str(),
                web_cell["origin"]["kind"].as_str()
            ),
            (Some("can-see"), Some("set_here"))
        );
        let api_cell = cell_of(&mut ada, Some(api)).await?;
        assert_eq!(api_cell["level"], "can-see");
        assert_eq!(api_cell["origin"]["from"]["short_name"], "WEB");
        assert_eq!(cell_of(&mut ada, Some(acme)).await?["level"], "hidden");

        // The engine agrees: a Contributor on ACME has *Cost center* hidden in ACME and *Can see*
        // in WEB, with the same reasons.
        let bea = org.person().await?;
        org.give(&bea, contributor, acme).await?;
        let level = async |place| -> Result<_, Failure> {
            let answer = org.check(&org.admin, bea.id, Some(place)).await?;
            Ok(answer
                .fields
                .iter()
                .find(|field| field.subject == bb_core::fields::Field::CostCenter)
                .map(|field| field.level))
        };
        assert_eq!(
            level(acme).await?,
            Some(bb_core::access::FieldLevel::Hidden)
        );
        assert_eq!(level(api).await?, Some(bb_core::access::FieldLevel::CanSee));

        // Taking WEB's setting away: ACME's counts again below it.
        let reset = ada
            .send("PUT", &path(web), Some(json!({"level": null})))
            .await?;
        assert_eq!(reset.status, StatusCode::NO_CONTENT);
        let again = cell_of(&mut ada, Some(api)).await?;
        assert_eq!(
            (
                again["level"].as_str(),
                again["origin"]["from"]["short_name"].as_str()
            ),
            (Some("hidden"), Some("ACME"))
        );
        assert_eq!(level(api).await?, Some(bb_core::access::FieldLevel::Hidden));

        // A role's own level cannot be taken away, only changed.
        let own_path = format!("{FIELDS}/{contributor}/cost-center");
        let refused = ada
            .send("PUT", &own_path, Some(json!({"level": null})))
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::BAD_REQUEST, "request.invalid")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_04_who_may_change_a_level_is_judged_in_the_place_and_never_above_their_own() -> Outcome
{
    each_database("sec_04_levels_who_may", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let acme = org.project(None, "ACME").await?.id;
        let web = org.project(Some(acme), "WEB").await?.id;
        let other = org.project(None, "OTHER").await?.id;
        let lead_role = role(&org, StartingRole::ProjectLead).await?;
        let contributor = role(&org, StartingRole::Contributor).await?;
        let lea = org.person().await?;
        org.give(&lea, lead_role, acme).await?;
        let mut lea_browser = browser_of(&app, "person1@example.test").await?;
        let cell_path = |project: Option<ProjectId>| {
            format!(
                "{FIELDS}/{contributor}/cost-center{}",
                project.map_or_else(String::new, at)
            )
        };

        // A lead sets a level in their project and its sub-projects, not for the organization,
        // not in a project they have no role in (which they do not even know of).
        let in_acme = lea_browser
            .send(
                "PUT",
                &cell_path(Some(acme)),
                Some(json!({"level": "hidden"})),
            )
            .await?;
        assert_eq!(in_acme.status, StatusCode::NO_CONTENT, "{}", in_acme.body);
        let in_web = lea_browser
            .send(
                "PUT",
                &cell_path(Some(web)),
                Some(json!({"level": "can-see"})),
            )
            .await?;
        assert_eq!(in_web.status, StatusCode::NO_CONTENT);
        let everywhere = lea_browser
            .send("PUT", &cell_path(None), Some(json!({"level": "hidden"})))
            .await?;
        assert_eq!(
            (everywhere.status, everywhere.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        let org_table = lea_browser.send("GET", FIELDS, None).await?;
        assert_eq!(org_table.code(), "access.forbidden");
        let elsewhere = lea_browser
            .send(
                "PUT",
                &cell_path(Some(other)),
                Some(json!({"level": "hidden"})),
            )
            .await?;
        assert_eq!(
            (elsewhere.status, elsewhere.code()),
            (StatusCode::NOT_FOUND, "resource.not_found")
        );
        let table = lea_browser
            .send("GET", &format!("{PEOPLE}{}", at(other)), None)
            .await?;
        assert_eq!(table.code(), "resource.not_found");
        // Nobody who is not signed in reads or sets anything.
        let mut nobody = Browser::new(&app, &called());
        let unknown = nobody.send("GET", FIELDS, None).await?;
        assert_eq!(unknown.code(), "auth.not_signed_in");

        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_04_nobody_sets_a_level_above_their_own_over_http() -> Outcome {
    each_database("sec_04_levels_above_own", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let acme = org.project(None, "ACME").await?.id;
        let contributor = role(&org, StartingRole::Contributor).await?;
        // Never above their own: a person who decides what roles see, and sees *Title* only,
        // cannot make it changeable for somebody else.
        let narrow = org
            .role(
                "Decider",
                &[Permission::SeeProjects, Permission::DecideWhatRolesSee],
            )
            .await?;
        let mut ada = browser_of(&app, "ada@example.test").await?;
        let own = ada
            .send(
                "PUT",
                &format!("{FIELDS}/{}/title", narrow.id),
                Some(json!({"level": "can-see"})),
            )
            .await?;
        assert_eq!(own.status, StatusCode::NO_CONTENT);
        let dee = org.person().await?;
        org.give(&dee, narrow.id, acme).await?;
        let mut dee_browser = browser_of(&app, "person1@example.test").await?;
        let table = dee_browser
            .send("GET", &format!("{FIELDS}{}", at(acme)), None)
            .await?;
        let title_row = table.body["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|row| row["subject"] == "title")
            .ok_or("no row")?;
        assert_eq!(title_row["yours"], "can-see");
        let too_much = dee_browser
            .send(
                "PUT",
                &format!("{FIELDS}/{contributor}/title{}", at(acme)),
                Some(json!({"level": "can-change"})),
            )
            .await?;
        assert_eq!(
            (too_much.status, too_much.code()),
            (StatusCode::FORBIDDEN, "access.too_much")
        );
        // A level that is no level, a field that does not exist and a role that does not exist.
        let wrong = dee_browser
            .send(
                "PUT",
                &format!("{FIELDS}/{contributor}/title{}", at(acme)),
                Some(json!({"level": "can-eat"})),
            )
            .await?;
        assert_eq!(wrong.status, StatusCode::BAD_REQUEST);
        let no_field = dee_browser
            .send(
                "PUT",
                &format!("{FIELDS}/{contributor}/no-such-field{}", at(acme)),
                Some(json!({"level": "hidden"})),
            )
            .await?;
        assert!(no_field.status.is_client_error());
        let no_role = dee_browser
            .send(
                "PUT",
                &format!("{FIELDS}/{NOBODY}/title{}", at(acme)),
                Some(json!({"level": "hidden"})),
            )
            .await?;
        assert_eq!(no_role.code(), "resource.not_found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_12_who_sees_people_is_set_per_role_and_kind_and_inherited_like_field_levels()
-> Outcome {
    each_database("priv_12_kinds_over_http", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let acme = org.project(None, "ACME").await?.id;
        let web = org.project(Some(acme), "WEB").await?.id;
        let guest = role(&org, StartingRole::Guest).await?;
        let mut ada = browser_of(&app, "ada@example.test").await?;
        let table = ada.send("GET", PEOPLE, None).await?;
        assert_eq!(table.status, StatusCode::OK, "{}", table.body);
        // The ten kinds, as the architecture lists them, in the order of the catalogue; three
        // levels to choose from, the lowest first.
        let kinds: Vec<&str> = table.body["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["subject"].as_str())
            .collect();
        assert_eq!(kinds.len(), 10);
        assert!(kinds.contains(&"project-members"));
        assert_eq!(
            table.body["levels"],
            json!(["hidden", "role-only", "names"])
        );
        // A Guest starts with no names for project members and role only for who is assigned.
        assert_eq!(
            cell(&table.body, "project-members", guest)?["level"],
            "hidden"
        );
        assert_eq!(
            cell(&table.body, "assigned-to", guest)?["level"],
            "role-only"
        );

        // Names for Guests in ACME, inherited by WEB.
        let set = ada
            .send(
                "PUT",
                &format!("{PEOPLE}/{guest}/project-members{}", at(acme)),
                Some(json!({"level": "names"})),
            )
            .await?;
        assert_eq!(set.status, StatusCode::NO_CONTENT, "{}", set.body);
        let below = ada
            .send("GET", &format!("{PEOPLE}{}", at(web)), None)
            .await?;
        let inherited = cell(&below.body, "project-members", guest)?;
        assert_eq!(inherited["level"], "names");
        assert_eq!(inherited["origin"]["from"]["short_name"], "ACME");
        // And the other way: hidden for them in WEB.
        ada.send(
            "PUT",
            &format!("{PEOPLE}/{guest}/project-members{}", at(web)),
            Some(json!({"level": "hidden"})),
        )
        .await?;
        let web_table = ada
            .send("GET", &format!("{PEOPLE}{}", at(web)), None)
            .await?;
        assert_eq!(
            cell(&web_table.body, "project-members", guest)?["origin"]["kind"],
            "set_here"
        );
        // A person's kind that does not exist is no kind.
        let none = ada
            .send(
                "PUT",
                &format!("{PEOPLE}/{guest}/no-such-kind{}", at(acme)),
                Some(json!({"level": "names"})),
            )
            .await?;
        assert!(none.status.is_client_error());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_12_a_guest_sees_role_only_or_someone_as_set_and_no_name_reaches_their_browser()
-> Outcome {
    each_database("priv_12_guest_over_http", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let acme = org.project(None, "ACME").await?.id;
        let guest = role(&org, StartingRole::Guest).await?;
        let contributor = role(&org, StartingRole::Contributor).await?;
        // The Guest may check access here, to have a screen that shows a person to them.
        org.rig
            .api()
            .change_role(
                Rig::info(),
                ChangeRole {
                    session_token: org.admin.session.clone(),
                    role: guest,
                    name: None,
                    permissions: Some(vec![Permission::SeeProjects, Permission::CheckAccess]),
                    two_step: None,
                },
            )
            .await
            .map_err(|error| error.to_string())?;
        let gus = named(&org, "Gus Guest", "gus@example.test").await?;
        let bea = named(&org, "Bea Baker", "bea@example.test").await?;
        org.give(&gus, guest, acme).await?;
        org.give(&bea, contributor, acme).await?;
        let mut ada = browser_of(&app, "ada@example.test").await?;
        let mut gus_browser = browser_of(&app, "gus@example.test").await?;
        let secrets = ["Bea", "Baker", "bea@example.test", &bea.id.to_string()];
        let looks_at_bea = async |browser: &mut Browser| -> Result<Value, Failure> {
            let reply = browser
                .send("GET", &format!("{CHECK}/{}{}", bea.id, at(acme)), None)
                .await?;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
            Ok(reply.body)
        };
        let nothing_of_bea = |body: &Value| {
            let text = body.to_string();
            for secret in secrets {
                assert!(!text.contains(secret), "{secret} is in {text}");
            }
        };

        // Guests start with *Hidden* for project members: *Someone*, and nobody to choose from.
        let hidden = looks_at_bea(&mut gus_browser).await?;
        assert_eq!(hidden["person"], json!({"shown": "someone"}));
        nothing_of_bea(&hidden);
        let people = gus_browser
            .send("GET", &format!("{CHECK}{}", at(acme)), None)
            .await?;
        assert_eq!(people.status, StatusCode::OK);
        assert_eq!(people.body["items"], json!([]));
        nothing_of_bea(&people.body);

        // *Role only* for Guests in ACME: Bea is *a Contributor*, still without a name or an ID.
        ada.send(
            "PUT",
            &format!("{PEOPLE}/{guest}/project-members{}", at(acme)),
            Some(json!({"level": "role-only"})),
        )
        .await?;
        let by_role = looks_at_bea(&mut gus_browser).await?;
        assert_eq!(
            by_role["person"],
            json!({"shown": "role", "role": "Contributor"})
        );
        nothing_of_bea(&by_role);
        let still_nobody = gus_browser
            .send("GET", &format!("{CHECK}{}", at(acme)), None)
            .await?;
        assert_eq!(still_nobody.body["items"], json!([]));

        // *Names*: only now the person is named, and listed to choose.
        ada.send(
            "PUT",
            &format!("{PEOPLE}/{guest}/project-members{}", at(acme)),
            Some(json!({"level": "names"})),
        )
        .await?;
        let by_name = looks_at_bea(&mut gus_browser).await?;
        assert_eq!(by_name["person"]["shown"], "name");
        assert_eq!(by_name["person"]["name"], "Bea Baker");
        let listed = gus_browser
            .send("GET", &format!("{CHECK}{}&search=baker", at(acme)), None)
            .await?;
        assert_eq!(listed.body["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(listed.body["items"][0]["name"], "Bea Baker");

        // The administrators who set all this never get less than names in these screens (M2-Q3).
        let managers = looks_at_bea(&mut ada).await?;
        assert_eq!(managers["person"]["name"], "Bea Baker");
        ada.send(
            "PUT",
            &format!("{PEOPLE}/{guest}/project-members{}", at(acme)),
            Some(json!({"level": "hidden"})),
        )
        .await?;
        assert_eq!(looks_at_bea(&mut ada).await?["person"]["name"], "Bea Baker");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_a_hidden_field_is_missing_from_every_m2_answer_and_cannot_be_asked_for() -> Outcome
{
    each_database("sec_06_hidden_field_over_http", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        let acme = org.project(None, "ACME").await?.id;
        let checker = org
            .role(
                "Checker",
                &[Permission::SeeProjects, Permission::CheckAccess],
            )
            .await?;
        let cleo = org.person().await?;
        org.give(&cleo, checker.id, acme).await?;
        let mut cleo_browser = browser_of(&app, "person1@example.test").await?;
        // A new role holds no level: every field is hidden for Cleo.
        let answer = cleo_browser
            .send(
                "GET",
                &format!("/api/v1/check-access/people/{}{}", cleo.id, at(acme)),
                None,
            )
            .await?;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        // The answer names the fields of the catalogue only as rows with a level, never a value.
        let cost = answer.body["fields"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|field| field["subject"] == "cost-center")
            .ok_or("no cost center")?;
        assert_eq!(cost["level"], "hidden");
        // Asking for a field she may not see fails exactly as asking for one that does not exist.
        let hidden = cleo_browser
            .send(
                "GET",
                &format!("{CHECK}{}&fields=name,cost_center", at(acme)),
                None,
            )
            .await?;
        let unknown = cleo_browser
            .send(
                "GET",
                &format!("{CHECK}{}&fields=name,no_such_field", at(acme)),
                None,
            )
            .await?;
        assert_eq!(hidden.status, StatusCode::BAD_REQUEST);
        assert_eq!(hidden.code(), "request.unknown_field");
        // The two answers are the same but for the ID of the request.
        assert_eq!(hidden.status, unknown.status);
        for member in ["type", "title", "status", "code", "message_key"] {
            assert_eq!(hidden.body[member], unknown.body[member], "{member}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_check_access_over_http_names_no_project_the_checker_cannot_see_and_agrees_with_the_engine()
-> Outcome {
    each_database("sec_03_check_over_http", async |db| {
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
        let lea = org.person().await?;
        let bea = org.person().await?;
        // Lea checks in WEB; Bea has her role from ACME, a project Lea cannot see.
        org.give(&lea, checker.id, web).await?;
        org.give(&bea, looker.id, acme).await?;
        let mut lea_browser = browser_of(&app, "person1@example.test").await?;
        let reply = lea_browser
            .send(
                "GET",
                &format!("/api/v1/check-access/people/{}{}", bea.id, at(web)),
                None,
            )
            .await?;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let see = reply.body["permissions"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|permission| permission["permission"] == "see-projects")
            .ok_or("no answer for see-projects")?;
        assert_eq!(see["allowed"], true);
        assert_eq!(see["topic"], "projects");
        assert_eq!(see["reasons"][0]["kind"], "given_by_role");
        assert_eq!(see["reasons"][0]["row"]["inherited"], true);
        assert_eq!(
            see["reasons"][0]["row"]["project"],
            json!({"shown": "hidden"})
        );
        assert_eq!(see["reasons"][0]["role"]["name"], "Looker");
        let text = reply.body.to_string();
        assert!(!text.contains("ACME"), "{text}");
        assert!(text.contains("WEB"));
        // Every decision equals the engine's, which the administrator reads through the use case.
        let engine = org.check(&org.admin, bea.id, Some(web)).await?;
        for decision in reply.body["permissions"].as_array().into_iter().flatten() {
            let wanted = engine
                .permissions
                .iter()
                .find(|known| known.permission.code() == decision["permission"])
                .ok_or("the engine does not know the permission")?;
            assert_eq!(
                decision["allowed"], wanted.allowed,
                "{}",
                decision["permission"]
            );
        }
        // The administrator sees the project's name.
        let mut ada = browser_of(&app, "ada@example.test").await?;
        let all = ada
            .send(
                "GET",
                &format!("/api/v1/check-access/people/{}{}", bea.id, at(web)),
                None,
            )
            .await?;
        assert!(all.body.to_string().contains("ACME"));
        // Somebody who may not check, a project that is not there and a person that is not there.
        let mut bea_browser = browser_of(&app, "person2@example.test").await?;
        let refused = bea_browser
            .send(
                "GET",
                &format!("/api/v1/check-access/people/{}{}", lea.id, at(web)),
                None,
            )
            .await?;
        assert_eq!(refused.code(), "access.forbidden");
        let nobody = lea_browser
            .send(
                "GET",
                &format!("/api/v1/check-access/people/{NOBODY}{}", at(web)),
                None,
            )
            .await?;
        assert_eq!(nobody.code(), "resource.not_found");
        let no_project = lea_browser
            .send(
                "GET",
                &format!("/api/v1/check-access/people/{}?project=not-an-id", bea.id),
                None,
            )
            .await?;
        assert_eq!(no_project.code(), "resource.not_found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_the_people_to_check_are_listed_by_name_a_page_at_a_time_and_found_by_words()
-> Outcome {
    each_database("sec_03_people_to_check", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let app = app_with_probe(&rig, None);
        for (name, email) in [
            ("Cleo Coder", "cleo@example.test"),
            ("Bea Baker", "bea@example.test"),
            ("Ana Analyst", "ana@example.test"),
        ] {
            named(&org, name, email).await?;
        }
        let mut ada = browser_of(&app, "ada@example.test").await?;
        let names = |reply: &super::http::Reply| -> Vec<String> {
            reply.body["items"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item["name"].as_str())
                .map(str::to_owned)
                .collect()
        };
        let first = ada.send("GET", &format!("{CHECK}?limit=3"), None).await?;
        assert_eq!(first.status, StatusCode::OK, "{}", first.body);
        assert_eq!(names(&first), ["Ada Admin", "Ana Analyst", "Bea Baker"]);
        let cursor = first.body["next_cursor"].as_str().ok_or("no next page")?;
        let second = ada
            .send("GET", &format!("{CHECK}?limit=3&cursor={cursor}"), None)
            .await?;
        assert_eq!(names(&second), ["Cleo Coder"]);
        assert!(second.body["next_cursor"].is_null());
        let found = ada
            .send("GET", &format!("{CHECK}?search=analyst%20ana"), None)
            .await?;
        assert_eq!(names(&found), ["Ana Analyst"]);
        // Every item names the person by ID too, so the screen can ask for the answer.
        assert!(found.body["items"][0]["id"].is_string());
        assert_eq!(found.body["items"][0]["shown"], "name");
        // A cursor that was changed, and a field that does not exist.
        let bad = ada
            .send("GET", &format!("{CHECK}?cursor=nonsense"), None)
            .await?;
        assert_eq!(bad.code(), "request.bad_cursor");
        let unknown = ada
            .send("GET", &format!("{CHECK}?fields=email"), None)
            .await?;
        assert_eq!(unknown.code(), "request.unknown_field");
        Ok(())
    })
    .await
}
