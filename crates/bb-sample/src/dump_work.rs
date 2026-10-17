//! The part of the canonical dump that is about the people working together (M4 WP-10): the
//! workflows and where a project has one of its own, the files with their previews, the
//! notifications, and what each kind of role did (issues reported, status changes, issues followed).
//! Only counts and names: IDs are random by design.
//!
//! TEST-SAMPLE-DATA holds these numbers against the plan in `tools/dev/sample.json`
//! (`bb_dev_words::sample::WorkPlan`).

use std::collections::BTreeMap;

use bb_app::files::AttachmentStore as _;
use bb_app::notifications::NotificationStore as _;
use bb_app::projects::ProjectStore as _;
use bb_app::workflows::WorkflowStore as _;
use bb_core::event::{Actor, DomainEvent, EntityKind, RecordKind};
use bb_core::fields::Field;
use bb_core::files::PreviewStatus;
use bb_core::id::{ProjectId, UserId};
use bb_core::issue::Issue;
use bb_core::notification::NotificationFilter;
use bb_core::workflow::WorkflowContent;
use serde::Serialize;

use crate::error::SampleError;
use crate::generate::Sample;

/// How many notifications a page holds when the dump reads them all.
const PAGE: u64 = 1000;

/// A workflow: how big it is, and how many of its status changes ask for something.
#[derive(Debug, Serialize)]
struct DumpWorkflow {
    name: String,
    statuses: usize,
    changes: usize,
    asks: usize,
}

/// The files and where their previews are.
#[derive(Debug, Default, Serialize)]
struct DumpFiles {
    total: u64,
    pictures: u64,
    pdfs: u64,
    on_comments: u64,
    previews_ready: u64,
    without_preview: u64,
}

/// The notifications of everybody.
#[derive(Debug, Default, Serialize)]
struct DumpNotifications {
    total: u64,
    read: u64,
    by_kind: BTreeMap<String, u64>,
    /// By the role of the person who got it.
    to: BTreeMap<String, u64>,
    /// Notifications of Guests about an issue they did not report: a Guest sees only their own
    /// issues, so there must be none.
    to_guests_about_others: u64,
}

/// The people working together.
#[derive(Debug, Default, Serialize)]
pub(crate) struct DumpWork {
    workflows: Vec<DumpWorkflow>,
    /// The short names of the projects that have a workflow of their own.
    own_workflow_projects: Vec<String>,
    files: DumpFiles,
    notifications: DumpNotifications,
    /// Issues reported by Contributors and Guests, by role.
    reports: BTreeMap<String, u64>,
    /// Status changes after the people began to work together, by role (the Team members are the
    /// Administrators and the Team members).
    status_changes: BTreeMap<String, u64>,
    /// Changes Contributors made to what they reported, after the people began to work together.
    own_changes: BTreeMap<String, u64>,
    /// Issues followed by Viewers.
    follows: BTreeMap<String, u64>,
}

/// The role as the dump counts it: Administrators and Team members do the work of the team.
fn role_of(code: &str) -> String {
    match code {
        "administrator" | "team-member" => "team_member".to_owned(),
        other => other.replace('-', "_"),
    }
}

impl Sample {
    /// The role, as the dump counts it, of each person.
    fn roles(&self) -> BTreeMap<UserId, String> {
        self.people
            .iter()
            .map(|made| (made.id, role_of(made.person.role.code())))
            .collect()
    }

    /// Reads the working together from the store and counts it in the events.
    pub(crate) async fn work(
        &self,
        events: &[DomainEvent],
        issues: &[Issue],
    ) -> Result<DumpWork, SampleError> {
        let roles = self.roles();
        let mut work = DumpWork::default();
        self.workflows_of(&mut work).await?;
        self.files_of(&mut work, issues).await?;
        self.notifications_of(&mut work, &roles, issues).await?;
        self.count_roles(&mut work, &roles, events);
        Ok(work)
    }

    /// The workflows with the status changes that ask for something, and the projects that use one
    /// of their own.
    async fn workflows_of(&self, work: &mut DumpWork) -> Result<(), SampleError> {
        let storage = &self.storage;
        let ids = storage
            .workflow_headers()
            .await?
            .into_iter()
            .map(|header| header.id)
            .collect();
        for stored in storage.stored_workflows(ids).await? {
            let WorkflowContent { statuses, changes } = stored.content;
            work.workflows.push(DumpWorkflow {
                name: stored.workflow.name,
                statuses: statuses.len(),
                changes: changes.len(),
                asks: changes
                    .iter()
                    .filter(|change| !change.rules.ask_for.is_empty())
                    .count(),
            });
        }
        work.workflows
            .sort_by(|one, other| one.name.cmp(&other.name));
        let projects: BTreeMap<ProjectId, String> = storage
            .project_tree()
            .await?
            .into_iter()
            .map(|project| (project.id, project.short_name.to_string()))
            .collect();
        for used in storage.workflow_uses().await? {
            let named = used.project.and_then(|project| projects.get(&project));
            if let Some(name) = named.filter(|name| !work.own_workflow_projects.contains(name)) {
                work.own_workflow_projects.push(name.clone());
            }
        }
        work.own_workflow_projects.sort();
        Ok(())
    }

    /// The files of every issue, with where their previews are.
    async fn files_of(&self, work: &mut DumpWork, issues: &[Issue]) -> Result<(), SampleError> {
        for issue in issues {
            for file in self.storage.attachments_of_issue(issue.id).await? {
                let files = &mut work.files;
                files.total += 1;
                files.on_comments += u64::from(file.comment.is_some());
                if std::path::Path::new(&file.name)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
                {
                    files.pdfs += 1;
                } else {
                    files.pictures += 1;
                }
                match file.preview {
                    PreviewStatus::Ready => files.previews_ready += 1,
                    PreviewStatus::Pending | PreviewStatus::Without => files.without_preview += 1,
                }
            }
        }
        Ok(())
    }

    /// The notifications of every person.
    async fn notifications_of(
        &self,
        work: &mut DumpWork,
        roles: &BTreeMap<UserId, String>,
        issues: &[Issue],
    ) -> Result<(), SampleError> {
        let reporters: BTreeMap<_, _> = issues
            .iter()
            .map(|issue| (issue.id, issue.reported_by))
            .collect();
        for made in &self.people {
            let role = roles.get(&made.id).cloned().unwrap_or_default();
            let mut before = None;
            loop {
                let page = self
                    .storage
                    .notifications_of(made.id, NotificationFilter::All, before, PAGE)
                    .await?;
                before = page.last().map(|row| row.id);
                let full = page.len() as u64 == PAGE;
                for row in page {
                    let counted = &mut work.notifications;
                    counted.total += 1;
                    counted.read += u64::from(row.read_at.is_some());
                    *counted
                        .by_kind
                        .entry(row.kind.code().to_owned())
                        .or_default() += 1;
                    *counted.to.entry(role.clone()).or_default() += 1;
                    counted.to_guests_about_others += u64::from(
                        role == "guest"
                            && row
                                .issue
                                .is_some_and(|issue| reporters.get(&issue) != Some(&made.id)),
                    );
                }
                if !full {
                    break;
                }
            }
        }
        Ok(())
    }

    /// What each role did, counted in the events.
    fn count_roles(
        &self,
        work: &mut DumpWork,
        roles: &BTreeMap<UserId, String>,
        events: &[DomainEvent],
    ) {
        let (issue, follower) = (RecordKind::Issue.code(), RecordKind::Follower.code());
        let since = self.left_behind.work_since;
        for event in events {
            let Actor::Person(person) = event.actor else {
                continue;
            };
            let role = roles.get(&person).cloned().unwrap_or_default();
            let kind: &EntityKind = &event.entity.kind;
            let changes = |field: &str| event.changes.iter().find(|change| change.field == field);
            if kind.as_str() == issue
                && changes("title").is_some_and(|change| change.before.is_none())
                && (role == "contributor" || role == "guest")
            {
                *work.reports.entry(role.clone()).or_default() += 1;
            }
            let later = since.is_some_and(|since| event.time >= since);
            if kind.as_str() == issue
                && later
                && changes(Field::Status.code()).is_some_and(|change| change.before.is_some())
            {
                *work.status_changes.entry(role.clone()).or_default() += 1;
            }
            if kind.as_str() == issue
                && later
                && role == "contributor"
                && changes("title").is_none_or(|change| change.before.is_some())
            {
                *work.own_changes.entry(role.clone()).or_default() += 1;
            }
            if kind.as_str() == follower
                && role == "viewer"
                && event.changes.iter().all(|change| change.before.is_none())
            {
                *work.follows.entry(role).or_default() += 1;
            }
        }
    }
}
