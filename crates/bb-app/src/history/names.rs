//! The names a reader may read in a change of an issue (LM1 of the M4 exit review): the issue type
//! and the labels. An event keeps the names of its time, so a change can name a type or a label
//! that the issue has not got any more and that the reader has never been told about. The one rule
//! is `offers/known.rs`; this file only asks it, once for a page.

use std::collections::{BTreeMap, BTreeSet};

use bb_core::id::IssueTypeId;
use bb_core::message::{self, Message};

use super::read::Reading;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::Shown;

/// What the reader may know by name, read when a change first needs it.
#[derive(Default)]
pub(crate) struct KnownNames {
    /// What the reader sees of the tree of projects, read once for both lists.
    shown: Option<Shown>,
    types: Option<BTreeMap<IssueTypeId, String>>,
    labels: Option<BTreeSet<String>>,
    /// Names that only labels of groups the reader may not know have.
    hidden_labels: BTreeSet<String>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the reader sees of the tree of projects, read the first time it is asked for.
    async fn shown_for<'k>(&self, reading: &'k mut Reading<'_>) -> Result<&'k Shown, AppError> {
        if reading.names.shown.is_none() {
            reading.names.shown = Some(self.shown_to(reading.actor).await?);
        }
        reading
            .names
            .shown
            .as_ref()
            .ok_or_else(|| AppError::Internal("the projects the reader sees".into()))
    }

    /// What an issue type is called for this reader: its name where they may know it, else a
    /// neutral word.
    pub(super) async fn type_name_for(
        &self,
        reading: &mut Reading<'_>,
        id: IssueTypeId,
    ) -> Result<String, AppError> {
        if reading.names.types.is_none() {
            let all = self
                .store()
                .issue_types()
                .await
                .map_err(|error| internal::<S>(&error))?;
            let shown = self.shown_for(reading).await?;
            let known = self.known_types(shown, all).await?;
            reading.names.types =
                Some(known.into_iter().map(|each| (each.id, each.name)).collect());
        }
        Ok(reading
            .names
            .types
            .as_ref()
            .and_then(|known| known.get(&id))
            .cloned()
            .unwrap_or_else(|| Message::from(message::HISTORY_TYPE_UNKNOWN).render()))
    }

    /// The labels of `names` that the reader may know: those in a group of labels they may know,
    /// or in none, and those no label has any more (a label that was combined or deleted: nothing
    /// is left to say which group it was in). A name that only labels of groups the reader may not
    /// know have is left out.
    pub(super) async fn label_names_for(
        &self,
        reading: &mut Reading<'_>,
        names: Vec<String>,
    ) -> Result<Vec<String>, AppError> {
        if reading.names.labels.is_none() {
            let failed = |error| internal::<S>(&error);
            let labels = self.store().labels().await.map_err(failed)?;
            let groups = self.store().label_groups().await.map_err(failed)?;
            let shown = self.shown_for(reading).await?;
            let known: BTreeSet<_> = self
                .known_groups(shown, groups)
                .await?
                .into_iter()
                .map(|group| group.id)
                .collect();
            let (seen, unseen): (Vec<_>, Vec<_>) = labels
                .into_iter()
                .partition(|label| label.group.is_none_or(|group| known.contains(&group)));
            let seen: BTreeSet<String> = seen.into_iter().map(|label| label.name).collect();
            reading.names.hidden_labels = unseen
                .into_iter()
                .map(|label| label.name)
                .filter(|name| !seen.contains(name))
                .collect();
            reading.names.labels = Some(seen);
        }
        Ok(names
            .into_iter()
            .filter(|name| !reading.names.hidden_labels.contains(name))
            .collect())
    }
}
