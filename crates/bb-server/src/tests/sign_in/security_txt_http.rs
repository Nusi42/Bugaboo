//! `/.well-known/security.txt` over HTTP on both databases (COMP-03, ADR-0047 decision 4, M9-Q16):
//! anybody reads it, signed in or not; the organization's contact comes first once an
//! administrator sets one, and it expires a year after the release of the program.

use axum::http::StatusCode;
use bb_core::release::{OUR_SECURITY_CONTACT, RELEASE_DATE, SECURITY_POLICY, SECURITY_TXT_PATH};
use bb_testing::{TestResult, each_database};
use jiff::ToSpan as _;
use serde_json::json;

use super::http::{ADA_EMAIL, Browser, called, rig_and_app};
use super::policy_rig::Org;

#[tokio::test]
async fn comp_03_security_txt_names_the_organizations_contact_first_and_expires_a_year_after_the_release()
-> TestResult {
    each_database("comp_03_security_txt", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        Org::start(&rig).await?;
        let expires = format!(
            "Expires: {}T00:00:00Z\n",
            RELEASE_DATE.checked_add(1.year())?
        );
        let mut stranger = Browser::new(&app, &called());
        let first = stranger.fetch(SECURITY_TXT_PATH).await?;
        assert_eq!(first.status, StatusCode::OK);
        assert_eq!(first.header("content-type"), "text/plain; charset=utf-8");
        assert_eq!(first.header("x-content-type-options"), "nosniff");
        // Never kept by a cache: an administrator may change the contact at any time.
        assert_eq!(first.header("cache-control"), "no-store");
        assert!(!first.header("content-security-policy").is_empty());
        let text = String::from_utf8(first.bytes)?;
        assert!(
            text.starts_with(&format!("Contact: {OUR_SECURITY_CONTACT}\n")),
            "{text}"
        );
        assert!(text.contains(&expires), "{text}");
        assert!(
            text.contains(&format!("Policy: {SECURITY_POLICY}\n")),
            "{text}"
        );
        assert!(text.contains("Preferred-Languages: en, de\n"), "{text}");

        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let changed = ada
            .send(
                "PUT",
                "/api/v1/settings/security.contact",
                Some(json!({"value": "security@example.org"})),
            )
            .await?;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        // Signed out and signed in, the same text, with the organization's contact first.
        for person in [&mut stranger, &mut ada] {
            let answer = person.fetch(SECURITY_TXT_PATH).await?;
            assert_eq!(answer.status, StatusCode::OK);
            let text = String::from_utf8(answer.bytes)?;
            assert!(
                text.starts_with(&format!(
                    "Contact: mailto:security@example.org\nContact: {OUR_SECURITY_CONTACT}\n"
                )),
                "{text}"
            );
            assert!(text.contains(&expires), "{text}");
        }
        // It is only read: a POST, signed in or not, changes nothing and is refused.
        for person in [&mut stranger, &mut ada] {
            let posted = person.send("POST", SECURITY_TXT_PATH, None).await?;
            assert!(posted.status.is_client_error(), "{}", posted.status);
        }
        Ok(())
    })
    .await
}
