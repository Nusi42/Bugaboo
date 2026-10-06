//! What `/auth/me` tells (SEC-01, ADR-0025 decision 5): the permissions of the person for the whole
//! organization. That every route enforces the permission it declares, for every kind of actor, is
//! the permission matrix (`tests/matrix`, SEC-MATRIX; M1-F32, M2-F30).

use axum::http::StatusCode;
use bb_app::access::Permission;
use bb_testing::each_database;
use serde_json::{Value, json};

use super::caching::invited_person;
use super::http::{Browser, called, setup_body};
use super::rig::{Outcome, Rig};
use super::status::app_with_probe;

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
