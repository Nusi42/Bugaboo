//! The service on Windows: made with the service manager's own interface (no `sc.exe` and its
//! quoting), running as the virtual account `NT SERVICE\Bugaboo`. That account has no password,
//! belongs to this service only, and owns the data folder.
//!
//! The service manager starts the program as `bugaboo service run --config <file>`. The program
//! then talks to the service manager: it says when it runs and stops cleanly when asked to.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Context as _, anyhow};
use bb_api::WebSource;
use bb_core::message::{self, Message};
use tokio::sync::oneshot;
use windows_service::Error as ServiceError;
use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{
    self, ServiceControlHandlerResult, ServiceStatusHandle,
};
use windows_service::service_dispatcher;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use super::{Found, NAME, refused, tell_about_setup_code};
use crate::cli::{ConfigArgs, load_settings, serve};
use crate::data_folder::{DataFolder, check_for_service};
use crate::{log, output};

/// The Windows error numbers that are answered with a plain sentence.
const ACCESS_DENIED: i32 = 5;
const SERVICE_DOES_NOT_EXIST: i32 = 1060;
const NOT_RUN_BY_SERVICE_MANAGER: i32 = 1063;
const SERVICE_EXISTS: i32 = 1073;

/// The built-in group of administrators, by its fixed SID (names change with the language).
const ADMINISTRATORS: &str = "*S-1-5-32-544";
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
fn grant(data_dir: &std::path::Path) -> anyhow::Result<()> {
    let system = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_owned());
    let icacls = PathBuf::from(system).join("System32").join("icacls.exe");
    for permission in [
        format!("{}:(OI)(CI)F", account()),
        format!("{ADMINISTRATORS}:(OI)(CI)R"),
    ] {
        let output = std::process::Command::new(&icacls)
            .arg(data_dir)
            .args(["/grant", &permission, "/T", "/Q"])
            .output()
            .context("icacls.exe")?;
        if !output.status.success() {
            return Err(refused(String::from_utf8_lossy(&output.stdout).trim()));
        }
    }
    Ok(())
}

/// `bugaboo service install`.
pub(super) fn install(found: &Found) -> anyhow::Result<()> {
    let manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)
            .map_err(|error| explain(&error))?;
    check_for_service(&found.settings.data_dir)?;
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
    grant(&found.settings.data_dir)?;
    service
        .start::<&str>(&[])
        .map_err(|error| explain(&error))?;
    output::line(&message::service_installed(account(), NAME).render());
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

/// Reports a state to the service manager.
fn report(
    status: ServiceStatusHandle,
    state: ServiceState,
    accepted: ServiceControlAccept,
    exit_code: u32,
) -> anyhow::Result<()> {
    status
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accepted,
            exit_code: ServiceExitCode::Win32(exit_code),
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
        0,
    )?;

    let outcome = serve_until_stopped(args, stopped);
    let exit_code = u32::from(outcome.is_err());
    report(
        status,
        ServiceState::Stopped,
        ServiceControlAccept::empty(),
        exit_code,
    )?;
    outcome
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
