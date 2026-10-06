//! *Who has which role* over HTTP (SEC-02, SEC-03, PRIV-13, M2-Q1): rows for the whole
//! organization, for a project and for a project with its sub-projects, the rows that come from
//! above, *No access*, who is told what, and the last administrator.

use axum::http::StatusCode;
use bb_testing::each_database;
use serde_json::json;

use super::people_http::start;
use super::rig::Outcome;

/// The short names of the projects in a list, in the order they come.
fn short_names(list: &serde_json::Value) -> Vec<&str> {
    list["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["short_name"].as_str())
        .collect()
}

#[tokio::test]
async fn sec_02_people_and_groups_get_roles_on_the_whole_organization_a_project_or_a_project_and_its_sub_projects()
-> Outcome {
    each_database("sec_02_http_where_it_counts", async |db| {
        let mut org = start(db).await?;
        let viewer = org.role_named("Viewer").await?;
        let acme = org
            .make("/api/v1/projects", json!({"short_name": "ACME", "name": "Acme"}))
            .await?;
        org.make(
            &format!("/api/v1/projects/{acme}/sub-projects"),
            json!({"short_name": "WEB", "name": "Website"}),
        )
        .await?;
        org.make("/api/v1/projects", json!({"short_name": "LAB", "name": "Lab"}))
            .await?;
        let (mut bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        let (mut cleo, cleo_id) = org.person("Cleo Coder", "cleo@example.test").await?;
        let (mut dana, dana_id) = org.person("Dana Desk", "dana@example.test").await?;
        let (mut eve, eve_id) = org.person("Eve Example", "eve@example.test").await?;
        let (mut finn, finn_id) = org.person("Finn Field", "finn@example.test").await?;
        let group = org.make("/api/v1/groups", json!({"name": "Labs"})).await?;
        let lab = {
            let list = org.ada.send("GET", "/api/v1/projects?search=lab", None).await?;
            list.body["items"][0]["id"].as_str().ok_or("no lab")?.to_owned()
        };
        let web = {
            let list = org.ada.send("GET", "/api/v1/projects?search=web", None).await?;
            list.body["items"][0]["id"].as_str().ok_or("no web")?.to_owned()
        };
        org.ada
            .send("PUT", &format!("/api/v1/groups/{group}/members/{eve_id}"), None)
            .await?;

        // Bob: ACME and everything in it. Cleo: only the project WEB. Dana: the whole organization.
        // Eve: through her group, only LAB. Finn: only the project ACME, not what is in it.
        org.make(
            &format!("/api/v1/projects/{acme}/assignments"),
            json!({"who": {"person": bob_id}, "role": viewer, "applies_to": "project-and-sub-projects"}),
        )
        .await?;
        org.make(
            &format!("/api/v1/projects/{web}/assignments"),
            json!({"who": {"person": cleo_id}, "role": viewer, "applies_to": "project"}),
        )
        .await?;
        org.make(
            "/api/v1/assignments",
            json!({"who": {"person": dana_id}, "role": viewer}),
        )
        .await?;
        org.make(
            &format!("/api/v1/projects/{lab}/assignments"),
            json!({"who": {"group": group}, "role": viewer, "applies_to": "project-and-sub-projects"}),
        )
        .await?;
        org.make(
            &format!("/api/v1/projects/{acme}/assignments"),
            json!({"who": {"person": finn_id}, "role": viewer, "applies_to": "project"}),
        )
        .await?;

        let sees = async |person: &mut super::http::Browser, search: &str| -> Result<bool, super::http::Failure> {
            let list = person
                .send("GET", &format!("/api/v1/projects?search={search}"), None)
                .await?;
            Ok(!short_names(&list.body).is_empty())
        };
        // Bob sees ACME and WEB (inside it), not LAB.
        assert!(sees(&mut bob, "acme").await?);
        assert!(sees(&mut bob, "web").await?);
        assert!(!sees(&mut bob, "lab").await?);
        // Cleo sees WEB, and neither the project it is in nor the others.
        assert!(sees(&mut cleo, "web").await?);
        assert!(!sees(&mut cleo, "acme").await?);
        assert!(!sees(&mut cleo, "lab").await?);
        // Dana sees everything.
        for name in ["acme", "web", "lab"] {
            assert!(sees(&mut dana, name).await?, "{name}");
        }
        // Eve sees LAB through her group, nothing else.
        assert!(sees(&mut eve, "lab").await?);
        assert!(!sees(&mut eve, "acme").await?);
        // Finn sees ACME only: the row is for the project alone.
        assert!(sees(&mut finn, "acme").await?);
        assert!(!sees(&mut finn, "web").await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_the_rows_of_a_place_say_where_they_count() -> Outcome {
    each_database("sec_02_http_rows_say_where", async |db| {
        let mut org = start(db).await?;
        let viewer = org.role_named("Viewer").await?;
        let acme = org
            .make(
                "/api/v1/projects",
                json!({"short_name": "ACME", "name": "Acme"}),
            )
            .await?;
        let (_bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        let (_dana, dana_id) = org.person("Dana Desk", "dana@example.test").await?;
        let (_finn, finn_id) = org.person("Finn Field", "finn@example.test").await?;
        org.make(
            &format!("/api/v1/projects/{acme}/assignments"),
            json!({"who": {"person": bob_id}, "role": viewer, "applies_to": "project-and-sub-projects"}),
        )
        .await?;
        org.make(
            &format!("/api/v1/projects/{acme}/assignments"),
            json!({"who": {"person": finn_id}, "role": viewer, "applies_to": "project"}),
        )
        .await?;
        org.make(
            "/api/v1/assignments",
            json!({"who": {"person": dana_id}, "role": viewer}),
        )
        .await?;

        // The rows say where they count.
        let rows = org
            .ada
            .send("GET", &format!("/api/v1/projects/{acme}/assignments"), None)
            .await?;
        let counts: Vec<(&str, &str)> = rows.body["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| {
                Some((
                    row["person"]["name"].as_str()?,
                    row["applies_to"].as_str()?,
                ))
            })
            .collect();
        assert!(counts.contains(&("Bob Builder", "project-and-sub-projects")), "{counts:?}");
        assert!(counts.contains(&("Finn Field", "project")), "{counts:?}");
        // The whole organization is another list, and a row there counts everywhere.
        let whole = org.ada.send("GET", "/api/v1/assignments", None).await?;
        let names: Vec<&str> = whole.body["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["person"]["name"].as_str())
            .collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Ada Admin") && names.contains(&"Dana Desk"), "{names:?}");
        assert!(whole.body["items"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|row| row["applies_to"] == "organization"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_a_row_from_above_is_listed_as_inherited_and_a_no_access_below_it_hides_the_project()
-> Outcome {
    each_database("sec_03_http_inherited_and_no_access", async |db| {
        let mut org = start(db).await?;
        let viewer = org.role_named("Viewer").await?;
        let acme = org
            .make("/api/v1/projects", json!({"short_name": "ACME", "name": "Acme"}))
            .await?;
        let web = org
            .make(
                &format!("/api/v1/projects/{acme}/sub-projects"),
                json!({"short_name": "WEB", "name": "Website"}),
            )
            .await?;
        let (mut bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        org.make(
            &format!("/api/v1/projects/{acme}/assignments"),
            json!({"who": {"person": bob_id}, "role": viewer, "applies_to": "project-and-sub-projects"}),
        )
        .await?;

        // On the sub-project the row of ACME is there, marked as coming from above, and the
        // whole organization's row of the administrator after it.
        let rows = org
            .ada
            .send("GET", &format!("/api/v1/projects/{web}/assignments"), None)
            .await?;
        assert_eq!(rows.status, StatusCode::OK, "{}", rows.body);
        let first = &rows.body["items"][0];
        assert_eq!(first["person"]["name"], "Bob Builder");
        assert_eq!(first["inherited"], true);
        assert_eq!(first["place"]["kind"], "project");
        assert_eq!(first["place"]["short_name"], "ACME");
        assert_eq!(rows.body["items"][1]["place"]["kind"], "organization");
        assert_eq!(rows.body["items"][1]["inherited"], true);

        // *No access* on the sub-project: Bob keeps ACME and loses WEB, and the row says so.
        let before = bob.send("GET", &format!("/api/v1/projects/{web}"), None).await?;
        assert_eq!(before.status, StatusCode::OK);
        let forbidden = org
            .make(
                &format!("/api/v1/projects/{web}/assignments"),
                json!({"who": {"person": bob_id}, "applies_to": "project-and-sub-projects"}),
            )
            .await?;
        let rows = org
            .ada
            .send("GET", &format!("/api/v1/projects/{web}/assignments"), None)
            .await?;
        let row = &rows.body["items"][0];
        assert_eq!(row["id"], forbidden);
        assert!(row["role"].is_null(), "No access has no role");
        assert_eq!(row["inherited"], false);
        let after = bob.send("GET", &format!("/api/v1/projects/{web}"), None).await?;
        assert_eq!(after.code(), "resource.not_found");
        let acme_still = bob
            .send("GET", &format!("/api/v1/projects/{acme}"), None)
            .await?;
        assert_eq!(acme_still.status, StatusCode::OK);

        // A role and *No access* on one project do not go together, said in a sentence.
        let conflict = org
            .ada
            .send(
                "POST",
                &format!("/api/v1/projects/{web}/assignments"),
                Some(json!({"who": {"person": bob_id}, "role": viewer, "applies_to": "project"})),
            )
            .await?;
        assert_eq!((conflict.status, conflict.code()), (StatusCode::CONFLICT, "assignment.conflict"));

        // Taking the row away gives the project back.
        let removed = org
            .ada
            .send("DELETE", &format!("/api/v1/assignments/{forbidden}"), None)
            .await?;
        assert_eq!(removed.status, StatusCode::NO_CONTENT);
        let back = bob.send("GET", &format!("/api/v1/projects/{web}"), None).await?;
        assert_eq!(back.status, StatusCode::OK);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_nobody_removes_the_last_way_to_manage_roles_over_http() -> Outcome {
    each_database("sec_08_http_last_administrator", async |db| {
        let mut org = start(db).await?;
        let rows = org.ada.send("GET", "/api/v1/assignments", None).await?;
        let ada_row = rows.body["items"][0]["id"]
            .as_str()
            .ok_or("no row")?
            .to_owned();
        let refused = org
            .ada
            .send("DELETE", &format!("/api/v1/assignments/{ada_row}"), None)
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::CONFLICT, "access.would_lock_out")
        );
        // A second administrator makes it possible.
        let administrator = org.role_named("Administrator").await?;
        let (_bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        org.make(
            "/api/v1/assignments",
            json!({"who": {"person": bob_id}, "role": administrator}),
        )
        .await?;
        let removed = org
            .ada
            .send("DELETE", &format!("/api/v1/assignments/{ada_row}"), None)
            .await?;
        assert_eq!(removed.status, StatusCode::NO_CONTENT, "{}", removed.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_13_used_by_counts_only_what_the_viewer_may_see() -> Outcome {
    each_database("priv_13_http_used_by", async |db| {
        let mut org = start(db).await?;
        let acme = org
            .make("/api/v1/projects", json!({"short_name": "ACME", "name": "Acme"}))
            .await?;
        let secret = org
            .make("/api/v1/projects", json!({"short_name": "SECRET", "name": "Secret"}))
            .await?;
        let used = org
            .make(
                "/api/v1/roles",
                json!({"name": "Used", "permissions": ["see-projects"], "two_step": false}),
            )
            .await?;
        let keeper = org
            .make(
                "/api/v1/roles",
                json!({"name": "Keeper", "permissions": ["manage-roles", "manage-groups", "see-projects"], "two_step": false}),
            )
            .await?;
        let group = org.make("/api/v1/groups", json!({"name": "Backend"})).await?;
        let (mut carol, carol_id) = org.person("Carol Clerk", "carol@example.test").await?;
        let (_bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        // Carol manages roles and groups, and cannot see SECRET.
        org.make(
            "/api/v1/assignments",
            json!({"who": {"person": carol_id}, "role": keeper}),
        )
        .await?;
        org.make(
            &format!("/api/v1/projects/{secret}/assignments"),
            json!({"who": {"person": carol_id}, "applies_to": "project-and-sub-projects"}),
        )
        .await?;
        // The role and the group are used in both projects.
        for project in [&acme, &secret] {
            org.make(
                &format!("/api/v1/projects/{project}/assignments"),
                json!({"who": {"person": bob_id}, "role": used, "applies_to": "project"}),
            )
            .await?;
            org.make(
                &format!("/api/v1/projects/{project}/assignments"),
                json!({"who": {"group": group}, "role": used, "applies_to": "project"}),
            )
            .await?;
        }
        let ada_sees = org
            .ada
            .send("GET", &format!("/api/v1/roles/{used}"), None)
            .await?;
        assert_eq!(ada_sees.body["used_by"]["projects"], 2);
        let carol_sees = carol
            .send("GET", &format!("/api/v1/roles/{used}"), None)
            .await?;
        assert_eq!(carol_sees.status, StatusCode::OK, "{}", carol_sees.body);
        // One project she can see, and the people and groups named there; not the others.
        assert_eq!(carol_sees.body["used_by"]["projects"], 1);
        assert_eq!(carol_sees.body["used_by"]["people"], 1);
        assert_eq!(carol_sees.body["used_by"]["groups"], 1);
        assert_eq!(carol_sees.body["used_by"]["whole_organization"], false);
        let group_page = carol
            .send("GET", &format!("/api/v1/groups/{group}"), None)
            .await?;
        assert_eq!(group_page.body["used_by"]["projects"], 1);
        let ada_group = org
            .ada
            .send("GET", &format!("/api/v1/groups/{group}"), None)
            .await?;
        assert_eq!(ada_group.body["used_by"]["projects"], 2);
        // The answer never names the project she cannot see.
        assert!(!carol_sees.body.to_string().contains("SECRET"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_a_project_lead_decides_who_has_which_role_only_in_their_projects() -> Outcome {
    each_database("sec_02_http_project_lead", async |db| {
        let mut org = start(db).await?;
        let lead_role = org.role_named("Project lead").await?;
        let viewer = org.role_named("Viewer").await?;
        let acme = org
            .make("/api/v1/projects", json!({"short_name": "ACME", "name": "Acme"}))
            .await?;
        let lab = org
            .make("/api/v1/projects", json!({"short_name": "LAB", "name": "Lab"}))
            .await?;
        let (mut lena, lena_id) = org.person("Lena Lead", "lena@example.test").await?;
        let (_bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
        org.make(
            &format!("/api/v1/projects/{acme}/assignments"),
            json!({"who": {"person": lena_id}, "role": lead_role, "applies_to": "project-and-sub-projects"}),
        )
        .await?;

        // In her project: the rows, the choices (by name), and giving a role.
        let rows = lena
            .send("GET", &format!("/api/v1/projects/{acme}/assignments"), None)
            .await?;
        assert_eq!(rows.status, StatusCode::OK, "{}", rows.body);
        let choices = lena
            .send(
                "GET",
                &format!("/api/v1/projects/{acme}/assignment-choices?kind=person&search=bob"),
                None,
            )
            .await?;
        assert_eq!(choices.body["items"][0]["name"], "Bob Builder");
        assert!(choices.body["items"][0].get("email").is_none());
        let given = lena
            .send(
                "POST",
                &format!("/api/v1/projects/{acme}/assignments"),
                Some(json!({"who": {"person": bob_id}, "role": viewer, "applies_to": "project"})),
            )
            .await?;
        assert_eq!(given.status, StatusCode::OK, "{}", given.body);
        assert_eq!(given.body["person"]["name"], "Bob Builder");

        // In the other project she does not know of, and for the whole organization and the
        // people, roles and groups of the organization, she is refused.
        for (method, path) in [
            ("GET", format!("/api/v1/projects/{lab}/assignments")),
            ("GET", format!("/api/v1/projects/{lab}/assignment-choices")),
        ] {
            let reply = lena.send(method, &path, None).await?;
            assert_eq!(
                (reply.status, reply.code()),
                (StatusCode::NOT_FOUND, "resource.not_found"),
                "{path}"
            );
        }
        for path in [
            "/api/v1/assignments",
            "/api/v1/assignment-choices",
            "/api/v1/people",
            "/api/v1/roles",
            "/api/v1/groups",
        ] {
            let reply = lena.send("GET", path, None).await?;
            assert_eq!(
                (reply.status, reply.code()),
                (StatusCode::FORBIDDEN, "access.forbidden"),
                "{path}"
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_the_choices_of_the_form_are_by_name_and_walk_page_by_page() -> Outcome {
    each_database("sec_02_http_choices", async |db| {
        let mut org = start(db).await?;
        for (name, email) in [
            ("Bob Builder", "bob@example.test"),
            ("Cleo Coder", "cleo@example.test"),
        ] {
            org.person(name, email).await?;
        }
        org.make("/api/v1/groups", json!({"name": "Backend"}))
            .await?;
        let first = org
            .ada
            .send(
                "GET",
                "/api/v1/assignment-choices?kind=person&limit=2",
                None,
            )
            .await?;
        let names: Vec<&str> = first.body["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| item["name"].as_str())
            .collect();
        assert_eq!(names, ["Ada Admin", "Bob Builder"]);
        let cursor = first.body["next_cursor"].as_str().ok_or("no next page")?;
        let second = org
            .ada
            .send(
                "GET",
                &format!("/api/v1/assignment-choices?kind=person&limit=2&cursor={cursor}"),
                None,
            )
            .await?;
        assert_eq!(second.body["items"][0]["name"], "Cleo Coder");
        assert!(second.body["next_cursor"].is_null());
        // Never an address, also not by searching for one.
        let by_address = org
            .ada
            .send(
                "GET",
                "/api/v1/assignment-choices?kind=person&search=bob@",
                None,
            )
            .await?;
        assert_eq!(by_address.body["items"], json!([]));
        let groups = org
            .ada
            .send("GET", "/api/v1/assignment-choices?kind=group", None)
            .await?;
        assert_eq!(groups.body["items"][0]["name"], "Backend");
        let roles = org
            .ada
            .send(
                "GET",
                "/api/v1/assignment-choices?kind=role&search=lead",
                None,
            )
            .await?;
        assert_eq!(roles.body["items"][0]["name"], "Project lead");
        let bad = org
            .ada
            .send("GET", "/api/v1/assignment-choices?kind=nonsense", None)
            .await?;
        assert_eq!(bad.status, StatusCode::BAD_REQUEST);
        Ok(())
    })
    .await
}

/// The names in the `items` of a list.
fn names_in(list: &serde_json::Value) -> Vec<&str> {
    list["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["name"].as_str())
        .collect()
}

#[tokio::test]
async fn sec_02_the_choices_of_roles_hold_only_the_roles_the_person_may_give() -> Outcome {
    each_database("sec_02_http_role_choices", async |db| {
        let mut org = start(db).await?;
        let (mut cleo, cleo_id) = org.person("Cleo Coder", "cleo@example.test").await?;
        let (mut dana, dana_id) = org.person("Dana Desk", "dana@example.test").await?;
        let decider = org
            .make(
                "/api/v1/roles",
                json!({"name": "Decider", "permissions": ["decide-roles", "see-projects"], "two_step": false}),
            )
            .await?;
        let keeper = org
            .make(
                "/api/v1/roles",
                json!({"name": "Keeper", "permissions": ["manage-roles", "see-projects"], "two_step": false}),
            )
            .await?;
        org.make(
            "/api/v1/assignments",
            json!({"who": {"person": cleo_id}, "role": decider}),
        )
        .await?;
        org.make(
            "/api/v1/assignments",
            json!({"who": {"person": dana_id}, "role": keeper}),
        )
        .await?;
        // Ada holds everything: she may give every role. Cleo holds less, so she is offered less.
        let all = org
            .ada
            .send("GET", "/api/v1/assignment-choices?kind=role", None)
            .await?;
        assert!(names_in(&all.body).contains(&"Administrator"));
        let hers = cleo
            .send("GET", "/api/v1/assignment-choices?kind=role", None)
            .await?;
        let names = names_in(&hers.body);
        assert!(names.contains(&"Decider"), "{names:?}");
        assert!(!names.contains(&"Administrator"), "{names:?}");
        assert!(!names.contains(&"Project lead"), "{names:?}");
        // The roles a person who manages roles may give a deleted role's holders: the same rule.
        let everything = dana.send("GET", "/api/v1/roles", None).await?;
        assert!(names_in(&everything.body).contains(&"Administrator"));
        let giveable = dana
            .send("GET", "/api/v1/roles?giveable=true", None)
            .await?;
        let names = names_in(&giveable.body);
        assert!(names.contains(&"Keeper"), "{names:?}");
        assert!(!names.contains(&"Administrator"), "{names:?}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_the_rows_of_a_place_are_listed_a_page_at_a_time() -> Outcome {
    each_database("sec_03_http_rows_paging", async |db| {
        let mut org = start(db).await?;
        let viewer = org.role_named("Viewer").await?;
        for name in ["One", "Two", "Three"] {
            let group = org.make("/api/v1/groups", json!({ "name": name })).await?;
            org.make(
                "/api/v1/assignments",
                json!({"who": {"group": group}, "role": viewer}),
            )
            .await?;
        }
        let mut seen = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let path = cursor.as_ref().map_or_else(
                || "/api/v1/assignments?limit=2".to_owned(),
                |cursor| format!("/api/v1/assignments?limit=2&cursor={cursor}"),
            );
            let page = org.ada.send("GET", &path, None).await?;
            assert_eq!(page.status, StatusCode::OK, "{}", page.body);
            seen.extend(super::people_http::ids_of(&page.body));
            match page.body["next_cursor"].as_str() {
                Some(next) => cursor = Some(next.to_owned()),
                None => break,
            }
        }
        assert_eq!(
            seen.len(),
            4,
            "the administrator and the three groups, each once"
        );
        let unique: std::collections::BTreeSet<&String> = seen.iter().collect();
        assert_eq!(unique.len(), 4);
        let bad = org
            .ada
            .send("GET", "/api/v1/assignments?cursor=nonsense", None)
            .await?;
        assert_eq!(bad.code(), "request.bad_cursor");
        Ok(())
    })
    .await
}
