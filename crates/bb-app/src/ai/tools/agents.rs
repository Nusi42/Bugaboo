//! The tools of agents (AI-36, ADR-0029, decision 9): *take*, *report*, *ask*, *attach*, *ready*,
//! *give up*, *changes since* and *propose*. They are in the one registry like every tool, and each
//! only calls the use case of agent work, as the agent behind the key: what the use case refuses
//! the tool refuses, and what the use case judges the preview judges, from the same plan.
//!
//! A tool of agents is for an agent only; the registry does not offer it to anybody else
//! (`Tool::is_for_agents`). Every answer says where the work stands, and every text in it names who
//! wrote it: what a person wrote (the note of the brief, an answer) comes with the kind of its
//! author, and what the agent wrote itself with *an agent*.

use bb_core::ErrorCode;
use bb_core::agent_records::ResultKind;
use bb_core::agent_work::{CheckResult, RuleField};
use bb_core::ai::AuthorKind;
use bb_core::event::FieldChange;
use bb_core::fields::FieldKey;
use bb_core::id::{AgentWorkId, StandingRuleId};
use std::future::Future;

use serde_json::{Value, json};

use super::call::Saved;
use super::inputs::{
    AskInput, AttachInput, ChangesSinceInput, GiveUpInput, ProposeInput, ReportInput, SubmitInput,
    TakeInput, limit_within,
};
use super::preview::preview_parts;
use super::registry::{Boxed, Run};
use crate::agent_work::WorkView;
use crate::agent_work::{
    AskQuestion, AttachResult, ChangesSinceQuery, EndAttempt, ProposeChange, ReportAttempt,
    TakeWork, ask_question, attach_result, changes_since, end_attempt, propose_change,
    report_attempt, take_work,
};
use crate::ai::context::Part;
use crate::auth::{AppError, AuthStore, Ctx};
use bb_policy::Authorized;

fn invalid() -> AppError {
    AppError::Refused(ErrorCode::InvalidRequest)
}

/// Refuses a call that does not come from an agent. The registry does not offer these tools to
/// anybody else, and the use cases refuse again.
fn ensure_agent(run: &Run<'_>) -> Result<(), AppError> {
    if run.actor.agent().is_none() {
        return Err(ErrorCode::Forbidden.into());
    }
    Ok(())
}

fn take_request(run: &Run<'_>) -> Result<TakeWork, AppError> {
    let input: TakeInput = run.input()?;
    Ok(TakeWork {
        session_token: run.token.to_owned(),
        issue: input.issue,
        version: input.version,
    })
}

fn report_request(run: &Run<'_>) -> Result<ReportAttempt, AppError> {
    let input: ReportInput = run.input()?;
    Ok(ReportAttempt {
        session_token: run.token.to_owned(),
        issue: input.issue,
        plan: input.plan,
        minutes: i64::from(input.minutes),
        cost: i64::from(input.cost_cents),
    })
}

fn ask_request(run: &Run<'_>) -> Result<AskQuestion, AppError> {
    let input: AskInput = run.input()?;
    Ok(AskQuestion {
        session_token: run.token.to_owned(),
        issue: input.issue,
        text: input.question,
    })
}

fn attach_request(run: &Run<'_>) -> Result<AttachResult, AppError> {
    let input: AttachInput = run.input()?;
    Ok(AttachResult {
        session_token: run.token.to_owned(),
        issue: input.issue,
        kind: ResultKind::from_code(&input.kind).ok_or_else(invalid)?,
        label: input.label,
        link: input.address,
        outcome: input
            .outcome
            .map(|outcome| CheckResult::from_code(&outcome).ok_or_else(invalid))
            .transpose()?,
    })
}

fn submit_request(run: &Run<'_>) -> Result<EndAttempt, AppError> {
    let input: SubmitInput = run.input()?;
    Ok(EndAttempt {
        session_token: run.token.to_owned(),
        issue: input.issue,
        gave_up: None,
    })
}

fn give_up_request(run: &Run<'_>) -> Result<EndAttempt, AppError> {
    let input: GiveUpInput = run.input()?;
    Ok(EndAttempt {
        session_token: run.token.to_owned(),
        issue: input.issue,
        gave_up: Some(input.reason),
    })
}

fn propose_request(run: &Run<'_>) -> Result<ProposeChange, AppError> {
    let input: ProposeInput = run.input()?;
    Ok(ProposeChange {
        session_token: run.token.to_owned(),
        rule: input
            .rule
            .parse::<StandingRuleId>()
            .map_err(|_| invalid())?,
        issue: input.issue,
        field: RuleField::from_code(&input.field).ok_or_else(invalid)?,
        after: input.value,
    })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Refuses an issue in a project kept from AI as one that is not there, like every tool does.
    async fn agent_issue_in_reach(
        &self,
        run: &Run<'_>,
        address: &str,
    ) -> Result<bb_core::issue::Issue, AppError> {
        let (issue, _) = self.visible_issue(run.actor, address).await?;
        if run.withheld.hides_project(issue.project) {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        Ok(issue)
    }

    /// The issue in the agent's reach at `address`, and the readable ID it is named by.
    async fn agent_issue_and_key(
        &self,
        run: &Run<'_>,
        address: &str,
    ) -> Result<(bb_core::issue::Issue, String), AppError> {
        let issue = self.agent_issue_in_reach(run, address).await?;
        let key = self
            .readable_id_for(run.actor, issue.id)
            .await?
            .unwrap_or_else(|| address.to_owned());
        Ok((issue, key))
    }

    /// What an agent reads about its work after a tool changed it: where it stands, the note of the
    /// brief, the answers and the notes of a review (each with the kind of the author who really
    /// wrote it), and what it asked itself.
    async fn work_parts(
        &self,
        run: &Run<'_>,
        address: &str,
        work: &Value,
    ) -> Result<Vec<Part>, AppError> {
        let failed = |error| crate::auth::internal::<S>(&error);
        let (issue, key) = self.agent_issue_and_key(run, address).await?;
        let attempt = work
            .get("attempts")
            .and_then(Value::as_array)
            .and_then(|attempts| attempts.last())
            .and_then(|last| last.get("number"))
            .cloned()
            .unwrap_or(Value::Null);
        let saved = Saved::On { key: key.clone() }.part();
        let mut parts = vec![Part {
            name: "work",
            value: json!({
                "key": key,
                "work_state": work.get("state").cloned().unwrap_or(Value::Null),
                "version": work.get("version").cloned().unwrap_or(Value::Null),
                "attempt": attempt,
            }),
            author: AuthorKind::Agent,
            project: Some(issue.project),
        }];
        parts.push(saved);
        let Some(id) = work
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| id.parse::<AgentWorkId>().ok())
        else {
            return Ok(parts);
        };
        let Some(record) = self.store().issue_work(id).await.map_err(failed)? else {
            return Ok(parts);
        };
        let text_of = |name: &'static str, text: &str, author: AuthorKind| Part {
            name,
            value: Value::String(text.to_owned()),
            author,
            project: Some(issue.project),
        };
        // The note of the brief is the word of the person who handed the issue over.
        if let Some(hand_over) = self
            .store()
            .hand_over(record.hand_over)
            .await
            .map_err(failed)?
            && !hand_over.note.is_empty()
        {
            let by = self.author_kind(hand_over.person, issue.project).await?;
            parts.push(text_of("brief", &hand_over.note, by));
        }
        // An answer is the word of the person who answered it, a note of a review the word of the
        // person who reviewed; when nobody can be named the least trusted kind stands for them.
        for question in self.store().questions_of(id).await.map_err(failed)? {
            parts.push(text_of("question", &question.text, AuthorKind::Agent));
            if let Some(answer) = &question.answer {
                let by = self.author_of(question.answered_by, issue.project).await?;
                parts.push(text_of("brief", answer, by));
            }
        }
        for review in self.store().reviews_of(id).await.map_err(failed)? {
            if let Some(note) = &review.note {
                let by = self.author_of(review.by, issue.project).await?;
                parts.push(text_of("review", note, by));
            }
        }
        Ok(parts)
    }

    /// The kind of author of a person who may be unknown.
    async fn author_of(
        &self,
        person: Option<bb_core::id::UserId>,
        project: bb_core::id::ProjectId,
    ) -> Result<AuthorKind, AppError> {
        match person {
            Some(person) => self.author_kind(person, project).await,
            None => Ok(AuthorKind::Outsider),
        }
    }

    /// The preview of a call: what the plan of the use case would change, saved nowhere.
    async fn agent_preview(
        &self,
        run: &Run<'_>,
        address: &str,
        planned: impl Future<Output = Result<Vec<FieldChange>, AppError>>,
    ) -> Result<Vec<Part>, AppError> {
        ensure_agent(run)?;
        self.agent_issue_in_reach(run, address).await?;
        Ok(preview_parts(run, planned.await?, AuthorKind::Agent))
    }

    /// A call that changes the work of an agent, made: the plan is judged as the use case will judge
    /// it (so a limit that is reached pauses the agent before anything is written), the use case
    /// runs, and the answer says what it changed and where the work stands.
    async fn agent_apply(
        &self,
        run: &Run<'_>,
        address: &str,
        planned: impl Future<Output = Result<Vec<FieldChange>, AppError>>,
        made: impl Future<Output = Result<Authorized<WorkView>, AppError>>,
    ) -> Result<Vec<Part>, AppError> {
        let mut parts = self.agent_preview(run, address, planned).await?;
        let work = made.await?;
        let value = work
            .masked_value()
            .map_err(|error| AppError::Internal(error.to_string()))?;
        parts.extend(self.work_parts(run, address, &value).await?);
        Ok(parts)
    }
}

/// `take`, asked about.
pub(super) fn preview_take<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = take_request(&run)?;
        ctx.agent_preview(&run, &request.issue, ctx.preview_take_work(&request, false))
            .await
    })
}

/// `take`, made.
pub(super) fn apply_take<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = take_request(&run)?;
        let planned = ctx.preview_take_work(&request, true);
        ctx.agent_apply(
            &run,
            &request.issue,
            planned,
            take_work(ctx, request.clone()),
        )
        .await
    })
}

/// `report`, asked about.
pub(super) fn preview_report<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = report_request(&run)?;
        ctx.agent_preview(&run, &request.issue, ctx.preview_report_attempt(&request))
            .await
    })
}

/// `report`, made.
pub(super) fn apply_report<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = report_request(&run)?;
        let planned = ctx.preview_report_attempt(&request);
        ctx.agent_apply(
            &run,
            &request.issue,
            planned,
            report_attempt(ctx, request.clone()),
        )
        .await
    })
}

/// `ask`, asked about.
pub(super) fn preview_ask<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = ask_request(&run)?;
        ctx.agent_preview(&run, &request.issue, ctx.preview_ask_question(&request))
            .await
    })
}

/// `ask`, made.
pub(super) fn apply_ask<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = ask_request(&run)?;
        let planned = ctx.preview_ask_question(&request);
        ctx.agent_apply(
            &run,
            &request.issue,
            planned,
            ask_question(ctx, request.clone()),
        )
        .await
    })
}

/// `attach`, asked about.
pub(super) fn preview_attach<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = attach_request(&run)?;
        ctx.agent_preview(&run, &request.issue, ctx.preview_attach_result(&request))
            .await
    })
}

/// `attach`, made.
pub(super) fn apply_attach<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = attach_request(&run)?;
        let planned = ctx.preview_attach_result(&request);
        ctx.agent_apply(
            &run,
            &request.issue,
            planned,
            attach_result(ctx, request.clone()),
        )
        .await
    })
}

/// `submit`, asked about.
pub(super) fn preview_submit<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = submit_request(&run)?;
        ctx.agent_preview(&run, &request.issue, ctx.preview_end_attempt(&request))
            .await
    })
}

/// `submit`, made.
pub(super) fn apply_submit<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = submit_request(&run)?;
        let planned = ctx.preview_end_attempt(&request);
        ctx.agent_apply(
            &run,
            &request.issue,
            planned,
            end_attempt(ctx, request.clone()),
        )
        .await
    })
}

/// `give_up`, asked about.
pub(super) fn preview_give_up<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = give_up_request(&run)?;
        ctx.agent_preview(&run, &request.issue, ctx.preview_end_attempt(&request))
            .await
    })
}

/// `give_up`, made.
pub(super) fn apply_give_up<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = give_up_request(&run)?;
        let planned = ctx.preview_end_attempt(&request);
        ctx.agent_apply(
            &run,
            &request.issue,
            planned,
            end_attempt(ctx, request.clone()),
        )
        .await
    })
}

/// `propose`: the change is the field of the rule and the value proposed.
pub(super) fn preview_propose<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        ensure_agent(&run)?;
        let request = propose_request(&run)?;
        ctx.agent_issue_in_reach(&run, &request.issue).await?;
        let changes = ctx.preview_propose_change(&request).await?;
        Ok(preview_parts(&run, changes, AuthorKind::Agent))
    })
}

/// `propose`, saved: a proposal on the issue; nothing else changes until a person applies it.
pub(super) fn apply_propose<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let mut shown = preview_propose(ctx, run).await?;
        let request = propose_request(&run)?;
        let (_, key) = ctx.agent_issue_and_key(&run, &request.issue).await?;
        propose_change(ctx, request).await?;
        shown.push(Saved::On { key }.part());
        Ok(shown)
    })
}

/// `changes_since`: what changed in the issues the agent may see, the issues that came under its
/// rules and where its work stands, each with the kind of author of what is new in it.
pub(super) fn changes<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        ensure_agent(&run)?;
        let input: ChangesSinceInput = run.input()?;
        let page = changes_since(
            ctx,
            ChangesSinceQuery {
                session_token: run.token.to_owned(),
                since: input.since,
                limit: limit_within(input.limit)?,
            },
        )
        .await?;
        let value = page
            .masked_value()
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(parts_of_changes(&run, &value))
    })
}

/// The members of one entry that tell where it is, taken out of the value the AI reads.
fn take_project(entry: &mut Value) -> Option<bb_core::id::ProjectId> {
    entry
        .as_object_mut()?
        .remove("project")?
        .as_str()?
        .parse()
        .ok()
}

/// The parts of the answer of *changes since*, without what is kept from AI: an issue in a project
/// that is, and a change of a field that is.
fn parts_of_changes(run: &Run<'_>, page: &Value) -> Vec<Part> {
    let mut parts = Vec::new();
    for mut entry in list(page, "changes") {
        let project = take_project(&mut entry);
        if project.is_some_and(|project| run.withheld.hides_project(project)) {
            continue;
        }
        let author = entry
            .as_object_mut()
            .and_then(|object| object.remove("author"))
            .and_then(|code| code.as_str().and_then(AuthorKind::from_code))
            .unwrap_or(AuthorKind::Outsider);
        if let Some(Value::Array(fields)) = entry.get_mut("changes") {
            fields.retain(|each| {
                !each
                    .get("field")
                    .and_then(Value::as_str)
                    .and_then(FieldKey::from_code)
                    .is_some_and(|key| run.withheld.hides_field(key))
            });
            if fields.is_empty() {
                continue;
            }
        }
        parts.push(Part {
            name: "change",
            value: entry,
            author,
            project,
        });
    }
    for (name, member) in [("match", "matches"), ("work", "work")] {
        for mut entry in list(page, member) {
            let project = take_project(&mut entry);
            if project.is_some_and(|project| run.withheld.hides_project(project)) {
                continue;
            }
            parts.push(Part {
                name,
                value: entry,
                author: AuthorKind::Agent,
                project,
            });
        }
    }
    parts.push(Part {
        name: "next",
        value: json!({
            "mark": page.get("mark").cloned().unwrap_or(Value::Null),
            "more": page.get("more").cloned().unwrap_or(Value::Null),
        }),
        author: AuthorKind::Agent,
        project: None,
    });
    parts
}

fn list(page: &Value, member: &str) -> Vec<Value> {
    page.get(member)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}
