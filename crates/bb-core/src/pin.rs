//! Pins (CFG-05, ADR-0050, decision 12): what a person pinned to their sidebar, in their own order.
//! A pin is the person's own: nobody pins for somebody else, and the organization sets none.

use crate::id::{DashboardId, EntityId, ProjectId, SavedSearchId, UserId};
use crate::personal::Entity;

/// The most pins a person has.
pub const PINS_MAX: usize = 30;

words! {
    /// What a pin points to.
    pub enum PinKind {
        /// A project.
        Project => "project",
        /// A saved search.
        SavedSearch => "saved-search",
        /// A dashboard.
        Dashboard => "dashboard",
    }
}

words_in_api!(
    PinKind,
    "What a pin points to: a project, a saved search or a dashboard."
);

/// What a pin points to, with the typed ID of the record: the one place that tells a kind of pin
/// and its record apart from the two columns a pin is stored in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinTarget {
    /// A project.
    Project(ProjectId),
    /// A saved search.
    SavedSearch(SavedSearchId),
    /// A dashboard.
    Dashboard(DashboardId),
}

impl PinTarget {
    /// What the pin points to.
    #[must_use]
    pub const fn kind(self) -> PinKind {
        match self {
            Self::Project(_) => PinKind::Project,
            Self::SavedSearch(_) => PinKind::SavedSearch,
            Self::Dashboard(_) => PinKind::Dashboard,
        }
    }

    /// The ID of the record, as the pin stores it.
    #[must_use]
    pub fn record(self) -> EntityId {
        match self {
            Self::Project(id) => id.into(),
            Self::SavedSearch(id) => id.into(),
            Self::Dashboard(id) => id.into(),
        }
    }

    /// The target of a stored pin; none when the ID is not the kind of ID the kind needs.
    #[must_use]
    pub fn of(kind: PinKind, record: EntityId) -> Option<Self> {
        let bytes = record.to_bytes();
        match kind {
            PinKind::Project => ProjectId::from_bytes(bytes).ok().map(Self::Project),
            PinKind::SavedSearch => SavedSearchId::from_bytes(bytes).ok().map(Self::SavedSearch),
            PinKind::Dashboard => DashboardId::from_bytes(bytes).ok().map(Self::Dashboard),
        }
    }
}

/// One pin of a person: what it points to and its place among theirs, counted from 0.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Pin {
    /// Whose pin it is.
    #[personal(category = "identity", basis = "contract")]
    #[person(holder)]
    pub person: UserId,
    /// What it points to.
    #[personal(category = "activity", basis = "contract")]
    pub kind: PinKind,
    /// The record it points to: a project or a saved search, never a person.
    #[personal(category = "activity", basis = "contract")]
    #[names_no_person]
    pub target: EntityId,
    /// Its place among the person's pins, counted from 0.
    #[personal(category = "activity", basis = "contract")]
    pub place: i64,
}

impl Pin {
    /// What the pin points to, when its record's ID is one of the kind.
    #[must_use]
    pub fn points_to(&self) -> Option<PinTarget> {
        PinTarget::of(self.kind, self.target)
    }

    /// Whether the pin points to `target`.
    #[must_use]
    pub fn is_to(&self, target: PinTarget) -> bool {
        self.points_to() == Some(target)
    }
}

/// The pin of `pins` that points to `target`, if the person pinned it: the one place that decides
/// whether something is pinned already.
#[must_use]
pub fn holds(pins: &[Pin], target: PinTarget) -> Option<&Pin> {
    pins.iter().find(|pin| pin.is_to(target))
}

/// The place of a new pin: after the others.
#[must_use]
pub fn next_place(pins: &[Pin]) -> i64 {
    pins.iter().map(|pin| pin.place + 1).max().unwrap_or(0)
}
