//! The releases an issue names (ISS-08, ADR-0049, decisions 1 and 3): *Found in release*, several,
//! and *Planned for release*, one. **The one rule of a choice**, for a form, the API, a change of
//! many issues and the tools of the registry: a release the issue does not name yet has to be one
//! the person may know (its project they see), offered where the issue is and not retired; a
//! release the issue names already stays, whatever happened to it since, so an issue that moved out
//! of the tree keeps its releases. A release the person may not know is refused like one that does
//! not exist (`release.not-offered`), and one the issue names that the person may not know is kept
//! when they set the others, so a change never takes away what they cannot see.

use bb_core::ErrorCode;
use bb_core::id::{ProjectId, ReleaseId};
use bb_core::project_release::{FOUND_IN_RELEASES_MAX, Release};

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::Shown;

/// What a choice of releases is judged on: where the issue is and what it names now.
pub(crate) struct ReleaseChoice<'a> {
    /// The projects from the top down to the issue's project.
    pub(crate) chain: &'a [ProjectId],
    /// The releases the issue was found in now.
    pub(crate) found_now: &'a [ReleaseId],
    /// The release the issue is planned for now.
    pub(crate) planned_now: Option<ReleaseId>,
}

/// Each once, in the order given.
fn once_each(releases: &[ReleaseId]) -> Vec<ReleaseId> {
    let mut distinct: Vec<ReleaseId> = Vec::with_capacity(releases.len());
    for release in releases {
        if !distinct.contains(release) {
            distinct.push(*release);
        }
    }
    distinct
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Refuses (`release.not-offered`) a release the issue does not name already that the person
    /// may not choose where the issue is.
    async fn ensure_choosable(
        &self,
        shown: &Shown,
        choice: &ReleaseChoice<'_>,
        wanted: &[ReleaseId],
    ) -> Result<(), AppError> {
        let new: Vec<ReleaseId> = wanted
            .iter()
            .copied()
            .filter(|release| {
                !choice.found_now.contains(release) && choice.planned_now != Some(*release)
            })
            .collect();
        if new.is_empty() {
            return Ok(());
        }
        let found = self
            .store()
            .releases_by_ids(new.clone())
            .await
            .map_err(|error| internal::<S>(&error))?;
        let choosable = |release: &ReleaseId| {
            found.iter().any(|each: &Release| {
                each.id == *release && shown.sees(each.project) && each.choosable_in(choice.chain)
            })
        };
        if new.iter().all(choosable) {
            Ok(())
        } else {
            Err(ErrorCode::ReleaseNotOffered.into())
        }
    }

    /// The releases the issue was found in after the person sets `wanted`: each once in the order
    /// given, then those the issue names that the person may not know (kept). `release.too-many`
    /// past the limit, `release.not-offered` for a new one that is not offered.
    pub(crate) async fn found_in_after(
        &self,
        shown: &Shown,
        choice: &ReleaseChoice<'_>,
        wanted: &[ReleaseId],
    ) -> Result<Vec<ReleaseId>, AppError> {
        let mut after = once_each(wanted);
        self.ensure_choosable(shown, choice, &after).await?;
        if !choice.found_now.is_empty() {
            let named = self
                .store()
                .releases_by_ids(choice.found_now.to_vec())
                .await
                .map_err(|error| internal::<S>(&error))?;
            for kept in &named {
                if !shown.sees(kept.project) && !after.contains(&kept.id) {
                    after.push(kept.id);
                }
            }
        }
        if after.len() > FOUND_IN_RELEASES_MAX {
            return Err(ErrorCode::ReleasesTooMany.into());
        }
        Ok(after)
    }

    /// The release the issue is planned for after the person sets `wanted` (none takes it away):
    /// `release.not-offered` for a new one that is not offered.
    pub(crate) async fn planned_for_after(
        &self,
        shown: &Shown,
        choice: &ReleaseChoice<'_>,
        wanted: Option<ReleaseId>,
    ) -> Result<Option<ReleaseId>, AppError> {
        if let Some(release) = wanted {
            self.ensure_choosable(shown, choice, &[release]).await?;
        }
        Ok(wanted)
    }
}

/// Inside the change that writes an issue's releases, after the lock of the releases: every
/// release it names that it did not name before is still offered and not in the trash (it may
/// have been retired or deleted since it was judged).
pub(crate) async fn ensure_still_choosable_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    chain: &[ProjectId],
    new: &[ReleaseId],
    stopped: &crate::projects::Why,
) -> Result<(), S::Error> {
    if new.is_empty() {
        return Ok(());
    }
    S::lock_releases(transaction).await?;
    let found = S::releases_by_ids_in(transaction, new.to_vec()).await?;
    let all = new.iter().all(|release| {
        found
            .iter()
            .any(|each| each.id == *release && each.choosable_in(chain))
    });
    if all {
        Ok(())
    } else {
        Err(stopped.stop::<S>(ErrorCode::ReleaseNotOffered))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bb_core::id::SequentialIds;

    #[test]
    fn iss_08_a_release_given_twice_counts_once() {
        let mut ids = SequentialIds::default();
        let (one, two) = (
            ReleaseId::generate(&mut ids).unwrap(),
            ReleaseId::generate(&mut ids).unwrap(),
        );
        assert_eq!(once_each(&[two, one, two]), [two, one]);
    }
}
