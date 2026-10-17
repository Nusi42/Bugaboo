//! Issues as one viewer sees them: the project and the type by name, the people as *Who sees
//! people* says, the readable ID as it is now.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use bb_core::event::RecordKind;
use bb_core::id::{EntityId, IssueId, IssueTypeId, ProjectId, UserId};
use bb_core::issue::Issue;
use bb_core::people_kinds::PeopleKind;
use bb_core::project::Project;
use bb_core::readable_id::ReadableId;
use bb_core::workflow::WorkflowContent;
use bb_policy::{Evaluation, PeopleContext, PersonRef};

use super::types::{IssueTypeRef, IssueView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::fields::ItemValues;
use crate::projects::ProjectRef;
use crate::workflows::{StatusChangeView, StatusCheck, StatusView};

/// What turns the issues of one project into views for one viewer. The people are resolved once
/// each: a list shows the same few people over and over.
pub(crate) struct IssueViews {
    actor: Actor,
    viewer: Arc<Evaluation>,
    project: Project,
    types: BTreeMap<IssueTypeId, IssueTypeRef>,
    people: HashMap<(PeopleKind, UserId), PersonRef>,
    /// The values of own fields of the issues shown so far.
    values: ItemValues,
    /// The projects from the top down to the project, read when the first issue needs them.
    chain: Option<Vec<ProjectId>>,
    /// The workflow that counts for each issue type of the project, found once.
    workflows: HashMap<IssueTypeId, Arc<WorkflowContent>>,
    /// Whether the people each issue is assigned to are shown. They are always read (a status
    /// change may name *the person assigned*), and left out of the answer when nobody asked.
    show_people: bool,
}

impl IssueViews {
    /// Leaves the people an issue is assigned to out of the views.
    pub(crate) const fn without_people(mut self) -> Self {
        self.show_people = false;
        self
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The views of the issues of `project` for the viewer, whose evaluation is the one at the
    /// project.
    pub(crate) async fn issue_views(
        &self,
        actor: &Actor,
        viewer: Arc<Evaluation>,
        project: Project,
    ) -> Result<IssueViews, AppError> {
        // Every type is read to name the type of an issue the viewer reads: a type in use on an
        // issue they read is known to them (`offers/known.rs`), and no other name is given out.
        let types = self
            .store()
            .issue_types()
            .await
            .map_err(|error| internal::<S>(&error))?
            .iter()
            .map(|found| (found.id, IssueTypeRef::from(found)))
            .collect();
        Ok(IssueViews {
            actor: actor.clone(),
            viewer,
            project,
            types,
            people: HashMap::new(),
            values: ItemValues::default(),
            chain: None,
            workflows: HashMap::new(),
            show_people: true,
        })
    }

    /// The workflow that counts for an issue type in the project of the views.
    async fn view_workflow(
        &self,
        views: &mut IssueViews,
        issue_type: IssueTypeId,
    ) -> Result<Arc<WorkflowContent>, AppError> {
        if let Some(found) = views.workflows.get(&issue_type) {
            return Ok(Arc::clone(found));
        }
        if views.chain.is_none() {
            views.chain = Some(self.chain_of(views.project.id).await?);
        }
        let chain = views.chain.as_deref().unwrap_or_default();
        let found = self.workflow_in_force(chain, Some(issue_type)).await?;
        views.workflows.insert(issue_type, Arc::clone(&found));
        Ok(found)
    }

    /// The status changes the viewer may make on the issue now.
    async fn view_status_changes(
        &self,
        views: &IssueViews,
        workflow: &WorkflowContent,
        issue: &Issue,
        assigned_to: &[UserId],
    ) -> Result<Vec<StatusChangeView>, AppError> {
        let held = &views.viewer;
        if issue.deleted_at.is_some()
            || !StatusCheck::may_change_status(held, issue.issue_type, issue.reported_by)
        {
            return Ok(Vec::new());
        }
        let own_filled = views.values.filled_fields(issue.id.into());
        let check = StatusCheck {
            held,
            person: views.actor.person.id,
            chain: views.chain.as_deref().unwrap_or_default(),
            issue_type: issue.issue_type,
            reported_by: issue.reported_by,
            may_change_status: true,
            assigned: assigned_to,
            description_filled: !issue.description.is_empty(),
            own_filled: &own_filled,
            given: &BTreeSet::new(),
        };
        self.status_changes_of(&check, workflow, &issue.status)
            .await
    }

    /// Reads the own-field values of these issues in one go, so a page of issues costs one read.
    pub(crate) async fn preload_values(
        &self,
        views: &mut IssueViews,
        issues: &[IssueId],
    ) -> Result<(), AppError> {
        let ids: Vec<EntityId> = issues.iter().map(|id| (*id).into()).collect();
        let loaded = self.load_values(RecordKind::Issue, &ids).await?;
        views.values.merge(loaded);
        Ok(())
    }

    async fn person_once(
        &self,
        views: &mut IssueViews,
        kind: PeopleKind,
        person: UserId,
    ) -> Result<PersonRef, AppError> {
        if let Some(known) = views.people.get(&(kind, person)) {
            return Ok(known.clone());
        }
        let shown = self
            .person_ref(&views.viewer, kind, PeopleContext::Ordinary, person)
            .await?;
        views.people.insert((kind, person), shown.clone());
        Ok(shown)
    }

    /// The issue as the viewer sees it. `assigned_to` are the people it is assigned to, in order.
    pub(crate) async fn issue_view(
        &self,
        views: &mut IssueViews,
        issue: &Issue,
        assigned_to: &[UserId],
    ) -> Result<IssueView, AppError> {
        let issue_type =
            views.types.get(&issue.issue_type).cloned().ok_or_else(|| {
                AppError::Internal("an issue has an issue type that is gone".into())
            })?;
        let item: EntityId = issue.id.into();
        if !views.values.has(item) {
            let loaded = self.load_values(RecordKind::Issue, &[item]).await?;
            views.values.merge(loaded);
        }
        let mut own_fields = views
            .values
            .shown(&views.viewer, Some(issue.issue_type), item);
        for (key, value) in &mut own_fields.0 {
            if let Some(kind) = views.values.kind_of(key) {
                *value = self
                    .display_value(
                        &views.actor,
                        &views.viewer,
                        &mut views.people,
                        kind,
                        std::mem::take(value),
                    )
                    .await?;
            }
        }
        // A value the viewer is not shown (an issue they do not see) is no value.
        own_fields.0.retain(|_, value| !value.is_null());
        let workflow = self.view_workflow(views, issue.issue_type).await?;
        let directory = self.workflow_directory().await?;
        let status = StatusView::of_issue(&workflow, issue, &directory);
        let status_changes = self
            .view_status_changes(views, &workflow, issue, assigned_to)
            .await?;
        let reported_by = self
            .person_once(views, PeopleKind::ReportedBy, issue.reported_by)
            .await?;
        let shown_people = if views.show_people { assigned_to } else { &[] };
        let mut people = Vec::with_capacity(shown_people.len());
        for person in shown_people {
            people.push(
                self.person_once(views, PeopleKind::AssignedTo, *person)
                    .await?,
            );
        }
        Ok(IssueView {
            id: issue.id,
            key: ReadableId::new(views.project.short_name.clone(), issue.code.clone()).to_string(),
            project: ProjectRef::from(&views.project),
            issue_type,
            title: issue.title.clone(),
            description: issue.description.clone(),
            status,
            status_changes,
            priority: issue.priority,
            reported_by,
            assigned_to: people,
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            closed_at: issue.closed_at,
            own_fields,
            version: issue.version,
        })
    }
}
