//! What the services of Bugaboo and the try-out of their install share (OPS-04): the names the
//! service managers know them by, how long a service waits before it starts again after a failure
//! and how often, and the exit code of a start that stopped at an upgrade, after which it must not
//! start again.

use std::time::Duration;

/// The name of the Windows service, as the service manager lists it.
pub const WINDOWS_NAME: &str = "Bugaboo";

/// The name of the systemd unit, and so of the service for `systemctl`.
pub const SYSTEMD_UNIT: &str = "bugaboo.service";

/// The account that runs Bugaboo on Linux: no password, no shell, no home folder.
pub const SYSTEMD_USER: &str = "bugaboo";

/// How long a service manager waits before it starts Bugaboo again after a failure.
pub const RESTART_DELAY: Duration = Duration::from_secs(10);

/// How many times in a row the Windows service manager starts Bugaboo again after a failure
/// before it leaves it stopped. systemd has no such limit: it tries again after every
/// [`RESTART_DELAY`].
pub const RESTARTS: usize = 3;

/// After how long without a failure the Windows service manager begins its count of restarts
/// again: so it starts Bugaboo again at most [`RESTARTS`] times a day.
pub const RESTART_COUNT_RESET: Duration = Duration::from_hours(24);

/// The exit code of a start that stopped at an upgrade: a database a newer program changed, one
/// with a migration this program does not have or has in another form, a backup before an
/// upgrade that failed, or a migration that failed and was undone by putting the backup back.
/// Starting again would stop the same way, so a service must not restart on it.
pub const UPGRADE_STOPPED: u8 = 65;
