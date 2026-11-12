//! The style sheets of own themes over HTTP on both databases (THM-04, ADR-0026 update M8): a sheet
//! is CSS, kept by the person's browser like a hashed asset while its address names the version it
//! has, asked for again otherwise, and served only to the people who may use the theme; the sheet
//! of a draft is served to its editor only.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_core::theme::PERSONAL_THEMES_MOST;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::http::{ADA_EMAIL, Browser, Reply, rig_and_app};
use super::issues_site::{ok, text_in};
use super::policy_rig::{Member, Org};

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
        assert_eq!(refused.header("cache-control"), "no-store");

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
        // A draft changes while it is edited: it is never kept, whatever its address says.
        assert_eq!(drafted.header("cache-control"), "private, no-store");
        let hidden = ben
            .send("GET", &format!("/themes/{id}/draft.css"), None)
            .await?;
        assert_eq!(hidden.code(), "resource.not-found");
        assert_eq!(hidden.header("cache-control"), "no-store");
        Ok(())
    })
    .await
}

/// Makes a new theme of `kind` as `browser`; the answer.
async fn create(browser: &mut Browser, kind: &str) -> Result<Reply, Failure> {
    browser
        .send(
            "POST",
            THEMES,
            Some(json!({"name": "Evening", "kind": kind, "extends": "dusk"})),
        )
        .await
}

/// The IDs and whether each was saved, of the themes `browser` is offered.
async fn listed(browser: &mut Browser) -> Result<Vec<(String, bool)>, Failure> {
    let list = browser.send("GET", THEMES, None).await?;
    Ok(ok(&list)?["themes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|theme| {
            (
                theme["id"].as_str().unwrap_or_default().to_owned(),
                theme["saved"] == true,
            )
        })
        .collect())
}

#[tokio::test]
async fn thm_04_two_new_themes_at_the_same_moment_cannot_both_take_the_last_place() -> TestResult {
    each_database("thm_04_theme_limit_race", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        Org::start(&rig).await?;
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let mut again = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        // One place is left.
        for _ in 1..PERSONAL_THEMES_MOST {
            ok(&create(&mut ada, "personal").await?)?;
        }
        let (first, second) =
            tokio::join!(create(&mut ada, "personal"), create(&mut again, "personal"));
        let mut codes = [first?.code().to_owned(), second?.code().to_owned()];
        codes.sort();
        assert_eq!(codes, ["", "theme.too-many-personal"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn thm_04_a_theme_never_saved_is_listed_to_its_maker_and_to_who_looks_after_themes()
-> TestResult {
    each_database("thm_04_unsaved_listed", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let org = Org::start(&rig).await?;
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let mut ben = ada.invite("ben@example.test", "Ben Baker").await?;
        let mut cleo = ada.invite("cleo@example.test", "Cleo Clark").await?;
        let cleo_id = text_in(ok(&cleo.send("GET", "/api/v1/auth/me", None).await?)?, "id")?;
        let administrator = org.starting_roles().await?[StartingRole::Administrator.code()];
        let as_member = Member {
            id: cleo_id.parse().map_err(|_| "no person")?,
            session: String::new(),
        };
        org.give_everywhere(&as_member, administrator).await?;
        let mine = text_in(ok(&create(&mut ben, "personal").await?)?, "id")?;
        let everyone = text_in(ok(&create(&mut ada, "for-everyone").await?)?, "id")?;
        // The maker sees their own, marked as not saved; nobody else sees a personal one.
        assert_eq!(listed(&mut ben).await?, [(mine.clone(), false)]);
        // A theme for everyone that was never saved is for those who look after themes only.
        assert_eq!(listed(&mut cleo).await?, [(everyone.clone(), false)]);
        // They may take it away.
        let gone = cleo
            .send("DELETE", &format!("{THEMES}/{everyone}?version=1"), None)
            .await?;
        assert_eq!(gone.status, StatusCode::NO_CONTENT, "{}", gone.body);
        assert!(listed(&mut ada).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn thm_04_who_looks_after_a_theme_for_everyone_learns_how_many_people_chose_it() -> TestResult
{
    each_database("thm_04_theme_used_by", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        Org::start(&rig).await?;
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let mut ben = ada.invite("ben@example.test", "Ben Baker").await?;
        let file = json!({"name": "Harbor", "extends": "dusk",
            "values": {"colors": {"accent": {"value": "#b4a8ff"}}}});
        let uploaded = ada
            .send(
                "POST",
                &format!("{THEMES}/upload?kind=for-everyone"),
                Some(file),
            )
            .await?;
        let theme = ok(&uploaded)?["theme"].clone();
        let id = text_in(&theme, "id")?;
        // The list rates each theme as it is saved: nothing in it is hard to read.
        assert_eq!(theme["readability"]["hard_to_read"], 0);
        assert!(theme["readability"]["easy"].as_u64() > Some(0));

        let chosen = ben
            .send(
                "PUT",
                "/api/v1/me/settings/appearance.dark-theme",
                Some(json!({"value": text_in(&theme, "choice")?})),
            )
            .await?;
        assert!(chosen.status.is_success(), "{}", chosen.body);
        let detail = ada.send("GET", &format!("{THEMES}/{id}"), None).await?;
        assert_eq!(ok(&detail)?["used_by"], 1);
        assert_eq!(ok(&detail)?["organization_choice"], false);
        // Made the organization's dark theme, it reaches everyone who keeps that choice.
        let organization = ada
            .send(
                "PUT",
                "/api/v1/settings/appearance.dark-theme",
                Some(json!({"value": text_in(&theme, "choice")?})),
            )
            .await?;
        assert!(organization.status.is_success(), "{}", organization.body);
        let detail = ada.send("GET", &format!("{THEMES}/{id}"), None).await?;
        assert_eq!(ok(&detail)?["organization_choice"], true);
        // Who may only use it is not told.
        let seen = ben.send("GET", &format!("{THEMES}/{id}"), None).await?;
        assert!(ok(&seen)?.get("used_by").is_none());
        assert!(ok(&seen)?.get("organization_choice").is_none());
        Ok(())
    })
    .await
}
