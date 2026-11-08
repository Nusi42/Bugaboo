//! The rules of agents after a run (architecture §11, "Ten people with agents", AI-30 to AI-38,
//! ADR-0029), written once and called by [`crate::judge`]:
//!
//! - One taker for each hand-over: of agents that took an issue at the same moment exactly one was
//!   told it had it, and no issue has two attempts running.
//! - No change by an agent beyond its person: every event of an agent names an agent of the book and
//!   the person responsible for it, on an issue that was handed to that agent, and nothing is kept
//!   after the person lost access.
//! - Every change has its event and its run: every event of an agent carries the run of its attempt.
//! - Every undone run is complete or named: each change of a run that was undone has its undo event
//!   on the issue, or the issue is named as skipped.

use std::collections::{BTreeMap, BTreeSet};

use bb_core::agent_work::AgentWorkState;
use bb_core::event::{Actor, DomainEvent};
use bb_core::id::IssueId;
use bb_core::run::undone_run;
use bb_store::Database;

use crate::book::Book;
use crate::facts::{Failure, MOST_EVENTS, describe, description_of};

/// An event an agent made.
#[derive(Debug)]
pub struct AgentEvent {
    /// The readable ID of the issue.
    pub key: String,
    /// The ID of the agent.
    pub agent: String,
    /// The ID of the person the event names as responsible.
    pub person: String,
    /// The run the event belongs to.
    pub run: Option<String>,
}

/// An event that is the undo of a run.
#[derive(Debug)]
pub struct UndoEvent {
    /// The readable ID of the issue.
    pub key: String,
    /// The run.
    pub run: String,
}

/// The attempts of an issue that have not ended.
#[derive(Debug)]
pub struct RunningFacts {
    /// The readable ID of the issue.
    pub key: String,
    /// How many attempts are running on it, of all agents together.
    pub attempts: usize,
}

/// The state of a piece of agent work on an issue of the book.
#[derive(Debug)]
pub struct WorkFacts {
    /// The readable ID of the issue.
    pub key: String,
    /// The ID of the agent.
    pub agent: String,
    /// Whether the work is stopped (`Stopped`).
    pub stopped: bool,
}

/// What agents did on the issues of the book.
#[derive(Debug, Default)]
pub struct AgentFacts {
    /// The work of agents on the issues of the book.
    pub work: Vec<WorkFacts>,
    /// Every event of an agent.
    pub events: Vec<AgentEvent>,
    /// Every event that is the undo of a run.
    pub undone: Vec<UndoEvent>,
    /// The attempts that are running now.
    pub running: Vec<RunningFacts>,
}

/// Reads what agents did on the issues of the book.
///
/// # Errors
/// Fails when the database cannot be read or the book names an ID that is none.
pub(crate) async fn gather(database: &Database, book: &Book) -> Result<AgentFacts, Failure> {
    let mut facts = AgentFacts::default();
    for work in &book.issues {
        let events = database
            .events_about(&description_of(&work.id)?.record, MOST_EVENTS)
            .await?;
        for event in &events {
            collect(&mut facts, &work.key, event);
        }
        let id: IssueId = work.id.parse().map_err(|e| describe(&work.id, e))?;
        let stored = database.stored_agent_work_on(id).await?;
        facts.work.extend(stored.iter().map(|(piece, _)| WorkFacts {
            key: work.key.clone(),
            agent: piece.agent.to_string(),
            stopped: piece.state == AgentWorkState::Stopped,
        }));
        let running = stored
            .iter()
            .flat_map(|(_, attempts)| attempts)
            .filter(|attempt| attempt.ended_at.is_none())
            .count();
        facts.running.push(RunningFacts {
            key: work.key.clone(),
            attempts: running,
        });
    }
    Ok(facts)
}

fn collect(facts: &mut AgentFacts, key: &str, event: &DomainEvent) {
    if let Actor::Agent { agent, person } = event.actor {
        facts.events.push(AgentEvent {
            key: key.to_owned(),
            agent: agent.to_string(),
            person: person.to_string(),
            run: event.run.map(|run| run.to_string()),
        });
    }
    if let Some(run) = event.reason.as_deref().and_then(undone_run) {
        facts.undone.push(UndoEvent {
            key: key.to_owned(),
            run: run.to_string(),
        });
    }
}

/// One taker for each hand-over.
fn one_taker(book: &Book, facts: &AgentFacts) -> Vec<String> {
    let mut found = Vec::new();
    for race in &book.races {
        if race.winners.len() != 1 {
            found.push(format!(
                "{} agents were told they took {} at the same moment ({} asked), not one",
                race.winners.len(),
                race.key,
                race.askers.len()
            ));
        }
    }
    for issue in facts.running.iter().filter(|issue| issue.attempts > 1) {
        found.push(format!(
            "{} has {} attempts running at once",
            issue.key, issue.attempts
        ));
    }
    found
}

/// Every event of an agent is within its person and its hand-overs, and carries its run.
fn nothing_beyond_the_person(book: &Book, facts: &AgentFacts) -> Vec<String> {
    let mut found = Vec::new();
    for event in &facts.events {
        let known = book.agents.iter().find(|agent| agent.agent == event.agent);
        match known {
            None => found.push(format!(
                "{}: an agent the book does not name made a change",
                event.key
            )),
            Some(agent) if agent.person != event.person => found.push(format!(
                "{}: the event of agent {} names another person than the one responsible",
                event.key, event.agent
            )),
            Some(_) => {}
        }
        if !book
            .handed
            .iter()
            .any(|hand| hand.agent == event.agent && hand.key == event.key)
        {
            found.push(format!(
                "{}: agent {} changed an issue that was not handed to it",
                event.key, event.agent
            ));
        }
        if event.run.is_none() {
            found.push(format!(
                "{}: a change of agent {} has no run",
                event.key, event.agent
            ));
        }
    }
    for lost in &book.lost {
        if lost.kept_after > 0 {
            found.push(format!(
                "agent {} kept {} changes after its person lost access",
                lost.agent, lost.kept_after
            ));
        }
        for piece in facts
            .work
            .iter()
            .filter(|piece| piece.agent == lost.agent && !piece.stopped)
        {
            found.push(format!(
                "{}: the work of agent {} is not stopped although its person lost access",
                piece.key, lost.agent
            ));
        }
    }
    found
}

/// Every run that was undone is complete on each issue it changed, or the issue is named.
fn undone_runs_are_complete(book: &Book, facts: &AgentFacts) -> Vec<String> {
    let undone: BTreeSet<&str> = facts.undone.iter().map(|undo| undo.run.as_str()).collect();
    let mut changed: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for event in &facts.events {
        if let Some(run) = event.run.as_deref().filter(|run| undone.contains(run)) {
            *changed.entry((run, event.key.as_str())).or_default() += 1;
        }
    }
    let mut found = Vec::new();
    for (run, key) in changed.keys() {
        let has_undo = facts
            .undone
            .iter()
            .any(|undo| undo.run == *run && undo.key == *key);
        if !has_undo && !book.skipped.iter().any(|skipped| skipped == key) {
            found.push(format!(
                "{key}: the run {run} was undone but its change here was neither undone nor named as skipped"
            ));
        }
    }
    found
}

/// Everything that must hold about agents after a run.
#[must_use]
pub fn agents_hold_the_rules(book: &Book, facts: &AgentFacts) -> Vec<String> {
    let mut found = one_taker(book, facts);
    found.extend(nothing_beyond_the_person(book, facts));
    found.extend(undone_runs_are_complete(book, facts));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::{Agented, HandedOver, Lost, Raced};

    fn book() -> Book {
        Book {
            agents: vec![Agented {
                agent: "a1".to_owned(),
                person: "p1".to_owned(),
            }],
            handed: vec![HandedOver {
                agent: "a1".to_owned(),
                key: "K-1".to_owned(),
            }],
            races: vec![Raced {
                key: "K-1".to_owned(),
                askers: vec!["a1".to_owned(), "a2".to_owned()],
                winners: vec!["a1".to_owned()],
            }],
            ..Book::default()
        }
    }

    fn event(run: Option<&str>) -> AgentEvent {
        AgentEvent {
            key: "K-1".to_owned(),
            agent: "a1".to_owned(),
            person: "p1".to_owned(),
            run: run.map(str::to_owned),
        }
    }

    #[test]
    fn test_concurrent_ai_36_a_good_run_of_agents_has_nothing_to_report() {
        let facts = AgentFacts {
            events: vec![event(Some("r1"))],
            ..AgentFacts::default()
        };
        assert_eq!(agents_hold_the_rules(&book(), &facts), Vec::<String>::new());
    }

    #[test]
    fn test_concurrent_ai_36_two_winners_or_two_running_attempts_are_found() {
        let mut two = book();
        two.races[0].winners.push("a2".to_owned());
        let facts = AgentFacts {
            running: vec![RunningFacts {
                key: "K-1".to_owned(),
                attempts: 2,
            }],
            ..AgentFacts::default()
        };
        let found = agents_hold_the_rules(&two, &facts);
        assert_eq!(found.len(), 2, "{found:?}");
    }

    #[test]
    fn test_concurrent_ai_30_a_change_beyond_the_person_or_without_a_run_is_found() {
        let mut other = event(None);
        other.person = "p2".to_owned();
        let mut not_handed = event(Some("r1"));
        not_handed.key = "K-2".to_owned();
        let facts = AgentFacts {
            events: vec![other, not_handed],
            ..AgentFacts::default()
        };
        let found = agents_hold_the_rules(&book(), &facts);
        assert!(found.iter().any(|line| line.contains("another person")));
        assert!(found.iter().any(|line| line.contains("has no run")));
        assert!(found.iter().any(|line| line.contains("not handed")));
        let mut lost = book();
        lost.lost.push(Lost {
            agent: "a1".to_owned(),
            person: "p1".to_owned(),
            kept_after: 1,
        });
        let found = agents_hold_the_rules(&lost, &AgentFacts::default());
        assert!(found.iter().any(|line| line.contains("lost access")));
        let running = AgentFacts {
            work: vec![WorkFacts {
                key: "K-1".to_owned(),
                agent: "a1".to_owned(),
                stopped: false,
            }],
            ..AgentFacts::default()
        };
        let mut still = book();
        still.lost.push(Lost {
            agent: "a1".to_owned(),
            person: "p1".to_owned(),
            kept_after: 0,
        });
        let found = agents_hold_the_rules(&still, &running);
        assert!(
            found.iter().any(|line| line.contains("not stopped")),
            "{found:?}"
        );
    }

    #[test]
    fn test_concurrent_ai_38_an_undone_run_is_complete_or_names_what_it_skipped() {
        let facts = AgentFacts {
            events: vec![event(Some("r1"))],
            undone: vec![UndoEvent {
                key: "K-2".to_owned(),
                run: "r1".to_owned(),
            }],
            ..AgentFacts::default()
        };
        let found = agents_hold_the_rules(&book(), &facts);
        assert!(found.iter().any(|line| line.contains("neither undone")));
        let mut named = book();
        named.skipped.push("K-1".to_owned());
        assert!(agents_hold_the_rules(&named, &facts).is_empty());
    }
}
