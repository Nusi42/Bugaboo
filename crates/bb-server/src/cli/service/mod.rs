//! `bugaboo service install`, `uninstall` and `status` (OPS-03, OPS-04): Bugaboo as a service
//! that starts with the computer and runs as an account of its own. Both service managers start
//! it again after a failure, but not after a start that stopped at an upgrade
//! ([`bb_core::service::UPGRADE_STOPPED`]): that start would stop the same way.
//!
//! - Linux: a systemd unit and the user `bugaboo` (`systemd.rs`).
//! - Windows: a Windows service that runs as its own virtual account `NT SERVICE\Bugaboo`, which
//!   needs no password (`windows.rs`).
//!
//! The first start of a service has no console, so the setup code of the first administrator is
//! saved in a file in the data folder instead (`serve.rs`); installing says where.

#[cfg(any(unix, test))]
mod systemd;
#[cfg(windows)]
mod windows;
#[cfg(any(windows, test))]
mod windows_rights;

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use bb_core::message::{self, Message};

use super::{ConfigArgs, InstallArgs, load_settings, program_folder};
use crate::config::{DEFAULT_FILE, Settings};
use crate::data_folder::DataFolder;
#[cfg(unix)]
use crate::data_folder::{OwnFiles, check_for_service};
use crate::output;

/// The name of the service, as the system lists it.
#[cfg(windows)]
pub(super) const NAME: &str = bb_core::service::WINDOWS_NAME;

/// The account that runs the service: the one that owns the data folder of a service install.
pub(super) fn account_name() -> String {
    #[cfg(windows)]
    {
        windows::account()
    }
    #[cfg(not(windows))]
    {
        systemd::USER.to_owned()
    }
}

/// What the installer found out: which program, which settings file, which data folder.
struct Found {
    program: PathBuf,
    config: PathBuf,
    settings: Settings,
}

/// Finds the program and the settings file the service will use. The service reads the same
/// settings file as `bugaboo serve`, so it must exist and be complete.
fn find(args: &ConfigArgs) -> anyhow::Result<Found> {
    let config = args
        .config
        .clone()
        .unwrap_or_else(|| program_folder().join(DEFAULT_FILE));
    let config = std::path::absolute(&config).unwrap_or(config);
    if !config.is_file() {
        return Err(anyhow!(
            message::service_no_settings(config.display()).render()
        ));
    }
    let settings = load_settings(Some(&config), &[], true)?;
    let program = std::env::current_exe()?;
    Ok(Found {
        program,
        config,
        settings,
    })
}

/// Tells where the setup code of the first start is, when nobody has made an administrator yet.
fn tell_about_setup_code(data_dir: &Path) {
    if let Some(data) = DataFolder::existing(data_dir) {
        output::line(&message::service_setup_code(data.setup_code_file().display()).render());
    }
}

/// `bugaboo service install`.
pub(super) fn install(args: &InstallArgs) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        windows::install(&find(&args.config)?, args.take_over_data_folder.into())
    }
    #[cfg(unix)]
    {
        systemd_install(&find(&args.config)?, args.take_over_data_folder.into())
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = args;
        unsupported()
    }
}

/// `bugaboo service uninstall`.
pub(super) fn uninstall() -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        windows::uninstall()
    }
    #[cfg(unix)]
    {
        systemd_uninstall()
    }
    #[cfg(not(any(windows, unix)))]
    {
        unsupported()
    }
}

/// How a service is, as `bugaboo service status` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum State {
    NotInstalled,
    Starting,
    Running,
    /// Stopped after a failure; systemd starts it again after the delay, as often as it fails.
    #[cfg(any(unix, test))]
    Restarting,
    /// Ended without saying that it stopped (a crash). The Windows service manager starts it again
    /// after the delay, but only [`bb_core::service::RESTARTS`] times a day, so it may stay
    /// stopped.
    #[cfg(any(windows, test))]
    EndedUnexpectedly,
    Stopping,
    /// Stopped by a person or with the computer, with no failure.
    Stopped,
    /// Stopped at an upgrade, and not started again by the service manager.
    StoppedAtUpgrade,
    /// Stopped with this exit code.
    Failed(u32),
    /// Ended by this signal, which nobody asked for (Linux).
    #[cfg(any(unix, test))]
    Killed(u32),
}

impl State {
    /// The state of a service that is not running and ended with `code`: one place for both
    /// service managers.
    pub(super) fn ended_with(code: u32) -> Self {
        match code {
            0 => Self::Stopped,
            code if code == u32::from(bb_core::service::UPGRADE_STOPPED) => Self::StoppedAtUpgrade,
            code => Self::Failed(code),
        }
    }

    /// The sentence that names the state of the service `name`.
    pub(super) fn sentence(self, name: &str) -> Message {
        match self {
            Self::NotInstalled => message::service_not_installed(name),
            Self::Starting => message::service_state_starting(name),
            Self::Running => message::service_state_running(name),
            #[cfg(any(unix, test))]
            Self::Restarting => message::service_state_restarting(name),
            #[cfg(any(windows, test))]
            Self::EndedUnexpectedly => {
                message::service_state_ended_unexpectedly(message::ServiceStateEndedUnexpectedly {
                    name: &name,
                    restarts: &bb_core::service::RESTARTS,
                })
            }
            Self::Stopping => message::service_state_stopping(name),
            Self::Stopped => message::service_state_stopped(name),
            Self::StoppedAtUpgrade => message::service_state_upgrade_stopped(name),
            Self::Failed(code) => message::service_state_failed(message::ServiceStateFailed {
                code: &code,
                name: &name,
            }),
            #[cfg(any(unix, test))]
            Self::Killed(signal) => message::service_state_killed(message::ServiceStateKilled {
                name: &name,
                signal: &signal,
            }),
        }
    }
}

/// `bugaboo service status`: says how the service is. It needs no administrator rights.
pub(super) fn status() -> anyhow::Result<()> {
    #[cfg(windows)]
    let (state, name) = (windows::state()?, NAME);
    #[cfg(unix)]
    let (state, name) = (systemd_state()?, systemd::UNIT);
    #[cfg(not(any(windows, unix)))]
    return unsupported();
    #[cfg(any(windows, unix))]
    {
        output::line(&state.sentence(name).render());
        Ok(())
    }
}

/// The state of the unit, as `systemctl show` names it.
#[cfg(unix)]
fn systemd_state() -> anyhow::Result<State> {
    let (program, arguments) = systemd::show_command();
    let output = std::process::Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| refused(&format!("{program}: {error}")))?;
    systemd::state_of(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| anyhow!(message::service_state_unknown(systemd::UNIT).render()))
}

/// `bugaboo service run`: started by the Windows service manager and never by a person.
pub(super) fn run(args: &ConfigArgs) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        windows::run(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err(anyhow!(
            Message::from(message::SERVICE_NOT_A_SERVICE).render()
        ))
    }
}

#[cfg(not(any(windows, unix)))]
fn unsupported() -> anyhow::Result<()> {
    Err(anyhow!(
        Message::from(message::SERVICE_UNSUPPORTED).render()
    ))
}

#[cfg(unix)]
fn systemd_install(found: &Found, own_files: OwnFiles) -> anyhow::Result<()> {
    use systemd::{Installation, Step, USER, plan};

    require_systemd()?;
    // What the system names is asked before anything is changed.
    let unit_folder = systemd::unit_folder().map_err(|said| anyhow!(said.render()))?;
    let new_account_shell = if crate::system::account_exists(USER) {
        None
    } else {
        Some(systemd::no_login().map_err(|said| anyhow!(said.render()))?)
    };
    // `chown -h` changes a link and not the folder it leads to: the plan and the unit name the
    // real folder, and the person is told when that is another one.
    let data_dir = systemd::real_path(&found.settings.data_dir);
    if data_dir != found.settings.data_dir {
        output::line(
            &message::service_data_folder_real(message::ServiceDataFolderReal {
                path: &found.settings.data_dir.display(),
                real: &data_dir.display(),
            })
            .render(),
        );
    }
    check_for_service(&data_dir, own_files)?;
    systemd::check_settings_file(&found.config, &data_dir)
        .map_err(|problem| anyhow!(problem.render()))?;
    check_service_sees(&[&found.program, &found.config, &data_dir])?;
    let known_files = DataFolder::existing(&data_dir)
        .map(|data| data.known_files())
        .unwrap_or_default();
    let installation = Installation {
        program: &found.program,
        config: &found.config,
        unit_folder: &unit_folder,
        data_dir: &data_dir,
        known_files: &known_files,
    };
    for step in plan(&installation, new_account_shell.as_deref()) {
        match step {
            Step::Run { program, arguments } => run_command(program, &arguments)?,
            Step::PrepareDataFolder(path) => {
                DataFolder::prepare(&path)?;
            }
            Step::Write { path, contents } => std::fs::write(&path, contents)
                .map_err(|error| refused(&format!("{}: {error}", path.display())))?,
        }
    }
    output_installed(found);
    Ok(())
}

/// Refuses when the unit's hardening would hide the program, the settings file or the data
/// folder from the service (a place in a home folder): the service could never start, and
/// systemd would try again and again. systemd itself is asked, with the unit's own line.
#[cfg(unix)]
fn check_service_sees(paths: &[&Path]) -> anyhow::Result<()> {
    let test = crate::system::program_on_path(systemd::TEST_PROGRAM).ok_or_else(|| {
        anyhow!(message::service_program_not_found(systemd::TEST_PROGRAM).render())
    })?;
    let hidden = systemd::first_hidden(paths, |path| {
        let (program, arguments) = systemd::sees_command(&test, path);
        std::process::Command::new(program)
            .args(arguments)
            .output()
            .is_ok_and(|output| output.status.success())
    });
    hidden.map_or(Ok(()), |path| {
        Err(anyhow!(
            message::service_home_hidden(path.display()).render()
        ))
    })
}

#[cfg(unix)]
fn output_installed(found: &Found) {
    output::line(
        &message::service_installed(message::ServiceInstalled {
            account: &systemd::USER,
            name: &systemd::UNIT,
        })
        .render(),
    );
    tell_about_setup_code(&found.settings.data_dir);
}

#[cfg(unix)]
fn systemd_uninstall() -> anyhow::Result<()> {
    use systemd::{Step, UNIT, removal};

    require_systemd()?;
    let unit_folder = systemd::unit_folder().map_err(|said| anyhow!(said.render()))?;
    if !unit_folder.join(UNIT).is_file() {
        return Err(anyhow!(message::service_not_installed(UNIT).render()));
    }
    for step in removal(&unit_folder) {
        if let Step::Run { program, arguments } = step {
            run_command(program, &arguments)?;
        }
    }
    output::line(&message::service_removed(UNIT).render());
    Ok(())
}

/// Whether the machine runs systemd (asked of systemd itself), and whether the person has the
/// rights (the account number of root is 0 on every Unix).
#[cfg(unix)]
fn require_systemd() -> anyhow::Result<()> {
    if !systemd::runs_this_computer() {
        return Err(anyhow!(
            Message::from(message::SERVICE_UNSUPPORTED).render()
        ));
    }
    if crate::system::account_number() == 0 {
        Ok(())
    } else {
        Err(anyhow!(
            Message::from(message::SERVICE_NEEDS_ADMIN).render()
        ))
    }
}

#[cfg(unix)]
fn run_command(program: &str, arguments: &[String]) -> anyhow::Result<()> {
    let output = std::process::Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| refused(&format!("{program}: {error}")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(refused(String::from_utf8_lossy(&output.stderr).trim()))
    }
}

/// "The system refused", with what the system said.
fn refused(reason: &str) -> anyhow::Error {
    anyhow!(message::service_failed(reason).render())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_04_service_status_names_the_state() {
        let states = [
            State::NotInstalled,
            State::Starting,
            State::Running,
            State::Restarting,
            State::Stopping,
            State::Stopped,
            State::StoppedAtUpgrade,
            State::Failed(3),
            State::EndedUnexpectedly,
            State::Killed(9),
        ];
        let sentences: Vec<String> = states
            .iter()
            .map(|state| state.sentence("Bugaboo").render())
            .collect();
        for (state, sentence) in states.iter().zip(&sentences) {
            assert!(sentence.contains("Bugaboo"), "{state:?}: {sentence}");
        }
        // Every state has its own words.
        let mut distinct = sentences.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), sentences.len(), "{sentences:?}");
        assert!(sentences[2].contains("is running"), "{}", sentences[2]);
        assert!(sentences[6].contains("upgrade"), "{}", sentences[6]);
        assert!(sentences[7].contains('3'), "{}", sentences[7]);
        // A crash on Windows says how often Windows starts it again; a signal is named.
        let restarts = bb_core::service::RESTARTS.to_string();
        assert!(sentences[8].contains(&restarts), "{}", sentences[8]);
        assert!(sentences[9].contains('9'), "{}", sentences[9]);
    }

    #[test]
    fn ops_04_the_exit_code_of_a_stopped_service_names_its_state() {
        assert_eq!(State::ended_with(0), State::Stopped);
        assert_eq!(
            State::ended_with(u32::from(bb_core::service::UPGRADE_STOPPED)),
            State::StoppedAtUpgrade
        );
        assert_eq!(State::ended_with(1), State::Failed(1));
    }
}
