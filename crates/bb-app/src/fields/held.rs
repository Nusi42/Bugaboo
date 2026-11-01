//! Which own fields an item holds: the fields made for its kind, and on an issue and a project
//! also the fields of time entries, whose value there is the default of the entries booked there
//! (ADR-0035, decision 1); and the lists of them that the reads and the layouts ask for.

use bb_core::fields::FieldKey;
use bb_core::own_fields::FieldFor;

use super::values::Item;
use crate::auth::{AppError, AuthStore, Ctx};

/// What an item holds values of own fields for: the fields made for its kind, and on an issue and
/// a project also the fields of time entries, whose value there is the default of the entries
/// booked there (ADR-0035, decision 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Holder(pub(super) FieldFor);

impl Holder {
    /// Whether the item holds values of a field made for `applies_to`.
    pub(super) fn holds(self, applies_to: FieldFor) -> bool {
        applies_to == self.0
            || (applies_to == FieldFor::TimeEntries
                && matches!(self.0, FieldFor::Issues | FieldFor::Projects))
    }
}

/// Which of the own fields an item holds a list is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Held {
    /// Every field the item holds a value of, the defaults of its time entries too: what may be
    /// changed on the item itself.
    Every,
    /// Only the fields made for its kind: what its forms and layouts place. The default of an
    /// issue for the time booked on it is not a field of the issue there; the issue's details
    /// show it in a group of its own (ADR-0035, decision 1).
    OfItsKind,
}

impl Held {
    /// Whether a field made for `applies_to` is in the list for an item that is `holder`.
    fn takes(self, holder: Holder, applies_to: FieldFor) -> bool {
        match self {
            Self::Every => holder.holds(applies_to),
            Self::OfItsKind => applies_to == holder.0,
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The own fields of an item that the mask lets the person change: `held` by the item, used
    /// for it and not retired. A long text is in the list: it is changed on its own route.
    pub(crate) async fn changeable_own_fields(
        &self,
        mask: &bb_policy::FieldMask,
        item: &Item,
        held: Held,
    ) -> Result<Vec<FieldKey>, AppError> {
        self.own_fields_at(item, held, |field| mask.can_change(field))
            .await
    }

    /// The own fields of an item that the mask lets the person see: `held` by the item, used for
    /// it and not retired. The layouts of the issue page ask it, next to `changeable_own_fields`.
    pub(crate) async fn visible_own_fields(
        &self,
        mask: &bb_policy::FieldMask,
        item: &Item,
        held: Held,
    ) -> Result<Vec<FieldKey>, AppError> {
        self.own_fields_at(item, held, |field| mask.can_see(field))
            .await
    }

    /// The own fields `held` by an item that reach it and that `level_allows`: the one filter of
    /// the two reads.
    pub(crate) async fn own_fields_at(
        &self,
        item: &Item,
        held: Held,
        level_allows: impl Fn(FieldKey) -> bool,
    ) -> Result<Vec<FieldKey>, AppError> {
        self.own_fields_reaching(
            item,
            held,
            |scope| scope.reaches(&item.chain, item.issue_type),
            level_allows,
        )
        .await
    }

    /// The own fields that are used for an item and not retired, whose scope `reached` and that
    /// `level_allows`. The layouts of the whole organization, which has no project, say `reached`
    /// by the issue type alone (`Scope::reaches_type`).
    pub(crate) async fn own_fields_reaching(
        &self,
        item: &Item,
        held: Held,
        reached: impl Fn(&bb_core::own_fields::Scope) -> bool,
        level_allows: impl Fn(FieldKey) -> bool,
    ) -> Result<Vec<FieldKey>, AppError> {
        Ok(self
            .field_catalogue()
            .await?
            .iter()
            .filter(|definition| {
                let field = &definition.field;
                held.takes(item.holder(), field.applies_to)
                    && field.retired_at.is_none()
                    && reached(&definition.scope)
                    && level_allows(FieldKey::Own(field.id))
            })
            .map(|definition| FieldKey::Own(definition.field.id))
            .collect())
    }
}
