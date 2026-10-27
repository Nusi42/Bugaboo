//! The people and group routes over HTTP (SEC-02, PRIV-12, M2-Q3): what the list of people and the
//! page of a person show, who is refused, and making, renaming, filling and deleting groups.
//! The helpers at the top are used by the other files of *Organization settings* too.

use axum::http::StatusCode;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::id_of;

/// What a test body of this family gets: the administrator, signed in.
pub(super) struct Organization {
    pub(super) ada: Browser,
}

/// A new organization with its administrator Ada.
pub(super) async fn start(db: &bb_testing::TestDatabase) -> Result<Organization, Failure> {
    let (rig, app) = rig_and_app(db).await?;
    let ada = Browser::signed_in(&app, &rig).await?;
    Ok(Organization { ada })
}

/// The IDs of the items of a page, in the order they come.
pub(super) fn ids_of(list: &Value) -> Vec<String> {
    list["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["id"].as_str())
        .map(str::to_owned)
        .collect()
}

impl Organization {
    /// A new person with the name and address, invited by Ada and signed in, and their ID.
    pub(super) async fn person(
        &mut self,
        name: &str,
        email: &str,
    ) -> Result<(Browser, String), Failure> {
        let mut person = self.ada.invite(email, name).await?;
        let me = person.send("GET", "/api/v1/auth/me", None).await?;
        let id = me.body["id"].as_str().ok_or("no id")?.to_owned();
        Ok((person, id))
    }

    /// Ada makes something and the answer must be a success: its ID.
    pub(super) async fn create(&mut self, path: &str, body: Value) -> Result<String, Failure> {
        let reply = self.ada.send("POST", path, Some(body)).await?;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
        id_of(&reply)
    }

    /// The ID of the role with this name, for the starting roles.
    pub(super) async fn role_named(&mut self, name: &str) -> Result<String, Failure> {
        let roles = self.ada.send("GET", "/api/v1/roles", None).await?;
        roles.body["items"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|role| role["name"] == name)
            .and_then(|role| role["id"].as_str())
            .map(str::to_owned)
            .ok_or_else(|| format!("no role {name}").into())
    }
}

#[tokio::test]
async fn sec_02_the_people_list_names_people_and_gives_addresses_to_people_who_manage_people()
-> TestResult {
    each_database("sec_02_http_people_list", async |db| {
        let mut org = start(db).await?;
        let (mut bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        let (_cleo, _) = org.person("Cleo Coder", "cleo@example.test").await?;

        let list = org.ada.send("GET", "/api/v1/people", None).await?;
        assert_eq!(list.status, StatusCode::OK, "{}", list.body);
        assert_eq!(list.header("cache-control"), "no-store");
        let names: Vec<&str> = list.body["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| item["person"]["name"].as_str())
            .collect();
        // By name, without regard to capitals: Ada, Bob, Cleo.
        assert_eq!(names, ["Ada Admin", "Bob Builder", "Cleo Coder"]);
        assert_eq!(list.body["items"][1]["person"]["shown"], "name");
        assert_eq!(list.body["items"][1]["person"]["id"], bob_id);
        assert_eq!(list.body["items"][1]["email"], "bob@example.test");
        // Groups and roles are only there when asked for.
        assert!(list.body["items"][1].get("groups").is_none());
        assert!(list.body["next_cursor"].is_null());

        // Words of the name or of the address, one page at a time.
        let found = org
            .ada
            .send("GET", "/api/v1/people?search=BUILD", None)
            .await?;
        assert_eq!(found.body["items"].as_array().map(Vec::len), Some(1));
        let by_address = org
            .ada
            .send("GET", "/api/v1/people?search=cleo@", None)
            .await?;
        assert_eq!(by_address.body["items"][0]["person"]["name"], "Cleo Coder");
        // A wildcard is a character like any other: no name has a `%`, so nobody is found.
        let wildcard = org
            .ada
            .send("GET", "/api/v1/people?search=%25", None)
            .await?;
        assert_eq!(wildcard.body["items"].as_array().map(Vec::len), Some(0));
        let first = org.ada.send("GET", "/api/v1/people?limit=2", None).await?;
        let cursor = first.body["next_cursor"].as_str().ok_or("no next page")?;
        let second = org
            .ada
            .send(
                "GET",
                &format!("/api/v1/people?limit=2&cursor={cursor}"),
                None,
            )
            .await?;
        assert_eq!(
            second.body["items"][0]["person"]["name"], "Cleo Coder",
            "the page after Bob"
        );
        assert!(second.body["next_cursor"].is_null());

        // A cursor that was changed, and a field that does not exist, are told apart from the rest.
        let bad = org
            .ada
            .send("GET", "/api/v1/people?cursor=not-a-cursor", None)
            .await?;
        assert_eq!(
            (bad.status, bad.code()),
            (StatusCode::BAD_REQUEST, "request.bad-cursor")
        );
        let unknown = org
            .ada
            .send("GET", "/api/v1/people?fields=password", None)
            .await?;
        assert_eq!(unknown.code(), "request.unknown-field");

        // Somebody without the permission is refused, and learns nothing.
        let refused = bob.send("GET", "/api/v1/people", None).await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        let refused = bob
            .send("GET", &format!("/api/v1/people/{bob_id}"), None)
            .await?;
        assert_eq!(refused.code(), "access.forbidden");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_the_page_of_a_person_lists_their_groups_and_the_roles_the_viewer_decides_about()
-> TestResult {
    each_database("sec_02_http_person_page", async |db| {
        let mut org = start(db).await?;
        let (_bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        let viewer = org.role_named("Viewer").await?;
        let acme = org
            .create("/api/v1/projects", json!({"short_name": "ACME", "name": "Acme"}))
            .await?;
        let group = org.create("/api/v1/groups", json!({"name": "Backend"})).await?;
        let added = org
            .ada
            .send("PUT", &format!("/api/v1/groups/{group}/members/{bob_id}"), None)
            .await?;
        assert_eq!(added.status, StatusCode::NO_CONTENT, "{}", added.body);
        org.create(
            "/api/v1/assignments",
            json!({"who": {"person": bob_id}, "role": viewer}),
        )
        .await?;
        org.create(
            &format!("/api/v1/projects/{acme}/assignments"),
            json!({"who": {"person": bob_id}, "role": viewer, "applies_to": "project-and-sub-projects"}),
        )
        .await?;

        let page = org
            .ada
            .send("GET", &format!("/api/v1/people/{bob_id}"), None)
            .await?;
        assert_eq!(page.status, StatusCode::OK, "{}", page.body);
        let person = &page.body["person"];
        assert_eq!(person["person"]["name"], "Bob Builder");
        assert_eq!(person["email"], "bob@example.test");
        assert_eq!(person["groups"][0]["name"], "Backend");
        let places: Vec<&str> = person["roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["place"]["kind"].as_str())
            .collect();
        assert_eq!(places, ["organization", "project"]);
        assert_eq!(person["roles"][1]["place"]["short_name"], "ACME");
        assert_eq!(person["roles"][1]["role"]["name"], "Viewer");

        // The same on the list, when asked for.
        let list = org
            .ada
            .send("GET", "/api/v1/people?fields=person,groups,roles", None)
            .await?;
        let bob = list.body["items"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|item| item["person"]["name"] == "Bob Builder")
            .ok_or("no Bob")?;
        assert_eq!(bob["groups"][0]["name"], "Backend");
        assert_eq!(bob["roles"].as_array().map(Vec::len), Some(2));

        // Somebody who manages people, and nothing else, sees neither groups nor roles: the page
        // shows what they may know, and says nothing about what they may not.
        let people_only = org
            .create(
                "/api/v1/roles",
                json!({"name": "People only", "permissions": ["manage-people"], "two_step": false}),
            )
            .await?;
        let (mut dana, dana_id) = org.person("Dana Desk", "dana@example.test").await?;
        org.create(
            "/api/v1/assignments",
            json!({"who": {"person": dana_id}, "role": people_only}),
        )
        .await?;
        let seen = dana
            .send("GET", &format!("/api/v1/people/{bob_id}"), None)
            .await?;
        assert_eq!(seen.status, StatusCode::OK, "{}", seen.body);
        assert!(seen.body["person"].get("groups").is_none());
        assert_eq!(seen.body["person"]["roles"], json!([]));
        assert_eq!(seen.body["person"]["email"], "bob@example.test");

        // A person who is not there.
        let unknown = org
            .ada
            .send("GET", "/api/v1/people/5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f", None)
            .await?;
        assert_eq!(unknown.code(), "resource.not-found");
        let garbage = org.ada.send("GET", "/api/v1/people/not-an-id", None).await?;
        assert_eq!(garbage.code(), "resource.not-found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_groups_are_made_and_renamed_over_http() -> TestResult {
    each_database("sec_02_http_groups_made", async |db| {
        let mut org = start(db).await?;
        let group = org
            .create("/api/v1/groups", json!({"name": "Backend"}))
            .await?;
        let taken = org
            .ada
            .send("POST", "/api/v1/groups", Some(json!({"name": "Backend"})))
            .await?;
        assert_eq!(
            (taken.status, taken.code()),
            (StatusCode::CONFLICT, "name.taken")
        );
        let empty = org
            .ada
            .send("POST", "/api/v1/groups", Some(json!({"name": "   "})))
            .await?;
        assert_eq!(empty.code(), "request.invalid");

        let renamed = org
            .ada
            .send(
                "PATCH",
                &format!("/api/v1/groups/{group}"),
                Some(json!({"name": "Back end", "version": 1})),
            )
            .await?;
        assert_eq!(renamed.body["name"], "Back end");
        let list = org.ada.send("GET", "/api/v1/groups", None).await?;
        assert_eq!(list.body["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(list.body["items"][0]["name"], "Back end");

        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_the_people_of_a_group_are_listed_picked_and_taken_out_over_http() -> TestResult {
    each_database("sec_02_http_group_members", async |db| {
        let mut org = start(db).await?;
        let (_bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        let (_cleo, cleo_id) = org.person("Cleo Coder", "cleo@example.test").await?;
        let group = org
            .create("/api/v1/groups", json!({"name": "Backend"}))
            .await?;

        // Putting people in, with the picker that says who is in already.
        for person in [&bob_id, &cleo_id] {
            let added = org
                .ada
                .send(
                    "PUT",
                    &format!("/api/v1/groups/{group}/members/{person}"),
                    None,
                )
                .await?;
            assert_eq!(added.status, StatusCode::NO_CONTENT);
        }
        let members = org
            .ada
            .send(
                "GET",
                &format!("/api/v1/groups/{group}/members?limit=1"),
                None,
            )
            .await?;
        let cursor = members.body["next_cursor"].as_str().ok_or("no next page")?;
        let more = org
            .ada
            .send(
                "GET",
                &format!("/api/v1/groups/{group}/members?limit=1&cursor={cursor}"),
                None,
            )
            .await?;
        assert!(more.body["next_cursor"].is_null());
        // Both people, each once, whatever the order they were put in.
        let mut names = vec![
            members.body["items"][0]["person"]["name"].clone(),
            more.body["items"][0]["person"]["name"].clone(),
        ];
        names.sort_by_key(ToString::to_string);
        assert_eq!(names, [json!("Bob Builder"), json!("Cleo Coder")]);

        // The page of the group says what it is used for: nothing yet.
        let page = org
            .ada
            .send("GET", &format!("/api/v1/groups/{group}"), None)
            .await?;
        assert_eq!(page.body["used_by"]["members"], 2);
        assert_eq!(page.body["used_by"]["projects"], 0);
        assert_eq!(page.body["used_by"]["whole_organization"], false);

        let removed = org
            .ada
            .send(
                "DELETE",
                &format!("/api/v1/groups/{group}/members/{bob_id}"),
                None,
            )
            .await?;
        assert_eq!(removed.status, StatusCode::NO_CONTENT);
        let again = org
            .ada
            .send(
                "DELETE",
                &format!("/api/v1/groups/{group}/members/{bob_id}"),
                None,
            )
            .await?;
        assert_eq!(
            again.code(),
            "resource.not-found",
            "Bob is not in it any more"
        );

        let deleted = org
            .ada
            .send("DELETE", &format!("/api/v1/groups/{group}"), None)
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        let gone = org
            .ada
            .send("GET", &format!("/api/v1/groups/{group}"), None)
            .await?;
        assert_eq!(gone.code(), "resource.not-found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_the_picker_of_a_group_names_people_and_says_who_is_in_already() -> TestResult {
    each_database("sec_02_http_group_picker", async |db| {
        let mut org = start(db).await?;
        let (_bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        let group = org
            .create("/api/v1/groups", json!({"name": "Backend"}))
            .await?;
        org.ada
            .send(
                "PUT",
                &format!("/api/v1/groups/{group}/members/{bob_id}"),
                None,
            )
            .await?;
        let choices = org
            .ada
            .send(
                "GET",
                &format!(
                    "/api/v1/groups/{group}/member-choices?fields=id,name,already_in&search=ada"
                ),
                None,
            )
            .await?;
        assert_eq!(choices.body["items"][0]["name"], "Ada Admin");
        assert_eq!(choices.body["items"][0]["already_in"], false);
        assert!(choices.body["items"][0].get("email").is_none());
        let in_already = org
            .ada
            .send(
                "GET",
                &format!(
                    "/api/v1/groups/{group}/member-choices?fields=id,name,already_in&search=bob"
                ),
                None,
            )
            .await?;
        assert_eq!(in_already.body["items"][0]["already_in"], true);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_a_person_who_may_not_manage_groups_is_refused_on_every_group_route() -> TestResult {
    each_database("sec_02_http_groups_refused", async |db| {
        let mut org = start(db).await?;
        let (mut bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        let group = org
            .create("/api/v1/groups", json!({"name": "Backend"}))
            .await?;
        let routes: [(&str, String, Option<Value>); 8] = [
            ("GET", "/api/v1/groups".into(), None),
            (
                "POST",
                "/api/v1/groups".into(),
                Some(json!({"name": "Mine"})),
            ),
            ("GET", format!("/api/v1/groups/{group}"), None),
            (
                "PATCH",
                format!("/api/v1/groups/{group}"),
                Some(json!({"name": "Mine", "version": 1})),
            ),
            ("DELETE", format!("/api/v1/groups/{group}"), None),
            ("GET", format!("/api/v1/groups/{group}/members"), None),
            (
                "PUT",
                format!("/api/v1/groups/{group}/members/{bob_id}"),
                None,
            ),
            (
                "GET",
                format!("/api/v1/groups/{group}/member-choices"),
                None,
            ),
        ];
        for (method, path, body) in routes {
            let reply = bob.send(method, &path, body).await?;
            assert_eq!(
                (reply.status, reply.code()),
                (StatusCode::FORBIDDEN, "access.forbidden"),
                "{method} {path}"
            );
        }
        // Nothing changed.
        let list = org.ada.send("GET", "/api/v1/groups", None).await?;
        assert_eq!(list.body["items"][0]["name"], "Backend");
        Ok(())
    })
    .await
}
