//! The style sheets of own themes over HTTP on both databases (THM-04, ADR-0026 update M8): a sheet
//! is CSS, kept by the person's browser like a hashed asset while its address names the version it
//! has, asked for again otherwise, and served only to the people who may use the theme; the sheet
//! of a draft is served to its editor only.

use axum::http::StatusCode;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::{ADA_EMAIL, Browser, rig_and_app};
use super::issues_site::{ok, text_in};
use super::policy_rig::Org;

const THEMES: &str = "/api/v1/themes";
const DRAFTS: &str = "/api/v1/drafts";

#[tokio::test]
async fn thm_04_a_sheet_is_kept_by_its_version_and_served_to_who_may_use_it_only() -> TestResult {
    each_database("thm_04_sheets_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        Org::start(&rig).await?;
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let mut ben = ada.invite("ben@example.test", "Ben Baker").await?;
        let file = json!({"name": "Evening", "extends": "dusk",
            "values": {"colors": {"accent": {"value": "#b4a8ff"}}}});
        let uploaded = ada
            .send("POST", &format!("{THEMES}/upload?kind=personal"), Some(file))
            .await?;
        let theme = ok(&uploaded)?["theme"].clone();
        let id = text_in(&theme, "id")?;
        let sheet = text_in(&theme, "sheet")?;

        let kept = ada.send("GET", &sheet, None).await?;
        assert_eq!(kept.status, StatusCode::OK);
        assert!(kept.header("content-type").starts_with("text/css"));
        assert_eq!(
            kept.header("cache-control"),
            "private, max-age=31536000, immutable"
        );
        let other = ada
            .send("GET", &format!("/themes/{id}.css?v=999"), None)
            .await?;
        assert_eq!(other.header("cache-control"), "private, no-cache");

        // A personal theme is its maker's: anybody else is told it does not exist.
        let refused = ben.send("GET", &sheet, None).await?;
        assert_eq!(refused.code(), "resource.not-found");

        // The draft of the editor is served to the editor only.
        let draft = json!({
            "target": {"record": {"kind": "theme", "id": id}, "part": "theme"},
            "draft": {"text": "{\"colors\":{\"accent\":{\"value\":\"#b0a0ff\"}}}", "base_version": 2},
        });
        assert!(ada.send("PUT", DRAFTS, Some(draft)).await?.status.is_success());
        let drafted = ada
            .send("GET", &format!("/themes/{id}/draft.css"), None)
            .await?;
        assert_eq!(drafted.status, StatusCode::OK);
        let hidden = ben
            .send("GET", &format!("/themes/{id}/draft.css"), None)
            .await?;
        assert_eq!(hidden.code(), "resource.not-found");
        Ok(())
    })
    .await
}
