//! *Changes since* (AI-36, ADR-0029, decision 9): what changed in the issues an agent may see since a
//! mark it got before, with a new mark, a page at a time. It is how an agent that works under a
//! standing rule learns of its matches and how any agent learns of the changes of people while it
//! works.
//!
//! An agent sees only what its work and its rules let it see, and on those issues only the fields
//! its person may see: the same reader as the history (`Ctx::changes_as_read`) decides, so a change
//! to a field the agent may not read is not in the answer, and an issue it may not see is not either.

use std::collections::HashMap;

use bb_core::ErrorCode;
use bb_core::agent_work::AgentWorkState;
use bb_core::ai::AuthorKind;
use bb_core::event::{Actor as EventActor, DomainEvent};
use bb_core::id::{ProjectId, StandingRuleId, UserId};
use bb_core::issue::Issue;
use bb_core::paging::page_len;
use bb_policy::{Authorized, Evaluation, Masked, Place};
use jiff::Timestamp;
use serde::Serialize;

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::history::FieldChangeView;
use crate::relations::{IssueLine, LineMaker};

/// How many events one look at the log reads.
const SCAN: u64 = 200;
/// How many looks one request makes at the most: it never reads the whole log for one page.
const SCANS: usize = 4;

/// Ask what changed in the issues the agent may see since a mark.
pub struct ChangesSinceQuery {
    /// The secret of the key of the agent.
    pub session_token: String,
    /// The mark the agent got last time. None for the first look: nothing is told, and the mark of
    /// now is given.
    pub since: Option<String>,
    /// How many changes a page holds at most.
    pub limit: Option<u32>,
}

/// A mark: where the agent stopped reading the log of changes, and when. The text is for the agent to
/// give back and nothing else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mark {
    place: i64,
    at: Timestamp,
}

impl Mark {
    fn write(self) -> String {
        format!("{}.{}", self.place, self.at.as_microsecond())
    }

    fn read(text: &str) -> Option<Self> {
        let (place, at) = text.split_once('.')?;
        Some(Self {
            place: place.parse().ok().filter(|place| *place >= 0)?,
            at: Timestamp::from_microsecond(at.parse().ok()?).ok()?,
        })
    }
}

/// One change of an issue the agent may see.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChangeSeen {
    /// The readable ID of the issue.
    pub key: String,
    /// The project the issue is in.
    pub project: ProjectId,
    /// When it was made.
    pub at: Timestamp,
    /// Who wrote what is new in it, as the AI is told.
    pub author: AuthorKind,
    /// What changed, leaving out what the agent may not see.
    pub changes: Vec<FieldChangeView>,
}

/// An issue that came under a standing rule of the agent since the mark.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MatchSeen {
    /// The rule.
    pub rule: StandingRuleId,
    /// The readable ID of the issue.
    pub key: String,
    /// The project the issue is in.
    pub project: ProjectId,
    /// When it came under the rule.
    pub at: Timestamp,
}

/// A piece of work of the agent that is not at its end, with where it stands: how an agent that
/// asked a question learns that it was answered.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorkSeen {
    /// The readable ID of the issue.
    pub key: String,
    /// The project the issue is in.
    pub project: ProjectId,
    /// Where the work stands.
    pub work_state: AgentWorkState,
    /// The version of the work.
    pub version: i64,
}

/// What changed since a mark.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChangesPage {
    /// The changes, oldest first.
    pub changes: Vec<ChangeSeen>,
    /// The issues that came under the rules of the agent.
    pub matches: Vec<MatchSeen>,
    /// The work of the agent that is not at its end.
    pub work: Vec<WorkSeen>,
    /// The mark to give back next time.
    pub mark: String,
    /// Whether there is more to read right away: the page was full, or the log is longer than one
    /// request reads.
    pub more: bool,
}

impl Masked for ChangesPage {}

/// The issues one request has looked at, each once.
struct Seen {
    issues: HashMap<bb_core::id::IssueId, Option<(Issue, std::sync::Arc<Evaluation>, String)>>,
    authors: HashMap<(UserId, ProjectId), AuthorKind>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// An issue the actor may see, with its readable ID, looked up once for the request.
    async fn seen_once<'a>(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        seen: &'a mut Seen,
        id: bb_core::id::IssueId,
    ) -> Result<Option<&'a (Issue, std::sync::Arc<Evaluation>, String)>, AppError> {
        if seen.issues.contains_key(&id) {
            return Ok(seen.issues.get(&id).and_then(Option::as_ref));
        }
        let found = match self.seen_issue_by_id(actor, id).await? {
            Some((issue, held)) => {
                let line: Option<IssueLine> = self.line_of(actor, maker, &issue).await?;
                line.map(|line| (issue, held, line.key))
            }
            None => None,
        };
        Ok(seen.issues.entry(id).or_insert(found).as_ref())
    }

    /// Who wrote what is new in an event, as the AI is told.
    async fn author_of_event(
        &self,
        seen: &mut Seen,
        event: &DomainEvent,
        project: ProjectId,
    ) -> Result<AuthorKind, AppError> {
        match event.actor {
            EventActor::Agent { .. } => Ok(AuthorKind::Agent),
            EventActor::Person(person) => {
                if let Some(kind) = seen.authors.get(&(person, project)) {
                    return Ok(*kind);
                }
                let kind = self.author_kind(person, project).await?;
                seen.authors.insert((person, project), kind);
                Ok(kind)
            }
            // Bugaboo itself wrote no text of its own; the least trusted kind stands for it.
            EventActor::System => Ok(AuthorKind::Outsider),
        }
    }

    /// The work of the agent that is not at its end, on the issues it may see.
    async fn open_work_seen(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        seen: &mut Seen,
        agent: bb_core::id::AgentId,
    ) -> Result<Vec<WorkSeen>, AppError> {
        let works = self
            .store()
            .work_of_agent(agent, None, 100)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut found = Vec::new();
        for work in works
            .iter()
            .filter(|work| work.state.is_open() && !work.closed)
        {
            if let Some((issue, _, key)) = self.seen_once(actor, maker, seen, work.issue).await? {
                found.push(WorkSeen {
                    key: key.clone(),
                    project: issue.project,
                    work_state: work.state,
                    version: work.version,
                });
            }
        }
        Ok(found)
    }

    /// The issues that came under an open rule of the agent after `since`, the ones the agent may
    /// see.
    async fn matches_since(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        seen: &mut Seen,
        agent: bb_core::id::AgentId,
        since: Timestamp,
    ) -> Result<Vec<MatchSeen>, AppError> {
        let failed = |error| internal::<S>(&error);
        let rules = self
            .store()
            .rules_of_agent(agent, None, 100)
            .await
            .map_err(failed)?;
        let mut found = Vec::new();
        for rule in rules
            .iter()
            .filter(|rule| rule.state == bb_core::agent_records::GivingState::Open)
        {
            let matches = self
                .store()
                .matches_of(rule.id, None, 200)
                .await
                .map_err(failed)?;
            for each in matches.iter().filter(|each| each.at > since) {
                if let Some((issue, _, key)) =
                    self.seen_once(actor, maker, seen, each.issue).await?
                {
                    found.push(MatchSeen {
                        rule: rule.id,
                        key: key.clone(),
                        project: issue.project,
                        at: each.at,
                    });
                }
            }
        }
        found.sort_by_key(|each| each.at);
        Ok(found)
    }
}

/// What changed in the issues the agent may see since a mark, a page at a time.
///
/// # Errors
/// `access.forbidden` for a request that does not come with the key of an agent,
/// `request.invalid` for a mark that is not one.
pub async fn changes_since<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangesSinceQuery,
) -> Result<Authorized<ChangesPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let Some(agent) = actor.agent() else {
        return Err(ErrorCode::Forbidden.into());
    };
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let failed = |error| internal::<S>(&error);
    let Some(text) = input.since.as_deref() else {
        // The first look tells nothing and gives the mark of now.
        let place = ctx.store().last_event_place().await.map_err(failed)?;
        let mark = Mark {
            place,
            at: ctx.now(),
        };
        return Ok(held.authorize(ChangesPage {
            changes: Vec::new(),
            matches: Vec::new(),
            work: Vec::new(),
            mark: mark.write(),
            more: false,
        }));
    };
    let since = Mark::read(text).ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    let wanted = page_len(input.limit);
    let mut reader = ctx.change_reader().await?;
    let mut maker = ctx.line_maker().await?;
    let mut seen = Seen {
        issues: HashMap::new(),
        authors: HashMap::new(),
    };
    let mut changes = Vec::new();
    let mut place = since.place;
    let mut full_page = false;
    let mut log_goes_on = false;
    'scan: for _ in 0..SCANS {
        let events = ctx
            .store()
            .issue_events_after(place, SCAN)
            .await
            .map_err(failed)?;
        let full = events.len() as u64 == SCAN;
        for (at, event) in events {
            let Ok(id) = bb_core::id::IssueId::from_bytes(event.entity.id.to_bytes()) else {
                place = at;
                continue;
            };
            let found = ctx
                .seen_once(&actor, &mut maker, &mut seen, id)
                .await?
                .cloned();
            if let Some((issue, in_project, key)) = found {
                let shown = ctx
                    .changes_as_read(&mut reader, &actor, &in_project, &issue, &event.changes)
                    .await?;
                if !shown.is_empty() {
                    if changes.len() == wanted {
                        // The page is full: this event is the first of the next one.
                        full_page = true;
                        break 'scan;
                    }
                    let author = ctx
                        .author_of_event(&mut seen, &event, issue.project)
                        .await?;
                    changes.push(ChangeSeen {
                        key,
                        project: issue.project,
                        at: event.time,
                        author,
                        changes: shown,
                    });
                }
            }
            place = at;
        }
        log_goes_on = full;
        if !full {
            break;
        }
    }
    let matches = ctx
        .matches_since(&actor, &mut maker, &mut seen, agent, since.at)
        .await?;
    let work = ctx
        .open_work_seen(&actor, &mut maker, &mut seen, agent)
        .await?;
    let mark = Mark {
        place,
        at: ctx.now(),
    };
    Ok(held.authorize(ChangesPage {
        changes,
        matches,
        work,
        mark: mark.write(),
        more: full_page || log_goes_on,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_36_a_mark_is_read_back_as_it_was_written_and_nothing_else_is_a_mark() {
        let mark = Mark {
            place: 42,
            at: Timestamp::from_second(1_800_000_000).unwrap(),
        };
        assert_eq!(Mark::read(&mark.write()), Some(mark));
        for text in ["", "x", "1", "-1.5", "1.x", "1.99999999999999999999999"] {
            assert_eq!(Mark::read(text), None, "{text}");
        }
    }
}
