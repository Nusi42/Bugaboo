//! Lists over HTTP: who is asking is judged before what they ask (PERF-02, SEC-06).

use axum::http::StatusCode;
use bb_testing::{TestResult, each_database};

use super::caching::invited_person;
use super::http::{Browser, called, rig_and_app, setup_body};

#[tokio::test]
async fn perf_02_a_list_judges_who_asks_before_it_judges_the_query() -> TestResult {
    each_database("perf_02_http_list_order", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let mut ada = Browser::new(&app, &called());
        ada.send("POST", "/api/v1/auth/setup", Some(setup_body(&rig)))
            .await?;
        let mut bob = invited_person(&mut ada, &app, "bob@example.test").await?;
        let mut stranger = Browser::new(&app, &called());
        let list = "/api/v1/auth/invitations";

        for bad in ["fields=password", "cursor=nonsense", "limit=many"] {
            // Not signed in: 401, whatever is wrong with the query.
            let reply = stranger.send("GET", &format!("{list}?{bad}"), None).await?;
            assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{bad}");
            assert_eq!(reply.code(), "auth.not_signed_in", "{bad}");
        }
        for bad in ["fields=password", "cursor=nonsense"] {
            // Signed in but not an administrator: 403, not a hint about the query.
            let reply = bob.send("GET", &format!("{list}?{bad}"), None).await?;
            assert_eq!(reply.status, StatusCode::FORBIDDEN, "{bad}");
            assert_eq!(reply.code(), "access.forbidden", "{bad}");
        }
        // Only for the person who may list, the query is judged.
        for (bad, code) in [
            ("fields=password", "request.unknown_field"),
            ("cursor=nonsense", "request.bad_cursor"),
        ] {
            let reply = ada.send("GET", &format!("{list}?{bad}"), None).await?;
            assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{bad}");
            assert_eq!(reply.code(), code, "{bad}");
        }
        Ok(())
    })
    .await
}
