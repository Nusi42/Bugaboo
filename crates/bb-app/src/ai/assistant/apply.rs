//! *Apply* (AI-05, COMP-08): the person decides, and the use case of the tool runs as the person.

use bb_core::ErrorCode;
use bb_core::ai::{PROPOSAL_INPUT_BYTES_MAX, ToolCaller, ToolScope};
use bb_policy::{Authorized, Masked, Place};
use serde::Serialize;
use serde_json::Value;

use super::{reference, session_only};
use crate::ai::context::PeopleMode;
use crate::ai::tools::{ProposedChange, ToolAnswer, ToolCallQuery, ToolMode, ToolUndo, call_tool};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::history::FieldChangeView;

/// A proposal the person applies.
pub struct ApplyProposal {
    /// The secret of the session, or the access key, *Apply* is run as.
    pub session_token: String,
    /// The reference the proposal came with.
    pub reference: String,
    /// The tool of the proposal.
    pub tool: String,
    /// Its input, as it came.
    pub input: Value,
}

/// What *Apply* saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Applied {
    /// The fields it changed, with the value before and after.
    pub changes: Vec<ProposedChange>,
    /// The same fields in the words of the history of the issue.
    pub words: Vec<FieldChangeView>,
    /// What *Undo* takes away, where the change made something new.
    pub undo: Option<ToolUndo>,
    /// The version the issue is at after the change, as the write itself gave it back, when the
    /// change is on an issue: *Undo* of fields is made on this version, so a change by somebody
    /// else in between is never overwritten.
    pub version: Option<i64>,
}

impl Masked for Applied {}

/// Saves exactly what a proposal showed, as the person, through the use case of its tool and the
/// channel `assistant`. The reference has to be one this server made for this person, this tool and
/// this input a short time ago. Everything else is judged now, as for any change by the person: a
/// right that was taken away, an issue that is gone or kept from AI since, a version that moved on
/// (the answer then holds the newer value, as it does for a person).
///
/// # Errors
/// `auth.access-key-not-allowed` for an access key, `ai.proposal-invalid` for a reference that does not hold, and what the use case of the tool
/// refuses.
pub async fn apply_proposal<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ApplyProposal,
) -> Result<Authorized<Applied>, AppError> {
    session_only(&input.session_token)?;
    let actor = ctx.actor(&input.session_token).await?;
    let invalid = || AppError::Refused(ErrorCode::AiProposalInvalid);
    if input.input.to_string().len() > PROPOSAL_INPUT_BYTES_MAX {
        return Err(invalid());
    }
    let opened = reference::open(
        &ctx.services.links,
        &input.reference,
        actor.person.id,
        ctx.now(),
        &input.tool,
        &input.input,
    )
    .ok_or_else(invalid)?;
    // A reference works once: a retry or a second click saves nothing again.
    let first = ctx
        .store()
        .use_assistant_reference(&opened.word, ctx.now(), opened.expires_at)
        .await
        .map_err(|error| internal::<S>(&error))?;
    if !first {
        return Err(invalid());
    }
    let input_for_words = input.input.clone();
    let applied = call_tool(
        ctx,
        ToolCallQuery {
            session_token: input.session_token,
            caller: ToolCaller::Assistant,
            app: None,
            granted: ToolScope::SeeAndChange,
            people: PeopleMode::AsSeen,
            tool: input.tool,
            input: input.input,
            mode: ToolMode::Apply,
        },
    )
    .await?;
    let saved: ToolAnswer = serde_json::from_value(
        applied
            .masked_value()
            .map_err(|error| AppError::Internal(error.to_string()))?,
    )
    .map_err(|error| AppError::Internal(error.to_string()))?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let words = ctx.words_of(&actor, &input_for_words, &saved.changes).await;
    Ok(held.authorize(Applied {
        changes: saved.changes,
        words,
        undo: saved.undo,
        version: saved.version,
    }))
}
