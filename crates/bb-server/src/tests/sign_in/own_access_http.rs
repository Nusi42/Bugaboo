//! What a change takes from the person who makes it, over HTTP (SEC-08, ADR-0025 decision 6): the
//! page asks first only when the person would lose their own way to manage roles, whether the
//! row that gives it is theirs or their group's.

use axum::http::StatusCode;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::Browser;
use super::people_http::{Organization, start};

const CHECK: &str = "/api/v1/own-access/check-change";
const BOTH: [&str; 2] = ["decide-roles", "manage-roles"];

/// What the answer says is lost, as the words of the permissions, in order.
async fn loses(person: &mut Browser, change: Value) -> Result<Vec<String>, Failure> {
    let answer = person.send("POST", CHECK, Some(change)).await?;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let mut words: Vec<String> = answer.body["loses"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|word| word.as_str())
        .map(str::to_owned)
        .collect();
    words.sort();
    Ok(words)
}

/// The ID of the row that gives `role` to the person or group, among the rows of the organization.
async fn row_of(person: &mut Browser, who: &str, id: &str, role: &str) -> Result<String, Failure> {
    let rows = person.send("GET", "/api/v1/assignments", None).await?;
    rows.body["items"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["role"]["id"] == role && row[who]["id"] == id)
        .and_then(|row| row["id"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| "no such row".into())
}

/// The ID of the person who is signed in.
async fn me(person: &mut Browser) -> Result<String, Failure> {
    let me = person.send("GET", "/api/v1/auth/me", None).await?;
    Ok(me.body["id"].as_str().ok_or("no id")?.to_owned())
}

/// Ada, Bob (an administrator with a row of his own) and a group of administrators that Ada is in
/// while she keeps no row of her own.
struct Admins {
    bob: Browser,
    ada_id: String,
    group: String,
    group_row: String,
    bob_row: String,
    administrator: String,
}

async fn admins_through_a_group(org: &mut Organization) -> Result<Admins, Failure> {
    let administrator = org.role_named("Administrator").await?;
    let (mut bob, bob_id) = org.person("Bob Builder", "bob@example.test").await?;
    let ada_id = me(&mut org.ada).await?;
    org.create(
        "/api/v1/assignments",
        json!({"who": {"person": bob_id}, "role": administrator}),
    )
    .await?;
    let group = org
        .create("/api/v1/groups", json!({"name": "Admins"}))
        .await?;
    let put = org
        .ada
        .send(
            "PUT",
            &format!("/api/v1/groups/{group}/members/{ada_id}"),
            None,
        )
        .await?;
    assert_eq!(put.status, StatusCode::NO_CONTENT, "{}", put.body);
    let group_row = org
        .create(
            "/api/v1/assignments",
            json!({"who": {"group": group}, "role": administrator}),
        )
        .await?;
    let ada_row = row_of(&mut bob, "person", &ada_id, &administrator).await?;
    let removed = bob
        .send("DELETE", &format!("/api/v1/assignments/{ada_row}"), None)
        .await?;
    assert_eq!(removed.status, StatusCode::NO_CONTENT, "{}", removed.body);
    let bob_row = row_of(&mut bob, "person", &bob_id, &administrator).await?;
    Ok(Admins {
        bob,
        ada_id,
        group,
        group_row,
        bob_row,
        administrator,
    })
}

#[tokio::test]
async fn sec_08_taking_a_row_of_a_group_you_are_in_is_told_to_take_your_way_to_manage_roles()
-> TestResult {
    each_database("sec_08_own_loss_rows", async |db| {
        let mut org = start(db).await?;
        let mut admins = admins_through_a_group(&mut org).await?;
        // Ada has the way only through the group: its row takes it from her.
        assert_eq!(
            loses(
                &mut org.ada,
                json!({"kind": "remove-assignment", "assignment": admins.group_row})
            )
            .await?,
            BOTH
        );
        // Bob's row is not hers, and Bob has a row of his own besides the group's.
        assert!(
            loses(
                &mut org.ada,
                json!({"kind": "remove-assignment", "assignment": admins.bob_row})
            )
            .await?
            .is_empty()
        );
        assert!(
            loses(
                &mut admins.bob,
                json!({"kind": "remove-assignment", "assignment": admins.group_row})
            )
            .await?
            .is_empty()
        );
        assert_eq!(
            loses(
                &mut admins.bob,
                json!({"kind": "remove-assignment", "assignment": admins.bob_row})
            )
            .await?,
            BOTH
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_leaving_or_deleting_a_group_that_gives_you_the_way_is_told() -> TestResult {
    each_database("sec_08_own_loss_groups", async |db| {
        let mut org = start(db).await?;
        let admins = admins_through_a_group(&mut org).await?;
        let (_cleo, cleo_id) = org.person("Cleo Coder", "cleo@example.test").await?;
        let (group, ada_id) = (&admins.group, &admins.ada_id);
        assert_eq!(
            loses(
                &mut org.ada,
                json!({"kind": "leave-group", "group": group, "person": ada_id})
            )
            .await?,
            BOTH
        );
        assert_eq!(
            loses(
                &mut org.ada,
                json!({"kind": "delete-group", "group": group})
            )
            .await?,
            BOTH
        );
        // Somebody else leaving takes nothing from Ada.
        assert!(
            loses(
                &mut org.ada,
                json!({"kind": "leave-group", "group": group, "person": cleo_id})
            )
            .await?
            .is_empty()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_no_access_on_yourself_or_your_group_is_told_when_it_takes_the_way_to_decide()
-> TestResult {
    each_database("sec_08_own_loss_no_access", async |db| {
        let mut org = start(db).await?;
        let lead = org.role_named("Project lead").await?;
        let acme = org
            .create("/api/v1/projects", json!({"short_name": "ACME", "name": "Acme"}))
            .await?;
        let (mut cleo, cleo_id) = org.person("Cleo Coder", "cleo@example.test").await?;
        let (_dana, dana_id) = org.person("Dana Desk", "dana@example.test").await?;
        let group = org.create("/api/v1/groups", json!({"name": "Leads"})).await?;
        org.ada
            .send("PUT", &format!("/api/v1/groups/{group}/members/{cleo_id}"), None)
            .await?;
        // Cleo leads ACME through the group Leads and decides who has which role there.
        org.create(
            &format!("/api/v1/projects/{acme}/assignments"),
            json!({"who": {"group": group}, "role": lead, "applies_to": "project-and-sub-projects"}),
        )
        .await?;
        let on_herself = json!({"kind": "no-access", "who": {"person": cleo_id}, "project": acme});
        let on_group = json!({"kind": "no-access", "who": {"group": group}, "project": acme});
        let on_dana = json!({"kind": "no-access", "who": {"person": dana_id}, "project": acme});
        assert_eq!(loses(&mut cleo, on_herself).await?, ["decide-roles"]);
        assert_eq!(loses(&mut cleo, on_group).await?, ["decide-roles"]);
        assert!(loses(&mut cleo, on_dana).await?.is_empty());
        // Ada decides from the whole organization: No access on a project never takes it from her.
        let ada_id = me(&mut org.ada).await?;
        let on_ada = json!({"kind": "no-access", "who": {"person": ada_id}, "project": acme});
        assert!(loses(&mut org.ada, on_ada).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_changing_or_deleting_a_role_you_hold_is_told_when_it_takes_the_way_away()
-> TestResult {
    each_database("sec_08_own_loss_roles", async |db| {
        let mut org = start(db).await?;
        let administrator = org.role_named("Administrator").await?;
        let viewer = org.role_named("Viewer").await?;
        let detail = org
            .ada
            .send("GET", &format!("/api/v1/roles/{administrator}"), None)
            .await?;
        let all: Vec<Value> = detail.body["role"]["permissions"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let without = |word: &str| -> Vec<Value> {
            all.iter().filter(|each| *each != word).cloned().collect()
        };
        let change = |role: &str, permissions: Vec<Value>| {
            json!({"kind": "change-role", "role": role, "permissions": permissions})
        };
        assert_eq!(
            loses(&mut org.ada, change(&administrator, without("manage-roles"))).await?,
            ["manage-roles"]
        );
        assert_eq!(
            loses(&mut org.ada, change(&administrator, without("decide-roles"))).await?,
            ["decide-roles"]
        );
        assert!(
            loses(&mut org.ada, change(&administrator, without("see-status")))
                .await?
                .is_empty()
        );
        // A role Ada does not hold changes nothing for her.
        assert!(loses(&mut org.ada, change(&viewer, Vec::new())).await?.is_empty());
        // Deleting the role with another one that holds less takes it; with a copy, nothing.
        assert_eq!(
            loses(
                &mut org.ada,
                json!({"kind": "delete-role", "role": administrator, "replacement": viewer})
            )
            .await?,
            BOTH
        );
        let copy = org
            .create(&format!("/api/v1/roles/{administrator}/copy"), json!({"name": "Second"}))
            .await?;
        assert!(
            loses(
                &mut org.ada,
                json!({"kind": "delete-role", "role": administrator, "replacement": copy})
            )
            .await?
            .is_empty()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_a_person_who_may_not_apply_the_change_is_told_it_takes_nothing_and_nothing_more()
-> TestResult {
    each_database("sec_08_own_loss_silent", async |db| {
        let mut org = start(db).await?;
        let admins = admins_through_a_group(&mut org).await?;
        let (mut dana, dana_id) = org.person("Dana Desk", "dana@example.test").await?;
        let changes = [
            json!({"kind": "remove-assignment", "assignment": admins.group_row}),
            json!({"kind": "leave-group", "group": admins.group, "person": dana_id}),
            json!({"kind": "delete-group", "group": admins.group}),
            json!({"kind": "change-role", "role": admins.administrator, "permissions": []}),
            json!({"kind": "delete-role", "role": admins.administrator, "replacement": null}),
        ];
        for change in changes {
            assert!(
                loses(&mut dana, change.clone()).await?.is_empty(),
                "{change}"
            );
        }
        // A row that is gone, and a word that is no kind: nothing, and not understood.
        let removed = org
            .ada
            .send(
                "DELETE",
                &format!("/api/v1/assignments/{}", admins.bob_row),
                None,
            )
            .await?;
        assert_eq!(removed.status, StatusCode::NO_CONTENT, "{}", removed.body);
        assert!(
            loses(
                &mut org.ada,
                json!({"kind": "remove-assignment", "assignment": admins.bob_row})
            )
            .await?
            .is_empty()
        );
        let answer = org
            .ada
            .send("POST", CHECK, Some(json!({"kind": "make-coffee"})))
            .await?;
        assert!(answer.status.is_client_error(), "{}", answer.body);
        Ok(())
    })
    .await
}
