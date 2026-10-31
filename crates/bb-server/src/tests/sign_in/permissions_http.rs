//! What `/auth/me` tells (SEC-01, ADR-0025 decision 5): the permissions of the person for the whole
//! organization. That every route enforces the permission it declares, for every kind of actor, is
//! the permission matrix (`tests/matrix`, SEC-MATRIX; M1-F32, M2-F30).

use axum::http::StatusCode;
use bb_app::access::Permission;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::http::{ADA_EMAIL, Browser};
use super::rig::Rig;
use super::status::app_with_probe;

#[tokio::test]
async fn sec_01_who_is_signed_in_answers_the_permissions_for_the_whole_organization() -> TestResult
{
    each_database("sec_01_me_permissions", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let mut ada = Browser::signed_in(&app, &rig).await?;
        let me = ada.send("GET", "/api/v1/auth/me", None).await?;
        assert_eq!(me.status, StatusCode::OK);
        assert_eq!(me.body["email"], ADA_EMAIL);
        assert!(me.body.get("administrator").is_none(), "the flag is gone");
        let held: Vec<&str> = me.body["permissions"]
            .as_array()
            .ok_or("no permissions")?
            .iter()
            .filter_map(Value::as_str)
            .collect();
        // Everything but the permissions of working hours, which an administrator gives with *HR*
        // (ADR-0035, decision 8).
        for permission in Permission::ALL {
            let of_working_hours = bb_policy::OF_WORKING_HOURS.contains(permission);
            assert_eq!(
                held.contains(&permission.code()),
                !of_working_hours,
                "{permission}"
            );
        }
        // A person without a role holds nothing, and the answer is still a list.
        let mut bob = ada.invite("bob@example.test", "Bob").await?;
        let me = bob.send("GET", "/api/v1/auth/me", None).await?;
        assert_eq!(me.body["permissions"], json!([]));
        Ok(())
    })
    .await
}
