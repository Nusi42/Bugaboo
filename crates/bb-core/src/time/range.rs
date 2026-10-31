//! The ranges that lengths of time are grouped in (TIME-03): *Group by* a length (*Estimated time*,
//! *Time left*, *Time spent*) puts each issue in the range of its length, never in a group of its
//! own minutes. The ranges are counted in hours, which mean the same in every organization (ADR-0005,
//! decision 10); the compiler of the search and the screens read them from here.

use serde::Serialize;

use super::{MINUTES_PER_HOUR, Minutes};

/// Where each range starts, in hours: under 1 hour, 1 to 4 hours, 4 to 8, 8 to 16, 16 to 40, and 40
/// hours or more.
const STARTS_IN_HOURS: [u32; 6] = [0, 1, 4, 8, 16, 40];

/// A range of lengths of time: from `from` (included) up to `to` (not included), or with no end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct LengthRange {
    /// Where it starts.
    pub from: Minutes,
    /// Where the next one starts; none for the last.
    pub to: Option<Minutes>,
}

impl LengthRange {
    /// Every range, from the shortest lengths to the longest.
    #[must_use]
    pub fn all() -> Vec<Self> {
        let starts: Vec<Minutes> = STARTS_IN_HOURS
            .iter()
            .map(|hours| Minutes::new(hours * MINUTES_PER_HOUR))
            .collect();
        starts
            .iter()
            .enumerate()
            .map(|(at, from)| Self {
                from: *from,
                to: starts.get(at + 1).copied(),
            })
            .collect()
    }

    /// The range that starts at `from`; none when no range starts there.
    #[must_use]
    pub fn starting_at(from: Minutes) -> Option<Self> {
        Self::all().into_iter().find(|range| range.from == from)
    }

    /// The range a length is in.
    #[must_use]
    pub fn of(length: Minutes) -> Self {
        let all = Self::all();
        all.iter()
            .rev()
            .find(|range| range.from <= length)
            .or_else(|| all.first())
            .copied()
            .unwrap_or(Self {
                from: Minutes::new(0),
                to: None,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_03_every_length_is_in_one_range_and_the_ranges_follow_each_other() {
        let all = LengthRange::all();
        assert_eq!(all.first().map(|range| range.from), Some(Minutes::new(0)));
        assert_eq!(all.last().and_then(|range| range.to), None);
        for pair in all.windows(2) {
            assert_eq!(pair[0].to, Some(pair[1].from));
        }
        assert_eq!(LengthRange::of(Minutes::new(59)), all[0]);
        assert_eq!(LengthRange::of(Minutes::new(60)), all[1]);
        assert_eq!(LengthRange::of(Minutes::new(100_000)), all[5]);
        assert_eq!(LengthRange::starting_at(Minutes::new(240)), Some(all[2]));
        assert_eq!(LengthRange::starting_at(Minutes::new(241)), None);
    }
}
