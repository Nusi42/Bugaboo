//! The work of a person's own agents in one list (AI-37, *My agents*): what waits for the person,
//! the work in progress, and the work that was accepted by itself and that nobody has looked at.

use bb_policy::{Authorized, Place};

use super::types::{MyAgentWork, MyAgentWorkQuery, MyWorkItem};
use crate::agents::AgentStore;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The most pieces of work *My agents* shows; the answer says when it was cut.
pub const MOST_SHOWN_IN_MY_AGENTS: usize = 200;

/// The current work of every agent of the person, newest first. Work on an issue the person may no
/// longer see is left out, like any list of issues.
///
/// # Errors
/// `auth.not-signed-in` without a session.
pub async fn my_agent_work<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MyAgentWorkQuery,
) -> Result<Authorized<MyAgentWork>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let actor = crate::access::Actor {
        person: record.person,
        key: None,
    };
    let failed = |error| internal::<S>(&error);
    let agents: Vec<_> = <S as AgentStore>::agents_of(ctx.store(), person, None, 100)
        .await
        .map_err(failed)?
        .into_iter()
        .map(|agent| agent.id)
        .collect();
    let mut works = ctx
        .store()
        .current_work_of_agents(agents, MOST_SHOWN_IN_MY_AGENTS as u64 + 1)
        .await
        .map_err(failed)?;
    let truncated = works.len() > MOST_SHOWN_IN_MY_AGENTS;
    works.truncate(MOST_SHOWN_IN_MY_AGENTS);
    let held = ctx.evaluate(person, Place::Organization).await?;
    let views = ctx.work_views(&held, &works).await?;
    let ids: Vec<_> = works.iter().map(|work| work.issue).collect();
    let issues = ctx.issues_named(&ids).await?;
    let mut maker = ctx.line_maker().await?;
    let mut items = Vec::with_capacity(views.len());
    for (work, view) in works.iter().zip(views) {
        let Some(issue) = issues.iter().find(|found| found.id == work.issue) else {
            continue;
        };
        let Some(line) = ctx.line_of(&actor, &mut maker, issue).await? else {
            continue;
        };
        let question_open = view
            .questions
            .iter()
            .any(|question| question.answer.is_none());
        items.push(MyWorkItem {
            waits_for: view.state.waits_for(question_open),
            work: view,
            issue: line,
        });
    }
    Ok(held.authorize(MyAgentWork { items, truncated }))
}
