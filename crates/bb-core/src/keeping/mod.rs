//! *Keep and delete rules* (PRIV-05, ADR-0045): the one list of every kind of data Bugaboo deletes
//! after a time, how long each is kept and who deletes it then. [`KEEP_KINDS`] has one row for each
//! kind; the jobs that delete by themselves, the administrator's *Delete them*, the page of the
//! rules and the table of the user docs all read it. Pure: nothing here reads the clock or a store.

mod length;
mod table;
#[cfg(test)]
mod tests;

use jiff::civil::Date;
use jiff::{Timestamp, tz::TimeZone};

pub use length::{Length, LengthUnit};
pub use table::keep_table;

use crate::edit::DRAFT_KEEP_DAYS;
use crate::files::UNFINISHED_KEPT;
use crate::oauth::ENDED_GRANT_KEEP_DAYS;
use crate::settings::keeping::{
    KEEPING_DEAD_JOB_DAYS, KEEPING_IP_ADDRESS_DAYS, KEEPING_NOTIFICATION_DAYS,
    KEEPING_TIME_ENTRY_YEARS, KEEPING_TRASH_DAYS,
};
use crate::settings::working_hours::WORKING_HOURS_KEEP_YEARS;
use crate::settings::{
    BACKUPS_KEEP_DAILY, BACKUPS_KEEP_WEEKLY, Integer, Layers, SECURITY_LOG_KEEP_DAYS,
    SECURITY_SESSION_ABSOLUTE_HOURS, Setting,
};
use crate::work_times::AWAY_TIME_KEPT_FOR_YEARS;

words! {
    /// The kinds of data that are deleted after a time, each with one row in [`KEEP_KINDS`].
    pub enum KeepKind {
        /// The shortened network address a session was started from.
        IpAddresses => "ip-addresses",
        /// The sessions of people who are signed in; one that ended is deleted at once.
        Sessions => "sessions",
        /// The links of sign-ups that were used or ran out.
        SignUpLinks => "sign-up-links",
        /// The links that confirm a new e-mail address, with the address, until they run out.
        EmailChangeLinks => "e-mail-change-links",
        /// The requests for agents of programs, answered or not (ADR-0029, amendment M9).
        AgentRequests => "agent-requests",
        /// Text being written and not yet saved, and the notes of who is editing.
        Drafts => "drafts",
        /// Notifications in the app.
        Notifications => "notifications",
        /// Issues and attachments in the trash.
        Trash => "trash",
        /// Uploads of files that were never finished.
        Uploads => "uploads",
        /// Jobs that gave up, with their payload.
        DeadJobs => "dead-jobs",
        /// Grants of AI apps that ended, and apps nobody allowed.
        AppGrants => "app-grants",
        /// The log of the tools the assistant and the AI apps called.
        ToolCalls => "tool-calls",
        /// Away times, after their last day.
        AwayTimes => "away-times",
        /// The security log, in whole blocks.
        SecurityLog => "security-log",
        /// The backups of the schedule and those made by hand (ADR-0046).
        Backups => "backups",
        /// The files of *Download my data* with their records (ADR-0044, decision 2).
        DataExports => "data-exports",
        /// Work days with their breaks and the reasons of their changes (ADR-0035, decision 7).
        WorkDays => "work-days",
        /// Time entries on issues (M9-Q2).
        TimeEntries => "time-entries",
    }
    messages name = "keep-kind.name.";
    messages why = "keep-kind.why."
}

words! {
    /// Who deletes a kind of data after its keep period (ADR-0045, decision 1).
    pub enum KeepGroup {
        /// Bugaboo, by a daily job: data with no business purpose of its own.
        ByItself => "by-itself",
        /// An administrator, after a count of what is past the period and *Delete them*: business
        /// records, which only the organization can judge to be no longer needed.
        ByAnAdministrator => "by-an-administrator",
    }
    messages name = "keep-group.name."
}

/// How long one kind of data is kept.
#[derive(Clone, Copy)]
pub enum Period {
    /// A setting of the registry: the row's setting, and how its value becomes a length (the
    /// period of work days also follows the preset of the working-time rules).
    Setting {
        /// The setting that changes the period; its lower bound is the floor of the row.
        setting: &'static Setting<Integer>,
        /// The length the stored values give.
        length: fn(&Layers) -> Length,
    },
    /// A length of the code, which no setting changes: the row only shows it.
    Fixed(Length),
    /// The rotation of backups (ADR-0046, decision 3): it keeps the newest backup of each of the
    /// days and weeks the settings say, counted among the days and weeks that have a backup, and
    /// deletes nothing by its age. The length is how far back they reach while a backup is made
    /// every day; a record has no fixed last day ([`KeptUntil::NoFixedEnd`]).
    Rotation {
        /// The setting that changes the period; its lower bound is the floor of the row.
        setting: &'static Setting<Integer>,
        /// The length the stored values give while a backup is made every day.
        length: fn(&Layers) -> Length,
    },
}

/// The last day a record of a kind is kept: the answer of the port `KeepPeriods`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeptUntil {
    /// Through this day; on the day after it, the record is past its period.
    Day(Date),
    /// No day can be named: the kind is not deleted by its age (the rotation of backups keeps the
    /// newest whatever their age), or the day lies past the end of the calendar.
    NoFixedEnd,
}

/// One row of [`KEEP_KINDS`].
#[derive(Clone, Copy)]
pub struct KeepRule {
    /// The kind of data.
    pub kind: KeepKind,
    /// Who deletes it after its period.
    pub group: KeepGroup,
    /// How long it is kept.
    pub period: Period,
}

impl KeepRule {
    /// The keep period with the stored settings.
    #[must_use]
    pub fn length(&self, layers: &Layers) -> Length {
        match self.period {
            Period::Setting { length, .. } | Period::Rotation { length, .. } => length(layers),
            Period::Fixed(length) => length,
        }
    }

    /// The setting that changes the period, if one does.
    #[must_use]
    pub const fn setting(&self) -> Option<&'static Setting<Integer>> {
        match self.period {
            Period::Setting { setting, .. } | Period::Rotation { setting, .. } => Some(setting),
            Period::Fixed(_) => None,
        }
    }

    /// The shortest period the row allows: the lower bound of its setting (ADR-0045, decision 3).
    /// `None` for a fixed period.
    #[must_use]
    pub fn floor(&self) -> Option<i64> {
        self.setting()
            .and_then(|setting| setting.definition().bounds())
            .map(|(least, _)| least)
    }

    /// The moment before which a record of the kind is past its period, at `now`: what a job
    /// that deletes by itself looks for. `None` only at the start of time.
    #[must_use]
    pub fn cutoff(&self, layers: &Layers, now: Timestamp, zone: &TimeZone) -> Option<Timestamp> {
        self.length(layers).before(now, zone)
    }

    /// The first date that is still inside the period on `today`: a record with an earlier date
    /// is past it. One day later than the length alone, because the day of a person in another
    /// zone has another date for the same moment: a record goes a day late rather than a day early.
    #[must_use]
    pub fn kept_from(&self, layers: &Layers, today: Date) -> Option<Date> {
        self.length(layers).first_kept(today)
    }

    /// The last day a record of the kind dated `date` is kept: the answer of the port
    /// `KeepPeriods`. The opposite of [`Self::kept_from`]; none for the rotation of backups.
    #[must_use]
    pub fn kept_until(&self, layers: &Layers, date: Date) -> KeptUntil {
        if !self.fixed_end() {
            return KeptUntil::NoFixedEnd;
        }
        self.length(layers)
            .last_kept(date)
            .map_or(KeptUntil::NoFixedEnd, KeptUntil::Day)
    }

    /// Whether a record of the kind goes when its period is over: not with the rotation of backups,
    /// which keeps the newest whatever their age. The one source of the port, the page of the rules
    /// and the table of the user docs.
    #[must_use]
    pub const fn fixed_end(&self) -> bool {
        !matches!(self.period, Period::Rotation { .. })
    }
}

/// Every kind of data that is deleted after a time, in the order of the page of the rules: first
/// what Bugaboo deletes by itself, then what an administrator deletes (ADR-0045, decision 1).
pub const KEEP_KINDS: &[KeepRule] = &[
    KeepRule {
        kind: KeepKind::IpAddresses,
        group: KeepGroup::ByItself,
        period: Period::Setting {
            setting: &KEEPING_IP_ADDRESS_DAYS,
            length: |layers| Length::days(layers.number(&KEEPING_IP_ADDRESS_DAYS)),
        },
    },
    KeepRule {
        kind: KeepKind::Sessions,
        group: KeepGroup::ByItself,
        period: Period::Setting {
            setting: &SECURITY_SESSION_ABSOLUTE_HOURS,
            length: |layers| Length::hours(layers.number(&SECURITY_SESSION_ABSOLUTE_HOURS)),
        },
    },
    KeepRule {
        kind: KeepKind::SignUpLinks,
        group: KeepGroup::ByItself,
        period: Period::Fixed(Length::hours(crate::sign_up_invitation::KEEP_HOURS)),
    },
    KeepRule {
        kind: KeepKind::EmailChangeLinks,
        group: KeepGroup::ByItself,
        period: Period::Fixed(Length::hours(crate::e_mail_change::LINK_HOURS)),
    },
    KeepRule {
        kind: KeepKind::AgentRequests,
        group: KeepGroup::ByItself,
        period: Period::Fixed(Length::days(crate::agent_request::REQUEST_KEEP_DAYS)),
    },
    KeepRule {
        kind: KeepKind::Drafts,
        group: KeepGroup::ByItself,
        period: Period::Fixed(Length::days(DRAFT_KEEP_DAYS)),
    },
    KeepRule {
        kind: KeepKind::Notifications,
        group: KeepGroup::ByItself,
        period: Period::Setting {
            setting: &KEEPING_NOTIFICATION_DAYS,
            length: |layers| Length::days(layers.number(&KEEPING_NOTIFICATION_DAYS)),
        },
    },
    KeepRule {
        kind: KeepKind::Trash,
        group: KeepGroup::ByItself,
        period: Period::Setting {
            setting: &KEEPING_TRASH_DAYS,
            length: |layers| Length::days(layers.number(&KEEPING_TRASH_DAYS)),
        },
    },
    KeepRule {
        kind: KeepKind::Uploads,
        group: KeepGroup::ByItself,
        period: Period::Fixed(Length::hours(UNFINISHED_KEPT.as_hours())),
    },
    KeepRule {
        kind: KeepKind::DeadJobs,
        group: KeepGroup::ByItself,
        period: Period::Setting {
            setting: &KEEPING_DEAD_JOB_DAYS,
            length: |layers| Length::days(layers.number(&KEEPING_DEAD_JOB_DAYS)),
        },
    },
    KeepRule {
        kind: KeepKind::AppGrants,
        group: KeepGroup::ByItself,
        period: Period::Fixed(Length::days(ENDED_GRANT_KEEP_DAYS)),
    },
    KeepRule {
        kind: KeepKind::ToolCalls,
        group: KeepGroup::ByItself,
        period: Period::Fixed(Length::days(crate::ai::TOOL_CALL_KEEP_DAYS)),
    },
    KeepRule {
        kind: KeepKind::AwayTimes,
        group: KeepGroup::ByItself,
        period: Period::Fixed(Length::years(AWAY_TIME_KEPT_FOR_YEARS)),
    },
    KeepRule {
        kind: KeepKind::SecurityLog,
        group: KeepGroup::ByItself,
        period: Period::Setting {
            setting: &SECURITY_LOG_KEEP_DAYS,
            length: |layers| Length::days(layers.number(&SECURITY_LOG_KEEP_DAYS)),
        },
    },
    KeepRule {
        kind: KeepKind::Backups,
        group: KeepGroup::ByItself,
        period: Period::Rotation {
            setting: &BACKUPS_KEEP_DAILY,
            // The rotation keeps the newest backup of each of the days and of each of the weeks the
            // settings say (ADR-0046, decision 3): while a backup is made every day, the oldest it
            // keeps is the longer of the two.
            length: |layers| {
                let week = i64::try_from(crate::time::day::DAYS_PER_WEEK).unwrap_or(i64::MAX);
                let weeks = layers.number(&BACKUPS_KEEP_WEEKLY).saturating_mul(week);
                Length::days(layers.number(&BACKUPS_KEEP_DAILY).max(weeks))
            },
        },
    },
    KeepRule {
        kind: KeepKind::DataExports,
        group: KeepGroup::ByItself,
        period: Period::Fixed(Length::days(crate::data_export::EXPORT_KEPT_DAYS)),
    },
    KeepRule {
        kind: KeepKind::WorkDays,
        group: KeepGroup::ByAnAdministrator,
        period: Period::Setting {
            setting: &WORKING_HOURS_KEEP_YEARS,
            // The preset *Germany* never keeps them less than the law (ADR-0035, decision 7).
            length: |layers| Length::years(crate::working_time::keep_years(layers)),
        },
    },
    KeepRule {
        kind: KeepKind::TimeEntries,
        group: KeepGroup::ByAnAdministrator,
        period: Period::Setting {
            setting: &KEEPING_TIME_ENTRY_YEARS,
            length: |layers| Length::years(layers.number(&KEEPING_TIME_ENTRY_YEARS)),
        },
    },
];

// One row for each kind, in the order of the kinds, so a kind finds its row by its place: the build
// stops when a row is missing, doubled or out of order.
const _: () = {
    assert!(KEEP_KINDS.len() == KeepKind::ALL.len());
    let mut at = 0;
    while at < KEEP_KINDS.len() {
        assert!(crate::words::same_text(
            KEEP_KINDS[at].kind.code(),
            KeepKind::ALL[at].code()
        ));
        at += 1;
    }
};

/// The row of `kind`.
#[must_use]
pub const fn rule(kind: KeepKind) -> &'static KeepRule {
    &KEEP_KINDS[kind as usize]
}

/// What one step of deleting did (ADR-0045, decision 5): how many rows went, and whether more may be
/// left, because a part of the step deleted as many rows as it was allowed to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Step {
    /// The rows deleted.
    pub rows: u64,
    /// Whether a part of the step reached its limit, so another step is needed.
    pub more: bool,
}

impl Step {
    /// One part of a step that deleted `rows` and was allowed `limit`.
    #[must_use]
    pub const fn of(rows: u64, limit: u64) -> Self {
        Self {
            rows,
            more: rows >= limit,
        }
    }

    /// This step and another part of it.
    #[must_use]
    pub const fn and(self, other: Self) -> Self {
        Self {
            rows: self.rows.saturating_add(other.rows),
            more: self.more || other.more,
        }
    }
}

words_in_api!(
    KeepKind,
    "A kind of data that is deleted after its keep period (Keep and delete rules)."
);
words_in_api!(
    KeepGroup,
    "Who deletes a kind of data after its keep period: Bugaboo by itself, or an administrator."
);
words_in_api!(LengthUnit, "What a keep period counts in.");
