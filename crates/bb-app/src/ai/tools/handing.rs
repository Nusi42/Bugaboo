//! The tools by which an agent hands work on and undoes its own runs (ADR-0029, amendment M9):
//! `hand_over_work` runs the one use case of a hand-over, `undo_my_run` runs *Undo the run*, each as the
//! agent behind the key, and each only when its person switched it on.

use bb_core::ErrorCode;
use bb_core::agent_work::HandOverPower;
use bb_core::ai::AuthorKind;
use bb_core::event::FieldChange;
use serde_json::{Value, json};

use super::agents::ensure_agent;
use super::inputs::{HandOverInput, UndoMyRunInput};
use super::preview::preview_parts;
use super::refused::word_at;
use super::registry::{Boxed, Run};
use crate::agent_work::HandOver;
use crate::ai::context::Part;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::runs::{RunQuery, UndoRun, preview_undo_run, undo_run};

impl<S: AuthStore> Ctx<'_, S> {
    /// The other agent of the person behind the key, by its name or its ID.
    async fn other_agent(
        &self,
        run: &Run<'_>,
        named: &str,
    ) -> Result<bb_core::agent::Agent, AppError> {
        let ctx = self;
        let person = run.actor.person.id;
        let most = u64::try_from(bb_core::agent::MOST_AGENTS).unwrap_or(u64::MAX);
        ctx.store()
            .agents_of(person, None, most)
            .await
            .map_err(|error| crate::auth::internal::<S>(&error))?
            .into_iter()
            .find(|agent| {
                agent.id.to_string() == named
                    || agent.name.trim().eq_ignore_ascii_case(named.trim())
            })
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }

    /// The hand-over the input of `hand_over_work` asks for, as the one use case takes it.
    async fn hand_over_request(&self, run: &Run<'_>) -> Result<HandOver, AppError> {
        let input: HandOverInput = run.input()?;
        let agent = self.other_agent(run, &input.agent).await?;
        let powers = input
            .powers
            .unwrap_or_default()
            .iter()
            .map(|code| word_at::<HandOverPower>("powers", code))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(HandOver {
            session_token: run.token.to_owned(),
            agent: agent.id,
            issues: input.issues,
            note: input.note.unwrap_or_default(),
            powers,
            fields: input.fields.unwrap_or_default(),
            budget_attempts: None,
            budget_minutes: None,
            budget_cost: None,
            plan_needs_approval: false,
            accept_by_itself: false,
            days: None,
        })
    }
}

/// What a hand-over gives: the agent and the issues, as the event of a hand-over names them.
fn hand_over_changes(request: &HandOver) -> Vec<FieldChange> {
    let set = |field: &str, value: Value| FieldChange {
        field: field.to_owned(),
        before: None,
        after: Some(value),
    };
    vec![
        set("agent", json!(request.agent.to_string())),
        set("issues", json!(request.issues)),
    ]
}

/// `hand_over_work`, asked about: the switch, the other agent and every issue are judged as the use case
/// judges them, and nothing is written.
pub(super) fn preview_hand_over<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        ensure_agent(&run)?;
        let request = ctx.hand_over_request(&run).await?;
        for issue in &request.issues {
            ctx.agent_issue_in_reach(&run, issue).await?;
        }
        ctx.preview_hand_over(&request).await?;
        Ok(preview_parts(
            &run,
            hand_over_changes(&request),
            AuthorKind::Agent,
        ))
    })
}

/// `hand_over_work`, made through the one use case of a hand-over.
pub(super) fn apply_hand_over<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let mut parts = preview_hand_over(ctx, run).await?;
        let request = ctx.hand_over_request(&run).await?;
        let given = json!({ "agent": request.agent.to_string(), "issues": request.issues });
        ctx.agent_hand_over(request).await?;
        parts.push(Part {
            name: "handed_over",
            value: given,
            author: AuthorKind::Agent,
            project: None,
        });
        Ok(parts)
    })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The run of an attempt of the agent's open work on the issue: the attempt it names, or its
    /// newest. Anything else is not there.
    async fn run_of_my_attempt(
        &self,
        run: &Run<'_>,
        input: &UndoMyRunInput,
    ) -> Result<bb_core::id::RunId, AppError> {
        let failed = |error| crate::auth::internal::<S>(&error);
        let agent = run
            .actor
            .agent()
            .ok_or(AppError::Refused(ErrorCode::Forbidden))?;
        let issue = self.agent_issue_in_reach(run, &input.issue).await?;
        let work = self
            .store()
            .open_work_of(agent, issue.id)
            .await
            .map_err(failed)?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let attempts = self.store().attempts_of(work.id).await.map_err(failed)?;
        attempts
            .iter()
            .filter(|attempt| input.attempt.is_none_or(|number| attempt.number == number))
            .max_by_key(|attempt| attempt.number)
            .and_then(|attempt| attempt.run)
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }
}

/// The part of what an undo would do or did: how many changes go back and how many stay.
fn undo_part(value: &Value, name: &'static str) -> Part {
    Part {
        name,
        value: json!({
            "goes_back": value["goes_back"],
            "stays": value["skipped"],
        }),
        author: AuthorKind::Agent,
        project: None,
    }
}

/// `undo_my_run`, asked about: the preview of *Undo the run* for the run of the attempt.
pub(super) fn preview_undo_my_run<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        ensure_agent(&run)?;
        let input: UndoMyRunInput = run.input()?;
        let id = ctx.run_of_my_attempt(&run, &input).await?;
        ctx.ensure_may_undo(run.actor).await?;
        let preview = preview_undo_run(
            ctx,
            RunQuery {
                session_token: run.token.to_owned(),
                run: id,
            },
        )
        .await?;
        Ok(vec![undo_part(
            &super::read::masked(&preview)?,
            "would_undo",
        )])
    })
}

/// `undo_my_run`, made through *Undo the run* with the version of the preview it makes first.
pub(super) fn apply_undo_my_run<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        ensure_agent(&run)?;
        let input: UndoMyRunInput = run.input()?;
        let id = ctx.run_of_my_attempt(&run, &input).await?;
        let query = || RunQuery {
            session_token: run.token.to_owned(),
            run: id,
        };
        let preview = super::read::masked(&preview_undo_run(ctx, query()).await?)?;
        let version_of = |value: &Value| {
            serde_json::from_value::<bb_core::run::UndoVersion>(value["version"].clone())
                .map_err(|error| AppError::Internal(error.to_string()))
        };
        let mut version = version_of(&preview)?;
        // Something may change between the preview and the undo: the newer preview is used once.
        for _ in 0..2 {
            let done = undo_run(
                ctx,
                UndoRun {
                    session_token: run.token.to_owned(),
                    run: id,
                    version: version.clone(),
                },
            )
            .await?;
            let value = super::read::masked(&done)?;
            if value["changed_since_preview"] != Value::Bool(true) {
                return Ok(vec![undo_part(&value, "undone")]);
            }
            version = version_of(&value)?;
        }
        Err(ErrorCode::VersionChanged.into())
    })
}
