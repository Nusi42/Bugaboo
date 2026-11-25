//! The pages of the groups of a total (M5-F44): the groups in the order asked for, a page of
//! them, and the place of the last one, which the cursor of the next page holds. Groups of people
//! stand in the order of the names the reader sees; every other group in the order of
//! `GroupKey::place`, which the store cuts its page by.

use bb_core::names::name_key;
use bb_policy::PersonRef;
use serde::{Deserialize, Serialize};

use super::super::request::{GroupPlace, Total};
use super::super::run::ShownKey;
use super::GroupOrder;
use crate::auth::AppError;

/// The most groups one page of totals holds (M5-F44).
pub const GROUPS_PAGE: usize = 200;

/// The place of a group in the order of a page, which a cursor holds.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(super) struct Place {
    /// For the order by size: how much smaller than the biggest it is; 0 in the order of keys.
    smaller: u64,
    /// The place of the key.
    key: GroupPlace,
}

impl Place {
    /// The place of the key alone, where the store starts its page.
    pub(super) fn key(&self) -> &GroupPlace {
        &self.key
    }
}

/// The place of a shown key in the order of keys: a person by the name the reader sees, then
/// those shown by a role, by the role, then *Someone*, then former users by their code; every other
/// key by `GroupKey::place`.
pub(crate) fn key_place(key: &ShownKey) -> GroupPlace {
    match key {
        ShownKey::Person(person) => GroupPlace {
            last: false,
            text: match person {
                PersonRef::Name { id, name } => format!("0{}\u{0}{id}", name_key(name)),
                PersonRef::Role { role } => format!("1{}", name_key(role)),
                PersonRef::Someone => "2".to_owned(),
                PersonRef::Former { code } => format!("3{code}"),
            },
            number: 0,
        },
        ShownKey::Other(key) => key.place(),
        ShownKey::Tag { id, .. } => super::super::request::GroupKey::Tag(*id).place(),
        ShownKey::Label { id, .. } => super::super::request::GroupKey::Label(*id).place(),
    }
}

/// The place of a group in the order asked for.
fn place_of(key: &ShownKey, total: &Total, order: GroupOrder) -> Place {
    Place {
        // The group without a value is last in either order.
        smaller: match order {
            GroupOrder::Key => 0,
            GroupOrder::Count
                if key == &ShownKey::Other(super::super::request::GroupKey::Empty) =>
            {
                u64::MAX
            }
            GroupOrder::Count => u64::MAX - 1 - total.issues.min(u64::MAX - 1),
        },
        key: key_place(key),
    }
}

/// A group with its place in the order of a page.
pub(super) type Placed = (Place, ShownKey, Total);

/// The groups after `after` in the order asked for, a page of them, and the cursor of the next
/// page when there are more.
///
/// # Errors
/// An internal error when the place of the last group cannot be written down.
pub(super) fn page_after(
    groups: Vec<(ShownKey, Total)>,
    order: GroupOrder,
    after: Option<&Place>,
) -> Result<(Vec<Placed>, Option<String>), AppError> {
    let mut ordered: Vec<Placed> = groups
        .into_iter()
        .map(|(key, total)| (place_of(&key, &total, order), key, total))
        .collect();
    ordered.sort_by(|one, other| one.0.cmp(&other.0));
    let mut page: Vec<Placed> = ordered
        .into_iter()
        .filter(|(place, _, _)| after.is_none_or(|after| place > after))
        .take(GROUPS_PAGE + 1)
        .collect();
    if page.len() <= GROUPS_PAGE {
        return Ok((page, None));
    }
    page.truncate(GROUPS_PAGE);
    let next = page
        .last()
        .map(|(place, _, _)| serde_json::to_string(place))
        .transpose()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok((page, next))
}
