//! The service on Linux: a systemd unit that runs Bugaboo as a dedicated user.
//!
//! Installing is a list of steps (`plan`) that is made first and only then carried out, so the
//! list can be tested without a Linux machine or root rights. Everything here is plain text and
//! plain data; the commands are started in `mod.rs`.

use std::path::{Path, PathBuf};

use bb_core::message::{self, Message};
#[cfg(unix)]
use bb_core::system_answer::folder_in_answer;

use crate::data_folder::SUB_FOLDERS;
use crate::host_rights::{Untrusted, untrusted};
use crate::system::real_path;
#[cfg(unix)]
use crate::system::{answer_of, program_on_path};

use bb_core::service::{RESTART_DELAY, UPGRADE_STOPPED};
pub(super) use bb_core::service::{SYSTEMD_UNIT as UNIT, SYSTEMD_USER as USER};

use super::State;
/// The program that tells where systemd looks for the units an administrator installs.
#[cfg(unix)]
const PATH_PROGRAM: &str = "systemd-path";
#[cfg(unix)]
const PATH_ARGUMENT: &str = "systemd-system-conf";
/// The same answer from the development files of systemd, when `systemd-path` is not there.
#[cfg(unix)]
const PKG_CONFIG: (&str, [&str; 2]) =
    ("pkg-config", ["--variable=systemdsystemconfdir", "systemd"]);
/// The program that is the shell of an account that cannot sign in.
#[cfg(unix)]
const NO_LOGIN_PROGRAM: &str = "nologin";
/// What `systemctl is-system-running` prints when systemd runs the computer, in any state.
const RUNNING_STATES: [&str; 6] = [
    "initializing",
    "starting",
    "running",
    "degraded",
    "maintenance",
    "stopping",
];

/// One thing to do, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Step {
    /// Start a program.
    Run {
        program: &'static str,
        arguments: Vec<String>,
    },
    /// Make the data folder with its sub-folders.
    PrepareDataFolder(PathBuf),
    /// Write a text file.
    Write { path: PathBuf, contents: String },
}

/// What the unit needs to know about this installation.
pub(super) struct Installation<'a> {
    /// The program, in full.
    pub(super) program: &'a Path,
    /// The settings file, in full.
    pub(super) config: &'a Path,
    /// The folder for the units that an administrator installs, as systemd names it.
    pub(super) unit_folder: &'a Path,
    /// The data folder, in full.
    pub(super) data_dir: &'a Path,
    /// The files directly in the data folder that Bugaboo made, which the account takes over.
    pub(super) known_files: &'a [PathBuf],
}

fn run(program: &'static str, arguments: &[&str]) -> Step {
    Step::Run {
        program,
        arguments: arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect(),
    }
}

fn path_argument(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// A word of a unit line, quoted so that spaces and special characters stay in it. systemd reads
/// `"…"` with C escapes, and `%` starts a specifier, so it is written twice.
fn quote(value: &str) -> String {
    let mut quoted = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '%' => quoted.push_str("%%"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

/// The text of the unit file. Bugaboo may write only to its data folder, sees no home folders and
/// no devices, holds no capabilities and never gains rights, opens only network and local
/// sockets, and is started again when it fails, except after a start that stopped at an upgrade.
/// The empty `CapabilityBoundingSet=` also means that it cannot listen on a port below 1024.
pub(super) fn unit(installation: &Installation<'_>) -> String {
    let name = Message::from(message::APP_NAME).render();
    let program = quote(&path_argument(installation.program));
    let config = quote(&path_argument(installation.config));
    let data = quote(&path_argument(installation.data_dir));
    let restart_seconds = RESTART_DELAY.as_secs();
    format!(
        "[Unit]\n\
         Description={name}\n\
         After=network.target\n\
         \n\
         [Service]\n\
         User={USER}\n\
         Group={USER}\n\
         ExecStart={program} serve --config {config}\n\
         Restart=on-failure\n\
         RestartSec={restart_seconds}\n\
         RestartPreventExitStatus={UPGRADE_STOPPED}\n\
         NoNewPrivileges=yes\n\
         CapabilityBoundingSet=\n\
         ProtectSystem=strict\n\
         ReadWritePaths={data}\n\
         {PROTECT_HOME}\n\
         PrivateTmp=yes\n\
         PrivateDevices=yes\n\
         RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX\n\
         \n\
         [Install]\n\
         WantedBy=multi-user.target\n"
    )
}

/// The line of the unit that hides the home folders (`/home`, `/root` and `/run/user`, as
/// systemd defines it) from the service; asked of systemd at install with the same line.
const PROTECT_HOME: &str = "ProtectHome=yes";

/// The program that tells whether a path is there (`test -e`), found on the search path.
#[cfg(unix)]
pub(super) const TEST_PROGRAM: &str = "test";

/// The command that asks systemd whether a service with [`PROTECT_HOME`] sees `path`: a
/// short-lived service that runs `test -e <path>` (`test` is the program found on the search
/// path), and whose exit code `systemd-run --wait` hands back.
pub(super) fn sees_command(test: &Path, path: &Path) -> (&'static str, Vec<String>) {
    (
        "systemd-run",
        vec![
            "--quiet".to_owned(),
            "--wait".to_owned(),
            "--collect".to_owned(),
            format!("--property={PROTECT_HOME}"),
            "--".to_owned(),
            path_argument(test),
            "-e".to_owned(),
            path_argument(path),
        ],
    )
}

/// The first of `paths` that the service would not see, by `sees`, which asks systemd. A path
/// that is not there yet (a data folder the install makes) is asked by the nearest folder above
/// it that is.
pub(super) fn first_hidden<'a>(
    paths: &[&'a Path],
    sees: impl Fn(&Path) -> bool,
) -> Option<&'a Path> {
    paths.iter().copied().find(|path| {
        let existing = path.ancestors().find(|part| part.exists()).unwrap_or(path);
        !sees(existing)
    })
}

/// Whether the settings file may be given to the account of the service: it is a file and no
/// link (a `chmod` would follow a link to whatever it points to), it is not inside the data
/// folder (the service could replace it there; both are compared by their real paths, so `..`
/// and links do not hide it), and the folder that holds it belongs to root and cannot be changed
/// by other accounts (they could replace the file or put a link in its place).
pub(super) fn check_settings_file(config: &Path, data_dir: &Path) -> Result<(), Message> {
    let shown = config.display();
    if std::fs::symlink_metadata(config).is_ok_and(|data| data.file_type().is_symlink()) {
        return Err(message::service_settings_is_link(shown));
    }
    if crate::host_rights::in_data_folder(config, data_dir) {
        return Err(message::service_settings_in_data(
            real_path(config).display(),
        ));
    }
    let config = real_path(config);
    let Some(folder) = config.parent() else {
        return Ok(());
    };
    match untrusted(folder, None) {
        Some(Untrusted::OthersCanWrite) => {
            Err(message::service_settings_folder_open(folder.display()))
        }
        Some(Untrusted::NotRoots) => Err(message::service_settings_folder_owner(folder.display())),
        #[cfg(unix)]
        Some(Untrusted::Unreadable) => Err(message::service_answer_not_trusted(folder.display())),
        None => Ok(()),
    }
}

/// Refuses an answer of the system that somebody but root could have changed: it is used as root
/// (the shell of an account, the folder of a unit), so it must belong to root and not be writable
/// by the group or by everybody. `path` is the real path, with every link followed.
#[cfg(unix)]
fn trusted(path: PathBuf) -> Result<PathBuf, Message> {
    match untrusted(&path, None) {
        None => Ok(path),
        Some(_) => Err(message::service_answer_not_trusted(path.display())),
    }
}

/// The folder for the units that an administrator installs, as systemd names it (`systemd-path`,
/// or else `pkg-config`). A folder that does not exist is no answer, and neither is one that
/// root does not own alone: the program stops with a message and never guesses.
#[cfg(unix)]
pub(super) fn unit_folder() -> Result<PathBuf, Message> {
    let (pkg_program, pkg_arguments) = PKG_CONFIG;
    let folder = [
        answer_of(PATH_PROGRAM, &[PATH_ARGUMENT]),
        answer_of(pkg_program, &pkg_arguments),
    ]
    .into_iter()
    .flatten()
    .filter_map(|answer| folder_in_answer(&answer))
    .find(|folder| folder.is_dir())
    .ok_or_else(|| Message::from(message::SERVICE_UNIT_FOLDER_UNKNOWN))?;
    trusted(real_path(&folder))
}

/// The shell of an account that cannot sign in, found on the search path, as its real path, and
/// only when root owns the file and the folder that holds it alone.
#[cfg(unix)]
pub(super) fn no_login() -> Result<PathBuf, Message> {
    let found = program_on_path(NO_LOGIN_PROGRAM)
        .ok_or_else(|| message::service_program_not_found(NO_LOGIN_PROGRAM))?;
    let shell = trusted(real_path(&found))?;
    if let Some(folder) = shell.parent() {
        trusted(folder.to_path_buf())?;
    }
    Ok(shell)
}

/// Whether the text `systemctl is-system-running` printed says that systemd runs the computer.
fn is_running(answer: &str) -> bool {
    RUNNING_STATES.contains(&answer.trim())
}

/// Whether systemd runs this computer. `systemctl is-system-running` ends with an error in the
/// state `degraded`, so what it printed counts, not how it ended.
#[cfg(unix)]
pub(super) fn runs_this_computer() -> bool {
    std::process::Command::new("systemctl")
        .arg("is-system-running")
        .output()
        .is_ok_and(|output| is_running(&String::from_utf8_lossy(&output.stdout)))
}

/// What `systemctl show` is asked for the state of the unit.
#[cfg(unix)]
const SHOW_PROPERTIES: &str =
    "--property=LoadState,ActiveState,SubState,ExecMainCode,ExecMainStatus,Result";

/// How the main process ended, as `ExecMainCode` names it (the `si_code` of `waitid`): it was
/// killed by a signal, or killed and wrote a core dump. Then `ExecMainStatus` is the signal.
const ENDED_BY_SIGNAL: [&str; 2] = ["2", "3"];

/// The command that asks systemd how the unit is; it needs no administrator rights.
#[cfg(unix)]
pub(super) fn show_command() -> (&'static str, [&'static str; 3]) {
    ("systemctl", ["show", UNIT, SHOW_PROPERTIES])
}

/// The state in the answer of `systemctl show` (`Key=value` lines), `None` when it names none.
pub(super) fn state_of(show: &str) -> Option<State> {
    let value = |key: &str| {
        show.lines()
            .filter_map(|line| line.trim().split_once('='))
            .find_map(|(name, value)| (name == key).then_some(value))
    };
    if value("LoadState") == Some("not-found") {
        return Some(State::NotInstalled);
    }
    match (value("ActiveState")?, value("SubState")) {
        ("active" | "reloading", _) => Some(State::Running),
        ("activating", Some("auto-restart")) => Some(State::Restarting),
        ("activating", _) => Some(State::Starting),
        ("deactivating", _) => Some(State::Stopping),
        ("inactive" | "failed", _) => {
            let status = value("ExecMainStatus")?.parse().ok()?;
            let by_signal =
                value("ExecMainCode").is_some_and(|code| ENDED_BY_SIGNAL.contains(&code));
            Some(match (by_signal, value("Result")) {
                // systemd stops a service with a signal, and counts that as a clean end.
                (true, Some("success")) => State::Stopped,
                (true, _) => State::Killed(status),
                (false, _) => State::ended_with(status),
            })
        }
        _ => None,
    }
}

/// The steps that install the service. The account is made only when `new_account_shell` is
/// given, which is the shell of an account that cannot sign in; `None` when it exists already.
pub(super) fn plan(installation: &Installation<'_>, new_account_shell: Option<&Path>) -> Vec<Step> {
    let owner = format!("{USER}:{USER}");
    let mut steps = Vec::new();
    if let Some(shell) = new_account_shell {
        steps.push(run(
            "useradd",
            &[
                "--system",
                "--no-create-home",
                "--shell",
                &path_argument(shell),
                USER,
            ],
        ));
    }
    let data = path_argument(installation.data_dir);
    let config = path_argument(installation.config);
    steps.push(Step::PrepareDataFolder(installation.data_dir.to_path_buf()));
    // Only what Bugaboo made changes owner: the folder itself, its known files and its own
    // sub-folders. Never the whole folder with whatever else lies in it.
    // `-h` changes a link itself and never the file it points to, `-P` keeps the walk of `-R`
    // from stepping through links.
    steps.push(run("chown", &["-h", &owner, &data]));
    for file in installation.known_files {
        steps.push(run("chown", &["-h", &owner, &path_argument(file)]));
    }
    for name in SUB_FOLDERS {
        // Plain text, because this plan is for Linux whichever system the test runs on.
        let folder = format!("{}/{name}", data.trim_end_matches('/'));
        steps.push(run("chown", &["-h", "-R", "-P", &owner, &folder]));
    }
    // The settings file may hold a database password: the account that runs Bugaboo reads it,
    // nobody else does. `check_settings_file` made sure it is no link.
    steps.push(run("chown", &["-h", "root:bugaboo", &config]));
    steps.push(run("chmod", &["640", &config]));
    steps.push(Step::Write {
        path: installation.unit_folder.join(UNIT),
        contents: unit(installation),
    });
    steps.push(run("systemctl", &["daemon-reload"]));
    steps.push(run("systemctl", &["enable", "--now", UNIT]));
    steps
}

/// The steps that remove the service. The account and the data stay.
pub(super) fn removal(unit_folder: &Path) -> Vec<Step> {
    vec![
        run("systemctl", &["disable", "--now", UNIT]),
        Step::Run {
            program: "rm",
            arguments: vec![path_argument(&unit_folder.join(UNIT))],
        },
        run("systemctl", &["daemon-reload"]),
    ]
}

#[cfg(test)]
mod tests;
