//! Issues as one viewer sees them: the project and the type by name, the people as *Who sees
//! people* says, the readable ID as it is now.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::fields::{ASSIGNED_TO_PEOPLE, REPORTED_BY_PEOPLE};
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
use crate::tags::{LabelCatalogue, LabelRef};
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
    /// The labels of the issues shown so far, by issue, in the order they were put on. Read once
    /// for the page; an issue not in it has none.
    /// `None` when the labels were not asked for.
    labels: Option<Arc<HashMap<IssueId, Vec<LabelRef>>>>,
}

/// What a page of issues shows besides what every issue has: each is read only when it is asked
/// for.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Shown {
    /// The people each issue is assigned to.
    pub(crate) people: bool,
    /// The labels of each issue.
    pub(crate) labels: bool,
}

/// A project that a page of issues has issues of: the projects from the top down to it, what the
/// viewer may do there, and the project itself.
pub(crate) struct PagePlace {
    pub(crate) chain: Vec<ProjectId>,
    pub(crate) held: Arc<Evaluation>,
    pub(crate) project: Project,
}

impl IssueViews {
    /// Gives the views what the page has read already, so they read nothing of their own: the
    /// projects from the top down to the project, and the values of own fields of all the issues
    /// of the page (a page over many projects costs the same statements as one over a single
    /// project).
    pub(crate) fn with_page(mut self, chain: Vec<ProjectId>, values: ItemValues) -> Self {
        self.chain = Some(chain);
        self.values = values;
        self
    }

    /// Leaves the people an issue is assigned to out of the views.
    pub(crate) const fn without_people(mut self) -> Self {
        self.show_people = false;
        self
    }

    /// Gives the views the labels of the issues of the page.
    pub(crate) fn with_labels(mut self, labels: Arc<HashMap<IssueId, Vec<LabelRef>>>) -> Self {
        self.labels = Some(labels);
        self
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The views of a page of issues, each with what the viewer may do in its project. The one way
    /// a page is made, for the list of a project and for a search: the people the issues are
    /// assigned to, the values of own fields of all of them and the rights and the chain of every
    /// project of the page are read once, however many projects the page spans (PERF-N1).
    pub(crate) async fn issue_items(
        &self,
        actor: &Actor,
        issues: &[Issue],
        places: &HashMap<ProjectId, PagePlace>,
        shown: Shown,
    ) -> Result<Vec<(Arc<Evaluation>, IssueView)>, AppError> {
        let ids: Vec<IssueId> = issues.iter().map(|issue| issue.id).collect();
        let assigned = self
            .store()
            .assigned_people_of(&ids)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let values = self
            .load_values(
                RecordKind::Issue,
                &ids.iter().map(|id| (*id).into()).collect::<Vec<EntityId>>(),
            )
            .await?;
        // One statement for the links of the whole page and one for the catalogue of labels.
        let labels = if shown.labels {
            Some(Arc::new(self.labels_of_page(&ids).await?))
        } else {
            None
        };
        let mut views: HashMap<ProjectId, IssueViews> = HashMap::new();
        let mut projects: Vec<ProjectId> = issues.iter().map(|issue| issue.project).collect();
        projects.sort_unstable();
        projects.dedup();
        for project in projects {
            let place = places
                .get(&project)
                .ok_or(AppError::Refused(ErrorCode::NotFound))?;
            let mut made = self
                .issue_views(actor, Arc::clone(&place.held), place.project.clone())
                .await?
                .with_page(place.chain.clone(), values.clone());
            if let Some(labels) = &labels {
                made = made.with_labels(Arc::clone(labels));
            }
            if !shown.people {
                made = made.without_people();
            }
            views.insert(project, made);
        }
        let mut items = Vec::with_capacity(issues.len());
        for issue in issues {
            let place = places
                .get(&issue.project)
                .ok_or(AppError::Refused(ErrorCode::NotFound))?;
            let Some(shown) = views.get_mut(&issue.project) else {
                continue;
            };
            let people: Vec<UserId> = assigned
                .iter()
                .filter(|each| each.issue == issue.id)
                .map(|each| each.person)
                .collect();
            items.push((
                Arc::clone(&place.held),
                self.issue_view(shown, issue, &people).await?,
            ));
        }
        Ok(items)
    }

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
            .all_issue_types()
            .await?
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
            labels: None,
        })
    }

    /// The labels on each of these issues, in the order they were put there: the links of all of
    /// them in one statement, and the labels they point to in another (PERF-N1). Whether the
    /// viewer may see labels at all is the mask of each issue's project, applied by the answer.
    ///
    /// # Errors
    /// When the store fails.
    pub(crate) async fn labels_of_page(
        &self,
        ids: &[IssueId],
    ) -> Result<HashMap<IssueId, Vec<LabelRef>>, AppError> {
        let internal = |error| internal::<S>(&error);
        let links = self
            .store()
            .labels_of_issues(ids.to_vec())
            .await
            .map_err(internal)?;
        let mut found: HashMap<IssueId, Vec<LabelRef>> = HashMap::new();
        if links.is_empty() {
            return Ok(found);
        }
        let labels = self.store().labels().await.map_err(internal)?;
        let catalogue = LabelCatalogue::new(&labels);
        for link in &links {
            if let Some(label) = catalogue.of(link) {
                found.entry(link.issue).or_default().push(label);
            }
        }
        Ok(found)
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
            .person_once(views, REPORTED_BY_PEOPLE, issue.reported_by)
            .await?;
        let shown_people = if views.show_people { assigned_to } else { &[] };
        let mut people = Vec::with_capacity(shown_people.len());
        for person in shown_people {
            people.push(self.person_once(views, ASSIGNED_TO_PEOPLE, *person).await?);
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
            labels: views
                .labels
                .as_ref()
                .map(|page| page.get(&issue.id).cloned().unwrap_or_default()),
            version: issue.version,
        })
    }
}
