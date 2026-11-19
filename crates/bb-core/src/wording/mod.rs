//! The words of a change and of a notification, chosen once for the server and the browser
//! (ADR-0033, the amendment about value wording).
//!
//! This code picks the message keys and builds the words of the values: the choice of sentence
//! (*changed*, *set*, *cleared*, the long text), the words of a status and a priority, yes and no,
//! a length of time and a list of names, and the sentences of a notification. It writes no text of
//! its own: the result is a [`Said`], a tree of message keys and values, and each side looks the
//! keys up in its own catalog. The server renders it with the English catalog it holds
//! ([`Said::render`]); the browser gets the same tree from `bb-wasm` and renders it with its
//! catalog.
//!
//! What depends on the region (the order of a day, the marks of a number, the zone of a moment) is
//! not decided here: the screen uses `Intl` for the choice *device*, which a server cannot run. The
//! caller words those values (the server with [`Region`]) and hands the words in. The two ways of
//! writing are held together by test vectors that this crate makes ([`vectors`]).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::edit::NoticeKind;
use crate::fields::Field;
use crate::history::{FileEvent, TimeEvent};
use crate::issue::Priority;
use crate::message;
use crate::notification::NotificationKind;
use crate::own_fields::FieldType;
use crate::time::{MINUTES_PER_HOUR, Minutes};
use crate::workflow::starter::STARTERS;

#[macro_use]
mod said;
pub mod region;
mod typed_length;
pub mod vectors;

pub use region::{Choices, Region};
pub use said::Said;
pub use typed_length::{length_read, time_entry_read};

/// What ends a sentence of Bugaboo in a line of an e-mail: the screen says it without.
const SENTENCE_END: &str = ".";

/// What already ends a sentence: a value of a person may end in one of these, then the sentence
/// gets no second mark.
const ENDS: [char; 4] = ['.', '?', '!', '\u{2026}'];

/// A sentence with a full stop, unless it ends in a full stop, a question mark, an exclamation
/// mark or an ellipsis already (a value of a person may end in one: *Login fails.*; a line cut
/// short ends in an ellipsis).
#[must_use]
pub fn end_sentence(text: &str) -> String {
    if text.ends_with(ENDS) {
        text.to_owned()
    } else {
        format!("{text}{SENTENCE_END}")
    }
}

/// Names in a sentence as the catalog lists them: `Ana, Ben, and Cleo`. `None` for no name.
#[must_use]
pub fn list_said(items: &[String]) -> Option<Said> {
    let said = |text: &String| Said::text(text.as_str());
    match items {
        [] => None,
        [one] => Some(said(one)),
        [first, second] => Some(said!(
            message::list_two,
            message::ListTwo {
                first: said(first),
                second: said(second)
            }
        )),
        [start @ .., last] => Some(said!(
            message::list_many,
            message::ListMany {
                last: said(last),
                start: Said::text(start.join(", "))
            }
        )),
    }
}

/// The word of a status or a priority; any other text as it is.
#[must_use]
pub fn word_of(field: Option<Field>, text: &str) -> Said {
    let known = match field {
        Some(Field::Status) => STARTERS
            .iter()
            .flat_map(|starter| starter.statuses)
            .find(|status| status.key == text)
            .map(|status| status.name),
        Some(Field::Priority) => Priority::from_code(text).map(Priority::name),
        _ => None,
    };
    known.map_or_else(|| Said::text(text), Said::key)
}

/// A length of time in a field, a column or a short status line: "2h 5m", "45m" or "0m" (ADR-0033,
/// amendment M6). A length has three written forms and this writer makes all of them: this one,
/// [`length_sentence`] in sentences, and [`Minutes::on_clock`] (`2:05`) in a grid and on a running
/// timer.
#[must_use]
pub fn length_said(length: Minutes) -> Said {
    let (hours, minutes) = (
        length.get() / MINUTES_PER_HOUR,
        length.get() % MINUTES_PER_HOUR,
    );
    let hour = Said::key(message::FIELDS_UNIT_HOUR);
    let minute = Said::key(message::FIELDS_UNIT_MINUTE);
    if hours == 0 {
        Said::Parts(vec![Said::text(minutes.to_string()), minute])
    } else if minutes == 0 {
        Said::Parts(vec![Said::text(hours.to_string()), hour])
    } else {
        Said::Parts(vec![
            Said::text(hours.to_string()),
            hour,
            Said::text(" "),
            Said::text(minutes.to_string()),
            minute,
        ])
    }
}

/// A length of time in a sentence: "42 minutes", "10 hours", "39 hours 28 minutes", each unit in
/// the singular or the plural (see [`length_said`]).
#[must_use]
pub fn length_sentence(length: Minutes) -> Said {
    let (hours, minutes) = (
        length.get() / MINUTES_PER_HOUR,
        length.get() % MINUTES_PER_HOUR,
    );
    let count = Said::Count;
    let hours_said = Said::with(&message::time_length_hours(0), count(hours));
    let minutes_said = Said::with(&message::time_length_minutes(0), count(minutes));
    match (hours, minutes) {
        (0, _) => minutes_said,
        (_, 0) => hours_said,
        _ => said!(
            message::time_length_hours_and_minutes,
            message::TimeLengthHoursAndMinutes {
                hours: hours_said,
                minutes: minutes_said
            }
        ),
    }
}

/// The whole minutes in seconds of the API (a length of an own field): the rest of a minute and a
/// part of a second are dropped; `None` for a negative number or one too long to count.
fn minutes_in(seconds: f64) -> Option<Minutes> {
    if seconds < 0.0 {
        return None;
    }
    let whole = jiff::SignedDuration::try_from_secs_f64(seconds).ok()?;
    Minutes::from_seconds(whole.as_secs())
}

/// The words of a value of an own field. `worded` is the words of a number, a day or a moment in
/// the region of the reader, which the caller wrote (see [`Region::worded`]); it is `None` when the
/// value does not fit its kind. The result is `None` for a value that does not fit the kind (the
/// screen shows nothing for it either).
#[must_use]
pub fn typed_said(kind: FieldType, value: &Value, worded: Option<&str>) -> Option<Said> {
    match (kind, value) {
        (FieldType::YesNo, Value::Bool(yes)) => Some(Said::key(if *yes {
            message::FIELDS_YES
        } else {
            message::FIELDS_NO
        })),
        (FieldType::Duration, Value::Number(seconds)) => {
            seconds.as_f64().and_then(minutes_in).map(length_said)
        }
        (FieldType::WholeNumber | FieldType::Decimal, Value::Number(_))
        | (FieldType::Date | FieldType::DateTime, Value::String(_)) => worded.map(Said::text),
        (FieldType::WholeNumber | FieldType::Decimal, Value::String(text)) => {
            Some(Said::text(text.as_str()))
        }
        (
            FieldType::YesNo
            | FieldType::Duration
            | FieldType::WholeNumber
            | FieldType::Decimal
            | FieldType::Date
            | FieldType::DateTime,
            _,
        ) => None,
        (_, Value::String(text)) => Some(Said::text(text.as_str())),
        _ => None,
    }
}

/// One side of a change: what the field was or what it became.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "shown", rename_all = "kebab-case")]
pub enum Side {
    /// A word: a status, a priority or any other text.
    Text {
        /// The text.
        text: String,
    },
    /// Names, already written for the reader. No name reads as no value.
    Names {
        /// The names.
        names: Vec<String>,
    },
    /// People, already named for the reader. No person reads as *nobody*.
    People {
        /// The names, as they stand in the middle of a sentence.
        people: Vec<String>,
    },
    /// A value of an own field.
    Typed {
        /// The kind of the field.
        kind: FieldType,
        /// The value, as the API gives it.
        value: Value,
        /// The words of a number, a day or a moment in the region of the reader.
        #[serde(default)]
        worded: Option<String>,
    },
    /// An issue ID or the name of a project, as it stands.
    Plain {
        /// The text.
        text: String,
    },
}

/// The words of one side of a change; `None` when the side holds no value.
#[must_use]
pub fn side_said(field: Option<Field>, side: &Side) -> Option<Said> {
    let said = match side {
        Side::Text { text } => word_of(field, text),
        Side::Names { names } => list_said(names)?,
        Side::People { people } if people.is_empty() => Said::key(message::HISTORY_PEOPLE_NONE),
        Side::People { people } => list_said(people)?,
        Side::Typed {
            kind,
            value,
            worded,
        } => typed_said(*kind, value, worded.as_deref())?,
        Side::Plain { text } => Said::text(text.as_str()),
    };
    (said != Said::text("")).then_some(said)
}

/// A change of one field, with the words the reader sees for the people and the field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    /// Who did it, at the start of a sentence.
    pub who: String,
    /// The code of the field: a built-in field, or the ID of an own field.
    pub field: String,
    /// What the field is called.
    pub field_name: String,
    /// What it was, if it held a value.
    #[serde(default)]
    pub before: Option<Side>,
    /// What it became, if it holds a value.
    #[serde(default)]
    pub after: Option<Side>,
}

/// Whether a side holds no person: nothing, or an empty list of people.
fn is_nobody(side: Option<&Side>) -> bool {
    match side {
        None => true,
        Some(Side::People { people }) => people.is_empty(),
        Some(_) => false,
    }
}

/// The sentence of a change of one field: *Ben changed Priority from Normal to High*.
#[must_use]
pub fn change_sentence(change: &Change) -> Said {
    let field = Field::from_code(&change.field);
    let side = |side: &Option<Side>| side.as_ref().and_then(|side| side_said(field, side));
    let (who, name) = (
        Said::text(change.who.as_str()),
        Said::text(change.field_name.as_str()),
    );
    // Nobody was assigned before: the sentence is about the assignment, not about a field changing
    // from *nobody* to somebody.
    // Somebody is put where nobody was: an assignment. A field that was never there and is set (an
    // issue made with it) stays *set Assigned to: Thea*.
    let from_nobody = matches!(&change.before, Some(Side::People { people }) if people.is_empty())
        && !is_nobody(change.after.as_ref());
    match (side(&change.before), side(&change.after)) {
        (_, Some(after)) if field == Some(Field::AssignedTo) && from_nobody => {
            said!(
                message::history_assigned,
                message::HistoryAssigned { after, who }
            )
        }
        (Some(before), Some(after)) => {
            said!(
                message::history_changed,
                message::HistoryChanged {
                    after,
                    before,
                    field: name,
                    who
                }
            )
        }
        // After a colon for every field: a field such as *Assigned to* ends in the word that
        // *set … to* would need.
        (None, Some(after)) => said!(
            message::history_set,
            message::HistorySet {
                after,
                field: name,
                who
            }
        ),
        (Some(_), None) => said!(
            message::history_cleared,
            message::HistoryCleared { field: name, who }
        ),
        (None, None) => said!(
            message::history_changed_long_text,
            message::HistoryChangedLongText { field: name, who }
        ),
    }
}

/// The *Start date* or the *Due date* of a change, with the day before and the day after as the
/// region of the reader words them: none unless the change is one of the two dates and holds a day
/// on both sides.
fn worded_days(change: &Change, field: Field) -> Option<(&str, &str)> {
    fn day(side: Option<&Side>) -> Option<&str> {
        match side {
            Some(Side::Typed {
                kind: FieldType::Date,
                worded: Some(worded),
                ..
            }) => Some(worded.as_str()),
            _ => None,
        }
    }
    (Field::from_code(&change.field) == Some(field))
        .then(|| day(change.before.as_ref()).zip(day(change.after.as_ref())))
        .flatten()
}

/// The sentence of both dates of an issue changing in one event (ADR-0043, decision 6): *Lea moved
/// the dates from Nov 3–Nov 7 to Nov 10–Nov 14*. The days are the words the changes carry.
fn dates_moved_sentence(who: &str, (start, due): (&Change, &Change)) -> Option<Said> {
    let (start, due) = (
        worded_days(start, Field::StartDate)?,
        worded_days(due, Field::DueDate)?,
    );
    let range = |start: &str, due: &str| {
        said!(
            message::history_date_range,
            message::HistoryDateRange {
                due: Said::text(due),
                start: Said::text(start)
            }
        )
    };
    Some(said!(
        message::history_dates_moved,
        message::HistoryDatesMoved {
            after: range(start.1, due.1),
            before: range(start.0, due.0),
            who: Said::text(who)
        }
    ))
}

/// The sentences of the changes of one event: one for each change, except that a *Start date* and a
/// *Due date* that both move from one day to another are one sentence (ADR-0043, decision 6). The
/// one rule for every reader: the history of an issue and a notification say an event the same way,
/// and the sentence takes the place of the first of the two.
#[must_use]
pub fn change_sentences(changes: &[Change]) -> Vec<Said> {
    let first = |field: Field| {
        changes
            .iter()
            .position(|each| Field::from_code(&each.field) == Some(field))
    };
    let folded = first(Field::StartDate)
        .zip(first(Field::DueDate))
        .and_then(|(start, due)| {
            dates_moved_sentence(&changes[start].who, (&changes[start], &changes[due]))
                .map(|said| (start.min(due), start.max(due), said))
        });
    let Some((place, other, moved)) = folded else {
        return changes.iter().map(change_sentence).collect();
    };
    let mut said = Vec::with_capacity(changes.len());
    for (at, change) in changes.iter().enumerate() {
        if at == place {
            said.push(moved.clone());
        } else if at != other {
            said.push(change_sentence(change));
        }
    }
    said
}

/// What happened to a file of an issue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChange {
    /// What happened; none for an event this version does not know (the sentence then says only
    /// that a file changed).
    #[serde(default)]
    pub event: Option<FileEvent>,
    /// The name of the file; none once the file is gone for good.
    #[serde(default)]
    pub name: Option<String>,
}

/// What happened to a file, and who did it: the history names who for each item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct File {
    /// Who did it, at the start of a sentence.
    pub who: String,
    /// What happened.
    pub what: FileChange,
}

/// The sentence of a file that was added, deleted or brought back: *Ben added the file jam.txt*.
/// Once the file is gone for good it has no name and the sentence says *a file*.
#[must_use]
pub fn file_sentence(file: &File) -> Said {
    file_words(&file.who, &file.what)
}

/// The sentence of a file for the person `who` named.
fn file_words(who: &str, what: &FileChange) -> Said {
    let who = Said::text(who);
    let name = what.name.as_deref().map(Said::text);
    match (what.event, name) {
        (None, _) => said!(message::history_file_changed_unnamed, who),
        (Some(FileEvent::Added), Some(name)) => said!(
            message::history_file_added,
            message::HistoryFileAdded { name, who }
        ),
        (Some(FileEvent::Added), None) => said!(message::history_file_added_unnamed, who),
        (Some(FileEvent::Deleted), Some(name)) => said!(
            message::history_file_deleted,
            message::HistoryFileDeleted { name, who }
        ),
        (Some(FileEvent::Deleted), None) => said!(message::history_file_deleted_unnamed, who),
        (Some(FileEvent::Restored), Some(name)) => {
            said!(
                message::history_file_restored,
                message::HistoryFileRestored { name, who }
            )
        }
        (Some(FileEvent::Restored), None) => said!(message::history_file_restored_unnamed, who),
    }
}

/// What happened to a time entry of an issue (TIME-02): booked, changed or deleted, with its length
/// for a reader who may read *Time spent*.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeChange {
    /// What happened; none for an event this version does not know (the sentence then says only
    /// that time on the issue changed).
    #[serde(default)]
    pub event: Option<TimeEvent>,
    /// The length in whole minutes; none for a reader who may not read *Time spent*, and for a
    /// change that left the length as it was.
    #[serde(default)]
    pub minutes: Option<u32>,
}

/// What happened to a time entry, and who did it: the history names who for each item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Time {
    /// Who did it, at the start of a sentence.
    pub who: String,
    /// What happened.
    pub what: TimeChange,
}

/// The sentence of a time entry that was booked, changed or deleted: *Ana booked 1h 30m*. The
/// length is written by the one writer of lengths ([`length_said`]); without it the sentence says
/// only that time was booked.
#[must_use]
pub fn time_sentence(time: &Time) -> Said {
    let who = Said::text(&time.who);
    // An event this version does not know says only that booked time changed, never a length.
    let length = time
        .what
        .event
        .and(time.what.minutes)
        .map(|minutes| length_said(Minutes::new(minutes)));
    match (time.what.event, length) {
        (Some(TimeEvent::Booked), Some(length)) => said!(
            message::history_time_booked,
            message::HistoryTimeBooked { length, who }
        ),
        (Some(TimeEvent::Booked), None) => said!(message::history_time_booked_unmeasured, who),
        (Some(TimeEvent::Deleted), Some(length)) => said!(
            message::history_time_deleted,
            message::HistoryTimeDeleted { length, who }
        ),
        (Some(TimeEvent::Deleted), None) => said!(message::history_time_deleted_unmeasured, who),
        // An event this version does not know says that booked time changed.
        (Some(TimeEvent::Changed) | None, Some(length)) => said!(
            message::history_time_changed,
            message::HistoryTimeChanged { length, who }
        ),
        (Some(TimeEvent::Changed) | None, None) => {
            said!(message::history_time_changed_unmeasured, who)
        }
    }
}

/// A renewal of issue IDs that gave up.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Renewal {
    /// The name of the project, if the reader may know it.
    #[serde(default)]
    pub project: Option<String>,
}

/// A work day of the reader that somebody else changed (ADR-0035, decision 7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkDayNotice {
    /// The date of the day, already written the way the reader writes dates.
    pub date: String,
    /// Why, in the words of whoever changed it.
    #[serde(default)]
    pub reason: Option<String>,
}

words! {
    /// What somebody else did to the reader's work times (ADR-0033, amendment M8).
    pub enum WorkTimesEvent {
        /// An away time was entered for the reader.
        AwayEntered => "away-entered",
        /// An away time of the reader was changed.
        AwayChanged => "away-changed",
        /// An away time of the reader was removed.
        AwayRemoved => "away-removed",
        /// The usual work days or the work hours of the reader were changed.
        UsualChanged => "usual-changed",
    }
}

words_in_api!(
    WorkTimesEvent,
    "What somebody else did to the work times of the person: entered, changed or removed an away time, or changed the usual work days or the work hours."
);

/// The work times of the reader that somebody else changed. It names the days of an away time and
/// never its note, which only its person reads (ADR-0030, decision 2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkTimesNotice {
    /// What was done.
    pub event: WorkTimesEvent,
    /// The days of an away time, already written the way the reader writes dates (*26 Oct – 30
    /// Oct*); empty for a change of the usual work days or the work hours.
    #[serde(default)]
    pub days: String,
}

/// What a notification says, with what the sentences need.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    /// What the notification is about.
    pub kind: NotificationKind,
    /// Who did it, at the start of a sentence.
    pub who: String,
    /// Whether it is about a comment.
    #[serde(default)]
    pub comment: bool,
    /// The changes of fields it is about.
    #[serde(default)]
    pub changes: Vec<Change>,
    /// What a note about the reader's editing says.
    #[serde(default)]
    pub notice: Option<NoticeKind>,
    /// Whether a run of many changes ended well.
    #[serde(default)]
    pub run_done: Option<bool>,
    /// A renewal of issue IDs that gave up.
    #[serde(default)]
    pub renewal: Option<Renewal>,
    /// A file that was added, deleted or brought back; the notice names who did it.
    #[serde(default)]
    pub file: Option<FileChange>,
    /// A work day of the reader that somebody else changed.
    #[serde(default)]
    pub work_day: Option<WorkDayNotice>,
    /// Work times of the reader that somebody else changed.
    #[serde(default)]
    pub work_times: Option<WorkTimesNotice>,
    /// Why an administrator created a file of the reader's data (ADR-0044, decision 4) or corrected
    /// their details (decision 9), in their words.
    #[serde(default)]
    pub administrators_reason: Option<String>,
}

/// What happened, in words: one sentence, or one for each field a change touched.
#[must_use]
pub fn notification_sentences(notice: &Notice) -> Vec<Said> {
    let who = || Said::text(notice.who.as_str());
    let one = |said| vec![said];
    match notice.kind {
        NotificationKind::Mentioned => one(said!(message::notifications_mentioned, who())),
        NotificationKind::Assigned => one(said!(message::notifications_assigned, who())),
        NotificationKind::TimeFactorChanged => {
            one(said!(message::notifications_time_factor_changed, who()))
        }
        NotificationKind::EditNotice => one(match notice.notice {
            Some(NoticeKind::TakenOver) => said!(message::notifications_taken_over, who()),
            Some(NoticeKind::LetGo) => said!(message::notifications_let_go, who()),
            Some(NoticeKind::FinishAsked) | None => {
                said!(message::notifications_finish_asked, who())
            }
        }),
        NotificationKind::LongTimer => one(Said::key(message::NOTIFICATIONS_LONG_TIMER)),
        NotificationKind::AgentAsks => one(Said::key(message::NOTIFICATIONS_AGENT_ASKS)),
        NotificationKind::PlanWaits => one(Said::key(message::NOTIFICATIONS_PLAN_WAITS)),
        NotificationKind::ReadyForReview => one(Said::key(message::NOTIFICATIONS_READY_FOR_REVIEW)),
        NotificationKind::AgentGaveUp => one(Said::key(message::NOTIFICATIONS_AGENT_GAVE_UP)),
        NotificationKind::RuleRunsOutSoon => {
            one(Said::key(message::NOTIFICATIONS_RULE_RUNS_OUT_SOON))
        }
        NotificationKind::AgentStopped => one(Said::key(message::NOTIFICATIONS_AGENT_STOPPED)),
        NotificationKind::ExportReady => one(Said::key(message::NOTIFICATIONS_EXPORT_READY)),
        NotificationKind::ExportByAdministrator | NotificationKind::DetailsCorrected => {
            let mut said = vec![if notice.kind == NotificationKind::DetailsCorrected {
                said!(message::notifications_details_corrected, who())
            } else {
                said!(message::notifications_export_by_administrator, who())
            }];
            if let Some(reason) = &notice.administrators_reason {
                said.push(said!(
                    message::notifications_administrators_reason,
                    Said::text(reason.as_str())
                ));
            }
            said
        }
        NotificationKind::RunEnded => one(match (&notice.renewal, notice.run_done) {
            (
                Some(Renewal {
                    project: Some(name),
                }),
                _,
            ) => said!(
                message::notifications_issue_ids_failed,
                Said::text(name.as_str())
            ),
            (Some(Renewal { project: None }), _) => {
                Said::key(message::NOTIFICATIONS_ISSUE_IDS_FAILED_HIDDEN)
            }
            (None, Some(false)) => Said::key(message::NOTIFICATIONS_RUN_FAILED),
            (None, _) => Said::key(message::NOTIFICATIONS_RUN_DONE),
        }),
        NotificationKind::WorkDayChanged => {
            let date = notice
                .work_day
                .as_ref()
                .map_or_else(String::new, |day| day.date.clone());
            let mut said = vec![said!(
                message::notifications_work_day_changed,
                message::NotificationsWorkDayChanged {
                    date: Said::text(date),
                    who: who()
                }
            )];
            if let Some(reason) = notice.work_day.as_ref().and_then(|day| day.reason.as_ref()) {
                said.push(said!(
                    message::notifications_work_day_reason,
                    Said::text(reason.as_str())
                ));
            }
            said
        }
        NotificationKind::WorkTimesChanged => one(work_times_sentence(notice)),
        NotificationKind::FollowedActivity | NotificationKind::FollowedFields => {
            if !notice.changes.is_empty() {
                change_sentences(&notice.changes)
            } else if let Some(file) = &notice.file {
                one(file_words(&notice.who, file))
            } else if notice.comment {
                one(said!(message::notifications_commented, who()))
            } else {
                one(Said::key(message::NOTIFICATIONS_NEWS))
            }
        }
    }
}

/// The sentence of a change of work times: who did what, and for an away time its days.
fn work_times_sentence(notice: &Notice) -> Said {
    let who = Said::text(notice.who.as_str());
    let Some(times) = &notice.work_times else {
        return said!(message::notifications_work_times_usual_changed, who);
    };
    let days = Said::text(times.days.as_str());
    match times.event {
        WorkTimesEvent::AwayEntered => {
            said!(
                message::notifications_work_times_away_entered,
                message::NotificationsWorkTimesAwayEntered { days, who }
            )
        }
        WorkTimesEvent::AwayChanged => {
            said!(
                message::notifications_work_times_away_changed,
                message::NotificationsWorkTimesAwayChanged { days, who }
            )
        }
        WorkTimesEvent::AwayRemoved => {
            said!(
                message::notifications_work_times_away_removed,
                message::NotificationsWorkTimesAwayRemoved { days, who }
            )
        }
        WorkTimesEvent::UsualChanged => said!(message::notifications_work_times_usual_changed, who),
    }
}

/// A length of time a person typed, as the browser hands it to the reader: the text, the person's
/// *Number format* that says the decimal mark (the browser names the format of the device by the
/// fixed one that writes the same mark), and the short words of the units from the browser's
/// catalog, so the module carries no catalog of its own.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct LengthTyped {
    /// What the person typed.
    pub text: String,
    /// A value of the setting *Number format*.
    pub number_format: String,
    /// The text of `fields.unit-hour`.
    pub hour_unit: String,
    /// The text of `fields.unit-minute`.
    pub minute_unit: String,
    /// The text of `fields.unit-day-words`, where days count; empty where they do not.
    #[serde(default)]
    pub day_units: String,
    /// The minutes of one work day of the person the length is for; 0 where days do not count.
    #[serde(default)]
    pub day_minutes: u32,
}

/// What the reader of lengths made of a typed text ([`crate::time::length::read`]): the minutes,
/// or the sentence that says what is wrong and how to write it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LengthRead {
    /// The whole minutes, when the text is a length of one entry.
    pub minutes: Option<u32>,
    /// What is wrong with the text, when it is none.
    pub mistake: Option<Said>,
}

#[cfg(test)]
mod tests;
