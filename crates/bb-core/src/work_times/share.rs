//! The share of a person in a project (PLAN-15, ADR-0030, decision 2).

/// How much of a person's work hours belong to one project, from 0 to 100 percent. A person with
/// no row has [`Share::START`]; while the person has no role in the project the share counts as
/// nothing and is kept, and it counts again when a role comes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct Share(u8);

impl Share {
    /// The share every person starts with: all of the work hours.
    pub const START: Self = Self(100);

    /// No share: what a person's share counts as while they have no role in the project.
    pub const NOTHING: Self = Self(0);

    /// The most a share can be, in percent.
    pub const PERCENT_MAX: u8 = 100;

    /// The share of `percent`; `None` above 100.
    #[must_use]
    pub const fn new(percent: u8) -> Option<Self> {
        if percent <= Self::PERCENT_MAX {
            Some(Self(percent))
        } else {
            None
        }
    }

    /// The share in percent.
    #[must_use]
    pub const fn percent(self) -> u8 {
        self.0
    }

    /// The part of `minutes` that belongs to the project, in whole minutes (the lower one).
    #[must_use]
    pub const fn of(self, minutes: u32) -> u32 {
        minutes * self.0 as u32 / Self::PERCENT_MAX as u32
    }
}

/// What the shares of one person add up to, in percent: more than 100 is *Too much at once* for
/// the person and the leads of those projects.
#[must_use]
pub fn sum_of_shares(shares: impl IntoIterator<Item = Share>) -> u32 {
    shares
        .into_iter()
        .map(|share| u32::from(share.percent()))
        .sum()
}

/// Whether shares that are in force add up to more than 100 percent: the one rule of *Too much at
/// once*, for the person and for a project lead alike. Only shares that were set and apply count
/// (a person with no row has the start value, which is no choice of anybody).
#[must_use]
pub fn over_100(shares: impl IntoIterator<Item = Share>) -> bool {
    sum_of_shares(shares) > u32::from(Share::PERCENT_MAX)
}
