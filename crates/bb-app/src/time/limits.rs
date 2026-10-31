//! The limits of time on issues (ADR-0035, decisions 1 and 3), each defined once with the refusal
//! that says it. The longest and the shortest entry are those of `bb_core::time`.

use bb_core::ErrorCode;
use bb_core::text::has_stray_control;
use bb_core::time::Minutes;
use bb_core::time::length::{self, LengthMistake};
use jiff::{SignedDuration, Timestamp};

use crate::auth::AppError;

/// How many times a second a person may start or switch their timer: one. A second request in the
/// same second is refused with `timer.too-fast`.
pub const TIMER_CHANGES_PER_SECOND: i64 = 1;

/// The most time entries a person makes on one date: refused beyond with `time.too-many-entries`.
pub const TIME_ENTRIES_PER_DAY_MAX: u64 = 200;

/// The longest note of a time entry, in characters: refused beyond with `time.note-too-long`.
pub const TIME_NOTE_CHARS_MAX: usize = 2_000;

/// How long a timer runs before its person is told that it runs long: the longest day of the
/// working-time rules of the preset *Germany*, 10 hours, which is also the length with *No rules*
/// (ADR-0035, decision 3). The organization's own rules take its place when they exist.
pub const LONG_TIMER_HOURS: i64 = 10;

/// When a timer that started at `started_at` runs long.
///
/// # Errors
/// An internal error for a start so late that no moment comes after it.
pub(crate) fn long_timer_at(started_at: Timestamp) -> Result<Timestamp, AppError> {
    started_at
        .checked_add(SignedDuration::from_hours(LONG_TIMER_HOURS))
        .map_err(|error| AppError::Internal(error.to_string()))
}

/// Whether a timer that was last started or switched at `changed_at` may be started or switched
/// again at `now`: at most [`TIMER_CHANGES_PER_SECOND`] changes a second.
///
/// # Errors
/// `timer.too-fast` when the last change was less than the share of a second ago.
pub(crate) fn may_change_timer(changed_at: Timestamp, now: Timestamp) -> Result<(), AppError> {
    let gap = SignedDuration::from_secs(1)
        .checked_div(i32::try_from(TIMER_CHANGES_PER_SECOND).unwrap_or(1))
        .unwrap_or(SignedDuration::from_secs(1));
    if now.duration_since(changed_at) < gap {
        return Err(ErrorCode::TimerTooFast.into());
    }
    Ok(())
}

/// Whether a person who has `booked` entries on a date may make one more.
///
/// # Errors
/// `time.too-many-entries` when the date holds [`TIME_ENTRIES_PER_DAY_MAX`] already.
pub(crate) fn room_for_an_entry(booked: u64) -> Result<(), AppError> {
    if booked >= TIME_ENTRIES_PER_DAY_MAX {
        return Err(ErrorCode::TooManyTimeEntries.into());
    }
    Ok(())
}

/// The note of an entry as it is kept: as typed, without the spaces around it, empty for none.
///
/// # Errors
/// `time.note-too-long` beyond [`TIME_NOTE_CHARS_MAX`] characters, `request.invalid` for a control
/// character other than a line break or a tab.
pub(crate) fn clean_note(note: &str) -> Result<String, AppError> {
    let note = note.trim();
    if note.chars().count() > TIME_NOTE_CHARS_MAX {
        return Err(ErrorCode::TimeNoteTooLong.into());
    }
    if has_stray_control(note) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Ok(note.to_owned())
}

/// A length of one entry, held to the bound of `bb_core::time` (the one rule of it).
///
/// # Errors
/// `time.entry-too-short` for less than the shortest entry, `time.entry-too-long` for more than the
/// longest.
pub(crate) fn entry_length(minutes: Minutes) -> Result<Minutes, AppError> {
    length::time_entry(minutes).map_err(|mistake| {
        match mistake {
            LengthMistake::TooLong => ErrorCode::TimeEntryTooLong,
            LengthMistake::Zero | LengthMistake::NotALength => ErrorCode::TimeEntryTooShort,
        }
        .into()
    })
}

#[cfg(test)]
mod tests {
    use bb_core::time::{TIME_ENTRY_MINUTES_MAX, TIME_ENTRY_MINUTES_MIN};

    use super::*;

    fn refused<T>(result: &Result<T, AppError>) -> Option<ErrorCode> {
        match result {
            Err(AppError::Refused(code)) => Some(*code),
            _ => None,
        }
    }

    #[test]
    fn time_01_a_timer_changes_at_most_once_a_second() {
        let at = Timestamp::from_second(1_000).unwrap();
        let soon = at.checked_add(SignedDuration::from_millis(999)).unwrap();
        let later = at.checked_add(SignedDuration::from_secs(1)).unwrap();
        assert_eq!(
            refused(&may_change_timer(at, soon)),
            Some(ErrorCode::TimerTooFast)
        );
        assert!(may_change_timer(at, later).is_ok());
    }

    #[test]
    fn time_02_a_day_holds_at_most_two_hundred_entries() {
        assert!(room_for_an_entry(TIME_ENTRIES_PER_DAY_MAX - 1).is_ok());
        assert_eq!(
            refused(&room_for_an_entry(TIME_ENTRIES_PER_DAY_MAX)),
            Some(ErrorCode::TooManyTimeEntries)
        );
    }

    #[test]
    fn time_02_a_note_is_plain_text_of_at_most_two_thousand_characters() {
        assert_eq!(
            clean_note("  fixed the login \n").unwrap(),
            "fixed the login"
        );
        assert_eq!(clean_note("   ").unwrap(), "");
        let longest = "a".repeat(TIME_NOTE_CHARS_MAX);
        assert_eq!(clean_note(&longest).unwrap(), longest);
        assert_eq!(
            refused(&clean_note(&format!("{longest}a"))),
            Some(ErrorCode::TimeNoteTooLong)
        );
        assert_eq!(
            refused(&clean_note("bell\u{7}")),
            Some(ErrorCode::InvalidRequest)
        );
    }

    #[test]
    fn time_02_an_entry_is_held_to_the_shortest_and_the_longest() {
        assert_eq!(
            entry_length(TIME_ENTRY_MINUTES_MIN).unwrap(),
            TIME_ENTRY_MINUTES_MIN
        );
        assert_eq!(
            entry_length(TIME_ENTRY_MINUTES_MAX).unwrap(),
            TIME_ENTRY_MINUTES_MAX
        );
        assert_eq!(
            refused(&entry_length(Minutes::default())),
            Some(ErrorCode::TimeEntryTooShort)
        );
        assert_eq!(
            refused(&entry_length(Minutes::new(
                TIME_ENTRY_MINUTES_MAX.get() + 1
            ))),
            Some(ErrorCode::TimeEntryTooLong)
        );
    }
}
