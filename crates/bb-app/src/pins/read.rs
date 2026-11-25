//! Reading pins: each pin is shown by the rule of its target, so a person never learns of a target
//! they may not open (ADR-0050, decision 12).

use std::collections::BTreeMap;

use bb_core::id::ProjectId;
use bb_core::pin::{Pin, PinTarget};
use bb_policy::{Authorized, Place};

use crate::saved_searches::reach::Reached;

use super::types::{PinList, PinView, PinsQuery};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// The pins of the actor, in their order.
    pub(super) async fn pins_of_actor(&self, actor: &Actor) -> Result<Vec<Pin>, AppError> {
        self.store()
            .pins_of(actor.person.id)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Each target as the actor sees it: its view when they may open it, else none. The one rule
    /// that listing and pinning both ask: a project is there for the person who sees it in the
    /// tree, a saved search for the person who `may_open` it.
    pub(super) async fn viewed(
        &self,
        actor: &Actor,
        targets: &[PinTarget],
    ) -> Result<Vec<Option<PinView>>, AppError> {
        if targets.is_empty() {
            return Ok(Vec::new());
        }
        let reaching = self.reaching(actor).await?;
        let projects: BTreeMap<ProjectId, _> = reaching
            .shown
            .everything()
            .into_iter()
            .map(|project| (project.id, project))
            .collect();
        // The judge is made whenever a saved search is asked for, found or not, so a search that is not there
        // and one that is not the person's run the same reads (ADR-0034, decision 4).
        let mut judge = if targets
            .iter()
            .any(|target| matches!(target, PinTarget::SavedSearch(_)))
        {
            Some(self.judge(actor, &reaching).await?)
        } else {
            None
        };
        let mut views = Vec::with_capacity(targets.len());
        for target in targets {
            let view = match *target {
                PinTarget::Project(id) => projects.get(&id).map(|project| PinView {
                    kind: target.kind(),
                    id: target.record(),
                    name: project.name.clone(),
                    short_name: Some(project.short_name.as_str().to_owned()),
                }),
                PinTarget::SavedSearch(id) => match (self.stored_search(id).await?, &mut judge) {
                    (Some(found), Some(judged)) => self
                        .may_open(judged, &reaching, &found)
                        .await?
                        .then(|| PinView {
                            kind: target.kind(),
                            id: target.record(),
                            name: found.search.name.clone(),
                            short_name: None,
                        }),
                    _ => None,
                },
                // The same reads whether the dashboard is there or not (`stored_dashboard`); a person's
                // *Home* is their start page, not something to pin.
                PinTarget::Dashboard(id) => self
                    .stored_dashboard(id)
                    .await?
                    .filter(|found| {
                        !found.dashboard.home && found.reached(&reaching) != Reached::Not
                    })
                    .map(|found| PinView {
                        kind: target.kind(),
                        id: target.record(),
                        name: found.dashboard.name,
                        short_name: None,
                    }),
            };
            views.push(view);
        }
        Ok(views)
    }
}

/// The pins of the person that they may open, in their order.
///
/// # Errors
/// `auth.access-key-not-allowed` for an access key: pins are the person's own.
pub async fn my_pins<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PinsQuery,
) -> Result<Authorized<PinList>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let pins = ctx.pins_of_actor(&actor).await?;
    let targets: Vec<PinTarget> = pins.iter().filter_map(Pin::points_to).collect();
    let items = ctx
        .viewed(&actor, &targets)
        .await?
        .into_iter()
        .flatten()
        .collect();
    let organization = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(organization.authorize(PinList { items }))
}
