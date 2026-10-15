//! Field masks and `Authorized<T>` (SEC-04, SEC-06, ADR-0004, ADR-0025 decision 10).
//!
//! `Authorized<T>` is the only way data leaves a use case: it carries the mask of the person who
//! asked and drops every hidden field when it is serialized. Its constructor is private to this
//! crate, so code that did not ask the engine cannot make one (ARCH-AUTHZ-ONLY: a compile-fail
//! test, and a source check for other ways). A hidden field and an unknown one are refused
//! alike: [`Masked::hidden_properties`] gives the readable names to the `fields` helper of the
//! API, so a screen that asks for a hidden field gets exactly the answer for a field that does
//! not exist.

use std::collections::BTreeMap;

use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_core::id::IssueTypeId;
use serde::{Serialize, Serializer};
use serde_json::Value;

/// The serialized form that [`Masked::mask`] works on; `#[derive(Masked)]` names it by this path.
pub type MaskedValue = Value;

use crate::evaluate::Evaluation;

/// What the person who asked may do with each field, built-in or own (the highest level of their
/// roles, step 7 of the evaluation), and, where some role set levels for an issue type, what they
/// may do with the fields of an item of that type (ADR-0027).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldMask {
    levels: BTreeMap<FieldKey, FieldLevel>,
    /// The levels for each issue type that has levels of its own; empty when no role set a level
    /// for an issue type.
    types: BTreeMap<IssueTypeId, BTreeMap<FieldKey, FieldLevel>>,
}

impl FieldMask {
    /// The level of a field; *Hidden* for a field that is not known.
    #[must_use]
    pub fn level(&self, field: impl Into<FieldKey>) -> FieldLevel {
        self.levels
            .get(&field.into())
            .copied()
            .unwrap_or(FieldLevel::Hidden)
    }

    /// Whether the field may be shown.
    #[must_use]
    pub fn can_see(&self, field: impl Into<FieldKey>) -> bool {
        self.level(field) >= FieldLevel::CanSee
    }

    /// Whether the field may be changed (where the person can change the item at all).
    #[must_use]
    pub fn can_change(&self, field: impl Into<FieldKey>) -> bool {
        self.level(field) >= FieldLevel::CanChange
    }

    /// Whether the field may be shown on an item of some issue type: in general, or in a type
    /// that has a level of its own.
    #[must_use]
    pub fn can_see_somewhere(&self, field: impl Into<FieldKey>) -> bool {
        let field = field.into();
        self.can_see(field)
            || self.types.values().any(|levels| {
                levels
                    .get(&field)
                    .is_some_and(|level| *level >= FieldLevel::CanSee)
            })
    }

    /// The fields that may be shown: the catalogue in its order, then the own fields.
    pub fn readable(&self) -> impl Iterator<Item = FieldKey> + '_ {
        self.levels
            .iter()
            .filter(|(_, level)| **level >= FieldLevel::CanSee)
            .map(|(field, _)| *field)
    }

    /// The mask for an item of this issue type. An issue type with levels of its own has its own
    /// mask; one without has the general one. An item whose type is not known gets, for each field,
    /// the lowest level it has in any type, so a field that is hidden for some type is never shown
    /// by a view that does not say its type.
    #[must_use]
    pub fn for_type(&self, issue_type: Option<IssueTypeId>) -> std::borrow::Cow<'_, Self> {
        use std::borrow::Cow;
        if self.types.is_empty() {
            return Cow::Borrowed(self);
        }
        if let Some(known) = issue_type {
            return self
                .types
                .get(&known)
                .map_or(Cow::Borrowed(self), |levels| {
                    Cow::Owned(Self {
                        levels: levels.clone(),
                        types: BTreeMap::new(),
                    })
                });
        }
        let lowest = self
            .levels
            .iter()
            .map(|(field, level)| {
                let floor = self
                    .types
                    .values()
                    .filter_map(|levels| levels.get(field))
                    .fold(*level, |floor, other| floor.min(*other));
                (*field, floor)
            })
            .collect();
        Cow::Owned(Self {
            levels: lowest,
            types: BTreeMap::new(),
        })
    }
}

impl Evaluation {
    /// The mask of this evaluation with the levels of every issue type. Only the policy holds it:
    /// a use case asks for the mask of the type of the issue it works on ([`Self::field_mask_for`]),
    /// so no path can judge an issue by the levels of the general role (SEC-04, ADR-0027).
    pub(crate) fn field_mask(&self) -> FieldMask {
        let levels = self.fields.all().clone();
        let types: std::collections::BTreeSet<IssueTypeId> = self
            .type_rules
            .typed
            .iter()
            .map(|rule| rule.issue_type)
            .collect();
        let types = types
            .into_iter()
            .map(|issue_type| {
                let levels = levels
                    .keys()
                    .map(|field| (*field, self.field_level_for(Some(issue_type), *field)))
                    .collect();
                (issue_type, levels)
            })
            .collect();
        FieldMask { levels, types }
    }

    /// Whether the viewer may see the field on an issue of some type (a name, not a value).
    #[must_use]
    pub fn can_see_somewhere(&self, field: impl Into<FieldKey>) -> bool {
        self.field_mask().can_see_somewhere(field)
    }

    /// The mask on an item of one issue type (ADR-0027): the levels set for the type count.
    #[must_use]
    pub fn field_mask_for(&self, issue_type: Option<IssueTypeId>) -> FieldMask {
        self.field_mask().for_type(issue_type).into_owned()
    }

    /// Wraps `value` for leaving a use case, with the mask of this evaluation. A view that holds
    /// its issue type (`#[masked(item_type)]`) is masked with the mask of that type.
    #[must_use]
    pub fn authorize<T>(&self, value: T) -> Authorized<T> {
        Authorized {
            value,
            mask: self.field_mask(),
        }
    }
}

/// A view that holds fields of the catalogue under known names, so the mask can drop them.
///
/// A view lists which of its JSON members hold which field ([`Masked::FIELDS`]); the default
/// [`Masked::mask`] drops the hidden ones from the serialized object. A view with objects inside
/// (a list of items, a related issue) overrides `mask` and masks what it holds with
/// [`mask_member`], so a mask works inside views too (M2-F01).
pub trait Masked {
    /// The members of the serialized form that hold a field of the catalogue: the name in the
    /// JSON and the field.
    const FIELDS: &'static [(&'static str, Field)] = &[];

    /// Drops the hidden fields from `value`, the serialized form of `Self`.
    fn mask(value: &mut Value, mask: &FieldMask) {
        drop_hidden(value, Self::FIELDS, mask);
    }

    /// The names of the JSON members that hold a field the person may not see: asking for one of
    /// these by `fields=` fails like asking for a field that does not exist.
    #[must_use]
    fn hidden_properties(mask: &FieldMask) -> Vec<&'static str> {
        Self::FIELDS
            .iter()
            .filter(|(_, field)| !mask.can_see(*field))
            .map(|(name, _)| *name)
            .collect()
    }
}

/// Removes the members of an object that hold a hidden field.
pub fn drop_hidden(value: &mut Value, fields: &[(&str, Field)], mask: &FieldMask) {
    if let Value::Object(members) = value {
        for (name, field) in fields {
            if !mask.can_see(*field) {
                members.remove(*name);
            }
        }
    }
}

/// Removes the values of own fields that the mask hides from the member `name` of an object, which
/// maps the key of a field to its value (`#[masked(own_fields)]`). A key that names no own field
/// is removed too: a value that cannot be placed is not shown. The view builder has judged the
/// issue type already; this is the net under it (CF-06).
pub fn drop_hidden_own_fields(value: &mut Value, name: &str, mask: &FieldMask) {
    if let Some(Value::Object(values)) = value.get_mut(name) {
        values.retain(|key, _| {
            FieldKey::from_code(key)
                .is_some_and(|field| field.own().is_some() && mask.can_see(field))
        });
    }
}

/// The mask for the item `value`, whose member `name` is its issue type (an object with an `id`):
/// the levels set for that type count (ADR-0027). Without a readable type the lowest levels count.
#[must_use]
pub fn mask_of_item<'m>(
    value: &Value,
    name: &str,
    mask: &'m FieldMask,
) -> std::borrow::Cow<'m, FieldMask> {
    let issue_type = value
        .get(name)
        .and_then(|member| member.get("id"))
        .and_then(Value::as_str)
        .and_then(|text| text.parse::<IssueTypeId>().ok());
    mask.for_type(issue_type)
}

/// Masks the member `name` of an object as a `T`.
pub fn mask_member<T: Masked>(value: &mut Value, name: &str, mask: &FieldMask) {
    if let Some(member) = value.get_mut(name) {
        T::mask(member, mask);
    }
}

impl<T: Masked> Masked for Vec<T> {
    fn mask(value: &mut Value, mask: &FieldMask) {
        if let Value::Array(items) = value {
            for item in items {
                T::mask(item, mask);
            }
        }
    }

    fn hidden_properties(mask: &FieldMask) -> Vec<&'static str> {
        T::hidden_properties(mask)
    }
}

impl<T: Masked> Masked for Box<T> {
    fn mask(value: &mut Value, mask: &FieldMask) {
        T::mask(value, mask);
    }

    fn hidden_properties(mask: &FieldMask) -> Vec<&'static str> {
        T::hidden_properties(mask)
    }
}

impl<T: Masked> Masked for Option<T> {
    fn mask(value: &mut Value, mask: &FieldMask) {
        if !value.is_null() {
            T::mask(value, mask);
        }
    }

    fn hidden_properties(mask: &FieldMask) -> Vec<&'static str> {
        T::hidden_properties(mask)
    }
}

/// Data that may leave a use case: the value with the mask of the person who asked. Made only by
/// [`Evaluation::authorize`]; the mask is applied whenever it is serialized.
#[derive(Clone, Debug)]
pub struct Authorized<T> {
    value: T,
    mask: FieldMask,
}

impl<T> Authorized<T> {
    /// The mask the value is shown with.
    #[must_use]
    pub const fn mask(&self) -> &FieldMask {
        &self.mask
    }

    /// The same authorization for another shape of the same data (a list of views made from the
    /// use case's own type). The mask is the one the engine made; it is applied to the new shape.
    #[must_use]
    pub fn map<U>(self, change: impl FnOnce(T) -> U) -> Authorized<U> {
        Authorized {
            value: change(self.value),
            mask: self.mask,
        }
    }
}

#[cfg(feature = "peek")]
impl<T> Authorized<T> {
    /// The value without its mask, for tests that check what a use case answered. Not part of the
    /// product: only `[dev-dependencies]` switch the feature on.
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }
}

impl<T: Masked + Serialize> Authorized<T> {
    /// The value as JSON with the hidden fields left out.
    ///
    /// # Errors
    /// Fails when the value cannot be serialized.
    pub fn masked_value(&self) -> Result<Value, serde_json::Error> {
        let mut value = serde_json::to_value(&self.value)?;
        T::mask(&mut value, &self.mask);
        Ok(value)
    }

    /// The names of the JSON members this person may not read.
    #[must_use]
    pub fn hidden_properties(&self) -> Vec<&'static str> {
        T::hidden_properties(&self.mask)
    }
}

impl<T: Masked + Serialize> Serialize for Authorized<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.masked_value()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}
