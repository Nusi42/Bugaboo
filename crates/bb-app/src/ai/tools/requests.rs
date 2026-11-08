//! What each tool that changes something asks of its use case, made from the input of the call.
//!
//! The preview (`preview.rs`) and *Apply* (`apply.rs`) both build the request here, so what is
//! shown is exactly what is saved: there is one reading of an input, not two.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::id::{IssueTypeId, UserId};
use bb_core::issue::Priority;
use bb_core::time::Minutes;
use bb_core::workflow::StatusKey;

use super::inputs::{
    AddCommentInput, ChangeStatusInput, CreateIssueInput, LogTimeInput, StartTimerInput,
    StopTimerInput, UpdateIssueInput,
};
use super::registry::Run;
use crate::auth::AppError;
use crate::comments::AddComment;
use crate::issues::{ChangeIssue, CreateIssue};
use crate::time::{AddTime, EntryDetails, StartTimer, StopTimer};

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

/// `create_issue`.
pub(super) fn create_issue(run: &Run<'_>) -> Result<CreateIssue, AppError> {
    let input: CreateIssueInput = run.input()?;
    Ok(CreateIssue {
        session_token: run.token.to_owned(),
        project: input.project,
        issue_type: invalid(input.issue_type.parse::<IssueTypeId>())?,
        title: input.title,
        description: input.description,
        status: None,
        priority: priority(input.priority.as_deref())?,
        assigned_to: None,
        estimated_time: None,
        fields: BTreeMap::new(),
        parent: input.parent,
    })
}

/// `update_issue`.
pub(super) fn update_issue(run: &Run<'_>) -> Result<ChangeIssue, AppError> {
    let input: UpdateIssueInput = run.input()?;
    Ok(ChangeIssue {
        session_token: run.token.to_owned(),
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
    })
}

/// `change_status`.
pub(super) fn change_status(run: &Run<'_>) -> Result<ChangeIssue, AppError> {
    let input: ChangeStatusInput = run.input()?;
    Ok(ChangeIssue {
        session_token: run.token.to_owned(),
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
    })
}

/// `add_comment`.
pub(super) fn add_comment(run: &Run<'_>) -> Result<AddComment, AppError> {
    let input: AddCommentInput = run.input()?;
    Ok(AddComment {
        session_token: run.token.to_owned(),
        issue: input.issue,
        text: input.text,
    })
}

/// `log_time`.
pub(super) fn log_time(run: &Run<'_>) -> Result<AddTime, AppError> {
    let input: LogTimeInput = run.input()?;
    Ok(AddTime {
        session_token: run.token.to_owned(),
        issue: input.issue,
        date: invalid(input.date.parse())?,
        minutes: Minutes::new(input.minutes),
        details: EntryDetails {
            note: input.note,
            ..EntryDetails::default()
        },
    })
}

/// `start_timer`.
pub(super) fn start_timer(run: &Run<'_>) -> Result<StartTimer, AppError> {
    let input: StartTimerInput = run.input()?;
    Ok(StartTimer {
        session_token: run.token.to_owned(),
        issue: input.issue,
    })
}

/// `stop_timer`.
pub(super) fn stop_timer(run: &Run<'_>) -> Result<StopTimer, AppError> {
    let input: StopTimerInput = run.input()?;
    Ok(StopTimer {
        session_token: run.token.to_owned(),
        version: input.version,
        details: EntryDetails::default(),
    })
}
