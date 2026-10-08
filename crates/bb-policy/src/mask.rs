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
use bb_core::fields::Field;
use serde::{Serialize, Serializer};
use serde_json::Value;

use crate::evaluate::Evaluation;

/// What the person who asked may do with each field of the catalogue (the highest level of their
/// roles, step 7 of the evaluation).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldMask {
    levels: BTreeMap<Field, FieldLevel>,
}

impl FieldMask {
    /// The level of a field.
    #[must_use]
    pub fn level(&self, field: Field) -> FieldLevel {
        self.levels
            .get(&field)
            .copied()
            .unwrap_or(FieldLevel::Hidden)
    }

    /// Whether the field may be shown.
    #[must_use]
    pub fn can_see(&self, field: Field) -> bool {
        self.level(field) >= FieldLevel::CanSee
    }

    /// Whether the field may be changed (where the person can change the item at all).
    #[must_use]
    pub fn can_change(&self, field: Field) -> bool {
        self.level(field) >= FieldLevel::CanChange
    }

    /// The fields that may be shown, in the order of the catalogue.
    pub fn readable(&self) -> impl Iterator<Item = Field> + '_ {
        Field::ALL
            .iter()
            .copied()
            .filter(|field| self.can_see(*field))
    }
}

impl Evaluation {
    /// The mask of this evaluation.
    #[must_use]
    pub fn field_mask(&self) -> FieldMask {
        FieldMask {
            levels: self.fields.all().clone(),
        }
    }

    /// Wraps `value` for leaving a use case, with the mask of this evaluation.
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
