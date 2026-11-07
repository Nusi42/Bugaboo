//! Runs (AI-38, ADR-0029 decision 7): everything one session of an AI app, one attempt of an agent
//! or one change to many issues changed, as a list of changes that can be undone in one step.
//!
//! The words and the shapes the port `RunStore` of `bb-app` speaks in. A run is a row of
//! `change_run`; a run of a change to many issues keeps its own start (`ManyChangeRun`).

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::event::{Actor, EntityRef, FieldChange};
use crate::id::{AgentId, EventId, UserId};
use crate::many_changes::ManyChangeStatus;

words! {
    /// What a run is of.
    pub enum RunKind {
        /// A change to many issues (ISS-09). It starts with its own use case, not through the port.
        ManyChanges => "many-changes",
        /// One session of an AI app acting for a person (ADR-0042).
        AppSession => "app-session",
        /// One attempt of an agent at a piece of work (ADR-0029).
        Attempt => "attempt",
    }
}

words! {
    /// How a run that was open ended.
    pub enum RunEnd {
        /// The session or the attempt came to its end.
        Done => "done",
        /// It stopped because something went wrong.
        Failed => "failed",
        /// Somebody stopped it, or the app or agent was cut off.
        Stopped => "stopped",
    }
}

impl RunEnd {
    /// The state a run that ended this way is in.
    #[must_use]
    pub const fn status(self) -> ManyChangeStatus {
        match self {
            Self::Done => ManyChangeStatus::Done,
            Self::Failed => ManyChangeStatus::Failed,
            Self::Stopped => ManyChangeStatus::Stopped,
        }
    }
}

/// Who a run belongs to: the person who answers for it, with the app or the agent that acted for
/// them. Never both.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunActor {
    /// The person responsible.
    pub person: UserId,
    /// The name of the AI app, for a session of an app.
    pub app: Option<String>,
    /// The agent, for an attempt.
    pub agent: Option<AgentId>,
}

impl RunActor {
    /// The actor an event written inside the run names (the person; the agent when there is one).
    #[must_use]
    pub const fn event_actor(&self) -> Actor {
        match self.agent {
            Some(agent) => Actor::Agent {
                agent,
                person: self.person,
            },
            None => Actor::Person(self.person),
        }
    }
}

/// One run, as the list of runs and the run page show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSummary {
    /// The ID of the run.
    pub id: crate::id::RunId,
    /// What it is of.
    pub kind: RunKind,
    /// Whose it is.
    pub actor: RunActor,
    /// Where it is: open runs are `Running`.
    pub status: ManyChangeStatus,
    /// When it began.
    pub started_at: Timestamp,
    /// When it ended.
    pub ended_at: Option<Timestamp>,
}

/// One change of a run: an event that carries it, with the record and the fields it changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunChange {
    /// The event.
    pub event: EventId,
    /// When it was made.
    pub at: Timestamp,
    /// The record it is about.
    pub entity: EntityRef,
    /// The fields that changed, with their values before and after.
    pub changes: Vec<FieldChange>,
}

words! {
    /// Why a change of a run does not go back.
    pub enum UndoSkipKind {
        /// Somebody changed the same field afterwards. The change is named with who and when.
        ChangedSince => "changed-since",
        /// The record is in the trash or gone.
        Gone => "gone",
        /// The person who undoes may not make this change (judged again as them).
        NotAllowed => "not-allowed",
    }
}

/// A field of a run that is left as it is, with why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoSkip {
    /// The record.
    pub entity: EntityRef,
    /// The field.
    pub field: String,
    /// Why it stays.
    pub kind: UndoSkipKind,
    /// For `ChangedSince`: who changed it afterwards and when.
    pub by: Option<Actor>,
    /// For `ChangedSince`: when.
    pub at: Option<Timestamp>,
}

/// The version of a preview of *Undo the run*: it differs as soon as anything that decides what
/// goes back or is skipped has changed, so an apply with an older version is refused. Opaque.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoVersion(pub String);

/// What *Undo the run* would do now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoPreview {
    /// The version to apply with.
    pub version: UndoVersion,
    /// How many field changes go back.
    pub goes_back: u64,
    /// The first of the fields that stay, with why; `skipped` says how many there are in all.
    pub skips: Vec<UndoSkip>,
    /// How many fields stay.
    pub skipped: u64,
}

/// What *Undo the run* did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoResult {
    /// How many field changes went back.
    pub undone: u64,
    /// The first of the fields that stayed, with why.
    pub skips: Vec<UndoSkip>,
    /// How many fields stayed.
    pub skipped: u64,
}

/// The answer to an apply: done, or the newer preview when anything changed since the preview the
/// person looked at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UndoOutcome {
    /// The run was undone.
    Undone(UndoResult),
    /// The version was not the current one; nothing was written.
    Changed(UndoPreview),
}
