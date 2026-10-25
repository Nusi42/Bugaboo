//! `bugaboo service install` and `uninstall` (OPS-03, OPS-04): Bugaboo as a service that starts
//! with the computer and runs as an account of its own.
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

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use bb_core::message;
#[cfg(not(windows))]
use bb_core::message::Message;

use super::{ConfigArgs, InstallArgs, load_settings, program_folder};
use crate::config::{DEFAULT_FILE, Settings};
use crate::data_folder::DataFolder;
#[cfg(unix)]
use crate::data_folder::{OwnFiles, check_for_service};
use crate::output;

/// The name of the service, as the system lists it.
#[cfg(windows)]
pub(super) const NAME: &str = "Bugaboo";

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
    let new_account_shell = if succeeds("id", &["-u", USER]) {
        None
    } else {
        Some(systemd::no_login().map_err(|said| anyhow!(said.render()))?)
    };
    // `chown -h` changes a link and not the folder it leads to: the plan and the unit name the
    // real folder, and the person is told when that is another one.
    let data_dir = systemd::real_path(&found.settings.data_dir);
    if data_dir != found.settings.data_dir {
        output::line(
            &message::service_data_folder_real(
                found.settings.data_dir.display(),
                data_dir.display(),
            )
            .render(),
        );
    }
    check_for_service(&data_dir, own_files)?;
    systemd::check_settings_file(&found.config, &data_dir)
        .map_err(|problem| anyhow!(problem.render()))?;
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

#[cfg(unix)]
fn output_installed(found: &Found) {
    output::line(&message::service_installed(systemd::USER, systemd::UNIT).render());
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
fn succeeds(program: &str, arguments: &[&str]) -> bool {
    std::process::Command::new(program)
        .args(arguments)
        .output()
        .is_ok_and(|output| output.status.success())
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
