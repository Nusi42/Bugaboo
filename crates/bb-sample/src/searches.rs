//! Saved searches in the sample (M5 WP-10, QRY-06): people of every role save a search, some share
//! it with groups and projects, a project shows one as its list, and one reads an own field that
//! Contributors may not read, so it never reaches them. All of it goes through the use cases the
//! screens call, one change a second of the sample's clock.

use bb_app::access::{PolicyStore as _, StartingRole};
use bb_app::saved_searches::{
    SaveSearch, SetProjectList, ShareSearch, save_search, set_project_list, share_search,
};
use bb_core::id::{GroupId, SavedSearchId};
use bb_core::saved_search::{Presentation, ShareTarget};
use bb_dev_words::sample::{Layout, SavedSearchPlan};

use crate::error::{SampleError, Step};
use crate::issues::Maker;
use crate::issues_setup::{id_in, json};

impl Maker<'_, '_> {
    /// Hides the own fields the plan names from Contributors, then makes the saved searches.
    pub(crate) async fn save_searches(&mut self) -> Result<(), SampleError> {
        let plan = self.names.work.clone();
        self.hide_from_contributors(&plan.hidden_from_contributors)
            .await?;
        for search in &plan.saved_searches {
            self.save_one(search).await?;
        }
        Ok(())
    }

    /// The own fields called so are not read by Contributors in any project.
    async fn hide_from_contributors(&mut self, names: &[String]) -> Result<(), SampleError> {
        let administrator = self
            .world
            .people
            .iter()
            .position(|made| {
                made.person.role.code() == StartingRole::Administrator.code()
                    && !made.person.code_app
            })
            .ok_or(SampleError::PersonMissing)?;
        for name in names {
            let token = self.hands.token(administrator).await?;
            let at = self
                .names
                .fields
                .iter()
                .position(|field| &field.name == name)
                .ok_or_else(|| SampleError::NotMade(name.clone()))?;
            let made = self
                .setup
                .fields
                .get(at)
                .ok_or_else(|| SampleError::NotMade(name.clone()))?;
            self.world
                .hide_from_contributors(&token, made.id, Step::SavedSearches)
                .await?;
        }
        Ok(())
    }

    /// One search: saved by the first person of its role, then shared and chosen as a list.
    async fn save_one(&mut self, plan: &SavedSearchPlan) -> Result<(), SampleError> {
        let step = |error: &dyn std::fmt::Display| SampleError::step(Step::SavedSearches, &error);
        let mut groups = Vec::new();
        for key in &plan.groups {
            groups.push(self.group_called(key).await?);
        }
        let maker = self.maker_of(plan, &groups).await?;
        let token = self.hands.token(maker).await?;
        let ctx = self.world.services.ctx(self.world.request());
        self.world.tick();
        let saved = json(
            save_search(
                &ctx,
                SaveSearch {
                    session_token: token.clone(),
                    name: plan.name.clone(),
                    text: plan.text.clone(),
                    presentation: Presentation::default(),
                },
            )
            .await,
            Step::SavedSearches,
        )?;
        let id: SavedSearchId = id_in(&saved, "id")?;
        let mut targets: Vec<ShareTarget> = groups.into_iter().map(ShareTarget::Group).collect();
        for short_name in &plan.projects {
            targets.push(ShareTarget::Project(
                self.world.project_named(short_name).await?,
            ));
        }
        for target in targets {
            self.world.tick();
            share_search(
                &ctx,
                ShareSearch {
                    session_token: token.clone(),
                    search: id,
                    target,
                },
            )
            .await
            .map_err(|error| step(&error))?;
        }
        if let Some(project) = &plan.project_list {
            self.world.tick();
            set_project_list(
                &ctx,
                SetProjectList {
                    session_token: token,
                    project: project.clone(),
                    search: Some(id),
                },
            )
            .await
            .map_err(|error| step(&error))?;
        }
        Ok(())
    }

    /// The first person of the role who may share with all the groups: in each of them, or
    /// somebody who manages groups.
    async fn maker_of(
        &self,
        plan: &SavedSearchPlan,
        groups: &[GroupId],
    ) -> Result<usize, SampleError> {
        let mut members = Vec::new();
        for group in groups {
            members.push(self.world.storage.group_members(*group).await?);
        }
        self.world
            .people
            .iter()
            .position(|made| {
                made.person.role.code() == plan.maker
                    && !made.person.code_app
                    && (plan.maker == StartingRole::Administrator.code()
                        || members
                            .iter()
                            .all(|group| group.iter().any(|member| member.person == made.id)))
            })
            .ok_or(SampleError::PersonMissing)
    }

    async fn group_called(&self, key: &str) -> Result<GroupId, SampleError> {
        let name = Layout::bundled()?
            .organization
            .groups
            .get(key)
            .cloned()
            .ok_or_else(|| SampleError::NotMade(key.to_owned()))?;
        self.world.group_named(&name).await
    }
}
