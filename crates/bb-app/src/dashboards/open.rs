//! Who a dashboard is for, and which of its boxes check for a viewer (ADR-0050, decisions 2 and
//! 3).
//!
//! A dashboard reaches a person by the one rule of saved searches (`Reaching::reached`): its maker,
//! or a share with a group of theirs or a project they see; one that does not reach them answers
//! exactly like one that does not exist. Each box is then judged as the viewer: its text by the
//! judge of saved searches, its saved search by the rule of who may open one, its field by the
//! fields they may read. A box that does not check is left out, and the places of the others are
//! packed again, so nothing tells of it.

use bb_core::ErrorCode;
use bb_core::dashboard::{BoxSource, BoxSpec, Dashboard, DashboardBox, DashboardShare, pack};
use bb_core::id::DashboardId;
use bb_core::saved_search::Presentation;

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::saved_searches::judge::Judge;
use crate::saved_searches::reach::{Reached, Reaching};

/// A dashboard that is there, with its boxes and its shares.
pub(crate) struct Stored {
    pub(crate) dashboard: Dashboard,
    pub(crate) boxes: Vec<DashboardBox>,
    pub(crate) shares: Vec<DashboardShare>,
}

impl Stored {
    /// How the dashboard reaches the person of `reaching`.
    pub(crate) fn reached(&self, reaching: &Reaching) -> Reached {
        reaching.reached(
            self.dashboard.deleted_at.is_some(),
            self.dashboard.maker,
            self.shares.iter().map(|share| share.target),
        )
    }
}

/// A dashboard the person may open.
pub(crate) struct Opened {
    pub(crate) stored: Stored,
    pub(crate) reaching: Reaching,
}

/// What a box reads for a viewer, once it checks for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reads {
    /// No search: *My timer* and the viewer's own week.
    Nothing,
    /// The search text, in its canonical spelling: the box's own, or that of its saved search.
    Text(String),
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The dashboard with its boxes and shares, in the trash or not, if there is one. The same
    /// reads whether it is there or not.
    pub(crate) async fn stored_dashboard(
        &self,
        id: DashboardId,
    ) -> Result<Option<Stored>, AppError> {
        let store = self.store();
        let failed = |error: &S::Error| internal::<S>(error);
        let dashboard = store.dashboard(id).await.map_err(|e| failed(&e))?;
        let boxes = store.dashboard_boxes(id).await.map_err(|e| failed(&e))?;
        let shares = store.dashboard_shares(id).await.map_err(|e| failed(&e))?;
        Ok(dashboard.map(|dashboard| Stored {
            dashboard,
            boxes,
            shares,
        }))
    }

    /// The dashboard the actor may open, or `resource.not-found`: one that does not exist, is in
    /// the trash, or is neither theirs nor shared with them is one and the same answer, after the
    /// same reads (ADR-0050, decision 2).
    pub(crate) async fn open_dashboard(
        &self,
        actor: &Actor,
        id: DashboardId,
    ) -> Result<Opened, AppError> {
        let stored = self.stored_dashboard(id).await?;
        let reaching = self.reaching(actor).await?;
        let stored = stored
            .filter(|found| found.reached(&reaching) != Reached::Not)
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        Ok(Opened { stored, reaching })
    }

    /// The dashboard the actor made and may change: one they do not see is not found, one they see
    /// and did not make is refused.
    pub(crate) async fn own_dashboard(
        &self,
        actor: &Actor,
        id: DashboardId,
    ) -> Result<Opened, AppError> {
        let opened = self.open_dashboard(actor, id).await?;
        if opened.stored.dashboard.maker == actor.person.id {
            Ok(opened)
        } else {
            Err(ErrorCode::Forbidden.into())
        }
    }

    /// What a box reads for the person `judge` is for, or `None` when it does not check for them:
    /// a text that does not read cleanly with what they may know, a saved search they may not
    /// open, or a field to group by that they may not read (ADR-0050, decision 3).
    pub(crate) async fn box_reads(
        &self,
        judge: &mut Judge<'_>,
        reaching: &Reaching,
        spec: &BoxSpec,
    ) -> Result<Option<Reads>, AppError> {
        let Some(source) = spec.source.as_ref().filter(|_| spec.reads_a_search()) else {
            return Ok(Some(Reads::Nothing));
        };
        // The field of a chart is checked as the grouping of a search is.
        let grouped = Presentation {
            group_by: spec.options.by.clone(),
            ..Presentation::default()
        };
        let text = match source {
            BoxSource::Text(text) => text.clone(),
            BoxSource::Saved(id) => match self.stored_search(*id).await? {
                Some(found) if self.may_open(judge, reaching, &found).await? => found.search.text,
                _ => return Ok(None),
            },
        };
        Ok(self
            .canonical_text(judge, &text, &grouped)
            .await?
            .map(Reads::Text))
    }

    /// The boxes of a dashboard that check for the actor, in order, with the places packed for
    /// them (ADR-0050, decision 3): a box left out leaves no gap, no title and no count. A text is
    /// in the spelling the actor's language reads it in.
    pub(crate) async fn boxes_for(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        boxes: &[DashboardBox],
    ) -> Result<Vec<DashboardBox>, AppError> {
        let mut judge = self.judge(actor, reaching).await?;
        // A dashboard holds at most twenty boxes: every name of every box is resolved.
        judge.lift_budget();
        let mut kept = Vec::with_capacity(boxes.len());
        for each in boxes {
            if let Some(reads) = self.box_reads(&mut judge, reaching, &each.spec).await? {
                let mut each = each.clone();
                if let (Some(BoxSource::Text(_)), Reads::Text(text)) = (&each.spec.source, reads) {
                    each.spec.source = Some(BoxSource::Text(text));
                }
                kept.push(each);
            }
        }
        let places: Vec<_> = kept.iter().map(|each| each.spec.place).collect();
        for (each, place) in kept.iter_mut().zip(pack(&places)) {
            each.spec.place = place;
        }
        Ok(kept)
    }
}
