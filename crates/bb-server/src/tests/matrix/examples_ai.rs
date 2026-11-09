//! The example requests of the routes of *Never sent to AI* (M7 WP-03).
//!
//! The routes that set a mark take the version of the mark, so each request reads it first as the
//! administrator of the matrix: every actor then writes over a mark that is at the version they
//! ask for.

use bb_app::ai::NeverSentQuery;
use bb_testing::Failure;
use serde_json::json;

use super::actors::Project;
use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::rig::Rig;

/// The word of the built-in field the matrix marks.
const FIELD: &str = "description";

/// The version of the mark of the target project, and of the field, as they are now.
async fn versions(fixture: &Fixture<'_>) -> Result<(i64, i64), Failure> {
    let list = fixture
        .rig()
        .api()
        .never_sent_list(
            Rig::info(),
            NeverSentQuery {
                session_token: fixture.admin_session().to_owned(),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    let target = fixture.project(Project::Target);
    let project = list
        .projects
        .iter()
        .find(|each| each.project.id == target)
        .map_or(0, |each| each.version);
    let field = list
        .fields
        .iter()
        .find(|each| each.field.code() == FIELD)
        .map_or(0, |each| each.version);
    Ok((project, field))
}

pub(super) async fn ai_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    let target = fixture.project(Project::Target).to_string();
    Ok(Some(match operation {
        // AI is off in the matrix, so a question is answered "AI is off" and sent nowhere; a
        // reference that was not made by this server for this person is no reference.
        "getAssistantStatus" => ("GET", "/api/v1/assistant".to_owned(), None),
        "askAssistant" => (
            "POST",
            "/api/v1/assistant/questions".to_owned(),
            Some(json!({"text": "What is open?"})),
        ),
        "applyAssistantProposal" => (
            "POST",
            "/api/v1/assistant/apply".to_owned(),
            Some(json!({"reference": "AAAA", "tool": "update_issue", "input": {}})),
        ),
        "getAiGuide" => ("GET", "/api/v1/help/ai-guide".to_owned(), None),
        "listNeverSentToAi" => ("GET", "/api/v1/never-sent-to-ai".to_owned(), None),
        "setFieldNeverSentToAi" => (
            "PUT",
            format!("/api/v1/never-sent-to-ai/fields/{FIELD}"),
            Some(json!({"marked": true, "version": versions(fixture).await?.1})),
        ),
        "setProjectNeverSentToAi" => (
            "PUT",
            format!("/api/v1/never-sent-to-ai/projects/{target}"),
            Some(json!({"marked": true, "version": versions(fixture).await?.0})),
        ),
        // The sign-in of AI apps (AI-22). AI is off in the matrix, so an actor who is let in is told
        // that apps are off (`ai.off`), which is no refusal of a permission.
        "getAuthorization" => (
            "GET",
            format!("/api/v1/oauth/authorization?{AUTHORIZATION_QUERY}"),
            None,
        ),
        "decideAuthorization" => (
            "POST",
            "/api/v1/oauth/authorization".into(),
            Some(json!({
                "request": {
                    "client": NOBODYS_APP,
                    "response_type": "code",
                    "redirect_uri": "http://127.0.0.1/callback",
                    "code_challenge": CHALLENGE,
                    "code_challenge_method": "S256",
                },
                "allow": true,
                "scope": "see-only",
                "projects": [],
            })),
        ),
        "listConnectedApps" => ("GET", "/api/v1/oauth/apps".into(), None),
        "takeBackApp" => ("DELETE", format!("/api/v1/oauth/apps/{NOBODYS_APP}"), None),
        "listAppAddresses" => ("GET", "/api/v1/oauth/web-addresses".into(), None),
        "addAppAddress" => (
            "POST",
            "/api/v1/oauth/web-addresses".into(),
            Some(json!({"address": "https://app.example.test/matrix"})),
        ),
        "removeAppAddress" => (
            "POST",
            "/api/v1/oauth/web-addresses/remove".into(),
            Some(json!({"address": "https://nobody.example.test/matrix"})),
        ),
        _ => return Ok(None),
    }))
}

/// An app that nobody registered.
const NOBODYS_APP: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";
/// A well-formed challenge of PKCE (43 symbols).
const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
/// The query of a request to the consent step for an app nobody registered.
const AUTHORIZATION_QUERY: &str = "client=5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f&response_type=code&redirect_uri=http%3A%2F%2F127.0.0.1%2Fcallback&code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM&code_challenge_method=S256";
