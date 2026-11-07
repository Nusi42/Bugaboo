//! *Never sent to AI* (AI-03, ADR-0013, ADR-0025 decision 8): fields and projects that the
//! assistant, AI apps and agents never read, even when the person can.
//!
//! An own field carries its mark in its definition (`never_to_assistant`, set with the field); a
//! built-in field and a project have a row of their own (`NeverSentMark`). A project's mark covers
//! the projects inside it, like *Offered in*. Only administrators set it: *Change organization
//! settings* for a field, *Change project settings* for a project. [`Withheld`] is the one answer
//! to "is this kept from AI", read once for a request; the context builder and the search of the
//! tools both ask it.

use std::collections::BTreeSet;
use std::future::Future;

use bb_core::ErrorCode;
use bb_core::ai::{NeverSentMark, NeverSentSubject};
use bb_core::event::{EntityRef, RecordKind};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::ProjectId;
use bb_core::project::Project;
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Masked, Permission, Place};
use serde::Serialize;

use crate::auth::{
    AppError, AuthStore, Ctx, Stops, by, change_failed, entity, fixed_entity, internal, set_to,
};
use crate::fields::FieldStore;
use crate::projects::{ProjectRef, ProjectStore};
use crate::search::FieldTarget;
use crate::store_macros::{queries, writes};

/// The most projects with a mark one answer lists; the answer says when it was cut.
pub const NEVER_SENT_PROJECTS_MAX: usize = 200;

/// What the use cases of *Never sent to AI* need from storage.
pub trait NeverSentStore: FieldStore + ProjectStore + 'static {
    queries! {
        /// Every row of the marks of built-in fields and projects.
        fn never_sent_marks() -> Vec<NeverSentMark> = never_sent_marks_in;
    }

    writes! {
        /// Writes the mark when the stored row is still at the version before `mark` (version 0
        /// when there is no row), and says whether it was.
        fn put_never_sent_mark(mark: &NeverSentMark) -> bool;
    }
}

/// What is kept from AI for one request: the fields, and the projects with every project inside
/// them. Made once ([`Ctx::withheld`]) and handed to whatever builds what the AI reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Withheld {
    fields: BTreeSet<FieldKey>,
    projects: BTreeSet<ProjectId>,
}

impl Withheld {
    /// Whether a field is kept from AI.
    #[must_use]
    pub fn hides_field(&self, field: FieldKey) -> bool {
        self.fields.contains(&field)
    }

    /// Whether a project, or a project it is inside, is kept from AI.
    #[must_use]
    pub fn hides_project(&self, project: ProjectId) -> bool {
        self.projects.contains(&project)
    }

    /// Whether what a word of a search reads is kept from AI. A status group and the day an issue
    /// closed are read from the status.
    pub(crate) fn hides_target(&self, target: FieldTarget) -> bool {
        match target {
            FieldTarget::Own { field, .. } => self.hides_field(FieldKey::Own(field)),
            other => other
                .catalogue_field()
                .is_some_and(|field| self.hides_field(FieldKey::BuiltIn(field))),
        }
    }

    /// The built-in texts a bare `contains` has to leave out.
    pub(crate) fn text(&self) -> crate::search::WithheldText {
        crate::search::WithheldText {
            title: self.hides_field(FieldKey::BuiltIn(Field::Title)),
            description: self.hides_field(FieldKey::BuiltIn(Field::Description)),
        }
    }

    /// The projects kept from AI, with the projects inside them.
    #[must_use]
    pub fn projects(&self) -> &BTreeSet<ProjectId> {
        &self.projects
    }

    /// What is kept from AI, from the rows and the tree: the marked projects and every project
    /// below one.
    pub(crate) fn of(
        marks: &[NeverSentMark],
        own: impl IntoIterator<Item = FieldKey>,
        tree: &[Project],
    ) -> Self {
        let mut fields: BTreeSet<FieldKey> = own.into_iter().collect();
        let mut projects: BTreeSet<ProjectId> = BTreeSet::new();
        for mark in marks.iter().filter(|mark| mark.marked) {
            match mark.subject {
                NeverSentSubject::Field(field) => {
                    fields.insert(FieldKey::BuiltIn(field));
                }
                NeverSentSubject::Project(project) => {
                    projects.insert(project);
                }
            }
        }
        // The tree is not in any order: go on until no project is added.
        loop {
            let before = projects.len();
            for project in tree {
                if project
                    .parent
                    .is_some_and(|parent| projects.contains(&parent))
                {
                    projects.insert(project.id);
                }
            }
            if projects.len() == before {
                break;
            }
        }
        Self { fields, projects }
    }
}

/// The record a mark is in the events: one fixed record for each field and each project.
fn mark_record(subject: NeverSentSubject) -> Result<EntityRef, AppError> {
    let name = match subject {
        NeverSentSubject::Field(field) => format!("never-sent-to-ai.field.{}", field.code()),
        NeverSentSubject::Project(project) => format!("never-sent-to-ai.project.{project}"),
    };
    fixed_entity(RecordKind::Setting, &name)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The version a mark has now, when `record` is the record of one: a field or a project without
    /// a row is at version 0. For the answer to a change on a version that is not the current one.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn never_sent_version(
        &self,
        record: &EntityRef,
    ) -> Result<Option<i64>, AppError> {
        let marks = self
            .store()
            .never_sent_marks()
            .await
            .map_err(|error| internal::<S>(&error))?;
        for mark in &marks {
            if mark_record(mark.subject)? == *record {
                return Ok(Some(mark.version));
            }
        }
        for field in Field::ALL {
            if mark_record(NeverSentSubject::Field(*field))? == *record {
                return Ok(Some(0));
            }
        }
        let tree = self
            .store()
            .project_tree()
            .await
            .map_err(|error| internal::<S>(&error))?;
        for project in &tree {
            if mark_record(NeverSentSubject::Project(project.id))? == *record {
                return Ok(Some(0));
            }
        }
        Ok(None)
    }

    /// What is kept from AI in this organization. It does not depend on who asks: the mark applies
    /// even when the person can read the field.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn withheld(&self) -> Result<Withheld, AppError> {
        let failed = |error| internal::<S>(&error);
        let marks = self.store().never_sent_marks().await.map_err(failed)?;
        let own = self
            .store()
            .field_definitions()
            .await
            .map_err(failed)?
            .into_iter()
            .filter(|definition| definition.field.never_to_assistant)
            .map(|definition| FieldKey::Own(definition.field.id));
        let tree = self.store().project_tree().await.map_err(failed)?;
        Ok(Withheld::of(&marks, own, &tree))
    }
}

/// Ask which built-in fields and projects are marked *Never sent to AI*.
pub struct NeverSentQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// One built-in field and whether it is kept from AI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NeverSentField {
    /// The field.
    pub field: Field,
    /// Whether it is kept from AI.
    pub marked: bool,
    /// The version to change it on.
    pub version: i64,
}

/// One project that is marked, or was and is not any more.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NeverSentProject {
    /// The project.
    pub project: ProjectRef,
    /// Whether it is kept from AI.
    pub marked: bool,
    /// The version to change it on.
    pub version: i64,
}

/// The marks the person may know about: every built-in field, and the projects they may see that
/// have a row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NeverSentList {
    /// The built-in fields, in the order of the catalogue.
    pub fields: Vec<NeverSentField>,
    /// The projects with a mark, the ones the person sees, at most [`NEVER_SENT_PROJECTS_MAX`].
    pub projects: Vec<NeverSentProject>,
    /// Whether more projects than that were left out.
    pub truncated: bool,
}

impl Masked for NeverSentList {}

/// What a mark is put on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NeverSentTarget {
    /// A built-in field.
    Field(Field),
    /// A project, by its ID or short name.
    Project(String),
}

/// Mark a built-in field or a project *Never sent to AI*, or take the mark away.
pub struct SetNeverSent {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What to mark.
    pub target: NeverSentTarget,
    /// Whether it is kept from AI from now on.
    pub marked: bool,
    /// The version the person looked at; 0 for a subject that has no row yet.
    pub version: i64,
}

/// The marks the person may know about.
///
/// # Errors
/// An internal error when the store fails.
pub async fn never_sent_list<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: NeverSentQuery,
) -> Result<Authorized<NeverSentList>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let marks = ctx
        .store()
        .never_sent_marks()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let fields = Field::ALL
        .iter()
        .map(|field| {
            let row = marks
                .iter()
                .find(|mark| mark.subject == NeverSentSubject::Field(*field));
            NeverSentField {
                field: *field,
                marked: row.is_some_and(|mark| mark.marked),
                version: row.map_or(0, |mark| mark.version),
            }
        })
        .collect();
    let shown = ctx.shown_to(&actor).await?;
    let mut projects = Vec::new();
    for mark in &marks {
        let NeverSentSubject::Project(id) = mark.subject else {
            continue;
        };
        if let Some(project) = shown.stored(id) {
            projects.push(NeverSentProject {
                project: ProjectRef::from(project),
                marked: mark.marked,
                version: mark.version,
            });
        }
    }
    let truncated = projects.len() > NEVER_SENT_PROJECTS_MAX;
    projects.truncate(NEVER_SENT_PROJECTS_MAX);
    Ok(held.authorize(NeverSentList {
        fields,
        projects,
        truncated,
    }))
}

/// Marks a built-in field or a project *Never sent to AI*, or takes the mark away. Only people who
/// may change the settings of the organization (a field) or of the project (a project) may.
///
/// # Errors
/// `resource.not-found` for a project that is not there for the person, `access.forbidden` without
/// the permission, `edit.version-changed` when somebody changed the mark since it was looked at.
pub async fn set_never_sent<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetNeverSent,
) -> Result<Authorized<NeverSentList>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (subject, place) = match &input.target {
        NeverSentTarget::Field(field) => {
            ctx.require(
                &actor,
                Permission::ChangeOrganizationSettings,
                Place::Organization,
            )
            .await?;
            (NeverSentSubject::Field(*field), Place::Organization)
        }
        NeverSentTarget::Project(address) => {
            let found = ctx
                .find_project(address)
                .await?
                .ok_or(AppError::Refused(ErrorCode::NotFound))?;
            let place = Place::Project(found.project.id);
            // A project the person may not know is answered as one that is not there.
            let held = ctx.evaluation_of(&actor, place).await?;
            if !held.sees_project() {
                return Err(AppError::Refused(ErrorCode::NotFound));
            }
            ctx.require(&actor, Permission::ChangeProjectSettings, place)
                .await?;
            (NeverSentSubject::Project(found.project.id), place)
        }
    };
    let mark = NeverSentMark {
        subject,
        marked: input.marked,
        version: input.version + 1,
    };
    let record = mark_record(subject)?;
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![set_to("never-sent-to-ai", input.marked.into())],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::put_never_sent_mark(transaction, &mark).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| {
            change_failed::<S>(
                &error,
                Stops::new(ErrorCode::VersionChanged, ErrorCode::VersionChanged),
            )
            .about(&record, place, actor.person.id)
        })?;
    ctx.note(
        SecurityAction::NeverSentToAiChanged,
        Some(actor.person.id),
        Some(entity(RecordKind::Setting, event.entity.id)),
    )
    .await;
    never_sent_list(
        ctx,
        NeverSentQuery {
            session_token: input.session_token,
        },
    )
    .await
}
