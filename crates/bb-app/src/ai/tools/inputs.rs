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
