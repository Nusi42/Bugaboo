//! The guide to AI for everybody (COMP-11), as the HTML of one page of the app.

use axum::extract::State;
use axum::response::Response;
use bb_app::ai::AiGuideQuery;
use bb_policy::Masked;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{AuthHandle, Info, NotSignedInAnswer, Session, answer_authorized};
use crate::errors::ApiError;

/// The guide to AI.
#[derive(Serialize, ToSchema)]
pub(crate) struct AiGuideBody {
    /// The guide as safe HTML, made from the text of the user docs when the program was built.
    html: String,
}

impl Masked for AiGuideBody {}

/// The guide to AI for everybody (`userDocs/ai/literacy.md`), rendered as safe HTML. Every signed-in
/// person may read it: it holds no data of the organization.
#[utoipa::path(
    get,
    path = "/api/v1/help/ai-guide",
    operation_id = "getAiGuide",
    tag = "ai",
    responses((status = 200, description = "The guide.", body = AiGuideBody), NotSignedInAnswer)
)]
async fn get_ai_guide(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let guide = auth
        .api()?
        .ai_guide(info, AiGuideQuery { session_token })
        .await?;
    answer_authorized(&guide.map(|found| AiGuideBody { html: found.html }))
}

/// The route of the guide.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(get_ai_guide))
}
