//! Where issue types and groups of labels are offered (ISS-02, TAG-02, ADR-0027 amendment).
//!
//! The answer to "is this offered here" is `bb_core::offer::is_offered`: one row per place,
//! the nearest place wins (`bb_core::lookup::nearest_row`). The use cases here read and change those
//! rows; the ones that write an issue or a label on an issue ask the same function, inside their
//! own change, after the shared lock of the workflows (`place.rs`).
//!
//! Who may change what is judged at a place. A subject created for the whole organization is looked
//! after by whoever holds the permission for the whole organization; one created in a project, by
//! whoever holds it in that project. A row is judged at the place it is for: the Project lead of a
//! project can switch an organization's type off in their project and below, and nowhere else.
//!
//! Who may know a subject is one rule (`known.rs`), used by every list and every route that takes a
//! subject by its ID.

mod change;
mod here;
mod known;
mod place;
mod store;
mod subject;
mod types;

pub use change::{change_offers, offers};
pub use store::OfferStore;
pub use types::*;

pub(crate) use here::{Here, Standing};
pub(crate) use known::label_is_known;
pub(crate) use place::{GroupOffers, ensure_type_takes_issues_in, group_offers_in};
pub(crate) use subject::Subject;
