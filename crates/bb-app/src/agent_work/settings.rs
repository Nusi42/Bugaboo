//! The settings of agent work, read where the use cases need them (ADR-0018, amendment M7).

use bb_core::id::ProjectId;
use bb_core::settings::{
    AGENTS_ACCEPT_BY_ITSELF, AGENTS_HAND_OVER_DAYS, AGENTS_RULE_DAYS, AGENTS_RULES_NEVER_RUN_OUT,
    SettingValue,
};

use crate::auth::{AppError, AuthStore, Ctx, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether the project accepts the result of an agent by itself when its checks pass: the
    /// nearest value in the chain of the project, else the organization's, which is off at the
    /// start (M7-Q8).
    pub(crate) async fn project_accepts_by_itself(
        &self,
        project: ProjectId,
    ) -> Result<bool, AppError> {
        let own = self
            .store()
            .project_values(AGENTS_ACCEPT_BY_ITSELF.definition())
            .await
            .map_err(|error| internal::<S>(&error))?;
        for each in self.chain_of(project).await?.into_iter().rev() {
            if let Some(SettingValue::Bool(on)) = own
                .iter()
                .find(|(found, _)| *found == each)
                .map(|(_, value)| value)
            {
                return Ok(*on);
            }
        }
        let layers = self.setting_layers_of(None).await?;
        Ok(layers.get(&AGENTS_ACCEPT_BY_ITSELF).unwrap_or(false))
    }

    /// After how many days a hand-over ends when its maker set no end.
    pub(crate) async fn hand_over_days(&self) -> Result<i64, AppError> {
        let layers = self.setting_layers_of(None).await?;
        Ok(layers
            .get(&AGENTS_HAND_OVER_DAYS)
            .unwrap_or(bb_core::agent_work::DEFAULT_HAND_OVER_DAYS))
    }

    /// After how many days a standing rule ends when its maker set no end.
    pub(crate) async fn rule_days(&self) -> Result<i64, AppError> {
        let layers = self.setting_layers_of(None).await?;
        Ok(layers
            .get(&AGENTS_RULE_DAYS)
            .unwrap_or(bb_core::agent_work::DEFAULT_RULE_DAYS))
    }

    /// The job that gives the rules that never run out an end, queued with the change of the
    /// setting that forbids them: none for any other change.
    ///
    /// # Errors
    /// An internal error when the job cannot be made.
    pub(crate) fn rules_get_an_end_job(
        &self,
        scope: bb_core::settings::Scope,
        key: &str,
        value: &SettingValue,
    ) -> Result<Option<crate::jobs::QueuedJob>, AppError> {
        let forbids = scope == bb_core::settings::Scope::Organization
            && key == AGENTS_RULES_NEVER_RUN_OUT.definition().key()
            && *value == SettingValue::Bool(false);
        forbids
            .then(|| {
                self.queued(
                    crate::jobs::JobKind::RulesGetAnEnd,
                    serde_json::json!({}),
                    self.now(),
                )
            })
            .transpose()
    }

    /// Whether the organization allows standing rules that never run out (off at the start).
    pub(crate) async fn rules_never_run_out(&self) -> Result<bool, AppError> {
        let layers = self.setting_layers_of(None).await?;
        Ok(layers.get(&AGENTS_RULES_NEVER_RUN_OUT).unwrap_or(false))
    }
}
