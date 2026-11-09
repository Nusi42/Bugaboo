//! The guide to AI for everybody (COMP-11, M7-F90): `userDocs/ai/literacy.md`, built into the
//! program and shown as a page of the app. The text has one home, the file; this module renders it
//! once and gives it to every signed-in person, so an organization on its own servers never has
//! to send anybody to the internet to read it.

use bb_policy::{Authorized, Masked, Place};

use crate::auth::{AppError, AuthStore, Ctx};

/// The guide as it is written in the user docs.
const GUIDE: &str = include_str!("../../../../userDocs/ai/literacy.md");

/// Ask for the guide.
pub struct AiGuideQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// The guide, as safe HTML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiGuide {
    /// The HTML of the guide, from `bb-markdown`.
    pub html: String,
}

impl Masked for AiGuide {}

/// The AI guide for a signed-in person. Nothing in it is private, so any person who is signed in
/// may read it.
///
/// # Errors
/// `auth.not-signed-in` without a session.
pub async fn ai_guide<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AiGuideQuery,
) -> Result<Authorized<AiGuide>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    // The other pages of the user docs are not in the app: their links keep their words only.
    let html = bb_markdown::render(
        GUIDE,
        &bb_markdown::Options {
            web_links_only: true,
            ..bb_markdown::Options::default()
        },
    )
    .html;
    Ok(held.authorize(AiGuide { html }))
}
