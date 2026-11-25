//! What each tool that changes something asks of its use case, made from the input of the call.
//!
//! The preview (`preview.rs`) and *Apply* (`apply.rs`) both build the request here, so what is
//! shown is exactly what is saved: there is one reading of an input, not two.

use std::collections::BTreeMap;

use bb_core::given::SetTo;
use bb_core::id::{IssueTypeId, ReleaseId, UserId};
use bb_core::issue::Priority;
use bb_core::time::Minutes;
use bb_core::workflow::StatusKey;
use jiff::civil::Date;

use super::inputs::{
    AddCommentInput, ChangeStatusInput, CreateIssueInput, LogTimeInput, StartTimerInput,
    StopTimerInput, UpdateIssueInput,
};
use super::refused::{refused_at, word_at};
use super::registry::Run;
use crate::auth::AppError;
use crate::comments::AddComment;
use crate::issues::{Asker, ChangeIssue, CreateIssue};
use crate::time::{AddTime, EntryDetails, StartTimer, StopTimer};

/// The value of a member read as what it names; one that does not read is refused at the member.
fn invalid<T, E>(field: &str, parsed: Result<T, E>) -> Result<T, AppError> {
    parsed.map_err(|_| refused_at(field))
}

fn priority(text: Option<&str>) -> Result<Option<Priority>, AppError> {
    text.map(|text| word_at("priority", text)).transpose()
}

fn people(ids: Option<&[String]>) -> Result<Option<Vec<UserId>>, AppError> {
    ids.map(|ids| {
        ids.iter()
            .map(|id| invalid("assigned_to", id.parse::<UserId>()))
            .collect::<Result<Vec<_>, _>>()
    })
    .transpose()
}

/// The releases of *Found in release* as a tool gives them, by their IDs.
fn releases(ids: Option<&[String]>) -> Result<Option<Vec<ReleaseId>>, AppError> {
    ids.map(|ids| {
        ids.iter()
            .map(|id| invalid("found_in_release", id.parse::<ReleaseId>()))
            .collect::<Result<Vec<_>, _>>()
    })
    .transpose()
}

/// The release of *Planned for release* a tool sets: none leaves it as it is, an empty text takes
/// it away.
fn release_to_set(text: Option<&str>) -> Result<Option<SetTo<ReleaseId>>, AppError> {
    text.map(|text| {
        if text.is_empty() {
            Ok(SetTo::Nothing)
        } else {
            invalid("planned_for_release", text.parse::<ReleaseId>()).map(SetTo::Value)
        }
    })
    .transpose()
}

/// A date of an issue as a tool gives it in the member `field`: `2026-11-14`.
fn day(field: &str, text: &str) -> Result<Date, AppError> {
    invalid(field, text.parse::<Date>())
}

/// A date a tool sets: none leaves it as it is, an empty text takes it away.
fn day_to_set(field: &str, text: Option<&str>) -> Result<Option<SetTo<Date>>, AppError> {
    text.map(|text| {
        if text.is_empty() {
            Ok(SetTo::Nothing)
        } else {
            day(field, text).map(SetTo::Value)
        }
    })
    .transpose()
}

/// `create_issue`.
pub(super) fn create_issue(run: &Run<'_>) -> Result<CreateIssue, AppError> {
    let input: CreateIssueInput = run.input()?;
    Ok(CreateIssue {
        session_token: run.token.to_owned(),
        project: input.project,
        issue_type: invalid("issue_type", input.issue_type.parse::<IssueTypeId>())?,
        title: input.title,
        description: input.description,
        status: None,
        priority: priority(input.priority.as_deref())?,
        assigned_to: None,
        start_date: input
            .start_date
            .as_deref()
            .map(|text| day("start_date", text))
            .transpose()?,
        due_date: input
            .due_date
            .as_deref()
            .map(|text| day("due_date", text))
            .transpose()?,
        estimated_time: None,
        found_in_release: None,
        planned_for_release: None,
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
        start_date: day_to_set("start_date", input.start_date.as_deref())?,
        due_date: day_to_set("due_date", input.due_date.as_deref())?,
        estimated_time: None,
        time_left: None,
        assigned_to: people(input.assigned_to.as_deref())?,
        found_in_release: releases(input.found_in_release.as_deref())?,
        planned_for_release: release_to_set(input.planned_for_release.as_deref())?,
        fields: BTreeMap::new(),
        asker: Asker::Ai(Box::new(run.withheld.clone())),
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
        status: Some(invalid("status", StatusKey::new(&input.status))?),
        priority: None,
        start_date: None,
        due_date: None,
        estimated_time: None,
        time_left: None,
        assigned_to: None,
        found_in_release: None,
        planned_for_release: None,
        fields: BTreeMap::new(),
        asker: Asker::Ai(Box::new(run.withheld.clone())),
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
        date: invalid("date", input.date.parse())?,
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
