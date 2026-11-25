//! The releases an issue names (ISS-08, ADR-0049, decisions 1 and 3): *Found in release*, several,
//! and *Planned for release*, one. **The one rule of a choice**, for a form, the API, a change of
//! many issues and the tools of the registry: a release the issue does not name yet has to be one
//! the person may know (its project they see), offered where the issue is and not retired; a
//! release the issue names already stays, whatever happened to it since, so an issue that moved out
//! of the tree keeps its releases. A release the person may not know is refused like one that does
//! not exist (`release.not-offered`), and one the issue names that the person may not know is kept
//! at its place when they set the others, and *Planned for release* keeps one they may not know when
//! they set none, so a change never takes away or reorders what they cannot see.

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

    /// The releases of `ids` the person may not know: of a project they do not see.
    async fn hidden_of(
        &self,
        shown: &Shown,
        ids: &[ReleaseId],
    ) -> Result<Vec<ReleaseId>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self
            .store()
            .releases_by_ids(ids.to_vec())
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .filter(|release| !shown.sees(release.project))
            .map(|release| release.id)
            .collect())
    }

    /// The releases the issue was found in after the person sets `wanted`: the ones they may know
    /// in the order given, each once, with every one they may not know kept at its place, so an
    /// order the person cannot see never changes. `release.too-many` when they name more than the
    /// limit (the ones they may not know are not counted, so the refusal tells nothing of them),
    /// `release.not-offered` for a new one that is not offered.
    pub(crate) async fn found_in_after(
        &self,
        shown: &Shown,
        choice: &ReleaseChoice<'_>,
        wanted: &[ReleaseId],
    ) -> Result<Vec<ReleaseId>, AppError> {
        let hidden = self.hidden_of(shown, choice.found_now).await?;
        let mut visible = once_each(wanted);
        visible.retain(|release| !hidden.contains(release));
        if visible.len() > FOUND_IN_RELEASES_MAX {
            return Err(ErrorCode::ReleasesTooMany.into());
        }
        self.ensure_choosable(shown, choice, &visible).await?;
        Ok(kept_in_place(choice.found_now, &hidden, visible))
    }

    /// The release the issue is planned for after the person sets `wanted`: one they may know, or
    /// none. A release the issue is planned for that they may not know stays when they set none,
    /// as they see none either way; naming one they may know takes its place.
    /// `release.not-offered` for a new one that is not offered.
    pub(crate) async fn planned_for_after(
        &self,
        shown: &Shown,
        choice: &ReleaseChoice<'_>,
        wanted: Option<ReleaseId>,
    ) -> Result<Option<ReleaseId>, AppError> {
        if let Some(release) = wanted {
            self.ensure_choosable(shown, choice, &[release]).await?;
            return Ok(Some(release));
        }
        let now: Vec<ReleaseId> = choice.planned_now.into_iter().collect();
        Ok(self.hidden_of(shown, &now).await?.into_iter().next())
    }
}

/// The list after a change: the releases the person may not know at the places they had in `now`,
/// the ones of `visible` in the other places in their order, the rest of `visible` after them.
fn kept_in_place(
    now: &[ReleaseId],
    hidden: &[ReleaseId],
    visible: Vec<ReleaseId>,
) -> Vec<ReleaseId> {
    let mut given = visible.into_iter();
    let mut after: Vec<ReleaseId> = now
        .iter()
        .filter_map(|release| {
            if hidden.contains(release) {
                Some(*release)
            } else {
                given.next()
            }
        })
        .collect();
    after.extend(given);
    after
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

    #[test]
    fn iss_08_a_release_the_person_may_not_know_keeps_its_place() {
        let mut ids = SequentialIds::default();
        let [a, hidden, b, c] = std::array::from_fn(|_| ReleaseId::generate(&mut ids).unwrap());
        // The hidden one stays second; the ones the person knows fill the other places in order.
        assert_eq!(
            kept_in_place(&[a, hidden, b], &[hidden], vec![b, a]),
            [b, hidden, a]
        );
        assert_eq!(kept_in_place(&[a, hidden], &[hidden], vec![]), [hidden]);
        assert_eq!(
            kept_in_place(&[hidden], &[hidden], vec![a, c]),
            [hidden, a, c]
        );
    }
}
