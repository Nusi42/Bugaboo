//! The working time of the sample: issues are made, commented and closed on working days, from
//! eight to eighteen o'clock UTC. The plan counts in *working seconds* from the start of the first
//! working day, so a delay of an hour or a day never lands on a night or a weekend; [`Calendar`]
//! turns a working second into a moment.

use bb_core::time::SECONDS_PER_DAY as DAY;
/// When work starts on a working day, in seconds after midnight.
const WORK_FROM: i64 = 8 * 3600;
/// How long a working day is, in seconds.
pub(crate) const WORK_DAY: i64 = 10 * 3600;

/// Whether the day number (days since 1970-01-01, a Thursday) is a Monday to Friday.
fn is_working_day(day: i64) -> bool {
    (day + 3).rem_euclid(7) < 5
}

/// How far apart the changes of a day are at most, in seconds: the people of the sample work
/// without a break, so a session never idles for long and signs in once a day.
const APART: i64 = 90;
/// When the first change of a day is made, after the start of work.
const WARM_UP: i64 = 3600;

/// The working days between two moments.
#[derive(Clone, Debug)]
pub(crate) struct Calendar {
    /// The numbers of the working days, in order.
    days: Vec<i64>,
}

impl Calendar {
    /// The working days from the day after `from` up to the day before `until` (seconds since
    /// 1970), so nothing is made before the first or after the last moment.
    pub(crate) fn between(from: i64, until: i64) -> Self {
        let first = from.div_euclid(DAY) + 1;
        let last = until.div_euclid(DAY) - 1;
        Self {
            days: (first..=last).filter(|day| is_working_day(*day)).collect(),
        }
    }

    /// How many working seconds there are.
    pub(crate) fn length(&self) -> i64 {
        i64::try_from(self.days.len()).unwrap_or(0) * WORK_DAY
    }

    /// The moment (seconds since 1970) of a working second; the last one for a second past the
    /// end.
    pub(crate) fn moment(&self, working: i64) -> i64 {
        let working = working.clamp(0, (self.length() - 1).max(0));
        let at = usize::try_from(working / WORK_DAY).unwrap_or(0);
        let day = self.days.get(at).copied().unwrap_or_default();
        day * DAY + WORK_FROM + working % WORK_DAY
    }

    /// The moments of changes planned at these working seconds (in order of time): the changes of
    /// a day follow each other [`APART`] seconds apart from an hour after work starts, or closer
    /// together when a day has so many that they would not fit. The order is the same as the order
    /// of the working seconds; no two changes have the same moment.
    pub(crate) fn pack(&self, working: &[i64]) -> Vec<i64> {
        let mut moments = Vec::with_capacity(working.len());
        let mut from = 0;
        while from < working.len() {
            let day = self.day(working[from]);
            let to = from
                + working[from..]
                    .iter()
                    .take_while(|at| self.day(**at) == day)
                    .count();
            let number = i64::try_from(to - from).unwrap_or(1).max(1);
            let day_start = self.moment(working[from] - working[from] % WORK_DAY);
            let (start, apart) = if number * APART + WARM_UP <= WORK_DAY {
                (day_start + WARM_UP, APART)
            } else {
                (day_start, (WORK_DAY / number).max(1))
            };
            moments.extend((0..number).map(|at| start + at * apart));
            from = to;
        }
        moments
    }

    /// The number of the day (since 1970) a working second falls on.
    pub(crate) fn day(&self, working: i64) -> i64 {
        self.moment(working).div_euclid(DAY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2027-01-04 is a Monday.
    const MONDAY: i64 = 1_799_020_800;

    #[test]
    fn only_monday_to_friday_between_the_moments_count_and_a_second_is_inside_working_hours() {
        assert!(is_working_day(MONDAY.div_euclid(DAY)));
        let calendar = Calendar::between(MONDAY - 1, MONDAY + 14 * DAY);
        assert_eq!(calendar.length(), 10 * WORK_DAY);
        for second in (0..calendar.length()).step_by(997) {
            let moment = calendar.moment(second);
            let day = moment.div_euclid(DAY);
            assert!(is_working_day(day), "{day}");
            let into = moment - day * DAY;
            assert!((WORK_FROM..WORK_FROM + WORK_DAY).contains(&into), "{into}");
        }
        assert_eq!(calendar.moment(0), MONDAY + WORK_FROM);
        assert_eq!(calendar.moment(5 * WORK_DAY), MONDAY + 7 * DAY + WORK_FROM);
    }

    #[test]
    fn the_changes_of_a_day_are_packed_close_together_inside_the_working_day() {
        let calendar = Calendar::between(MONDAY - 1, MONDAY + 14 * DAY);
        // Three changes on the first day, two on the second, a thousand on the third.
        let mut working = vec![5, 9_000, 30_000, WORK_DAY + 7, WORK_DAY + 100];
        working.extend((0..1000).map(|at| 2 * WORK_DAY + at * 30));
        let moments = calendar.pack(&working);
        assert_eq!(moments.len(), working.len());
        assert!(moments.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(moments[0], MONDAY + WORK_FROM + WARM_UP);
        assert_eq!(moments[1] - moments[0], APART);
        assert_eq!(moments[3], MONDAY + DAY + WORK_FROM + WARM_UP);
        // The crowded day is spread over the whole working day and still ends inside it.
        let last = moments[moments.len() - 1];
        assert!(last < MONDAY + 2 * DAY + WORK_FROM + WORK_DAY, "{last}");
        for moment in moments {
            assert!(is_working_day(moment.div_euclid(DAY)));
        }
    }

    #[test]
    fn working_seconds_only_go_forward_and_a_second_past_the_end_is_the_last_one() {
        let calendar = Calendar::between(MONDAY, MONDAY + 30 * DAY);
        let mut before = i64::MIN;
        for second in (0..calendar.length()).step_by(1013) {
            let moment = calendar.moment(second);
            assert!(moment > before);
            before = moment;
        }
        assert_eq!(
            calendar.moment(calendar.length() + 99),
            calendar.moment(calendar.length() - 1)
        );
    }
}
