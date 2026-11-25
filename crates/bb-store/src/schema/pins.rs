//! The table of the pins of a person (CFG-05, ADR-0050, decision 12).

use super::{Iden, StoredTable};

/// What each person pinned to their sidebar, in their own order.
#[derive(Clone, Copy, Iden)]
pub(crate) enum PinRow {
    #[iden = "pin"]
    Table,
    Person,
    Kind,
    Target,
    Place,
}

stored_tables! {
    PinRow => bb_core::pin::Pin {
        Person => person,
        Kind => kind,
        Target => target,
        Place => place,
    },
}
