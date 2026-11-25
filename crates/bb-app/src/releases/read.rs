//! The releases offered in a project (ISS-08, ADR-0049, decision 1): those made in it and in the
//! projects above it that the person sees, retired ones too. Seeing a release needs only seeing its
//! project (decision 5).

use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_core::paging::window;
use bb_core::project_release::Release;
use bb_policy::{Authorized, Permission};

use super::types::{ReleasePage, ReleaseView, ReleasesQuery};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::Shown;

/// The order of a list of releases: by *Release date*, the ones without one last, then by name.
pub(crate) fn in_order(releases: &mut [Release]) {
    releases.sort_by(|one, other| {
        (one.release_date.is_none(), one.release_date)
            .cmp(&(other.release_date.is_none(), other.release_date))
            .then_with(|| one.name.cmp(&other.name))
            .then_with(|| one.id.cmp(&other.id))
    });
}

/// The views of releases for the person: each with its project and whether they manage it.
fn release_views(shown: &Shown, releases: &[Release]) -> Vec<ReleaseView> {
    releases
        .iter()
        .filter_map(|release| {
            let project = shown.reference(release.project)?;
            let may_manage = shown.allows(release.project, Permission::ManageReleases);
            Some(ReleaseView::of(release, project, may_manage))
        })
        .collect()
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The releases offered where the projects of `chain` end, that the person may know: made in a
    /// project of the chain they see, not in the trash, in the order of a list.
    pub(crate) async fn releases_offered(
        &self,
        shown: &Shown,
        chain: &[ProjectId],
    ) -> Result<Vec<Release>, AppError> {
        let seen: Vec<ProjectId> = chain
            .iter()
            .copied()
            .filter(|project| shown.sees(*project))
            .collect();
        let mut releases = self
            .store()
            .releases_of_projects(seen)
            .await
            .map_err(|error| internal::<S>(&error))?;
        in_order(&mut releases);
        Ok(releases)
    }

    /// The releases of the projects the person sees, also the retired ones: what the names of a
    /// search can stand for (ADR-0049, decision 4).
    pub(crate) async fn releases_seen(&self, shown: &Shown) -> Result<Vec<Release>, AppError> {
        let seen: Vec<ProjectId> = shown
            .everything()
            .into_iter()
            .map(|project| project.id)
            .filter(|project| shown.sees(*project))
            .collect();
        self.store()
            .releases_of_projects(seen)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// The person's view of the tree, for the releases.
    pub(crate) async fn release_sight(&self, actor: &Actor) -> Result<Shown, AppError> {
        self.shown_to(actor).await
    }
}

/// The releases offered in a project, a page at a time.
///
/// # Errors
/// `resource.not-found` for a project the person does not see, `request.invalid` for a cursor
/// that is not one.
pub async fn releases<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ReleasesQuery,
) -> Result<Authorized<ReleasePage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (project, held) = ctx.release_project(&actor, &input.project).await?;
    let shown = ctx.release_sight(&actor).await?;
    let chain = ctx.chain_of(project.id).await?;
    let releases = ctx.releases_offered(&shown, &chain).await?;
    let page = window(releases.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(held.authorize(ReleasePage {
        items: release_views(&shown, &releases[page.start..page.end]),
        next_cursor: page.next,
    }))
}

#[cfg(test)]
mod tests {
    use bb_core::id::{ReleaseId, SequentialIds, UserId};
    use bb_core::project_release::ReleaseState;
    use jiff::Timestamp;

    use super::*;

    #[test]
    fn iss_08_releases_are_listed_by_date_with_the_undated_last() {
        let mut ids = SequentialIds::default();
        let project = ProjectId::generate(&mut ids).unwrap();
        let person = UserId::generate(&mut ids);
        let release = |name: &str, day: Option<&str>, ids: &mut SequentialIds| Release {
            id: ReleaseId::generate(ids).unwrap(),
            project,
            name: name.to_owned(),
            release_date: day.map(|text| text.parse().unwrap()),
            state: ReleaseState::Planned,
            released_on: None,
            made_by: person,
            made_at: Timestamp::UNIX_EPOCH,
            changed_by: person,
            changed_at: Timestamp::UNIX_EPOCH,
            deleted_at: None,
            version: 1,
        };
        let mut list = vec![
            release("later", None, &mut ids),
            release("2.5", Some("2026-12-01"), &mut ids),
            release("2.4", Some("2026-11-01"), &mut ids),
        ];
        in_order(&mut list);
        let names: Vec<&str> = list.iter().map(|each| each.name.as_str()).collect();
        assert_eq!(names, ["2.4", "2.5", "later"]);
    }
}
