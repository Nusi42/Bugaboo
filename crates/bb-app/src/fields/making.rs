//! Whether a request makes an item or changes it, and what a new item inherits before the defaults
//! of its own fields (a time entry: the values of its issue and its projects, ADR-0035).

use std::collections::BTreeMap;

use bb_core::id::FieldId;
use bb_core::own_fields_value::FieldValue;

/// Whether a request makes the item or changes it, and the values a new item inherits (a time
/// entry from its issue and its projects) before the defaults of the fields.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Making<'a> {
    /// The item is there and changes.
    Change,
    /// The item is being made.
    New(&'a BTreeMap<FieldId, FieldValue>),
}

/// The values a new item inherits that inherits nothing.
static NOTHING_INHERITED: BTreeMap<FieldId, FieldValue> = BTreeMap::new();

impl Making<'static> {
    /// A new item that inherits nothing: only the defaults of the fields.
    pub(crate) const FRESH: Self = Self::New(&NOTHING_INHERITED);
}
