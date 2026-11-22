//! The service on Windows: made with the service manager's own interface (no `sc.exe` and its
//! quoting), running as the virtual account `NT SERVICE\Bugaboo`. That account has no password,
//! belongs to this service only, and owns the data folder.
//!
//! The service manager starts the program as `bugaboo service run --config <file>`. The program
//! then talks to the service manager: it says when it runs and stops cleanly when asked to.
//!
//! After a failure the service manager starts it again. Its failure actions apply only to a
//! service that ends without saying that it stopped (a crash): a start that stopped at an upgrade
//! reports *stopped* with its own code, so it is not started again, and any other failure ends
//! the process without that report, so it is.

use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Context as _, anyhow};
use bb_api::WebSource;
use bb_core::message::{self, Message};
use bb_core::service::{RESTART_COUNT_RESET, RESTART_DELAY, RESTARTS};
use tokio::sync::oneshot;
use windows_service::Error as ServiceError;
use windows_service::service::{
    ServiceAccess, ServiceAction, ServiceActionType, ServiceControl, ServiceControlAccept,
    ServiceErrorControl, ServiceExitCode, ServiceFailureActions, ServiceFailureResetPeriod,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{
    self, ServiceControlHandlerResult, ServiceStatusHandle,
};
use windows_service::service_dispatcher;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use super::windows_rights::{Need, service_can, service_sid_in};
use super::{Found, NAME, State, refused, tell_about_setup_code};
use crate::cli::{ConfigArgs, load_settings, serve};
use crate::data_folder::{ADMINISTRATORS_SID, DataFolder, OwnFiles, check_for_service};
use crate::system::system_tool;
use crate::{log, output};

/// The Windows error numbers that are answered with a plain sentence.
const ACCESS_DENIED: i32 = 5;
const SERVICE_DOES_NOT_EXIST: i32 = 1060;
const NOT_RUN_BY_SERVICE_MANAGER: i32 = 1063;
const SERVICE_EXISTS: i32 = 1073;
/// The exit code the service manager records for a service whose process ended without
/// reporting that it stopped (`ERROR_PROCESS_ABORTED`).
const PROCESS_ABORTED: u32 = 1067;

/// How long a service gets to stop before the uninstall gives up.
const STOP_WAIT: Duration = Duration::from_secs(30);
const STOP_POLL: Duration = Duration::from_millis(250);
/// How long the service manager is told to wait for the service to start and to stop.
const WAIT_HINT: Duration = Duration::from_secs(30);

/// The account the service runs as: a virtual account named after the service.
pub(super) fn account() -> String {
    format!("NT SERVICE\\{NAME}")
}

/// The service as the service manager stores it.
fn info(found: &Found) -> ServiceInfo {
    ServiceInfo {
        name: OsString::from(NAME),
        display_name: OsString::from(Message::from(message::APP_NAME).render()),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: found.program.clone(),
        launch_arguments: vec![
            OsString::from("service"),
            OsString::from("run"),
            OsString::from("--config"),
            found.config.clone().into_os_string(),
        ],
        dependencies: vec![],
        account_name: Some(OsString::from(account())),
        account_password: None,
    }
}

/// A plain sentence for what the service manager refused, by its Windows error number.
fn explain(error: &ServiceError) -> anyhow::Error {
    let ServiceError::Winapi(source) = error else {
        return refused(&error.to_string());
    };
    match source.raw_os_error() {
        Some(ACCESS_DENIED) => anyhow!(Message::from(message::SERVICE_NEEDS_ADMIN).render()),
        Some(SERVICE_EXISTS) => anyhow!(message::service_exists(NAME).render()),
        Some(SERVICE_DOES_NOT_EXIST) => anyhow!(message::service_not_installed(NAME).render()),
        Some(NOT_RUN_BY_SERVICE_MANAGER) => {
            anyhow!(Message::from(message::SERVICE_NOT_A_SERVICE).render())
        }
        _ => refused(&source.to_string()),
    }
}

/// Lets the service account own the data folder, and lets administrators read what is directly
/// in it (the setup code of the first start, which a service cannot print). The folders with
/// secrets (`keys`, `tls`, `logs`) are made private to the service account again at every start.
fn grant(data_dir: &std::path::Path, icacls: &std::path::Path) -> anyhow::Result<()> {
    for permission in [
        format!("{}:(OI)(CI)F", account()),
        format!("*{ADMINISTRATORS_SID}:(OI)(CI)R"),
    ] {
        let output = std::process::Command::new(icacls)
            .arg(data_dir)
            .args(["/grant", &permission, "/T", "/Q"])
            .output()
            .context("icacls.exe")?;
        if !output.status.success() {
            // `icacls` names the path it could not change on its error stream, and only counts
            // on its normal one.
            let said = format!(
                "{} {}",
                String::from_utf8_lossy(&output.stderr).trim(),
                String::from_utf8_lossy(&output.stdout).trim()
            );
            return Err(refused(said.trim()));
        }
    }
    Ok(())
}

/// What the service manager does when the service fails: it starts it again after
/// [`RESTART_DELAY`], [`RESTARTS`] times in a row, and begins to count again after a day without
/// a failure.
fn failure_actions() -> ServiceFailureActions {
    ServiceFailureActions {
        reset_period: ServiceFailureResetPeriod::After(RESTART_COUNT_RESET),
        reboot_msg: None,
        command: None,
        actions: Some(
            std::iter::repeat_with(|| ServiceAction {
                action_type: ServiceActionType::Restart,
                delay: RESTART_DELAY,
            })
            .take(RESTARTS)
            .collect(),
        ),
    }
}

/// Whether the failure actions apply when the service reports *stopped* with an error code. They
/// must not: a start that stopped at an upgrade reports so, and would stop again (see the module).
const FAILURE_ACTIONS_ON_REPORTED_STOPS: bool = false;

/// The SID of the service's own account, which Windows derives from the name of the service, also
/// before the service exists.
fn service_sid() -> anyhow::Result<String> {
    let sc = system_tool("sc.exe").map_err(|said| refused(&said.render()))?;
    let output = std::process::Command::new(sc)
        .args(["showsid", NAME])
        .output()
        .context("sc.exe")?;
    service_sid_in(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| refused(String::from_utf8_lossy(&output.stdout).trim()))
}

/// Refuses when the account of the service cannot run the program or read the settings file.
fn check_account_can_read(found: &Found) -> anyhow::Result<()> {
    let sid = service_sid()?;
    for (path, need) in [(&found.program, Need::Run), (&found.config, Need::Read)] {
        let readable = crate::data_folder::rights::access_list(path)
            .is_some_and(|sddl| service_can(&sddl, &sid, need));
        if !readable {
            return Err(anyhow!(
                message::service_account_cannot_read(message::ServiceAccountCannotRead {
                    account: &account(),
                    path: &path.display(),
                })
                .render()
            ));
        }
    }
    Ok(())
}

/// `bugaboo service install`.
pub(super) fn install(found: &Found, own_files: OwnFiles) -> anyhow::Result<()> {
    // What Windows names is asked before anything is created, so a failure leaves no half-made service.
    let icacls = system_tool("icacls.exe").map_err(|said| refused(&said.render()))?;
    check_account_can_read(found)?;
    let manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)
            .map_err(|error| explain(&error))?;
    check_for_service(&found.settings.data_dir, own_files)?;
    DataFolder::prepare(&found.settings.data_dir)?;
    let service = manager
        .create_service(
            &info(found),
            ServiceAccess::START | ServiceAccess::CHANGE_CONFIG,
        )
        .map_err(|error| explain(&error))?;
    service
        .set_description(Message::from(message::SERVER_ABOUT).render())
        .map_err(|error| explain(&error))?;
    service
        .update_failure_actions(failure_actions())
        .map_err(|error| explain(&error))?;
    service
        .set_failure_actions_on_non_crash_failures(FAILURE_ACTIONS_ON_REPORTED_STOPS)
        .map_err(|error| explain(&error))?;
    grant(&found.settings.data_dir, &icacls)?;
    service
        .start::<&str>(&[])
        .map_err(|error| explain(&error))?;
    output::line(
        &message::service_installed(message::ServiceInstalled {
            account: &account(),
            name: &NAME,
        })
        .render(),
    );
    tell_about_setup_code(&found.settings.data_dir);
    Ok(())
}

/// `bugaboo service uninstall`: stops the service if it runs and removes it.
pub(super) fn uninstall() -> anyhow::Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|error| explain(&error))?;
    let service = manager
        .open_service(
            NAME,
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
        )
        .map_err(|error| explain(&error))?;
    if service
        .query_status()
        .map_err(|error| explain(&error))?
        .current_state
        != ServiceState::Stopped
    {
        service.stop().map_err(|error| explain(&error))?;
        let started = Instant::now();
        while service
            .query_status()
            .map_err(|error| explain(&error))?
            .current_state
            != ServiceState::Stopped
        {
            if started.elapsed() > STOP_WAIT {
                return Err(anyhow!(message::service_stop_timeout(NAME).render()));
            }
            std::thread::sleep(STOP_POLL);
        }
    }
    service.delete().map_err(|error| explain(&error))?;
    output::line(&message::service_removed(NAME).render());
    Ok(())
}

/// `bugaboo service status`: the state of the service, which any account may ask for.
pub(super) fn state() -> anyhow::Result<State> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|error| explain(&error))?;
    let service = match manager.open_service(NAME, ServiceAccess::QUERY_STATUS) {
        Ok(service) => service,
        Err(ServiceError::Winapi(source))
            if source.raw_os_error() == Some(SERVICE_DOES_NOT_EXIST) =>
        {
            return Ok(State::NotInstalled);
        }
        Err(error) => return Err(explain(&error)),
    };
    Ok(state_of(
        &service.query_status().map_err(|error| explain(&error))?,
    ))
}

/// The state in what the service manager reports.
fn state_of(status: &ServiceStatus) -> State {
    match status.current_state {
        ServiceState::StartPending | ServiceState::ContinuePending => State::Starting,
        ServiceState::Running | ServiceState::Paused | ServiceState::PausePending => State::Running,
        ServiceState::StopPending => State::Stopping,
        // A crash. The failure actions answer it with a start after the delay, but only a few
        // times a day: after that the service stays stopped with the same code, so the state
        // names the crash and not a restart that may never come.
        ServiceState::Stopped if status.exit_code == ServiceExitCode::Win32(PROCESS_ABORTED) => {
            State::EndedUnexpectedly
        }
        ServiceState::Stopped => State::ended_with(match status.exit_code {
            ServiceExitCode::Win32(code) | ServiceExitCode::ServiceSpecific(code) => code,
        }),
    }
}

/// What the service main function needs, handed over by [`run`]: the service manager calls it
/// on a thread of its own and gives it nothing else.
static START: OnceLock<ConfigArgs> = OnceLock::new();

/// `bugaboo service run`: hands the program over to the service manager, which calls
/// [`service_main`] and keeps this thread until the service has stopped.
pub(super) fn run(args: &ConfigArgs) -> anyhow::Result<()> {
    START
        .set(args.clone())
        .map_err(|_| anyhow!(Message::from(message::SERVICE_NOT_A_SERVICE).render()))?;
    service_dispatcher::start(NAME, service_main).map_err(|error| explain(&error))
}

/// The entry point the service manager calls. It is written out here instead of with the macro
/// of the library, because the macro contains `unsafe` code and this crate has none; the
/// arguments of the service are not needed, everything is in the settings file.
extern "system" fn service_main(_count: u32, _arguments: *mut *mut u16) {
    if let Err(error) = run_service() {
        // The service manager has no console to show this on: it goes to the log file.
        tracing::error!(cause = %format!("{error:#}"), "the service stopped with an error");
    }
}

/// The exit code of the process of a service that failed in any other way: the common code of a
/// failure, as the exit code 1 of the command line.
const FAILED_PROCESS: u8 = 1;

/// Reports a state to the service manager.
fn report(
    status: ServiceStatusHandle,
    state: ServiceState,
    accepted: ServiceControlAccept,
    exit_code: ServiceExitCode,
) -> anyhow::Result<()> {
    status
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accepted,
            exit_code,
            checkpoint: 0,
            wait_hint: WAIT_HINT,
            process_id: None,
        })
        .map_err(|error| explain(&error))
}

fn run_service() -> anyhow::Result<()> {
    let args = START
        .get()
        .ok_or_else(|| anyhow!(Message::from(message::SERVICE_NOT_A_SERVICE).render()))?;
    let (stop, stopped) = oneshot::channel::<()>();
    let stop = Mutex::new(Some(stop));
    let status = service_control_handler::register(NAME, move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            if let Some(sender) = stop.lock().ok().and_then(|mut sender| sender.take()) {
                // The receiver is gone only when the server has stopped already.
                let _ = sender.send(());
            }
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    })
    .map_err(|error| explain(&error))?;
    // Running is reported at once: the server needs a few seconds to open its database, and the
    // service manager only waits for the start to be reported, not for the port to be open.
    report(
        status,
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        ServiceExitCode::NO_ERROR,
    )?;

    let outcome = serve_until_stopped(args, stopped);
    match ending(&outcome) {
        Ending::Reported(exit_code) => report(
            status,
            ServiceState::Stopped,
            ServiceControlAccept::empty(),
            exit_code,
        )?,
        Ending::Crash => {
            if let Err(error) = &outcome {
                tracing::error!(cause = %format!("{error:#}"), "the service failed and starts again");
            }
            std::process::exit(i32::from(FAILED_PROCESS));
        }
    }
    outcome
}

/// How the service ends, by what its start came to.
#[derive(Debug, PartialEq, Eq)]
enum Ending {
    /// It reports *stopped* with this code: the service manager does not start it again.
    Reported(ServiceExitCode),
    /// It ends the process without reporting: the service manager counts a crash and starts it
    /// again after the delay of its failure actions.
    Crash,
}

/// A stop that was asked for, and a start that stopped at an upgrade (with its own code, so the
/// service manager and `service status` can tell), are reported; any other failure is a crash.
fn ending(outcome: &anyhow::Result<()>) -> Ending {
    match outcome {
        Ok(()) => Ending::Reported(ServiceExitCode::NO_ERROR),
        Err(error) => crate::upgrade::stop_code(error).map_or(Ending::Crash, |code| {
            Ending::Reported(ServiceExitCode::ServiceSpecific(u32::from(code)))
        }),
    }
}

/// Starts the server like `bugaboo serve`, with its log in a file, and runs it until the
/// service manager asks it to stop.
fn serve_until_stopped(args: &ConfigArgs, stopped: oneshot::Receiver<()>) -> anyhow::Result<()> {
    let settings = load_settings(args.config.as_deref(), &[], true)?;
    let data = DataFolder::prepare(&settings.data_dir)?;
    log::init_file(settings.log_level, &data.server_log());
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(serve::serve_settings(
        settings,
        WebSource::Embedded,
        // A service has no console to print on.
        false,
        async move {
            let _ = stopped.await;
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_04_a_stopped_upgrade_does_not_restart_the_service() {
        // The failure actions restart, after the shared delay, but only after a crash.
        let actions = failure_actions();
        assert_eq!(
            actions.reset_period,
            ServiceFailureResetPeriod::After(RESTART_COUNT_RESET)
        );
        let restarts = actions.actions.unwrap();
        assert_eq!(restarts.len(), RESTARTS);
        assert!(restarts.iter().all(|action| {
            action.action_type == ServiceActionType::Restart && action.delay == RESTART_DELAY
        }));
        const { assert!(!FAILURE_ACTIONS_ON_REPORTED_STOPS) };
        // A start that stopped at an upgrade reports its code: no restart.
        let stopped: anyhow::Result<()> = Err(crate::upgrade::stopped_for_tests());
        assert_eq!(
            ending(&stopped),
            Ending::Reported(ServiceExitCode::ServiceSpecific(u32::from(
                bb_core::service::UPGRADE_STOPPED
            )))
        );
        // Any other failure is a crash: restarted.
        assert_eq!(ending(&Err(anyhow!("the port is taken"))), Ending::Crash);
        assert_eq!(ending(&Ok(())), Ending::Reported(ServiceExitCode::NO_ERROR));
    }

    #[test]
    fn ops_04_service_status_reads_what_the_service_manager_reports() {
        let status = |state, exit_code| ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code,
            checkpoint: 0,
            wait_hint: WAIT_HINT,
            process_id: None,
        };
        assert_eq!(
            state_of(&status(ServiceState::Running, ServiceExitCode::NO_ERROR)),
            State::Running
        );
        assert_eq!(
            state_of(&status(ServiceState::Stopped, ServiceExitCode::NO_ERROR)),
            State::Stopped
        );
        assert_eq!(
            state_of(&status(
                ServiceState::Stopped,
                ServiceExitCode::ServiceSpecific(u32::from(bb_core::service::UPGRADE_STOPPED))
            )),
            State::StoppedAtUpgrade
        );
        assert_eq!(
            state_of(&status(
                ServiceState::Stopped,
                ServiceExitCode::Win32(PROCESS_ABORTED)
            )),
            State::EndedUnexpectedly
        );
        assert_eq!(
            state_of(&status(ServiceState::Stopped, ServiceExitCode::Win32(5))),
            State::Failed(5)
        );
    }
}
