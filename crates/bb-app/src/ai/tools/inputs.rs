//! What each tool takes (AI-21). The types are the single source: the JSON schema an AI app is
//! shown is generated from them, and the call is read into them, so the two cannot differ.
//!
//! No input asks the AI to choose a person to give work to, to rank or to judge people (COMP-10).
//! `assigned_to` is for the case where the person named the people themselves (ADR-0029
//! decision 9).

use serde::Deserialize;
use utoipa::ToSchema;

/// Search the issues the person may read.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchIssuesInput {
    /// The search, in the search language (for example `status group is not Done and priority is High`).
    pub query: String,
    /// The most issues to answer with.
    pub limit: Option<u32>,
}

/// Read one issue.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GetIssueInput {
    /// The issue, by its readable ID (`WEB-142`) or its internal ID.
    pub issue: String,
}

/// List the projects the person may see.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListProjectsInput {
    /// Words that the name or the short name of a project holds.
    pub search: Option<String>,
    /// The most projects to answer with.
    pub limit: Option<u32>,
}

/// The issues assigned to the person.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GetMyWorkInput {
    /// The most issues to answer with.
    pub limit: Option<u32>,
}

/// Create an issue.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateIssueInput {
    /// The project, by its ID or short name.
    pub project: String,
    /// The ID of the issue type.
    pub issue_type: String,
    /// What the issue is about.
    pub title: String,
    /// The long text, in Markdown.
    pub description: Option<String>,
    /// `low`, `normal`, `high` or `critical`.
    pub priority: Option<String>,
    /// The first day of work, as a date like `2026-11-14`.
    pub start_date: Option<String>,
    /// The day it should be done, as a date like `2026-11-14`. Never before the start date.
    pub due_date: Option<String>,
    /// The issue it is created under as a sub-issue, in the project of that issue. An agent can do
    /// this only under an issue that was handed over to it with *create sub-issues*.
    pub parent: Option<String>,
}

/// Change fields of an issue.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateIssueInput {
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// The version of the issue that was read.
    pub version: i64,
    /// A new title.
    pub title: Option<String>,
    /// `low`, `normal`, `high` or `critical`.
    pub priority: Option<String>,
    /// The first day of work, as a date like `2026-11-14`; an empty text takes the date away.
    pub start_date: Option<String>,
    /// The day it should be done, as a date like `2026-11-14`, never before the start date; an
    /// empty text takes the date away.
    pub due_date: Option<String>,
    /// The IDs of the people it is assigned to. Set it only when the person asked for exactly these
    /// people; never choose a person.
    pub assigned_to: Option<Vec<String>>,
}

/// Move an issue to another status.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangeStatusInput {
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// The version of the issue that was read.
    pub version: i64,
    /// The key of the new status.
    pub status: String,
}

/// Write a comment.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddCommentInput {
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// The comment, in Markdown.
    pub text: String,
}

/// Book the person's own time.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LogTimeInput {
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// The day, as `2027-01-15`.
    pub date: String,
    /// How long, in whole minutes.
    pub minutes: u32,
    /// A note, in plain text.
    pub note: Option<String>,
}

/// Start the person's timer.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StartTimerInput {
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
}

/// Stop the person's timer.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StopTimerInput {
    /// The version of the timer that was read.
    pub version: i64,
}

/// Take an issue that was handed over to you.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TakeInput {
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// The version of the issue you read. When it is given and the issue changed since, nothing is
    /// taken and you are told what changed.
    pub version: Option<i64>,
}

/// Report a step of your attempt.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReportInput {
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// Your plan, in Markdown, when you have one.
    pub plan: Option<String>,
    /// The minutes you spent on the attempt in all, as you count them. It is your word and never a
    /// time entry.
    pub minutes: u32,
    /// The cost of the attempt in all, in cents, as you count it.
    pub cost_cents: u32,
}

/// Ask a question.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AskInput {
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// What you ask, in plain text.
    pub question: String,
}

/// Attach a result to your attempt.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AttachInput {
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// `link` for an address such as a branch, a pull request or a test report, `check` for a check.
    pub kind: String,
    /// What you call it.
    pub label: String,
    /// The address, for `link`.
    pub address: Option<String>,
    /// `passed` or `failed`, for `check`. Bugaboo does not run the check; this is your word.
    pub outcome: Option<String>,
}

/// Give up on an issue.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GiveUpInput {
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// Why you give up, in plain text.
    pub reason: String,
}

/// Say that your attempt is ready for review.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SubmitInput {
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
}

/// Ask what changed since the mark you got last time.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangesSinceInput {
    /// The mark from the last answer. Leave it out the first time: nothing is told, and you get the
    /// mark of now.
    pub since: Option<String>,
    /// The most changes to answer with.
    pub limit: Option<u32>,
}

/// Propose a change under your standing rule.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProposeInput {
    /// The ID of the standing rule.
    pub rule: String,
    /// The issue the rule found, by its readable ID or its internal ID.
    pub issue: String,
    /// `labels`, `tags`, `type`, `duplicate-of` or `found-in-release`.
    pub field: String,
    /// The value you propose.
    pub value: serde_json::Value,
}
