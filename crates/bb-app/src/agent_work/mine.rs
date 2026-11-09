//! The work of a person's own agents in one list (AI-37, *My agents*): what waits for the person,
//! the work in progress, and the work that was accepted by itself and that nobody has looked at.

use std::sync::Arc;

use bb_core::agent::MOST_AGENTS;
use bb_core::agent_records::IssueWork;
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Authorized, Evaluation, PeopleContext, Place};

use super::rules::view_of;
use super::types::{MyAgentWork, MyAgentWorkQuery, MyWorkItem, OfferedAgent};
use crate::access::Actor;
use crate::agents::AgentStore;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::Where;
use crate::relations::IssueLine;

/// The most pieces of work *My agents* shows; the answer says when it was cut.
pub const MOST_SHOWN_IN_MY_AGENTS: usize = 200;

/// A piece of work with the line of its issue, in the place it stood in the store's list.
struct Seen<'a> {
    held: Arc<Evaluation>,
    work: &'a IssueWork,
    line: IssueLine,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The pieces of work whose issue the person sees, as items. The people in each piece (who handed
    /// it over, whose agent it is) are shown as *Who sees people* says in the project of its issue
    /// (ADR-0035), all pieces read together (PERF-N1).
    async fn my_items(
        &self,
        actor: &Actor,
        works: &[IssueWork],
    ) -> Result<Vec<MyWorkItem>, AppError> {
        let ids: Vec<_> = works.iter().map(|work| work.issue).collect();
        let issues = self.issues_named(&ids).await?;
        let mut maker = self.line_maker().await?;
        let mut seen = Vec::with_capacity(works.len());
        for work in works {
            let Some(issue) = issues.iter().find(|found| found.id == work.issue) else {
                continue;
            };
            let Some(held) = self.seen_in(actor, issue, Where::Live).await? else {
                continue;
            };
            let Some(line) = self.line_of(actor, &mut maker, issue).await? else {
                continue;
            };
            seen.push(Seen { held, work, line });
        }
        // One read for all of them, each seen with the rights of its project.
        let each: Vec<_> = seen.iter().map(|one| (&*one.held, one.work)).collect();
        let views = self.work_views_each(&each).await?;
        let mut items = Vec::with_capacity(seen.len());
        for (one, view) in seen.into_iter().zip(views) {
            let question_open = view
                .questions
                .iter()
                .any(|question| question.answer.is_none());
            items.push(MyWorkItem {
                // Work that was accepted or thrown away waits for nobody.
                waits_for: if view.open {
                    view.state.waits_for(question_open)
                } else {
                    None
                },
                work: view,
                issue: one.line,
            });
        }
        Ok(items)
    }
}

/// The current work of every agent of the person, what waits for them first, then the newest;
/// their standing rules that work; and the agents handed to them. Work on an issue the person may
/// no longer see is left out, like any list of issues.
///
/// # Errors
/// `auth.not-signed-in` without a session.
pub async fn my_agent_work<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MyAgentWorkQuery,
) -> Result<Authorized<MyAgentWork>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let actor = Actor {
        person: record.person,
        key: None,
    };
    let failed = |error| internal::<S>(&error);
    let most_agents = MOST_AGENTS as u64;
    let agents: Vec<_> = <S as AgentStore>::agents_of(ctx.store(), person, None, most_agents)
        .await
        .map_err(failed)?
        .into_iter()
        .map(|agent| agent.id)
        .collect();
    let mut works = ctx
        .store()
        .current_work_of_agents(
            agents.clone(),
            person,
            ctx.now(),
            MOST_SHOWN_IN_MY_AGENTS as u64 + 1,
        )
        .await
        .map_err(failed)?;
    let truncated = works.len() > MOST_SHOWN_IN_MY_AGENTS;
    works.truncate(MOST_SHOWN_IN_MY_AGENTS);
    let items = ctx.my_items(&actor, &works).await?;
    let held = ctx.evaluate(person, Place::Organization).await?;
    let mut found_rules = ctx
        .store()
        .open_rules_of_agents(agents, MOST_SHOWN_IN_MY_AGENTS as u64 + 1)
        .await
        .map_err(failed)?;
    let rules_truncated = found_rules.len() > MOST_SHOWN_IN_MY_AGENTS;
    found_rules.truncate(MOST_SHOWN_IN_MY_AGENTS);
    let rules = found_rules.iter().map(view_of).collect();
    let mut offered_agents = ctx
        .store()
        .agents_offered_to(person, most_agents + 1)
        .await
        .map_err(failed)?;
    let offered_truncated = offered_agents.len() > MOST_AGENTS;
    offered_agents.truncate(MOST_AGENTS);
    let mut offered = Vec::new();
    for agent in offered_agents {
        offered.push(OfferedAgent {
            id: agent.id,
            name: agent.name.clone(),
            from: ctx
                .person_ref(
                    &held,
                    PeopleKind::ProjectMembers,
                    PeopleContext::Ordinary,
                    agent.person,
                )
                .await?,
        });
    }
    Ok(held.authorize(MyAgentWork {
        offered,
        offered_truncated,
        items,
        truncated,
        rules,
        rules_truncated,
    }))
}
