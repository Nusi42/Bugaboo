//! The three kinds of AI and their switches (WP-15, ADR-0013 amendment M7): the assistant follows
//! `ai.enabled`, AI apps `ai-apps.enabled` and agents `agents.enabled`, each for the organization
//! and for each project, and none depends on another. The one place that says where a switch is
//! off: the nearest value along the chain of a project decides (its own, else its parent's, up to
//! the organization's), so a sub-project that is off stays off when the work names its parent, and
//! one that is switched on again is on below a parent that is off.

use std::collections::{BTreeMap, BTreeSet};

use bb_core::ai::ToolCaller;
use bb_core::id::ProjectId;
use bb_core::settings::{AGENTS_ENABLED, AI_APPS_ENABLED, AI_ENABLED, Bool, Setting, SettingValue};
use bb_policy::Tree;

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The switch of a kind of AI.
pub(crate) fn switch_of(kind: ToolCaller) -> &'static Setting<Bool> {
    match kind {
        ToolCaller::Assistant => &AI_ENABLED,
        ToolCaller::AiApp => &AI_APPS_ENABLED,
        ToolCaller::Agent => &AGENTS_ENABLED,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether a kind of AI is on for the organization at all.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn kind_on_in_the_organization(
        &self,
        kind: ToolCaller,
    ) -> Result<bool, AppError> {
        Ok(self.setting_layers_of(None).await?.yes(switch_of(kind)))
    }

    /// The projects of `tree` where the kind is switched off: the nearest value in the chain of a
    /// project decides, and the organization's value counts where no project of the chain has one.
    /// One read of the values of the projects, however many there are.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn projects_with_kind_off(
        &self,
        kind: ToolCaller,
        tree: &Tree,
    ) -> Result<BTreeSet<ProjectId>, AppError> {
        let switch = switch_of(kind);
        let own: BTreeMap<ProjectId, bool> = self
            .store()
            .project_values(switch.definition())
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .filter_map(|(project, value)| match value {
                SettingValue::Bool(on) => Some((project, on)),
                _ => None,
            })
            .collect();
        let organization_on = self.kind_on_in_the_organization(kind).await?;
        Ok(tree
            .projects()
            .filter(|project| {
                let chain = tree.chain(*project).unwrap_or_else(|| vec![*project]);
                // `chain` runs from the top down: the nearest value is the last one set.
                let on = chain
                    .iter()
                    .rev()
                    .find_map(|each| own.get(each).copied())
                    .unwrap_or(organization_on);
                !on
            })
            .collect())
    }
}
