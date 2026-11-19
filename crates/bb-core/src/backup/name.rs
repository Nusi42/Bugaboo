//! The name of a backup file (ADR-0046, decision 1): `bugaboo-<moment>-<version>.backup`, the
//! moment in UTC to the second (`20261118T031500Z`). The backup a restore makes of the present
//! first is `bugaboo-before-restore-<moment>-<version>.backup`: the schedule's rotation does not
//! delete it, only the keep period of the security log does (decision 4). The backup a new program
//! makes before it migrates the database is `bugaboo-before-upgrade-<moment>-<version>.backup`, the
//! version being the new program's: the rotation keeps the newest few (decision 5). A name this
//! module does not read (a file somebody put there) is never deleted by the rotation.

use core::fmt;

use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::Offset;

/// The start of every backup's name.
const PREFIX: &str = "bugaboo-";
/// What the name of a backup made before a restore has after the prefix.
const BEFORE_RESTORE: &str = "before-restore-";
/// What the name of a backup made before an upgrade has after the prefix.
const BEFORE_UPGRADE: &str = "before-upgrade-";
/// The end of every backup's name.
pub const EXTENSION: &str = ".backup";
/// How the moment is written in a name.
const MOMENT: &str = "%Y%m%dT%H%M%SZ";

/// What a backup was made for, as its name says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// Made by the schedule, the command or the button: the rotation keeps or deletes it.
    Regular,
    /// Made by a restore of what was there before: kept for the keep period of the security log.
    BeforeRestore,
    /// Made by a new program before it migrated the database: the newest few are kept.
    BeforeUpgrade,
}

impl Purpose {
    /// What the name of a backup made for this purpose has after the prefix.
    const fn word(self) -> &'static str {
        match self {
            Self::Regular => "",
            Self::BeforeRestore => BEFORE_RESTORE,
            Self::BeforeUpgrade => BEFORE_UPGRADE,
        }
    }
}

/// The name of a backup this program made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupName {
    /// What it was made for.
    pub purpose: Purpose,
    /// The moment of the copy, to the second.
    pub moment: Timestamp,
    /// The version of Bugaboo that made it.
    pub version: String,
}

impl BackupName {
    /// The name of a backup made at `moment` by `version`, cut to the second.
    #[must_use]
    pub fn new(purpose: Purpose, moment: Timestamp, version: &str) -> Self {
        Self {
            purpose,
            moment: Timestamp::from_second(moment.as_second()).unwrap_or(moment),
            version: version.to_owned(),
        }
    }

    /// Reads a file name; `None` when it is not the name of a backup this program makes.
    #[must_use]
    pub fn parse(file_name: &str) -> Option<Self> {
        let rest = file_name.strip_prefix(PREFIX)?.strip_suffix(EXTENSION)?;
        let (purpose, rest) = [Purpose::BeforeRestore, Purpose::BeforeUpgrade]
            .into_iter()
            .find_map(|purpose| Some((purpose, rest.strip_prefix(purpose.word())?)))
            .unwrap_or((Purpose::Regular, rest));
        let (moment, version) = rest.split_once('-')?;
        if !is_version(version) {
            return None;
        }
        let civil = DateTime::strptime(MOMENT, moment).ok()?;
        let moment = Offset::UTC.to_timestamp(civil).ok()?;
        Some(Self {
            purpose,
            moment,
            version: version.to_owned(),
        })
    }
}

/// Whether `text` can be the version of a release in a file name: digits, letters, dots, `+` and
/// `-`, nothing that a file system reads as a separator.
fn is_version(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-'))
}

impl fmt::Display for BackupName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let moment = self
            .moment
            .to_zoned(jiff::tz::TimeZone::UTC)
            .strftime(MOMENT);
        write!(
            formatter,
            "{PREFIX}{}{moment}-{}{EXTENSION}",
            self.purpose.word(),
            self.version
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_reads_back_what_was_written() {
        let moment: Timestamp = "2026-11-18T03:15:00.750Z".parse().unwrap();
        let name = BackupName::new(Purpose::Regular, moment, "0.6.0-rc.1");
        assert_eq!(
            name.to_string(),
            "bugaboo-20261118T031500Z-0.6.0-rc.1.backup"
        );
        assert_eq!(BackupName::parse(&name.to_string()), Some(name));
        let before = BackupName::new(Purpose::BeforeRestore, moment, "0.6.0");
        assert_eq!(
            before.to_string(),
            "bugaboo-before-restore-20261118T031500Z-0.6.0.backup"
        );
        assert_eq!(BackupName::parse(&before.to_string()), Some(before));
        let upgrade = BackupName::new(Purpose::BeforeUpgrade, moment, "0.6.0");
        assert_eq!(
            upgrade.to_string(),
            "bugaboo-before-upgrade-20261118T031500Z-0.6.0.backup"
        );
        assert_eq!(BackupName::parse(&upgrade.to_string()), Some(upgrade));
    }

    #[test]
    fn other_names_are_not_read_as_backups() {
        for name in [
            "bugaboo-before-0.6.0.backup",
            "bugaboo-20261118T031500Z-.backup",
            "bugaboo-20261118T031500Z-0.6.0.zip",
            "bugaboo-2026-11-18-0.6.0.backup",
            "bugaboo-20261118T031500Z-0.6/0.backup",
            "notes.txt",
        ] {
            assert_eq!(BackupName::parse(name), None, "{name}");
        }
    }
}
