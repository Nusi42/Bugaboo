//! The time factor (PLAN-14, ADR-0030, decision 4 and amendment M8): the person's own estimate for
//! one issue, as a share of *Estimated time*. It belongs to the issue and goes back to ×1 when the
//! first assigned person changes; it is never kept per person, never searched, sorted, grouped or
//! summed, and never set by AI.

use std::ops::RangeInclusive;

use serde::Serialize;

use crate::time::Minutes;

/// The time factors a person may give, in percent of the estimate: from ×0.1 to ×10.
pub const TIME_FACTOR_PERCENT_RANGE: RangeInclusive<u16> = 10..=1000;

/// How much longer or shorter than the estimate an issue takes its person on the bar, in percent:
/// 150 is *×1.5*. An issue without one has [`TimeFactor::ONE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct TimeFactor(u16);

impl Default for TimeFactor {
    fn default() -> Self {
        Self::ONE
    }
}

impl TimeFactor {
    /// ×1: the estimate as it is.
    pub const ONE: Self = Self(100);

    /// The factor of `percent` percent; `None` outside [`TIME_FACTOR_PERCENT_RANGE`].
    #[must_use]
    pub fn new(percent: u16) -> Option<Self> {
        TIME_FACTOR_PERCENT_RANGE
            .contains(&percent)
            .then_some(Self(percent))
    }

    /// The percent.
    #[must_use]
    pub const fn percent(self) -> u16 {
        self.0
    }

    /// The factor of *Takes me …* (`takes`) against the estimate, to the nearest percent; `None`
    /// without an estimate, or when the factor would be outside the range.
    #[must_use]
    pub fn of_length(estimate: Minutes, takes: Minutes) -> Option<Self> {
        let estimate = u64::from(estimate.get());
        if estimate == 0 {
            return None;
        }
        let percent = (u64::from(takes.get()) * 100 + estimate / 2) / estimate;
        u16::try_from(percent).ok().and_then(Self::new)
    }

    /// The effort of an issue with this estimate: the estimate times the factor, rounded up to the
    /// whole minute, so a factor never makes work disappear.
    #[must_use]
    pub fn effort(self, estimate: Minutes) -> Minutes {
        let minutes = (u64::from(estimate.get()) * u64::from(self.0)).div_ceil(100);
        Minutes::new(u32::try_from(minutes).unwrap_or(u32::MAX))
    }
}
