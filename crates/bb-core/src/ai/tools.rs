//! What AI is kept from and what is written down about it (AI-03, AI-04, AI-05, COMP-10, ADR-0013):
//! the marks *Never sent to AI* and the log of tool calls.
//!
//! A tool call is logged with who, how, which tool, the size and the time, never the content
//! (DATA-LOGS, architecture §7). The marks are rows of their own because a built-in field is no
//! row and a project is not changed by this; an own field carries its mark in its definition
//! (`Field::never_to_assistant`).

use jiff::Timestamp;

use crate::fields::Field;
use crate::id::{ProjectId, ToolCallId, UserId};
use crate::personal::Entity;

/// How long the log of tool calls is kept, in days (ADR-0013, amendment). The job that prunes the
/// security log prunes this log with it.
pub const TOOL_CALL_KEEP_DAYS: i64 = 90;

words! {
    /// Who calls a tool.
    pub enum ToolCaller {
        /// The assistant in the panel.
        Assistant => "assistant",
        /// An AI app that signed in through MCP.
        AiApp => "ai-app",
        /// An agent.
        Agent => "agent",
    }
}

words! {
    /// The tools of the one registry (AI-21): the names AI apps and the assistant call. A tool
    /// that ranks, evaluates or picks people, or reads working hours, is not in this list and
    /// never will be (COMP-10, ADR-0035 decision 9).
    pub enum Tool {
        /// Search the issues the person may read.
        SearchIssues => "search_issues",
        /// Read one issue.
        GetIssue => "get_issue",
        /// List the projects the person may see.
        ListProjects => "list_projects",
        /// The issues assigned to the person.
        GetMyWork => "get_my_work",
        /// Make an issue.
        CreateIssue => "create_issue",
        /// Change fields of an issue.
        UpdateIssue => "update_issue",
        /// Change the status of an issue.
        ChangeStatus => "change_status",
        /// Comment on an issue.
        AddComment => "add_comment",
        /// Book time on an issue, for the person themselves.
        LogTime => "log_time",
        /// Start the person's timer.
        StartTimer => "start_timer",
        /// Stop the person's timer.
        StopTimer => "stop_timer",
    }
    messages description = "ai-tool.";
}

words! {
    /// Whether a tool only looks or changes something.
    pub enum ToolEffect {
        /// It only reads.
        Reads => "reads",
        /// It changes something (the assistant proposes first).
        Changes => "changes",
    }
}

words! {
    /// Who wrote a text, as the AI is told (AI-05). The assistant and agents are not people: what
    /// they wrote earlier is data as well.
    pub enum AuthorKind {
        /// A person who holds a role in the project and sees all its issues.
        Member => "member",
        /// A person who sees only what they reported.
        Guest => "guest",
        /// A person with no access to the project, or one the reader may not tell more about.
        Outsider => "outsider",
        /// An agent.
        Agent => "agent",
        /// The assistant.
        Assistant => "assistant",
    }
    messages label = "ai-author.";
}

impl AuthorKind {
    /// How the AI is told who wrote a text (*someone in the project*, *a Guest*, ...).
    #[must_use]
    pub fn phrase(self) -> String {
        crate::message::Message::from(self.label()).render()
    }
}

words! {
    /// What an AI app, an access key or the assistant may do (AI-04, ADR-0025 decision 8): it only
    /// narrows what the person may do.
    pub enum ToolScope {
        /// Look at things.
        SeeOnly => "see-only",
        /// Look at things and change them.
        SeeAndChange => "see-and-change",
    }
}

impl ToolScope {
    /// Whether this scope is enough for a tool that needs `needed`.
    #[must_use]
    pub const fn covers(self, needed: Self) -> bool {
        matches!(self, Self::SeeAndChange) || matches!(needed, Self::SeeOnly)
    }
}

impl ToolEffect {
    /// The scope a tool with this effect needs: a tool that only reads needs *See only*, one that
    /// changes something needs *See and change*. The one place that says it.
    #[must_use]
    pub const fn scope(self) -> ToolScope {
        match self {
            Self::Reads => ToolScope::SeeOnly,
            Self::Changes => ToolScope::SeeAndChange,
        }
    }
}

words! {
    /// How a tool call ended.
    pub enum ToolOutcome {
        /// The tool did its work.
        Done => "done",
        /// The call was turned away: unknown tool, a scope that does not allow it, or a field kept
        /// from AI.
        Refused => "refused",
        /// The tool was allowed and failed.
        Failed => "failed",
    }
}

/// Something that is kept from AI: a built-in field or a project (with the projects inside it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NeverSentSubject {
    /// A built-in field.
    Field(Field),
    /// A project and, with it, every project inside it.
    Project(ProjectId),
}

impl crate::personal::NotPersonal for NeverSentSubject {}

/// The mark *Never sent to AI* of one built-in field or one project. A subject without a row is not
/// marked, at version 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Entity)]
pub struct NeverSentMark {
    /// What is kept from AI.
    #[not_personal]
    pub subject: NeverSentSubject,
    /// Whether it is kept from AI now.
    #[not_personal]
    pub marked: bool,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// One call of one tool. Never the content, only its size.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct ToolCall {
    /// The ID of the call.
    #[not_personal]
    pub id: ToolCallId,
    /// The person the call acted for.
    #[personal(category = "identity", basis = "legitimate_interests")]
    pub person: UserId,
    /// How the tool was called.
    #[not_personal]
    pub caller: ToolCaller,
    /// The name an AI app gave itself; none for the assistant.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub app: Option<String>,
    /// The name of the tool.
    #[not_personal]
    pub tool: Tool,
    /// Whether the tool reads or changes.
    #[not_personal]
    pub effect: ToolEffect,
    /// The bytes of the input.
    #[not_personal]
    pub bytes_in: i64,
    /// The bytes of the answer.
    #[not_personal]
    pub bytes_out: i64,
    /// When the call was made.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub at: Timestamp,
    /// How it ended.
    #[not_personal]
    pub outcome: ToolOutcome,
}
