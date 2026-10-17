//! *Check access* (SEC-03, ADR-0025 decision 12): what a person can do in a project, which field
//! levels and people levels they have, and **why**, as data the browser turns into sentences.
//!
//! The decision and its reasons come from the engine in one pass, so they cannot disagree. A reason
//! never names a project the person who checks cannot see: it says the row is on *a project you
//! can't see* ([`ProjectLabel::Hidden`]).

use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::FieldKey;
use bb_core::id::{AssignmentId, ProjectId};
use bb_core::issue::IssueType;
use bb_core::offer::type_takes_issues;
use bb_core::people_kinds::PeopleKind;
use bb_policy::{
    AccessReport, Authorized, Evaluation, LevelDecision, LevelSource, LevelSubject, Masked,
    PeopleContext, Permission, PersonRef, Place, Reason, ReasonRow, Who,
};
use serde::Serialize;

use super::types::{CheckAccessQuery, RoleRef};
use crate::access::{Actor, ProjectLabel};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::IssueTypeRef;
use crate::people::GroupView;

/// Whom a row of a reason was for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReasonWho {
    /// The person who is checked.
    Person,
    /// A group of theirs.
    Group {
        /// The group.
        group: GroupView,
    },
}

/// One row of *Who has which role* that played a part.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReasonRowView {
    /// The ID of the row.
    pub assignment: AssignmentId,
    /// Whom it is for.
    pub who: ReasonWho,
    /// The project it is on, or `None` for the whole organization.
    pub project: Option<ProjectLabel>,
    /// Where it counts.
    pub applies_to: AppliesTo,
    /// Whether it comes from above the place that was checked.
    pub inherited: bool,
}

/// Why a permission is given or not, with the names filled in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReasonView {
    /// The role of a row gives it.
    GivenByRole {
        /// The row.
        row: ReasonRowView,
        /// The role.
        role: RoleRef,
    },
    /// *Decide who has which role* is kept from a row that *No access* removed.
    KeptFromRemovedRow {
        /// The row.
        row: ReasonRowView,
        /// Its role.
        role: RoleRef,
        /// The *No access* that decided.
        no_access: ReasonRowView,
    },
    /// The role of a row has it, but *No access* removed the role.
    RemovedByNoAccess {
        /// The row.
        row: ReasonRowView,
        /// Its role.
        role: RoleRef,
        /// The *No access* that decided.
        no_access: ReasonRowView,
    },
    /// The role has it, but it counts only for the whole organization.
    CountsOnlyForTheOrganization {
        /// The row.
        row: ReasonRowView,
        /// The role.
        role: RoleRef,
    },
    /// The access key does not keep it.
    NarrowedByKey,
    /// The access key does not count here.
    OutsideKeyProjects,
    /// No role gives it.
    NoRoleGivesIt,
}

/// One permission with *yes* or *no* and the reasons.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PermissionAnswer {
    /// The permission.
    pub permission: Permission,
    /// Whether it counts here.
    pub allowed: bool,
    /// Why.
    pub reasons: Vec<ReasonView>,
}

/// Where the level a role says was set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LevelPlace {
    /// For a project (the place or the nearest parent that set one).
    Project {
        /// The project, as the viewer may name it.
        project: ProjectLabel,
    },
    /// The role's own level, for the whole organization.
    RoleOwn,
    /// Nothing is set: closed.
    Unset,
}

/// What one role says about a field or a kind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoleSays<L> {
    /// The role.
    pub role: RoleRef,
    /// Its level.
    pub level: L,
    /// Where it was set.
    pub set: LevelPlace,
}

/// A field or a kind with its level and what each role says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LevelAnswer<K, L> {
    /// The field or kind.
    pub subject: K,
    /// The name of an own field; none for a field of the catalogue or a kind.
    pub name: Option<String>,
    /// The level the person has.
    pub level: L,
    /// What each role that counts says.
    pub roles: Vec<RoleSays<L>>,
    /// An access key lowered the level to *Can see*.
    pub capped_by_key: bool,
}

/// The answer of *Check access*.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AccessAnswer {
    /// The person who was checked, shown as the viewer may see them.
    pub person: PersonRef,
    /// The project that was checked, or `None` for the whole organization.
    pub place: Option<ProjectLabel>,
    /// Every permission.
    pub permissions: Vec<PermissionAnswer>,
    /// Every field.
    pub fields: Vec<LevelAnswer<FieldKey, FieldLevel>>,
    /// Every kind of people information.
    pub kinds: Vec<LevelAnswer<PeopleKind, PeopleLevel>>,
    /// The issue type the field levels are for; none for all types.
    pub issue_type: Option<IssueTypeRef>,
    /// Whether the field levels of the person differ by issue type: some type has a level for some
    /// field that the others do not. The screen asks *"Which issue type?"* only then.
    pub differs_by_type: bool,
    /// The issue types the question may ask about: those the place offers and that are not
    /// retired, which the answer judged. The screen lists exactly these. Empty unless
    /// `differs_by_type`.
    pub issue_types: Vec<IssueTypeRef>,
}

impl Masked for AccessAnswer {}

impl<S: AuthStore> Ctx<'_, S> {
    async fn reason_row(
        &self,
        checker: &Actor,
        row: &ReasonRow,
    ) -> Result<ReasonRowView, AppError> {
        let who = match row.who {
            Who::Person => ReasonWho::Person,
            Who::Group(id) => ReasonWho::Group {
                group: self.group_view(id).await?,
            },
        };
        Ok(ReasonRowView {
            assignment: row.assignment,
            who,
            project: self.place_label(checker, row.project).await?,
            applies_to: row.applies_to,
            inherited: row.inherited,
        })
    }

    async fn reason_view(&self, checker: &Actor, reason: &Reason) -> Result<ReasonView, AppError> {
        Ok(match reason {
            Reason::GivenByRole { row, role } => ReasonView::GivenByRole {
                row: self.reason_row(checker, row).await?,
                role: self.role_ref(*role).await?,
            },
            Reason::KeptFromRemovedRow {
                row,
                role,
                no_access,
            } => ReasonView::KeptFromRemovedRow {
                row: self.reason_row(checker, row).await?,
                role: self.role_ref(*role).await?,
                no_access: self.reason_row(checker, no_access).await?,
            },
            Reason::RemovedByNoAccess {
                row,
                role,
                no_access,
            } => ReasonView::RemovedByNoAccess {
                row: self.reason_row(checker, row).await?,
                role: self.role_ref(*role).await?,
                no_access: self.reason_row(checker, no_access).await?,
            },
            Reason::CountsOnlyForTheOrganization { row, role } => {
                ReasonView::CountsOnlyForTheOrganization {
                    row: self.reason_row(checker, row).await?,
                    role: self.role_ref(*role).await?,
                }
            }
            Reason::NarrowedByKey => ReasonView::NarrowedByKey,
            Reason::OutsideKeyProjects => ReasonView::OutsideKeyProjects,
            Reason::NoRoleGivesIt => ReasonView::NoRoleGivesIt,
        })
    }

    /// Where a level was set, as the person who asks may name it. `named` keeps the projects
    /// already named for this answer, so a table that names one project often asks once.
    pub(crate) async fn level_place(
        &self,
        actor: &Actor,
        source: LevelSource,
        named: &mut Vec<(ProjectId, ProjectLabel)>,
    ) -> Result<LevelPlace, AppError> {
        Ok(match source {
            LevelSource::Project(project) => {
                let known = named.iter().find(|(known, _)| *known == project);
                let label = if let Some((_, label)) = known {
                    label.clone()
                } else {
                    let label = self.project_label(actor, project).await?;
                    named.push((project, label.clone()));
                    label
                };
                LevelPlace::Project { project: label }
            }
            LevelSource::RoleOwn => LevelPlace::RoleOwn,
            LevelSource::Unset => LevelPlace::Unset,
        })
    }

    async fn level_answer<K: LevelSubject>(
        &self,
        checker: &Actor,
        decision: &LevelDecision<K>,
        named: &mut Vec<(ProjectId, ProjectLabel)>,
    ) -> Result<LevelAnswer<K, K::Level>, AppError> {
        let mut roles = Vec::with_capacity(decision.roles.len());
        for says in &decision.roles {
            roles.push(RoleSays {
                role: self.role_ref(says.role).await?,
                level: says.level,
                set: self.level_place(checker, says.source, named).await?,
            });
        }
        Ok(LevelAnswer {
            subject: decision.subject,
            name: None,
            level: decision.level,
            roles,
            capped_by_key: decision.capped_by_key,
        })
    }

    async fn access_answer(
        &self,
        checker: &Actor,
        report: &AccessReport,
        person: PersonRef,
        own_names: &std::collections::HashMap<FieldKey, String>,
        by_type: (Option<IssueTypeRef>, bool, Vec<IssueTypeRef>),
    ) -> Result<AccessAnswer, AppError> {
        let mut permissions = Vec::with_capacity(report.permissions.len());
        for decision in &report.permissions {
            let mut reasons = Vec::with_capacity(decision.reasons.len());
            for reason in &decision.reasons {
                reasons.push(self.reason_view(checker, reason).await?);
            }
            permissions.push(PermissionAnswer {
                permission: decision.permission,
                allowed: decision.allowed,
                reasons,
            });
        }
        let mut named = Vec::new();
        let mut fields = Vec::with_capacity(report.fields.len());
        for decision in &report.fields {
            // An own field the checker may not know is not told at all.
            if decision.subject.own().is_some() && !own_names.contains_key(&decision.subject) {
                continue;
            }
            let mut answer = self.level_answer(checker, decision, &mut named).await?;
            answer.name = own_names.get(&answer.subject).cloned();
            fields.push(answer);
        }
        let mut kinds = Vec::with_capacity(report.kinds.len());
        for decision in &report.kinds {
            kinds.push(self.level_answer(checker, decision, &mut named).await?);
        }
        Ok(AccessAnswer {
            person,
            place: self.place_label(checker, report.place).await?,
            permissions,
            fields,
            kinds,
            issue_type: by_type.0,
            differs_by_type: by_type.1,
            issue_types: by_type.2,
        })
    }

    /// Whether the person's field levels differ by issue type, among the fields the checker may
    /// know: some type has another level for some field than the answer for all types gives
    /// (`all`). Only the types the place offers and that are not retired count: they are the ones
    /// the screen can ask about.
    fn levels_differ_by_type(
        theirs: &Evaluation,
        all: &AccessReport,
        types: &[&IssueType],
        own_names: &std::collections::HashMap<FieldKey, String>,
    ) -> bool {
        if !theirs.has_levels_per_issue_type() {
            return false;
        }
        all.fields
            .iter()
            .filter(|decision| {
                decision.subject.own().is_none() || own_names.contains_key(&decision.subject)
            })
            .any(|decision| {
                types.iter().any(|found| {
                    theirs.field_level_for(Some(found.id), decision.subject) != decision.level
                })
            })
    }
}

/// What a person can do in a project (or in the whole organization), which field levels and
/// people levels they have, and why.
///
/// # Errors
/// `access.forbidden` without *Check access* there (`resource.not_found` for a project the person
/// may not know about), `resource.not_found` for an unknown person to check, and for a person
/// without a role there when the checker does not manage people or roles.
pub async fn check_access<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CheckAccessQuery,
) -> Result<Authorized<AccessAnswer>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let place = Place::from(input.place);
    let checker = ctx.require(&actor, Permission::CheckAccess, place).await?;
    ctx.store()
        .person(input.person)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let theirs = ctx.evaluate(input.person, place).await?;
    // Whoever does not manage people or roles reads the access of the people they could choose:
    // those who have a role there. Anybody else is as good as not there.
    if !checker.manages_people() && theirs.roles().is_empty() {
        return Err(AppError::Refused(ErrorCode::NotFound));
    }
    let person = ctx
        .person_ref(
            &checker,
            PeopleKind::ProjectMembers,
            PeopleContext::Management,
            input.person,
        )
        .await?;
    let own_names = ctx.known_field_names(&actor, &checker, input.place).await?;
    // The checker asks about the issue types they may know (`offers/known.rs`).
    let shown = ctx.shown_to(&actor).await?;
    let types = ctx
        .store()
        .issue_types()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let types = ctx.known_types(&shown, types).await?;
    let asked = input
        .issue_type
        .map(|wanted| {
            types
                .iter()
                .find(|found| found.id == wanted)
                .ok_or(AppError::Refused(ErrorCode::NotFound))
        })
        .transpose()?;
    let offers = ctx
        .store()
        .offers()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let chain = shown.chain(place);
    let choices: Vec<&IssueType> = types
        .iter()
        .filter(|found| type_takes_issues(found, &offers, &chain).is_ok())
        .collect();
    // One report for all types; the report for one type is built only when one is asked.
    let all = theirs.report();
    let differs = Ctx::<S>::levels_differ_by_type(&theirs, &all, &choices, &own_names);
    // The screen asks which type is meant only when the levels differ by type, so only then does
    // the answer carry the choices.
    let by_type = (
        asked.map(IssueTypeRef::from),
        differs,
        if differs {
            choices
                .iter()
                .map(|found| IssueTypeRef::from(*found))
                .collect()
        } else {
            Vec::new()
        },
    );
    let report = match asked {
        Some(found) => theirs.report_for(Some(found.id)),
        None => all,
    };
    let answer = ctx
        .access_answer(&actor, &report, person, &own_names, by_type)
        .await?;
    Ok(checker.authorize(answer))
}
