//! A warning of the working-time rules: which rule a day broke, with the numbers that broke it,
//! stored with the day and shown as a plain sentence (TIME-09). The sentence is a [`Said`], a tree
//! of message keys and lengths of time that the server and the browser both render from their
//! catalog, so the words are written once.

use serde::{Deserialize, Serialize};

use crate::message;
use crate::time::Minutes;
use crate::wording::{Said, length_sentence};

/// One broken rule of a day, with its numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "kebab-case")]
pub enum Warning {
    /// More work in the day than the rules allow.
    LongDay {
        /// The work of the day.
        worked: Minutes,
        /// The most the rules allow.
        most: Minutes,
    },
    /// Less break than the rules ask for after this much work; only breaks of at least the
    /// shortest that counts are counted.
    ShortBreak {
        /// The work of the day.
        worked: Minutes,
        /// The breaks that count.
        breaks: Minutes,
        /// The break the rules ask for.
        needed: Minutes,
        /// The work after which they ask for it.
        after: Minutes,
    },
    /// More work in a row than the rules allow without a break that counts.
    LongStretch {
        /// The longest work in a row.
        stretch: Minutes,
        /// The most the rules allow.
        most: Minutes,
        /// The shortest break that ends work in a row.
        shortest: Minutes,
    },
    /// Less rest since the end of the day before than the rules ask for.
    ShortRest {
        /// The rest between the two days.
        rest: Minutes,
        /// The rest the rules ask for.
        needed: Minutes,
    },
}

impl Warning {
    /// The sentence of the warning, for example *"10 hours 35 minutes worked. The rules allow 10
    /// hours a day at most."* Each length goes to its placeholder by name.
    #[must_use]
    pub fn said(&self) -> Said {
        let length = |minutes: Minutes| length_sentence(minutes);
        match *self {
            Self::LongDay { worked, most } => said!(
                message::working_time_warning_long_day,
                message::WorkingTimeWarningLongDay {
                    most: length(most),
                    worked: length(worked),
                }
            ),
            Self::ShortBreak {
                worked,
                breaks,
                needed,
                after,
            } if breaks.get() == 0 => said!(
                message::working_time_warning_no_break,
                message::WorkingTimeWarningNoBreak {
                    after: length(after),
                    needed: length(needed),
                    worked: length(worked),
                }
            ),
            Self::ShortBreak {
                worked,
                breaks,
                needed,
                after,
            } => said!(
                message::working_time_warning_short_break,
                message::WorkingTimeWarningShortBreak {
                    after: length(after),
                    breaks: length(breaks),
                    needed: length(needed),
                    worked: length(worked),
                }
            ),
            Self::LongStretch {
                stretch,
                most,
                shortest,
            } => said!(
                message::working_time_warning_long_stretch,
                message::WorkingTimeWarningLongStretch {
                    most: length(most),
                    shortest: length(shortest),
                    stretch: length(stretch),
                }
            ),
            Self::ShortRest { rest, needed } => said!(
                message::working_time_warning_short_rest,
                message::WorkingTimeWarningShortRest {
                    needed: length(needed),
                    rest: length(rest),
                }
            ),
        }
    }
}
