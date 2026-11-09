//! The rights of many people read together, so a page that names many people does not read them
//! one by one.

use std::sync::Arc;

use bb_core::id::{ProjectId, UserId};
use bb_policy::Place;

use super::evaluation_from_person_policy;
use crate::auth::{AppError, AuthStore, Ctx, internal, locked};

impl<S: AuthStore> Ctx<'_, S> {
    /// Reads the rights of these people in one go, so the evaluation of each of them finds them
    /// read already (`person_policy`).
    pub(crate) async fn preload_policies(&self, people: &[UserId]) -> Result<(), AppError> {
        let mut wanted: Vec<UserId> = {
            let known = locked(&self.policies);
            people
                .iter()
                .copied()
                .filter(|person| !known.contains_key(person))
                .collect()
        };
        wanted.sort();
        wanted.dedup();
        if wanted.len() < 2 {
            return Ok(());
        }
        let policy = Arc::new(
            self.store()
                .people_policy(wanted.clone())
                .await
                .map_err(|error| internal::<S>(&error))?,
        );
        let mut known = locked(&self.policies);
        for person in wanted {
            known.entry(person).or_insert_with(|| Arc::clone(&policy));
        }
        Ok(())
    }

    /// Decides what each of these people may do in each of their projects from one read of the
    /// rows of all of them and one of the tree of the projects, and keeps the decisions for the
    /// rest of the request, so none of them is read one by one (`evaluate` finds them).
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn evaluate_people_in(
        &self,
        pairs: &[(UserId, ProjectId)],
    ) -> Result<(), AppError> {
        let wanted: Vec<(UserId, ProjectId)> = pairs
            .iter()
            .copied()
            .filter(|(person, project)| {
                self.cached_evaluation(*person, Place::Project(*project))
                    .is_none()
            })
            .collect();
        if wanted.len() < 2 {
            return Ok(());
        }
        let people: Vec<UserId> = wanted.iter().map(|(person, _)| *person).collect();
        self.preload_policies(&people).await?;
        let tree = self.project_tree_now().await?;
        for (person, project) in wanted {
            let Some(chain) = tree.chain(project) else {
                continue;
            };
            let policy = self.person_policy(person).await?;
            let found =
                evaluation_from_person_policy(&policy, person, Place::Project(project), chain);
            self.remember_evaluation(person, Place::Project(project), &Arc::new(found));
        }
        Ok(())
    }
}
