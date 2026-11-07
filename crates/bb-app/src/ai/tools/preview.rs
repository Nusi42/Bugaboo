//! The previews of the tools that change something (AI-21, ADR-0013): what the use case would
//! record, answered without saving. A preview is the use case's own plan, never a second judgment;
//! the assistant shows it as a proposal and *Apply* runs the use case itself.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::ai::Tool;
use bb_core::event::FieldChange;
use bb_core::id::{IssueTypeId, UserId};
use bb_core::issue::Priority;
use bb_core::time::Minutes;
use bb_core::workflow::StatusKey;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::inputs::{
    AddCommentInput, ChangeStatusInput, CreateIssueInput, LogTimeInput, StartTimerInput,
    StopTimerInput, UpdateIssueInput,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::comments::AddComment;
use crate::issues::{ChangeIssue, CreateIssue};
use crate::time::{AddTime, EntryDetails, StartTimer, StopTimer};

fn read<T: DeserializeOwned>(input: &Value) -> Result<T, AppError> {
    serde_json::from_value(input.clone()).map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))
}

fn invalid<T, E>(parsed: Result<T, E>) -> Result<T, AppError> {
    parsed.map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))
}

fn priority(text: Option<&str>) -> Result<Option<Priority>, AppError> {
    text.map(|text| Priority::from_code(text).ok_or(AppError::Refused(ErrorCode::InvalidRequest)))
        .transpose()
}

fn people(ids: Option<&[String]>) -> Result<Option<Vec<UserId>>, AppError> {
    ids.map(|ids| {
        ids.iter()
            .map(|id| invalid(id.parse::<UserId>()))
            .collect::<Result<Vec<_>, _>>()
    })
    .transpose()
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the tool would record, if it changes something, answered without saving.
    ///
    /// # Errors
    /// `request.invalid` for an input that is not the tool's, and what the use case refuses.
    pub(super) async fn tool_preview(
        &self,
        actor: &Actor,
        session_token: &str,
        tool: Tool,
        input: &Value,
    ) -> Result<Vec<FieldChange>, AppError> {
        let session_token = session_token.to_owned();
        match tool {
            Tool::CreateIssue => {
                let input: CreateIssueInput = read(input)?;
                self.preview_create_issue(
                    actor,
                    &CreateIssue {
                        session_token,
                        project: input.project,
                        issue_type: invalid(input.issue_type.parse::<IssueTypeId>())?,
                        title: input.title,
                        description: input.description,
                        status: None,
                        priority: priority(input.priority.as_deref())?,
                        assigned_to: None,
                        estimated_time: None,
                        fields: BTreeMap::new(),
                    },
                )
                .await
            }
            Tool::UpdateIssue => {
                let input: UpdateIssueInput = read(input)?;
                self.preview_change_issue(
                    actor,
                    &ChangeIssue {
                        session_token,
                        issue: input.issue,
                        version: input.version,
                        issue_type: None,
                        title: input.title,
                        status: None,
                        priority: priority(input.priority.as_deref())?,
                        estimated_time: None,
                        time_left: None,
                        assigned_to: people(input.assigned_to.as_deref())?,
                        fields: BTreeMap::new(),
                    },
                )
                .await
            }
            Tool::ChangeStatus => {
                let input: ChangeStatusInput = read(input)?;
                self.preview_change_issue(
                    actor,
                    &ChangeIssue {
                        session_token,
                        issue: input.issue,
                        version: input.version,
                        issue_type: None,
                        title: None,
                        status: Some(invalid(StatusKey::new(&input.status))?),
                        priority: None,
                        estimated_time: None,
                        time_left: None,
                        assigned_to: None,
                        fields: BTreeMap::new(),
                    },
                )
                .await
            }
            Tool::AddComment | Tool::LogTime | Tool::StartTimer | Tool::StopTimer => {
                self.preview_own_tool(actor, &session_token, tool, input)
                    .await
            }
            // A tool that only reads has no preview.
            Tool::SearchIssues | Tool::GetIssue | Tool::ListProjects | Tool::GetMyWork => {
                Err(AppError::Refused(ErrorCode::InvalidRequest))
            }
        }
    }

    /// The previews of the tools that act on the person's own comments, time and timer.
    async fn preview_own_tool(
        &self,
        actor: &Actor,
        session_token: &str,
        tool: Tool,
        input: &Value,
    ) -> Result<Vec<FieldChange>, AppError> {
        let session_token = session_token.to_owned();
        match tool {
            Tool::AddComment => {
                let input: AddCommentInput = read(input)?;
                self.preview_add_comment(
                    actor,
                    &AddComment {
                        session_token,
                        issue: input.issue,
                        text: input.text,
                    },
                )
                .await
            }
            Tool::LogTime => {
                let input: LogTimeInput = read(input)?;
                self.preview_add_time(
                    actor,
                    &AddTime {
                        session_token,
                        issue: input.issue,
                        date: invalid(input.date.parse())?,
                        minutes: Minutes::new(input.minutes),
                        details: EntryDetails {
                            note: input.note,
                            ..EntryDetails::default()
                        },
                    },
                )
                .await
            }
            Tool::StartTimer => {
                let input: StartTimerInput = read(input)?;
                self.preview_start_timer(
                    actor,
                    &StartTimer {
                        session_token,
                        issue: input.issue,
                    },
                )
                .await
            }
            Tool::StopTimer => {
                let input: StopTimerInput = read(input)?;
                self.preview_stop_timer(
                    actor,
                    &StopTimer {
                        session_token,
                        version: input.version,
                        details: EntryDetails::default(),
                    },
                )
                .await
            }
            _ => Err(AppError::Refused(ErrorCode::InvalidRequest)),
        }
    }
}
