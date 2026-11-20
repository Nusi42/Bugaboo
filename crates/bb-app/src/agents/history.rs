//! The history of an agent (AI-37, ADR-0029, amendment M9): what it handed over to its person's
//! other agents and which of its runs it undid, newest first, a page at a time. Only its person
//! reads it, on the agent's page: the runs are theirs, and how many changes an undo put back counts
//! what they may see. An administrator, who sees every agent, and the agent itself do not.

use std::cmp::Reverse;
use std::collections::HashMap;

use bb_core::id::{AgentId, EntityId, IssueId};
use bb_core::paging::{Cut, page_len, read_len, read_time_cursor, write_time_cursor};
use bb_policy::{Authorized, Masked, Place};
use jiff::Timestamp;
use serde::Serialize;

use super::store::{HandOverByAgent, UndoByAgent};
use super::{AgentNames, AgentStore};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::history::AgentLabel;
use crate::input::cursor_or_invalid;
use crate::relations::IssueLine;

/// The history of one agent, a page at a time.
pub struct AgentHistoryQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The agent.
    pub agent: AgentId,
    /// Where the page after the last one starts.
    pub cursor: Option<String>,
    /// How many entries a page holds at most.
    pub limit: u32,
}

/// What an agent did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum AgentDeed {
    /// It handed issues of its work over to another agent of its person.
    HandedOver {
        /// The agent it handed them to; none when the reader may not be told that agent.
        to: Option<AgentLabel>,
        /// The issues, those the reader may see.
        issues: Vec<IssueLine>,
    },
    /// It undid the run of one of its attempts.
    Undid {
        /// The issue of the attempt, when the reader may see it.
        issue: Option<IssueLine>,
        /// How many changes went back.
        went_back: u64,
    },
}

/// One entry of the history of an agent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AgentHistoryItem {
    /// What the entry is known by: the hand-over, or the event of the undo.
    pub id: EntityId,
    /// When it happened.
    pub at: Timestamp,
    /// What the agent did.
    pub deed: AgentDeed,
}

/// A page of the history of an agent, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AgentHistoryPage {
    /// The entries.
    pub items: Vec<AgentHistoryItem>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

// The issues are lines, each dropped by its own project; the agent is named by `AgentNames`.
impl Masked for AgentHistoryPage {}

/// An entry as the store found it, before it is shaped for the reader.
enum Found {
    HandOver(HandOverByAgent),
    Undo(UndoByAgent),
}

impl Found {
    fn at(&self) -> Timestamp {
        match self {
            Self::HandOver(hand_over) => hand_over.at,
            Self::Undo(undo) => undo.event.time,
        }
    }

    /// What the entry is known by in the one order of both kinds.
    fn key(&self) -> EntityId {
        match self {
            Self::HandOver(hand_over) => hand_over.id.into(),
            Self::Undo(undo) => undo.event.id.into(),
        }
    }

    fn issues(&self) -> Vec<IssueId> {
        match self {
            Self::HandOver(hand_over) => hand_over.issues.clone(),
            Self::Undo(undo) => undo.issue.into_iter().collect(),
        }
    }
}

/// The history of an agent: its hand-overs and its undos while it worked for its person, newest
/// first. An issue the reader may not see is left out of its entry.
///
/// # Errors
/// `auth.not-signed-in` without a session, `resource.not-found` for an agent that is not the
/// person's, `request.invalid` for a cursor that is not one.
pub async fn agent_history<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AgentHistoryQuery,
) -> Result<Authorized<AgentHistoryPage>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let agent = ctx
        .agent_to_handle(actor.person.id, false, input.agent)
        .await?;
    let len = page_len(Some(input.limit));
    let after = cursor_or_invalid(input.cursor.as_deref(), read_time_cursor::<EntityId>)?;
    let failed = |error| internal::<S>(&error);
    let store = ctx.store();
    let hand_overs = store
        .hand_overs_by_agent(agent.id, agent.person, after, read_len(len))
        .await
        .map_err(failed)?;
    let undos =
        <S as AgentStore>::undos_by_agent(store, agent.id, agent.person, after, read_len(len))
            .await
            .map_err(failed)?;
    // Each kind comes newest first and then by ID; both together in the same order.
    let mut found: Vec<Found> = hand_overs
        .into_iter()
        .map(Found::HandOver)
        .chain(undos.into_iter().map(Found::Undo))
        .collect();
    found.sort_by_key(|each| (Reverse(each.at()), each.key().to_bytes()));
    let page = Cut::of(found, len);
    let next_cursor = page.next_cursor(|last| write_time_cursor(last.at(), last.key()));

    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let ids: Vec<IssueId> = page.items.iter().flat_map(Found::issues).collect();
    let mut maker = ctx.line_maker().await?;
    let mut lines: HashMap<IssueId, IssueLine> = HashMap::new();
    for issue in ctx.issues_named(&ids).await? {
        if let Some(line) = ctx.line_of(&actor, &mut maker, &issue).await? {
            lines.insert(issue.id, line);
        }
    }
    let mut names = AgentNames::default();
    let mut items = Vec::with_capacity(page.items.len());
    for each in page.items {
        let (at, id) = (each.at(), each.key());
        let deed = match each {
            Found::HandOver(hand_over) => AgentDeed::HandedOver {
                to: ctx
                    .agent_named_to(&mut names, &held, hand_over.to)
                    .await?
                    .map(|name| AgentLabel {
                        id: hand_over.to,
                        name,
                    }),
                issues: hand_over
                    .issues
                    .iter()
                    .filter_map(|issue| lines.get(issue).cloned())
                    .collect(),
            },
            Found::Undo(undo) => AgentDeed::Undid {
                issue: undo.issue.and_then(|issue| lines.get(&issue).cloned()),
                went_back: crate::runs::went_back(&undo.event).unwrap_or_default(),
            },
        };
        items.push(AgentHistoryItem { id, at, deed });
    }
    Ok(held.authorize(AgentHistoryPage { items, next_cursor }))
}
