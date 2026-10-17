//! What the offers use cases take and give back. Inputs that change something are named after what
//! they do; inputs that only read end in `Query` (ARCH-EVENTS). Everything that is given back is
//! wrapped in `Authorized<T>` by the use case.

use bb_core::offer::OfferSubject;
use bb_policy::Masked;
use serde::Serialize;

use crate::access::ProjectLabel;

/// Read the rows that say where a subject is offered.
pub struct OffersQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue type or the group of labels.
    pub subject: OfferSubject,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many rows the page holds at most.
    pub limit: u32,
}

/// One row of where a subject is offered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfferRowView {
    /// The project (with its sub-projects); none for the whole organization.
    pub place: Option<ProjectLabel>,
    /// Whether the subject is offered there.
    pub offered: bool,
    /// Whether the person may change the row (or take it away) at that place.
    pub may_change: bool,
}

/// A page of the rows that say where a subject is offered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfferRowPage {
    /// The rows. A place without a row follows the place above it.
    pub items: Vec<OfferRowView>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for OfferRowPage {}

/// Where a subject is offered after a change: every row, and the version the subject has now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Offers {
    /// The rows.
    pub rows: Vec<OfferRowView>,
    /// Counts up with every change of the subject, rows included.
    pub version: i64,
}

impl Masked for Offers {}

/// A place to change and what it is to say.
pub struct OfferChange {
    /// The project, by its ID or short name; none for the whole organization.
    pub project: Option<String>,
    /// `Some(true)` offers the subject there, `Some(false)` does not, `None` takes the row away so
    /// the place follows the place above it again.
    pub offered: Option<bool>,
}

/// Change where a subject is offered: all the changes together or none.
pub struct ChangeOffers {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue type or the group of labels.
    pub subject: OfferSubject,
    /// The version the person looked at.
    pub version: i64,
    /// The places to change.
    pub changes: Vec<OfferChange>,
}

/// Where a subject is offered in one project, for the lists that take a project.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfferedHere {
    /// Whether it is offered in the project (by the row that counts there).
    pub offered: bool,
    /// The row at exactly this project, if it has one.
    pub own: Option<bool>,
    /// Whether the person may change it for this project.
    pub may_change: bool,
}

impl Masked for OfferedHere {}
