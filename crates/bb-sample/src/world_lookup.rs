//! What every step of the sample asks the store for by name: a group, a project, a starting role,
//! and the one change that hides an own field from Contributors. One place, so the organization,
//! the saved searches and the large dataset do not each write them.

use bb_app::access::{PolicyStore as _, StartingRole};
use bb_app::roles::{SetFieldLevel, set_level};
use bb_core::access::FieldLevel;
use bb_core::fields::FieldKey;
use bb_core::id::{FieldId, GroupId, ProjectId, RoleId};
use bb_core::project::ShortName;
use bb_core::technical::TechnicalName;

use crate::error::{SampleError, Step};
use crate::generate::World;

impl World<'_> {
    /// The group with this name.
    pub(crate) async fn group_named(&self, name: &str) -> Result<GroupId, SampleError> {
        self.storage
            .groups()
            .await?
            .into_iter()
            .find(|group| group.name == name)
            .map(|group| group.id)
            .ok_or_else(|| SampleError::NotMade(name.to_owned()))
    }

    /// The project with this short name.
    pub(crate) async fn project_named(&self, short_name: &str) -> Result<ProjectId, SampleError> {
        let parsed =
            ShortName::new(short_name).map_err(|_| SampleError::NotMade(short_name.to_owned()))?;
        self.storage
            .project_by_short_name(&parsed)
            .await?
            .map(|project| project.id)
            .ok_or_else(|| SampleError::NotMade(short_name.to_owned()))
    }

    /// The starting role of this kind.
    pub(crate) async fn starting_role(&self, role: StartingRole) -> Result<RoleId, SampleError> {
        self.storage
            .roles()
            .await?
            .into_iter()
            .find(|record| {
                record.role.starting.as_ref().map(TechnicalName::as_str) == Some(role.code())
            })
            .map(|record| record.role.id)
            .ok_or_else(|| SampleError::NotMade(role.code().to_owned()))
    }

    /// Hides an own field from Contributors in every project, as the person of `token` (an
    /// Administrator); `step` names the step that asks, for the error.
    pub(crate) async fn hide_from_contributors(
        &self,
        token: &str,
        field: FieldId,
        step: Step,
    ) -> Result<(), SampleError> {
        let role = self.starting_role(StartingRole::Contributor).await?;
        self.tick();
        set_level(
            &self.services.ctx(self.request()),
            SetFieldLevel {
                session_token: token.to_owned(),
                role,
                project: None,
                issue_type: None,
                field: FieldKey::Own(field),
                level: Some(FieldLevel::Hidden),
            },
        )
        .await
        .map_err(|error| SampleError::step(step, &error))?;
        Ok(())
    }
}
