//! A route does what it declares (M1-F32, ADR-0025 decision 1): over every route of the API
//! description that declares a permission of the catalogue in `x-permission`, a person without a
//! role is refused, a person whose role holds exactly that permission is let in, and the
//! administrator is let in. SEC-MATRIX (WP-10) widens this to every kind of actor.

use axum::http::StatusCode;
use bb_api::openapi;
use bb_app::access::Permission;
use bb_app::roles::{AssignRole, Given, MakeRole, Subject};
use bb_core::access::AppliesTo;
use bb_testing::each_database;
use serde_json::{Value, json};

use super::caching::invited_person;
use super::http::{Browser, Failure, called, setup_body};
use super::rig::{Outcome, PASSWORD, Rig};
use super::status::app_with_probe;

/// The routes that declare a permission of the catalogue: method, address, the key.
fn declared_routes() -> Result<Vec<(String, String, Permission)>, Failure> {
    let document = serde_json::to_value(openapi::document())?;
    let mut routes = Vec::new();
    for (path, item) in document["paths"].as_object().ok_or("no paths")? {
        for (method, operation) in item.as_object().ok_or("no operations")? {
            let declared = operation["x-permission"]
                .as_str()
                .ok_or("no x-permission")?;
            if let Some(permission) = Permission::from_code(declared) {
                routes.push((method.to_uppercase(), path.clone(), permission));
            }
        }
    }
    Ok(routes)
}

/// An example request for a route: the address with its parts filled in, and a valid body. A route
/// that is not here fails the test loudly, so a new route with a permission is never left out.
fn example(method: &str, path: &str, project: &str) -> Result<(String, Option<Value>), Failure> {
    let some_id = "0b8e5e6e-3b1d-4c1a-9d3e-5a5a5a5a5a5a";
    let in_project = |path: &str| path.replace("{project_id}", project);
    Ok(match (method, path) {
        (
            "GET",
            "/api/v1/auth/invitations"
            | "/api/v1/admin/status"
            | "/api/v1/auth/sign-up-policy"
            | "/api/v1/settings",
        ) => (path.to_owned(), None),
        ("POST", "/api/v1/auth/invitations") => (
            path.to_owned(),
            Some(json!({"email": "somebody@example.test"})),
        ),
        ("DELETE", "/api/v1/auth/invitations/{invitation_id}") => {
            (path.replace("{invitation_id}", some_id), None)
        }
        ("PUT", "/api/v1/auth/sign-up-policy") => (
            path.to_owned(),
            Some(json!({"enabled": false, "domains": []})),
        ),
        ("PUT", "/api/v1/settings/{key}") => (
            path.replace("{key}", "security.session-idle-minutes"),
            Some(json!({"value": 30})),
        ),
        ("DELETE", "/api/v1/settings/{key}") => {
            (path.replace("{key}", "security.session-idle-minutes"), None)
        }
        ("PUT" | "DELETE", "/api/v1/settings/{key}/lock") => {
            (path.replace("{key}", "appearance.mode"), None)
        }
        ("GET", "/api/v1/projects/{project_id}/settings") => (in_project(path), None),
        ("PUT", "/api/v1/projects/{project_id}/settings/{key}") => (
            in_project(path).replace("{key}", "test.project-note"),
            Some(json!({"value": "short"})),
        ),
        ("DELETE", "/api/v1/projects/{project_id}/settings/{key}") => {
            (in_project(path).replace("{key}", "test.project-note"), None)
        }
        _ => return Err(format!("{method} {path} needs an example request in this test").into()),
    })
}

#[tokio::test]
async fn m1_f32_a_route_enforces_the_permission_it_declares() -> Outcome {
    each_database("m1_f32_routes_enforce_declared_permissions", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let routes = declared_routes()?;
        assert!(routes.len() >= 6, "{routes:?}");
        let admin = rig
            .sign_in("ada@example.test", PASSWORD)
            .await?
            .session_token;
        let project = rig
            .api()
            .make_project(
                Rig::info(),
                bb_app::projects::MakeProject {
                    session_token: admin.clone(),
                    parent: None,
                    short_name: "ACME".to_owned(),
                    name: "Acme".to_owned(),
                },
            )
            .await?
            .into_value()
            .id
            .to_string();
        for (number, (method, path, permission)) in routes.iter().enumerate() {
            let (address, body) = example(method, path, &project)?;
            // A person without any role is refused; a project they may not see answers like one
            // that does not exist.
            let mut nobody =
                invited_person(&mut ada, &app, &format!("nobody{number}@example.test")).await?;
            let refused = nobody.send(method, &address, body.clone()).await?;
            let (status, code) = if path.contains("{project_id}") {
                (StatusCode::NOT_FOUND, "resource.not_found")
            } else {
                (StatusCode::FORBIDDEN, "access.forbidden")
            };
            assert_eq!(refused.status, status, "{method} {path}: no role");
            assert_eq!(refused.body["code"], code, "{method} {path}");
            // A person whose role holds exactly this permission is let in (what comes then is the
            // business of the route: not found, a missing probe, success).
            let mut only =
                invited_person(&mut ada, &app, &format!("only{number}@example.test")).await?;
            let role = rig
                .api()
                .make_role(
                    Rig::info(),
                    MakeRole {
                        session_token: admin.clone(),
                        name: format!("Only {permission} {number}"),
                        permissions: vec![*permission],
                        two_step: false,
                    },
                )
                .await?
                .into_value();
            let person = rig.person_id(&format!("only{number}@example.test")).await?;
            rig.api()
                .assign_role(
                    Rig::info(),
                    AssignRole {
                        session_token: admin.clone(),
                        subject: Subject::Person(person),
                        given: Given::Role(role.id),
                        project: None,
                        applies_to: AppliesTo::Organization,
                    },
                )
                .await?;
            let let_in = only.send(method, &address, body.clone()).await?;
            assert_ne!(
                let_in.status,
                StatusCode::FORBIDDEN,
                "{method} {path}: exactly {permission}"
            );
            assert_ne!(let_in.status, StatusCode::UNAUTHORIZED, "{method} {path}");
            // The administrator is let in too.
            let by_ada = ada.send(method, &address, body).await?;
            assert_ne!(
                by_ada.status,
                StatusCode::FORBIDDEN,
                "{method} {path}: administrator"
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_01_who_is_signed_in_answers_the_permissions_for_the_whole_organization() -> Outcome {
    each_database("sec_01_me_permissions", async |db| {
        let rig = Rig::start(db, 10).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let me = ada.send("GET", "/api/v1/auth/me", None).await?;
        assert_eq!(me.status, StatusCode::OK);
        assert_eq!(me.body["email"], "ada@example.test");
        assert!(me.body.get("administrator").is_none(), "the flag is gone");
        let held: Vec<&str> = me.body["permissions"]
            .as_array()
            .ok_or("no permissions")?
            .iter()
            .filter_map(Value::as_str)
            .collect();
        for permission in Permission::ALL {
            assert!(held.contains(&permission.code()), "{permission}");
        }
        // A person without a role holds nothing, and the answer is still a list.
        let mut bob = invited_person(&mut ada, &app, "bob@example.test").await?;
        let me = bob.send("GET", "/api/v1/auth/me", None).await?;
        assert_eq!(me.body["permissions"], json!([]));
        Ok(())
    })
    .await
}
