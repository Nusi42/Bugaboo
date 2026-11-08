//! What AI is kept from and what is written down about it (AI-03, AI-04, AI-05, COMP-10, ADR-0013):
//! the marks *Never sent to AI* and the log of tool calls.
//!
//! A tool call is logged with who, how, which tool, the size and the time, never the content
//! (DATA-LOGS, architecture §7). The marks are rows of their own because a built-in field is no
//! row and a project is not changed by this; an own field carries its mark in its definition
//! (`Field::never_to_assistant`).

use jiff::Timestamp;

use crate::event::Channel;
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

impl ToolCaller {
    /// The channel the events of what this caller saves carry. An agent that calls a tool signs in
    /// at `/mcp`, so its events say `mcp` like those of an AI app.
    #[must_use]
    pub const fn channel(self) -> Option<Channel> {
        match self {
            Self::Assistant => Some(Channel::Assistant),
            Self::AiApp | Self::Agent => Some(Channel::Mcp),
        }
    }

    /// Whether the caller makes its own changes at once (an AI app with the scope to change, an
    /// agent), where the assistant only proposes and the person applies.
    #[must_use]
    pub const fn acts_by_itself(self) -> bool {
        matches!(self, Self::AiApp | Self::Agent)
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
        /// An agent takes an issue that was handed over to it, and starts an attempt.
        Take => "take",
        /// An agent reports a step of its attempt.
        Report => "report",
        /// An agent asks a question and waits for the answer.
        Ask => "ask",
        /// An agent attaches a result: a link or a check.
        Attach => "attach",
        /// An agent says its attempt is ready for a person to review.
        Submit => "submit",
        /// An agent gives up with a reason.
        GiveUp => "give_up",
        /// An agent asks what changed in the issues it may see since a mark it got before.
        ChangesSince => "changes_since",
        /// An agent that works under a standing rule proposes a change to an issue the rule found.
        Propose => "propose",
    }
    messages description = "ai-tool.";
}

impl Tool {
    /// Whether only an agent can use the tool: it is about the work an agent was given, which a
    /// person or an app of a person does not have. The one place that says it.
    #[must_use]
    pub const fn is_for_agents(self) -> bool {
        matches!(
            self,
            Self::Take
                | Self::Report
                | Self::Ask
                | Self::Attach
                | Self::Submit
                | Self::GiveUp
                | Self::ChangesSince
                | Self::Propose
        )
    }

    /// Whether only a person can use the tool: it is about the person's own time (M7-Q7: what an
    /// agent reports is never a time entry).
    #[must_use]
    pub const fn is_for_people(self) -> bool {
        matches!(self, Self::LogTime | Self::StartTimer | Self::StopTimer)
    }
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
        /// No tool has the name that was called.
        Unknown => "unknown",
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
    /// The tool; none when the name that was called is no tool (the name itself is not kept).
    #[not_personal]
    pub tool: Option<Tool>,
    /// Whether the tool reads or changes; none for a call of no tool.
    #[not_personal]
    pub effect: Option<ToolEffect>,
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
