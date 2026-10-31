//! The one check of the working-time rules (ADR-0035, decision 6): a day and the day before it in,
//! the warnings of the day out. It runs once for every change of a day, inside the change, for the
//! day and for the day after it (whose rest the change may have moved), and its answer is stored
//! with the day. A warning never refuses anything: the record shows what really happened (TIME-09).
//!
//! Only what has ended is checked: a period or a break that runs counts as nothing yet, so the
//! warnings of a day that runs are those of what it has so far; the next change checks again.

use super::totals::{Piece, gaps, minutes_between, pieces, sum, worked};
use super::{Rules, Times, Warning};
use crate::time::Minutes;

/// The warnings of `day` under `rules`, with the day before it for the rest between the two. No
/// rules, no warnings.
#[must_use]
pub fn warnings(day: &Times, before: Option<&Times>, rules: Option<&Rules>) -> Vec<Warning> {
    let Some(rules) = rules else {
        return Vec::new();
    };
    let pieces = pieces(day, None);
    let worked = worked(&pieces);
    let mut found = Vec::new();
    if worked > rules.day_most {
        found.push(Warning::LongDay {
            worked,
            most: rules.day_most,
        });
    }
    let counted = sum(gaps(&pieces).filter(|gap| *gap >= rules.shortest_break));
    if let Some(due) = rules.break_due(worked)
        && counted < due.needed
    {
        found.push(Warning::ShortBreak {
            worked,
            breaks: counted,
            needed: due.needed,
            after: due.after,
        });
    }
    let stretch = longest_stretch(&pieces, rules.shortest_break);
    if stretch > rules.in_a_row_most {
        found.push(Warning::LongStretch {
            stretch,
            most: rules.in_a_row_most,
            shortest: rules.shortest_break,
        });
    }
    if let (Some(end), Some(start)) = (before.and_then(Times::last_end), day.first_start()) {
        let rest = minutes_between(end, start);
        if rest < rules.rest_least {
            found.push(Warning::ShortRest {
                rest,
                needed: rules.rest_least,
            });
        }
    }
    found
}

/// The longest time of work in a row: pieces whose gap is shorter than the shortest break that
/// counts belong to one stretch, from the start of its first piece to the end of its last.
fn longest_stretch(pieces: &[Piece], shortest: Minutes) -> Minutes {
    let mut longest = Minutes::default();
    let mut current: Option<Piece> = None;
    for (start, end) in pieces.iter().copied() {
        current = match current {
            Some((from, to)) if minutes_between(to, start) < shortest => Some((from, end)),
            _ => Some((start, end)),
        };
        if let Some((from, to)) = current {
            longest = longest.max(minutes_between(from, to));
        }
    }
    longest
}
