//! *About* over HTTP on both databases (COMP-01, ADR-0047 decision 3): the version, the license and
//! the parts of the program are for people who are signed in, not for strangers.

use axum::http::StatusCode;
use bb_testing::{TestResult, each_database};

use super::http::{ADA_EMAIL, Browser, called, rig_and_app};
use super::policy_rig::Org;

const ABOUT: &str = "/api/v1/about";

#[tokio::test]
async fn comp_01_about_answers_the_version_and_the_parts_to_signed_in_people_only() -> TestResult {
    each_database("comp_01_about_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        Org::start(&rig).await?;
        let mut stranger = Browser::new(&app, &called());
        let refused = stranger.send("GET", ABOUT, None).await?;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert!(refused.body.get("version").is_none(), "{}", refused.body);
        let refused = stranger
            .send("GET", &format!("{ABOUT}/files/notices"), None)
            .await?;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);

        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let mut ben = ada.invite("ben@example.test", "Ben Baker").await?;
        for person in [&mut ada, &mut ben] {
            let about = person.send("GET", ABOUT, None).await?;
            assert_eq!(about.status, StatusCode::OK, "{}", about.body);
            assert_eq!(about.body["version"], env!("CARGO_PKG_VERSION"));
            assert_eq!(about.body["license"], "AGPL-3.0-or-later");
            assert!(about.body["parts"].is_array());
            assert!(about.body["files"].is_array());
            // A file the build does not carry is not found, and nothing outside the three kinds
            // can be asked for.
            let none = person
                .send("GET", &format!("{ABOUT}/files/..%2Fsecret"), None)
                .await?;
            assert_eq!(none.status, StatusCode::NOT_FOUND);
        }
        Ok(())
    })
    .await
}
