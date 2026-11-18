//! *About* over HTTP on both databases (COMP-01, ADR-0047 decision 3): the version, the license and
//! the parts of the program are for people who are signed in, not for strangers.

use axum::http::StatusCode;
use bb_testing::{TestResult, each_database};

use super::http::{ADA_EMAIL, Browser, called, rig_and_app};
use super::policy_rig::Org;

const ABOUT: &str = "/api/v1/about";
const PARTS: &str = "/api/v1/about/parts";

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
        let refused = stranger.send("GET", PARTS, None).await?;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert!(refused.body.get("items").is_none(), "{}", refused.body);

        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let mut ben = ada.invite("ben@example.test", "Ben Baker").await?;
        for person in [&mut ada, &mut ben] {
            let about = person.send("GET", ABOUT, None).await?;
            assert_eq!(about.status, StatusCode::OK, "{}", about.body);
            assert_eq!(about.body["version"], env!("CARGO_PKG_VERSION"));
            assert_eq!(about.body["license"], "AGPL-3.0-or-later");
            assert!(about.body["part_count"].is_u64(), "{}", about.body);
            assert!(
                about.body.get("parts").is_none(),
                "the parts are their own list"
            );
            assert!(about.body["files"].is_array());
            // A file the build does not carry is not found, and nothing outside the three kinds
            // can be asked for.
            let none = person
                .send("GET", &format!("{ABOUT}/files/..%2Fsecret"), None)
                .await?;
            // The parts are a lean list: items, fields, a limit and a cursor, an ETag and a 304.
            let parts = person.send("GET", PARTS, None).await?;
            assert_eq!(parts.status, StatusCode::OK, "{}", parts.body);
            assert!(parts.body["items"].is_array());
            assert!(parts.body["next_cursor"].is_null());
            assert_eq!(
                parts.body["items"].as_array().map(Vec::len),
                about.body["part_count"]
                    .as_u64()
                    .and_then(|count| usize::try_from(count).ok()),
            );
            let named = person
                .send("GET", &format!("{PARTS}?fields=name&limit=5"), None)
                .await?;
            assert_eq!(named.status, StatusCode::OK, "{}", named.body);
            for item in named.body["items"].as_array().into_iter().flatten() {
                assert_eq!(
                    item.as_object().map(serde_json::Map::len),
                    Some(1),
                    "{item}"
                );
            }
            let unknown = person
                .send("GET", &format!("{PARTS}?fields=secret"), None)
                .await?;
            assert_eq!(unknown.status, StatusCode::BAD_REQUEST);
            let too_many = person
                .send("GET", &format!("{PARTS}?limit=0"), None)
                .await?;
            assert_eq!(too_many.status, StatusCode::BAD_REQUEST);
            let tag = parts.header("etag").to_owned();
            assert!(tag.starts_with("W/\""), "{tag}");
            person.set_header("if-none-match", &tag);
            let again = person.send("GET", PARTS, None).await?;
            assert_eq!(again.status, StatusCode::NOT_MODIFIED);
            person.clear_headers();
            assert_eq!(none.status, StatusCode::NOT_FOUND);
        }
        Ok(())
    })
    .await
}
