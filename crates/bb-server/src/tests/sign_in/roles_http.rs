//! The role routes over HTTP (SEC-08, AUTH-03): the permission catalogue, making a role from a
//! copy, changing and resetting a starting role, and deleting a role that somebody has.

use axum::http::StatusCode;
use bb_app::access::Permission;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::people_http::{id_of, ids_of, start};

/// The permissions in an answer, as their keys.
fn permissions_of(role: &Value) -> Vec<&str> {
    role["permissions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

#[tokio::test]
async fn sec_08_the_permission_catalogue_names_every_permission_and_where_it_counts() -> TestResult
{
    each_database("sec_08_http_catalogue", async |db| {
        let mut org = start(db).await?;
        let reply = org.ada.send("GET", "/api/v1/permissions", None).await?;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let listed: Vec<&str> = reply.body["permissions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|permission| permission["key"].as_str())
            .collect();
        let known: Vec<&str> = Permission::ALL.iter().map(|p| p.code()).collect();
        assert_eq!(listed, known, "the catalogue, in its order");
        let of = |key: &str| {
            reply.body["permissions"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|permission| permission["key"] == key)
                .cloned()
                .unwrap_or_default()
        };
        assert_eq!(of("make-projects")["scope"], "organization");
        assert_eq!(of("make-projects")["topic"], "projects");
        assert_eq!(of("see-projects")["scope"], "project");
        assert_eq!(of("manage-roles")["topic"], "people");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_a_role_is_made_from_a_copy_changed_and_deleted_over_http() -> TestResult {
    each_database("sec_08_http_copy_role", async |db| {
        let mut org = start(db).await?;
        let viewer = org.role_named("Viewer").await?;
        let list = org.ada.send("GET", "/api/v1/roles", None).await?;
        let names: Vec<&str> = list.body["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|role| role["name"].as_str())
            .collect();
        assert_eq!(
            names,
            [
                "Administrator",
                "Project lead",
                "Team member",
                "Contributor",
                "Viewer",
                "Guest"
            ],
            "the starting roles, in the order of roles"
        );
        assert_eq!(list.body["items"][4]["starting"], "viewer");

        let copy = org
            .ada
            .send(
                "POST",
                &format!("/api/v1/roles/{viewer}/copy"),
                Some(json!({"name": "Reviewer"})),
            )
            .await?;
        assert_eq!(copy.status, StatusCode::OK, "{}", copy.body);
        // The same permissions, but a role of its own: no starting role to reset to.
        assert_eq!(permissions_of(&copy.body), ["see-projects"]);
        assert!(copy.body["starting"].is_null());
        let reviewer = id_of(&copy)?;
        let taken = org
            .ada
            .send(
                "POST",
                &format!("/api/v1/roles/{viewer}/copy"),
                Some(json!({"name": "Reviewer"})),
            )
            .await?;
        assert_eq!(
            (taken.status, taken.code()),
            (StatusCode::CONFLICT, "name.taken")
        );
        let reset = org
            .ada
            .send("POST", &format!("/api/v1/roles/{reviewer}/reset"), None)
            .await?;
        assert_eq!(
            (reset.status, reset.code()),
            (StatusCode::BAD_REQUEST, "request.invalid"),
            "a copy of a starting role is no starting role"
        );

        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_a_role_is_changed_and_deleted_over_http() -> TestResult {
    each_database("sec_08_http_change_role", async |db| {
        let mut org = start(db).await?;
        let viewer = org.role_named("Viewer").await?;
        let copy = org
            .ada
            .send(
                "POST",
                &format!("/api/v1/roles/{viewer}/copy"),
                Some(json!({"name": "Reviewer"})),
            )
            .await?;
        let reviewer = id_of(&copy)?;

        // Change its permissions and the switch for two-step sign-in, and its name.
        let changed = org
            .ada
            .send(
                "PATCH",
                &format!("/api/v1/roles/{reviewer}"),
                Some(json!({
                    "name": "Reviewers",
                    "permissions": ["see-projects", "check-access"],
                    "two_step": true
                })),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        assert_eq!(
            permissions_of(&changed.body),
            ["see-projects", "check-access"]
        );
        assert_eq!(changed.body["two_step"], true);
        let page = org
            .ada
            .send("GET", &format!("/api/v1/roles/{reviewer}"), None)
            .await?;
        assert_eq!(page.body["role"]["name"], "Reviewers");
        assert_eq!(page.body["used_by"]["people"], 0);
        assert_eq!(page.body["used_by"]["whole_organization"], false);

        // Nobody has it, so it goes at once.
        let deleted = org
            .ada
            .send("DELETE", &format!("/api/v1/roles/{reviewer}"), None)
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        let gone = org
            .ada
            .send("GET", &format!("/api/v1/roles/{reviewer}"), None)
            .await?;
        assert_eq!(gone.code(), "resource.not_found");
        // A role made from nothing.
        let made = org
            .ada
            .send(
                "POST",
                "/api/v1/roles",
                Some(json!({"name": "Reader", "permissions": ["see-projects"], "two_step": false})),
            )
            .await?;
        assert_eq!(made.status, StatusCode::OK, "{}", made.body);
        let all = org
            .ada
            .send("GET", "/api/v1/roles?fields=id,name&limit=3", None)
            .await?;
        assert_eq!(ids_of(&all.body).len(), 3);
        assert!(all.body["next_cursor"].is_string());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_a_changed_starting_role_can_be_reset() -> TestResult {
    each_database("sec_08_http_reset_role", async |db| {
        let mut org = start(db).await?;
        let viewer = org.role_named("Viewer").await?;
        let changed = org
            .ada
            .send(
                "PATCH",
                &format!("/api/v1/roles/{viewer}"),
                Some(json!({
                    "name": "Lookers",
                    "permissions": ["see-projects", "make-projects", "see-status"],
                    "two_step": true
                })),
            )
            .await?;
        assert_eq!(permissions_of(&changed.body).len(), 3, "{}", changed.body);

        let reset = org
            .ada
            .send("POST", &format!("/api/v1/roles/{viewer}/reset"), None)
            .await?;
        assert_eq!(reset.status, StatusCode::OK, "{}", reset.body);
        // The starting permissions and the starting switch are back; the name stays, and the role
        // still knows where it came from.
        assert_eq!(permissions_of(&reset.body), ["see-projects"]);
        assert_eq!(reset.body["two_step"], false);
        assert_eq!(reset.body["name"], "Lookers");
        assert_eq!(reset.body["starting"], "viewer");
        let page = org
            .ada
            .send("GET", &format!("/api/v1/roles/{viewer}"), None)
            .await?;
        assert_eq!(permissions_of(&page.body["role"]), ["see-projects"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_a_role_in_use_is_deleted_only_with_a_replacement() -> TestResult {
    each_database("sec_08_http_delete_in_use", async |db| {
        let mut org = start(db).await?;
        let (mut bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        let viewer = org.role_named("Viewer").await?;
        let contributor = org.role_named("Contributor").await?;
        let acme = org
            .make("/api/v1/projects", json!({"short_name": "ACME", "name": "Acme"}))
            .await?;
        let group = org.make("/api/v1/groups", json!({"name": "Backend"})).await?;
        let used = org
            .make(
                "/api/v1/roles",
                json!({"name": "Used", "permissions": ["see-projects"], "two_step": false}),
            )
            .await?;
        org.make(
            &format!("/api/v1/projects/{acme}/assignments"),
            json!({"who": {"person": bob_id}, "role": used, "applies_to": "project-and-sub-projects"}),
        )
        .await?;
        org.make(
            "/api/v1/assignments",
            json!({"who": {"group": group}, "role": used}),
        )
        .await?;
        let before = bob
            .send("GET", &format!("/api/v1/projects/{acme}"), None)
            .await?;
        assert_eq!(before.status, StatusCode::OK, "Bob sees ACME through the role");

        // Somebody has it: it is not deleted without a replacement.
        let asked = org
            .ada
            .send("DELETE", &format!("/api/v1/roles/{used}"), None)
            .await?;
        assert_eq!((asked.status, asked.code()), (StatusCode::CONFLICT, "role.in_use"));
        let itself = org
            .ada
            .send("DELETE", &format!("/api/v1/roles/{used}?replacement={used}"), None)
            .await?;
        assert_eq!(itself.code(), "request.invalid");
        let nothing = org
            .ada
            .send(
                "DELETE",
                &format!("/api/v1/roles/{used}?replacement=00000000-0000-7000-8000-000000000000"),
                None,
            )
            .await?;
        assert_eq!(nothing.code(), "resource.not_found");
        let page = org
            .ada
            .send("GET", &format!("/api/v1/roles/{used}"), None)
            .await?;
        assert_eq!(page.body["used_by"]["projects"], 1);
        assert_eq!(page.body["used_by"]["people"], 1);
        assert_eq!(page.body["used_by"]["groups"], 1);
        assert_eq!(page.body["used_by"]["whole_organization"], true);

        // With a replacement every row gives the replacement instead.
        let deleted = org
            .ada
            .send(
                "DELETE",
                &format!("/api/v1/roles/{used}?replacement={contributor}"),
                None,
            )
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.body);
        let rows = org
            .ada
            .send("GET", &format!("/api/v1/projects/{acme}/assignments"), None)
            .await?;
        assert_eq!(rows.body["items"][0]["role"]["name"], "Contributor");
        assert_eq!(rows.body["items"][0]["applies_to"], "project-and-sub-projects");
        let org_rows = org.ada.send("GET", "/api/v1/assignments", None).await?;
        let roles: Vec<&str> = org_rows.body["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["role"]["name"].as_str())
            .collect();
        assert!(roles.contains(&"Contributor"), "{roles:?}");
        let after = bob
            .send("GET", &format!("/api/v1/projects/{acme}"), None)
            .await?;
        assert_eq!(after.status, StatusCode::OK, "Bob still sees ACME");

        // The Administrator role is the last way to manage roles: it cannot go, replacement or
        // not, and the refusal says why.
        let administrator = org.role_named("Administrator").await?;
        let lock_out = org
            .ada
            .send(
                "DELETE",
                &format!("/api/v1/roles/{administrator}?replacement={viewer}"),
                None,
            )
            .await?;
        assert_eq!(
            (lock_out.status, lock_out.code()),
            (StatusCode::CONFLICT, "access.would_lock_out")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_nobody_gives_a_replacement_with_more_than_they_hold() -> TestResult {
    each_database("sec_08_http_replacement_escalation", async |db| {
        let mut org = start(db).await?;
        let (_bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        let (mut dana, dana_id) = org.person("Dana Desk", "dana@example.test").await?;
        // Dana manages roles and decides who has them, and holds nothing else.
        let keeper = org
            .make(
                "/api/v1/roles",
                json!({"name": "Keeper", "permissions": ["manage-roles", "decide-roles", "see-projects"], "two_step": false}),
            )
            .await?;
        let strong = org
            .make(
                "/api/v1/roles",
                json!({"name": "Strong", "permissions": ["see-projects", "see-status"], "two_step": false}),
            )
            .await?;
        let weak = org
            .make(
                "/api/v1/roles",
                json!({"name": "Weak", "permissions": ["see-projects"], "two_step": false}),
            )
            .await?;
        org.make(
            "/api/v1/assignments",
            json!({"who": {"person": dana_id}, "role": keeper}),
        )
        .await?;
        org.make(
            "/api/v1/assignments",
            json!({"who": {"person": bob_id}, "role": weak}),
        )
        .await?;
        // Deleting Weak would give Bob Strong, which Dana does not hold: refused.
        let refused = dana
            .send(
                "DELETE",
                &format!("/api/v1/roles/{weak}?replacement={strong}"),
                None,
            )
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.too_much")
        );
        let still = org.ada.send("GET", &format!("/api/v1/roles/{weak}"), None).await?;
        assert_eq!(still.status, StatusCode::OK, "nothing was deleted");
        // A replacement she holds herself is fine.
        let allowed = dana
            .send(
                "DELETE",
                &format!("/api/v1/roles/{weak}?replacement={keeper}"),
                None,
            )
            .await?;
        assert_eq!(allowed.status, StatusCode::NO_CONTENT, "{}", allowed.body);
        Ok(())
    })
    .await
}
