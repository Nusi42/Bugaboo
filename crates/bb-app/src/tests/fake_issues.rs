//! The issues part of the in-memory store: issue types, issues, the people they are assigned to
//! and the codes of their readable IDs.

use bb_core::event::DomainEvent;
use bb_core::id::{IssueId, IssueTypeId, ProjectId, UserId};
use bb_core::issue::{Issue, IssueAlias, IssueAssignee, IssueIcon, IssueType};
use bb_core::issue_key::IssueCode;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, State, Tx};
use crate::issues::{InTrash, IssueFound, IssueStore};

fn types_of(state: &State) -> Vec<IssueType> {
    let mut types = state.issue_types.clone();
    types.sort_by_key(|found| (found.position, found.id));
    types
}

fn issue_of(state: &State, id: IssueId) -> Option<Issue> {
    state.issues.iter().find(|issue| issue.id == id).cloned()
}

fn at(state: &State, project: ProjectId, code: &IssueCode) -> Option<IssueFound> {
    if let Some(issue) = state
        .issues
        .iter()
        .find(|issue| issue.project == project && &issue.code == code)
    {
        return Some(IssueFound {
            issue: issue.clone(),
            old_code: false,
        });
    }
    let alias = state
        .issue_aliases
        .iter()
        .find(|alias| alias.project == project && &alias.code == code)?;
    Some(IssueFound {
        issue: issue_of(state, alias.issue)?,
        old_code: true,
    })
}

fn assignees_of(state: &State, issues: &[IssueId]) -> Vec<IssueAssignee> {
    let mut found: Vec<IssueAssignee> = state
        .assignees
        .iter()
        .filter(|assignee| issues.contains(&assignee.issue))
        .cloned()
        .collect();
    found.sort_by_key(|assignee| (assignee.added_at, assignee.person));
    found
}

impl IssueStore for FakeStore {
    async fn issue_types_in(tx: &mut Tx) -> Result<Vec<IssueType>, FakeError> {
        Ok(types_of(&tx.state))
    }

    async fn issue_in(tx: &mut Tx, id: IssueId) -> Result<Option<Issue>, FakeError> {
        Ok(issue_of(&tx.state, id))
    }

    async fn issue_at_in(
        tx: &mut Tx,
        project: ProjectId,
        code: IssueCode,
    ) -> Result<Option<IssueFound>, FakeError> {
        Ok(at(&tx.state, project, &code))
    }

    async fn issue_code_taken_in(
        tx: &mut Tx,
        project: ProjectId,
        code: IssueCode,
    ) -> Result<bool, FakeError> {
        Ok(at(&tx.state, project, &code).is_some())
    }

    async fn assignees_in(tx: &mut Tx, issue: IssueId) -> Result<Vec<IssueAssignee>, FakeError> {
        Ok(assignees_of(&tx.state, &[issue]))
    }

    async fn issue_codes_in(
        tx: &mut Tx,
        project: ProjectId,
    ) -> Result<Vec<(IssueId, IssueCode)>, FakeError> {
        let mut found: Vec<(IssueId, IssueCode)> = tx
            .state
            .issues
            .iter()
            .filter(|issue| issue.project == project)
            .map(|issue| (issue.id, issue.code.clone()))
            .collect();
        found.sort_by_key(|(id, _)| *id);
        Ok(found)
    }

    async fn issue_type(&self, id: IssueTypeId) -> Result<Option<IssueType>, FakeError> {
        Ok(self.read(|state| {
            state
                .issue_types
                .iter()
                .find(|found| found.id == id)
                .cloned()
        }))
    }

    async fn issues_page(
        &self,
        project: ProjectId,
        reported_by: Option<UserId>,
        before: Option<IssueId>,
        limit: u64,
    ) -> Result<Vec<Issue>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<Issue> = state
                .issues
                .iter()
                .filter(|issue| {
                    issue.project == project
                        && issue.deleted_at.is_none()
                        && reported_by.is_none_or(|person| issue.reported_by == person)
                        && before.is_none_or(|before| issue.id < before)
                })
                .cloned()
                .collect();
            found.sort_by_key(|issue| std::cmp::Reverse(issue.id));
            found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            found
        }))
    }

    async fn assignees_of(&self, issues: &[IssueId]) -> Result<Vec<IssueAssignee>, FakeError> {
        Ok(self.read(|state| assignees_of(state, issues)))
    }

    async fn insert_issue_type(tx: &mut Tx, issue_type: &IssueType) -> Result<(), FakeError> {
        tx.state.issue_types.push(issue_type.clone());
        Ok(())
    }

    async fn update_issue_type(
        tx: &mut Tx,
        id: IssueTypeId,
        name: &str,
        icon: IssueIcon,
        retired_at: Option<Timestamp>,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .issue_types
            .iter_mut()
            .find(|found| found.id == id && found.version == version)
        else {
            return Ok(false);
        };
        name.clone_into(&mut found.name);
        found.icon = icon;
        found.retired_at = retired_at;
        found.version += 1;
        Ok(true)
    }

    async fn insert_issue(
        tx: &mut Tx,
        issue: &Issue,
        assignees: &[IssueAssignee],
    ) -> Result<(), FakeError> {
        // The unique key of the codes of a project.
        if at(&tx.state, issue.project, &issue.code).is_some() {
            return Err(FakeError::Duplicate);
        }
        tx.state.issues.push(issue.clone());
        tx.state.assignees.extend(assignees.iter().cloned());
        Ok(())
    }

    async fn update_issue(tx: &mut Tx, issue: &Issue, version: i64) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .issues
            .iter_mut()
            .find(|found| found.id == issue.id && found.version == version)
        else {
            return Ok(false);
        };
        found.issue_type = issue.issue_type;
        found.title.clone_from(&issue.title);
        found.description.clone_from(&issue.description);
        found.status = issue.status;
        found.priority = issue.priority;
        found.updated_at = issue.updated_at;
        found.closed_at = issue.closed_at;
        found.version += 1;
        Ok(true)
    }

    async fn move_issue(
        tx: &mut Tx,
        id: IssueId,
        project: ProjectId,
        code: &IssueCode,
        version: i64,
        now: Timestamp,
    ) -> Result<bool, FakeError> {
        let Some(found) =
            tx.state.issues.iter_mut().find(|found| {
                found.id == id && found.version == version && found.deleted_at.is_none()
            })
        else {
            return Ok(false);
        };
        found.project = project;
        found.code = code.clone();
        found.updated_at = now;
        found.version += 1;
        Ok(true)
    }

    async fn replace_assignees(
        tx: &mut Tx,
        issue: IssueId,
        assignees: &[IssueAssignee],
    ) -> Result<(), FakeError> {
        tx.state
            .assignees
            .retain(|assignee| assignee.issue != issue);
        tx.state.assignees.extend(assignees.iter().cloned());
        Ok(())
    }

    async fn set_issue_deleted(
        tx: &mut Tx,
        id: IssueId,
        deleted_at: Option<Timestamp>,
        now: Timestamp,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .issues
            .iter_mut()
            .find(|found| found.id == id && found.deleted_at.is_some() != deleted_at.is_some())
        else {
            return Ok(false);
        };
        found.deleted_at = deleted_at;
        found.updated_at = now;
        found.version += 1;
        Ok(true)
    }

    async fn next_counting_number(tx: &mut Tx, project: ProjectId) -> Result<i64, FakeError> {
        if let Some((_, last)) = tx
            .state
            .counters
            .iter_mut()
            .find(|(counted, _)| *counted == project)
        {
            *last += 1;
            return Ok(*last);
        }
        tx.state.counters.push((project, 1));
        Ok(1)
    }

    async fn set_issue_code(tx: &mut Tx, id: IssueId, code: &IssueCode) -> Result<(), FakeError> {
        if let Some(found) = tx.state.issues.iter_mut().find(|found| found.id == id) {
            found.code = code.clone();
            found.version += 1;
        }
        Ok(())
    }

    async fn count_issue_up(
        tx: &mut Tx,
        issue: IssueId,
        only_at: Option<i64>,
        in_trash: InTrash,
        now: Timestamp,
    ) -> Result<Option<i64>, FakeError> {
        let Some(found) = tx.state.issues.iter_mut().find(|found| {
            found.id == issue
                && (in_trash == InTrash::Counted || found.deleted_at.is_none())
                && only_at.is_none_or(|version| found.version == version)
        }) else {
            return Ok(None);
        };
        found.updated_at = now;
        found.version += 1;
        Ok(Some(found.version))
    }

    async fn lock_issue_codes(_tx: &mut Tx, _project: ProjectId) -> Result<(), FakeError> {
        Ok(())
    }

    async fn append_follow_up_event(tx: &mut Tx, event: &DomainEvent) -> Result<(), FakeError> {
        tx.follow_ups.push(event.clone());
        Ok(())
    }

    async fn insert_issue_alias(tx: &mut Tx, alias: &IssueAlias) -> Result<(), FakeError> {
        tx.state.issue_aliases.push(alias.clone());
        Ok(())
    }
}
