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
use bb_core::id::{IssueId, ProjectId, StandingRuleId, UserId};
use bb_core::issue::Issue;
use bb_core::paging::{Cut, page_len, read_len};
use bb_policy::{Authorized, Evaluation, Masked, Place};
use jiff::Timestamp;
use serde::Serialize;

use crate::access::Actor;
use crate::agents::Hold;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::history::FieldChangeView;
use crate::issues::Where;
use crate::relations::{IssueLine, LineMaker};

/// How many events one look at the log reads.
const SCAN: u64 = 200;
/// How many looks one request makes at the most: it never reads the whole log for one page.
const SCANS: usize = 2;
/// How many pieces of open work one answer lists at the most.
const MOST_WORK_SEEN: u64 = 100;

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
    /// The issue and rule of the last match given at that very moment, when the matches go on after it.
    after: Option<(IssueId, StandingRuleId)>,
}

impl Mark {
    fn write(self) -> String {
        let mark = format!("{}.{}", self.place, self.at.as_microsecond());
        match self.after {
            Some((issue, rule)) => format!("{mark}.{issue}.{rule}"),
            None => mark,
        }
    }

    fn read(text: &str) -> Option<Self> {
        let mut parts = text.splitn(4, '.');
        let (place, at) = (parts.next()?, parts.next()?);
        let after = (parts.next(), parts.next());
        Some(Self {
            place: place.parse().ok().filter(|place| *place >= 0)?,
            at: Timestamp::from_microsecond(at.parse().ok()?).ok()?,
            after: match after {
                (Some(issue), Some(rule)) => Some((issue.parse().ok()?, rule.parse().ok()?)),
                (None, None) => None,
                _ => return None,
            },
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

/// The people a value of a change names: a person, or a list of them.
fn people_named(value: &serde_json::Value) -> Vec<UserId> {
    match value {
        serde_json::Value::String(text) => text.parse().into_iter().collect(),
        serde_json::Value::Array(items) => items.iter().flat_map(people_named).collect(),
        _ => Vec::new(),
    }
}

/// The issues one request has looked at, each once.
struct Seen {
    issues: HashMap<IssueId, Option<(Issue, std::sync::Arc<Evaluation>, String)>>,
    authors: HashMap<(UserId, ProjectId), AuthorKind>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// An issue the actor may see, with its readable ID, looked up once for the request.
    async fn seen_once<'a>(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        seen: &'a mut Seen,
        id: IssueId,
    ) -> Result<Option<&'a (Issue, std::sync::Arc<Evaluation>, String)>, AppError> {
        if seen.issues.contains_key(&id) {
            return Ok(seen.issues.get(&id).and_then(Option::as_ref));
        }
        let found = match self.seen_issue_by_id(actor, id).await? {
            Some((issue, held)) => self.line_key_of(actor, maker, issue, held).await?,
            None => None,
        };
        Ok(seen.issues.entry(id).or_insert(found).as_ref())
    }

    /// The same for an issue that is read already (the issues of a page of work are read together).
    async fn seen_loaded<'a>(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        seen: &'a mut Seen,
        issue: Issue,
    ) -> Result<Option<&'a (Issue, std::sync::Arc<Evaluation>, String)>, AppError> {
        let id = issue.id;
        if seen.issues.contains_key(&id) {
            return Ok(seen.issues.get(&id).and_then(Option::as_ref));
        }
        let found = match self.seen_in(actor, &issue, Where::Live).await? {
            Some(held) => self.line_key_of(actor, maker, issue, held).await?,
            None => None,
        };
        Ok(seen.issues.entry(id).or_insert(found).as_ref())
    }

    /// The issue with the rights it is seen with and its readable ID, when its line can be made.
    async fn line_key_of(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        issue: Issue,
        held: std::sync::Arc<Evaluation>,
    ) -> Result<Option<(Issue, std::sync::Arc<Evaluation>, String)>, AppError> {
        let line: Option<IssueLine> = self.line_of(actor, maker, &issue).await?;
        Ok(line.map(|line| (issue, held, line.key)))
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

    /// Gets everything the lines of these issues need in a few reads, whatever the number of
    /// projects they are in: the projects (for every maker), and what the person may do in each
    /// (PERF-N1). Nothing is read for one project.
    async fn prepare_lines(
        &self,
        actor: &Actor,
        makers: &mut [&mut LineMaker],
        projects: &[ProjectId],
    ) -> Result<(), AppError> {
        let distinct: std::collections::BTreeSet<ProjectId> = projects.iter().copied().collect();
        if distinct.len() > 1 {
            let all = self
                .store()
                .project_tree()
                .await
                .map_err(|error| internal::<S>(&error))?;
            for maker in makers.iter_mut() {
                maker.know_stored(projects, &all);
            }
            self.preload_projects(projects).await?;
        }
        self.evaluate_projects(actor.person.id, projects).await?;
        Ok(())
    }

    /// The issues with these IDs that the request has not looked at yet, read together and looked at
    /// each once: what the agent holds on them, the issues, their projects and the rights in them
    /// are one read each. An issue that is not there is not seen.
    async fn seen_together(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        seen: &mut Seen,
        agent: bb_core::id::AgentId,
        ids: &[IssueId],
    ) -> Result<(), AppError> {
        let fresh: Vec<IssueId> = ids
            .iter()
            .copied()
            .filter(|id| !seen.issues.contains_key(id))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if fresh.is_empty() {
            return Ok(());
        }
        self.holds_of_agent_on(agent, &fresh).await?;
        let loaded = self.issues_named(&fresh).await?;
        let projects: Vec<ProjectId> = loaded.iter().map(|issue| issue.project).collect();
        self.prepare_lines(actor, &mut [&mut *maker], &projects)
            .await?;
        for issue in loaded {
            self.seen_loaded(actor, maker, seen, issue).await?;
        }
        for id in fresh {
            seen.issues.entry(id).or_insert(None);
        }
        Ok(())
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
            .current_work_of_agent(agent, self.now(), MOST_WORK_SEEN)
            .await
            .map_err(|error| internal::<S>(&error))?;
        // The issues are read together, and what the agent holds on them in one more read.
        let ids: Vec<IssueId> = works.iter().map(|work| work.issue).collect();
        self.holds_of_agent_on(agent, &ids).await?;
        let mut issues = self.issues_named(&ids).await?;
        let projects: Vec<ProjectId> = issues.iter().map(|issue| issue.project).collect();
        self.prepare_lines(actor, &mut [&mut *maker], &projects)
            .await?;
        let mut found = Vec::new();
        for work in &works {
            let Some(at) = issues.iter().position(|issue| issue.id == work.issue) else {
                continue;
            };
            let issue = issues.swap_remove(at);
            if let Some((issue, _, key)) = self.seen_loaded(actor, maker, seen, issue).await? {
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

    /// The issues that came under an open rule of the agent after `since`, oldest first, one page: the
    /// ones the agent may see, and the moment to go on from when there are more (the match after the
    /// last one of the page). One statement, whatever the number of rules and matches.
    async fn matches_since(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        seen: &mut Seen,
        agent: bb_core::id::AgentId,
        since: (Timestamp, Option<(IssueId, StandingRuleId)>),
        wanted: usize,
    ) -> Result<(Vec<MatchSeen>, Option<(Timestamp, IssueId, StandingRuleId)>), AppError> {
        let rows = self
            .store()
            .matches_of_agent_since(agent, self.now(), since, read_len(wanted))
            .await
            .map_err(|error| internal::<S>(&error))?;
        let cut = Cut::of(rows, wanted);
        let resume = cut
            .more
            .then(|| {
                cut.items
                    .last()
                    .map(|each| (each.at, each.issue, each.rule))
            })
            .flatten();
        let named: Vec<IssueId> = cut.items.iter().map(|each| each.issue).collect();
        self.seen_together(actor, maker, seen, agent, &named)
            .await?;
        let mut found = Vec::new();
        for each in &cut.items {
            if let Some((issue, _, key)) = self.seen_once(actor, maker, seen, each.issue).await? {
                found.push(MatchSeen {
                    rule: each.rule,
                    key: key.clone(),
                    project: issue.project,
                    at: each.at,
                });
            }
        }
        Ok((found, resume))
    }
}

/// What one look at the log of changes found.
struct Scanned {
    changes: Vec<ChangeSeen>,
    /// The place of the last event given up or taken, where the next look goes on.
    place: i64,
    /// Whether the page was full before the log was read to its end.
    full_page: bool,
    /// Whether the log goes on after what one request reads.
    log_goes_on: bool,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The issues of the events that the agent reaches and the request has not looked at yet, read
    /// together, with the rights of the person and of the authors in their projects and the
    /// projects themselves: one read each, however many projects the issues are in (PERF-N1).
    async fn read_reached(
        &self,
        actor: &Actor,
        (maker, reader_maker): (&mut LineMaker, &mut LineMaker),
        seen: &Seen,
        events: &[(i64, DomainEvent)],
        named: &[IssueId],
        reach: &std::collections::BTreeMap<IssueId, Hold>,
    ) -> Result<HashMap<IssueId, Issue>, AppError> {
        let fresh: Vec<IssueId> = named
            .iter()
            .copied()
            .filter(|id| reach.contains_key(id) && !seen.issues.contains_key(id))
            .collect();
        let loaded: HashMap<IssueId, Issue> = self
            .issues_named(&fresh)
            .await?
            .into_iter()
            .map(|issue| (issue.id, issue))
            .collect();
        let projects: Vec<ProjectId> = loaded.values().map(|issue| issue.project).collect();
        // The projects of the lines, of this request and of the history it reads, in one read.
        self.prepare_lines(actor, &mut [maker, reader_maker], &projects)
            .await?;
        let mut authors: std::collections::BTreeSet<(UserId, ProjectId)> =
            std::collections::BTreeSet::new();
        for (_, event) in events {
            if let EventActor::Person(person) = event.actor
                && let Ok(id) = IssueId::from_bytes(event.entity.id.to_bytes())
                && let Some(issue) = loaded.get(&id)
            {
                authors.insert((person, issue.project));
            }
        }
        // The authors: who they are and what they may do, each read together with the others.
        let authors: Vec<(UserId, ProjectId)> = authors.into_iter().collect();
        // Besides the authors, the people the changes name (assigned people, reporters).
        let mut people: Vec<UserId> = authors.iter().map(|(person, _)| *person).collect();
        for (_, event) in events {
            if IssueId::from_bytes(event.entity.id.to_bytes())
                .is_ok_and(|id| loaded.contains_key(&id))
            {
                for change in &event.changes {
                    for value in [&change.before, &change.after].into_iter().flatten() {
                        people.extend(people_named(value));
                    }
                }
            }
        }
        self.preload_people(&people).await?;
        self.evaluate_people_in(&authors).await?;
        Ok(loaded)
    }

    /// Reads the log of changes of issues after `from`, for the issues the agent reaches, one page.
    async fn scan_changes(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        seen: &mut Seen,
        agent: bb_core::id::AgentId,
        from: i64,
        wanted: usize,
    ) -> Result<Scanned, AppError> {
        let mut reader = self.change_reader().await?;
        let mut scanned = Scanned {
            changes: Vec::new(),
            place: from,
            full_page: false,
            log_goes_on: false,
        };
        'scan: for _ in 0..SCANS {
            let events = self
                .store()
                .issue_events_after(scanned.place, SCAN)
                .await
                .map_err(|error| internal::<S>(&error))?;
            let full = events.len() as u64 == SCAN;
            // What the agent holds on the issues of the events, read together: one look at the log
            // asks once, not once for each event.
            let named: Vec<IssueId> = events
                .iter()
                .filter_map(|(_, event)| IssueId::from_bytes(event.entity.id.to_bytes()).ok())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let reach = self.holds_of_agent_on(agent, &named).await?;
            let mut loaded = self
                .read_reached(
                    actor,
                    (&mut *maker, reader.maker_mut()),
                    seen,
                    &events,
                    &named,
                    &reach,
                )
                .await?;
            for (at, event) in events {
                let reached = IssueId::from_bytes(event.entity.id.to_bytes())
                    .ok()
                    .filter(|id| reach.contains_key(id));
                // The issues the work and the rules of the agent reach are looked at first, so an
                // event of any other issue costs nothing.
                if let Some(id) = reached
                    && let Some((issue, in_project, key)) = match loaded.remove(&id) {
                        Some(issue) => self.seen_loaded(actor, maker, seen, issue).await?,
                        None => self.seen_once(actor, maker, seen, id).await?,
                    }
                    .cloned()
                {
                    let shown = self
                        .changes_as_read(&mut reader, actor, &in_project, &issue, &event.changes)
                        .await?;
                    if !shown.is_empty() {
                        if scanned.changes.len() == wanted {
                            // The page is full: this event is the first of the next one.
                            scanned.full_page = true;
                            break 'scan;
                        }
                        let author = self.author_of_event(seen, &event, issue.project).await?;
                        scanned.changes.push(ChangeSeen {
                            key,
                            project: issue.project,
                            at: event.time,
                            author,
                            changes: shown,
                        });
                    }
                }
                scanned.place = at;
            }
            scanned.log_goes_on = full;
            if !full {
                break;
            }
        }
        Ok(scanned)
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
            after: None,
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
    let mut maker = ctx.line_maker().await?;
    let mut seen = Seen {
        issues: HashMap::new(),
        authors: HashMap::new(),
    };
    let scanned = ctx
        .scan_changes(&actor, &mut maker, &mut seen, agent, since.place, wanted)
        .await?;
    let (matches, resume) = ctx
        .matches_since(
            &actor,
            &mut maker,
            &mut seen,
            agent,
            (since.at, since.after),
            wanted,
        )
        .await?;
    let work = ctx
        .open_work_seen(&actor, &mut maker, &mut seen, agent)
        .await?;
    // More matches than a page: the next look goes on after the last one it was given.
    let mark = Mark {
        place: scanned.place,
        at: resume.map_or_else(|| ctx.now(), |(at, _, _)| at),
        after: resume.map(|(_, issue, rule)| (issue, rule)),
    };
    Ok(held.authorize(ChangesPage {
        changes: scanned.changes,
        matches,
        work,
        mark: mark.write(),
        more: scanned.full_page || scanned.log_goes_on || resume.is_some(),
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
            after: None,
        };
        assert_eq!(Mark::read(&mark.write()), Some(mark));
        for text in ["", "x", "1", "-1.5", "1.x", "1.99999999999999999999999"] {
            assert_eq!(Mark::read(text), None, "{text}");
        }
    }
}
