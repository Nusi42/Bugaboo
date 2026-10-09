//! Issue IDs, counting or random (ISS-01a, ADR-0021): which mode a project is in, how the code of
//! a new issue is given out, and how every issue gets a new code when the mode changes.
//!
//! The mode is the setting `issues.id-mode`: the organization's value is the default, a project
//! can choose its own, and a sub-project follows its parent until it chooses (the layers of
//! ADR-0018). A code is given out inside the change that makes the issue, so two writers never
//! get the same one: a counting code comes from the counter of the project (the store raises it
//! in the transaction), a random one is drawn from the secure source and drawn again when the
//! project has it (or had it) already. When the mode of a project changes, every issue of it
//! gets a code of the new mode in the same change, and the old code stays with the issue.

use bb_core::id::{IdSource, ProjectId};
use bb_core::issue::IssueAlias;
use bb_core::issue_key::{IdMode, IssueCode, code_length, longest_code};
use bb_core::settings::{
    ISSUES_ID_LENGTH, ISSUES_ID_MODE, Layers, Scope, SettingDefinition, SettingValue,
};
use bb_core::text::CodeDraws;
use bb_policy::Tree;
use jiff::Timestamp;

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// How the next codes of a project are given out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CodePlan {
    /// Counting or random.
    pub(crate) mode: IdMode,
    /// The length a random code starts with.
    pub(crate) length: usize,
}

impl CodePlan {
    /// The plan that counts for the project whose layers these are. A value that is not a mode
    /// counts as the default mode (the registry accepts nothing else, so this is a stored value
    /// that was made by hand).
    pub(crate) fn of(layers: &Layers) -> Self {
        let value = layers.resolve(ISSUES_ID_MODE.definition()).value;
        let mode = ISSUES_ID_MODE
            .resolve(Some(&value))
            .ok()
            .and_then(|text| IdMode::from_code(&text))
            .unwrap_or(IdMode::Random);
        let length = ISSUES_ID_LENGTH
            .resolve(Some(&layers.resolve(ISSUES_ID_LENGTH.definition()).value))
            .unwrap_or_default();
        Self {
            mode,
            length: code_length(length),
        }
    }
}

/// The new code of an issue of a project, given out inside the change that needs it.
///
/// # Errors
/// The error of the store, or its "aborted" error when even the longest random codes were all
/// taken (which would mean the random source is broken).
pub(crate) async fn new_code_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    project: ProjectId,
    plan: CodePlan,
    source: &mut impl IdSource,
) -> Result<IssueCode, S::Error> {
    match plan.mode {
        IdMode::Counting => Ok(IssueCode::counting(
            S::next_counting_number(transaction, project).await?,
        )),
        IdMode::Random => {
            for symbols in CodeDraws::new(source, plan.length, longest_code()) {
                let Some(code) = IssueCode::random(&symbols) else {
                    continue;
                };
                if !S::issue_code_taken_in(transaction, project, code.clone()).await? {
                    return Ok(code);
                }
            }
            Err(S::aborted())
        }
    }
}

/// A project whose issues get a new code of a new mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Renewal {
    pub(crate) project: ProjectId,
    pub(crate) plan: CodePlan,
}

/// What happens to the value of the mode on a project.
#[derive(Clone, Debug)]
pub(crate) enum ModeChange {
    /// The project chooses this value.
    Set(SettingValue),
    /// The project goes back to the value of the layer above (*Reset to default*).
    Reset,
}

/// The layers of `project` as they would be after `change` of the mode on `on`.
fn after(
    layers: &Layers,
    on: ProjectId,
    definition: &SettingDefinition,
    change: &ModeChange,
) -> Layers {
    match change {
        ModeChange::Reset => {
            let mut lower = layers.clone();
            for (id, values) in &mut lower.projects {
                if *id == on {
                    values.remove(definition.key());
                }
            }
            lower
        }
        ModeChange::Set(value) => {
            let mut set = layers.clone();
            for (id, values) in &mut set.projects {
                if *id == on {
                    values.insert(definition.key().to_owned(), value.clone());
                }
            }
            set
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// How the codes of `project` are given out now.
    pub(crate) async fn code_plan(&self, project: ProjectId) -> Result<CodePlan, AppError> {
        let layers = self
            .store()
            .setting_layers(Some(project), None)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(CodePlan::of(&layers))
    }

    /// The projects whose issues get new codes when `change` is made to the setting
    /// `definition` on `scope`: only the mode of a project changes any, and then the project
    /// itself and the projects inside it that follow it do, each with a code of its new mode.
    pub(crate) async fn mode_renewals(
        &self,
        scope: Scope,
        definition: &SettingDefinition,
        change: &ModeChange,
    ) -> Result<Vec<Renewal>, AppError> {
        let Scope::Project(on) = scope else {
            return Ok(Vec::new());
        };
        if definition.key() != ISSUES_ID_MODE.definition().key() {
            return Ok(Vec::new());
        }
        let projects = self
            .store()
            .project_tree()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut renewals = Vec::new();
        for project in Tree::of(&projects).subtree(on) {
            let layers = self
                .store()
                .setting_layers(Some(project), None)
                .await
                .map_err(|error| internal::<S>(&error))?;
            let (before, to) = (
                CodePlan::of(&layers),
                CodePlan::of(&after(&layers, on, ISSUES_ID_MODE.definition(), change)),
            );
            if before.mode != to.mode {
                renewals.push(Renewal { project, plan: to });
            }
        }
        Ok(renewals)
    }
}

/// Gives every issue of the project a code of the new mode and keeps the old code with the issue
/// (`issue_alias`), inside the change that changed the mode.
///
/// # Errors
/// The error of the store.
pub(crate) async fn renew_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    renewal: Renewal,
    source: &mut impl IdSource,
    at: Timestamp,
) -> Result<(), S::Error> {
    for (issue, old) in S::issue_codes_in(transaction, renewal.project).await? {
        let code = new_code_in::<S>(transaction, renewal.project, renewal.plan, source).await?;
        S::set_issue_code(transaction, issue, &code).await?;
        S::insert_issue_alias(
            transaction,
            &IssueAlias {
                project: renewal.project,
                code: old,
                issue,
                retired_at: at,
            },
        )
        .await?;
    }
    Ok(())
}
