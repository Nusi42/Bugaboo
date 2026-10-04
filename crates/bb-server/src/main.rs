//! The `bugaboo` binary: command line, setup wizard, service install and wiring of all adapters.

#![forbid(unsafe_code)]

mod config;
mod data_folder;
mod log;
mod output;
mod wiring;

use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context as _;
use bb_api::tls::{TlsError, TlsSource, certificate_path, server_config};
use bb_api::{
    HSTS_MAX_AGE_SELF_SIGNED, HSTS_MAX_AGE_TRUSTED, Incoming, ListenerOptions, Options, WebSource,
};
use clap::{Args, Parser, Subcommand};
use tokio::net::TcpListener;

use crate::config::{ConfigError, FILE_ENV, Settings, Sources, TlsMode};
use crate::data_folder::DataFolder;
use bb_core::message::{self, Message};

#[derive(Parser)]
#[command(name = "bugaboo", about = Message::from(message::SERVER_ABOUT).render())]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the server.
    Serve(ServeArgs),
    /// Print the description of the API (OpenAPI) and stop.
    Openapi,
}

/// Start-up settings can also come from `bugaboo.toml` or `BUGABOO_…` variables; values given
/// here win. There is deliberately no default port (ADR-0023).
#[derive(Args)]
struct ServeArgs {
    /// The settings file (default: bugaboo.toml in this folder, if it exists).
    #[arg(long)]
    config: Option<PathBuf>,
    /// The port to listen on.
    #[arg(long)]
    port: Option<String>,
    /// The address to listen on (default 127.0.0.1, this computer only).
    #[arg(long)]
    address: Option<String>,
    /// The folder that holds all data.
    #[arg(long)]
    data_dir: Option<String>,
    /// HTTPS mode: self-signed, files or off.
    #[arg(long)]
    tls: Option<String>,
    /// Serve the browser app from this folder instead of the one built into the program.
    #[arg(long)]
    web: Option<PathBuf>,
}

impl ServeArgs {
    /// The settings given on the command line, by setting name.
    fn settings(&self) -> Vec<(&'static str, String)> {
        [
            ("port", &self.port),
            ("address", &self.address),
            ("data_dir", &self.data_dir),
            ("tls_mode", &self.tls),
        ]
        .into_iter()
        .filter_map(|(name, value)| value.clone().map(|value| (name, value)))
        .collect()
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Serve(args) => serve(args).await,
        Command::Openapi => {
            output::raw(bb_api::openapi::json());
            Ok(())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            output::error(&format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

fn load_settings(args: &ServeArgs) -> Result<Settings, ConfigError> {
    let named = args
        .config
        .clone()
        .or_else(|| std::env::var_os(FILE_ENV).map(PathBuf::from));
    let base = program_folder();
    let file = config::read_file(named, &base)?;
    config::resolve(&Sources {
        file: file
            .as_ref()
            .map(|(path, text)| (path.as_path(), text.as_str())),
        env: &|name| std::env::var(name).ok(),
        cli: &args.settings(),
        base: &base,
    })
}

/// The folder of the program. Defaults are relative to it, not to the current folder: a
/// Windows service starts in `System32`, a systemd unit in `/`.
fn program_folder() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

async fn serve(args: ServeArgs) -> anyhow::Result<()> {
    let settings = load_settings(&args)?;
    log::init(settings.log_level);
    let source = args.web.map_or(WebSource::Embedded, WebSource::Folder);
    let data = DataFolder::prepare(&settings.data_dir)?;
    let listener_options = listener_options(&settings, &data)?;
    let hsts_max_age = hsts_max_age(&settings);
    // Installed before listening, so a failure is reported instead of stopping silently.
    let shutdown = shutdown_signal()?;
    // The database, the keys and the sign-in services come first: when one of them fails the
    // server stops here, before it listens (M1-F19, M1-F24).
    let wired = wiring::wire(&settings, &data).await?;
    let address = SocketAddr::new(settings.address, settings.port);
    let tcp = TcpListener::bind(address)
        .await
        .with_context(|| message::server_port_busy(settings.port).render())?;
    let incoming = Incoming::start(tcp, listener_options);

    if !source.has_index().await {
        output::warning(&Message::from(message::SERVER_WEB_APP_MISSING).render());
    }
    if settings.tls == TlsMode::SelfSigned {
        let path = certificate_path(&data.tls());
        output::line(&message::server_self_signed(path.display()).render());
    }
    output::line(&database_line(settings.database_url.as_deref()));
    let scheme = if settings.tls == TlsMode::Off {
        "http"
    } else {
        "https"
    };
    output::line(&message::server_listening(format!("{scheme}://{address}")).render());
    if let Some(code) = &wired.setup_code {
        // Only to the terminal, never to the log file: the code is a secret until it is used.
        output::line(&message::server_setup_code(code).render());
        output::line(&Message::from(message::SERVER_SETUP_CODE_HINT).render());
    }
    tracing::info!(https = hsts_max_age.is_some(), "server started");

    let options = Options {
        web: source,
        hsts_max_age,
        header_timeout: settings.header_timeout,
        data_folder: data.root().to_path_buf(),
        trusted_proxies: settings.trusted_proxies,
        limits: settings.limits,
        auth: wired.auth,
    };
    bb_api::serve(incoming, options, shutdown).await?;
    tracing::info!("server stopped");
    Ok(())
}

/// What the listener does with connections: HTTPS with the configured certificate, or plain
/// HTTP that only trusted proxies may use.
/// How long browsers are told to insist on HTTPS: a day with a certificate Bugaboo made (a
/// person who trusts it for `localhost` must not pin their other local tools), a year with a
/// provided certificate or behind a trusted proxy, nothing for plain HTTP on this computer.
fn hsts_max_age(settings: &Settings) -> Option<u64> {
    match settings.tls {
        TlsMode::SelfSigned => Some(HSTS_MAX_AGE_SELF_SIGNED),
        TlsMode::Files { .. } => Some(HSTS_MAX_AGE_TRUSTED),
        TlsMode::Off => (!settings.trusted_proxies.is_empty()).then_some(HSTS_MAX_AGE_TRUSTED),
    }
}

fn listener_options(settings: &Settings, data: &DataFolder) -> anyhow::Result<ListenerOptions> {
    let source = match &settings.tls {
        TlsMode::Off => {
            return Ok(ListenerOptions {
                tls: None,
                only_from: (!settings.trusted_proxies.is_empty())
                    .then(|| settings.trusted_proxies.clone()),
                max_connections: settings.max_connections,
            });
        }
        TlsMode::SelfSigned => TlsSource::SelfSigned {
            folder: data.tls(),
            names: settings.tls_names.clone(),
        },
        TlsMode::Files { cert, key } => {
            // Windows folders carry their own access lists; there is no mode to read.
            #[cfg(unix)]
            if let Some(warning) = key_readable_warning(key) {
                output::warning(&warning);
            }
            TlsSource::Files {
                cert: cert.clone(),
                key: key.clone(),
            }
        }
    };
    let config = server_config(&source).map_err(|error| anyhow::anyhow!(tls_message(&error)))?;
    Ok(ListenerOptions {
        tls: Some(config),
        only_from: None,
        max_connections: settings.max_connections,
    })
}

/// A private key that other people on this Linux computer can read (group or world bits).
#[cfg(unix)]
fn key_readable_warning(key: &std::path::Path) -> Option<String> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = std::fs::metadata(key).ok()?.permissions().mode();
    (mode & 0o077 != 0).then(|| message::tls_key_readable(key.display()).render())
}

fn tls_message(error: &TlsError) -> String {
    let shown = |path: &std::path::Path| path.display().to_string();
    match error {
        TlsError::Read { path, source } => message::tls_read_failed(shown(path), source).render(),
        TlsError::Write { path, source } => message::tls_write_failed(shown(path), source).render(),
        TlsError::NoCertificate { path } => message::tls_no_certificate(shown(path)).render(),
        TlsError::NoKey { path } => message::tls_no_key(shown(path)).render(),
        TlsError::Unusable { reason } => message::tls_unusable(reason).render(),
        TlsError::Generate { reason } => message::tls_generate_failed(reason).render(),
    }
}

/// Which database is in use, without ever showing the address (it may hold a password).
fn database_line(url: Option<&str>) -> String {
    match url {
        None => Message::from(message::SERVER_DATABASE_BUILT_IN).render(),
        Some(url) if url.starts_with("sqlite:") => {
            Message::from(message::SERVER_DATABASE_SQLITE).render()
        }
        Some(_) => Message::from(message::SERVER_DATABASE_POSTGRES).render(),
    }
}

/// Completes when the system asks the server to stop: Ctrl+C, and on Unix SIGTERM (what
/// service managers send); on Windows also Ctrl+Break, closing the console and shutdown.
fn shutdown_signal() -> anyhow::Result<impl Future<Output = ()> + Send + 'static> {
    let failed = || Message::from(message::SERVER_SIGNAL_FAILED).render();
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut interrupt = signal(SignalKind::interrupt()).with_context(failed)?;
        let mut terminate = signal(SignalKind::terminate()).with_context(failed)?;
        Ok(async move {
            tokio::select! {
                _ = interrupt.recv() => {}
                _ = terminate.recv() => {}
            }
        })
    }
    #[cfg(windows)]
    {
        use tokio::signal::windows::{ctrl_break, ctrl_c, ctrl_close, ctrl_shutdown};
        let mut interrupt = ctrl_c().with_context(failed)?;
        let mut break_key = ctrl_break().with_context(failed)?;
        let mut close = ctrl_close().with_context(failed)?;
        let mut shutdown = ctrl_shutdown().with_context(failed)?;
        Ok(async move {
            tokio::select! {
                _ = interrupt.recv() => {}
                _ = break_key.recv() => {}
                _ = close.recv() => {}
                _ = shutdown.recv() => {}
            }
        })
    }
}

#[cfg(test)]
#[path = "tests/sign_in/mod.rs"]
mod sign_in_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dist_01_serve_takes_every_setting_from_the_command_line_or_elsewhere() {
        // Without a port on the command line the settings may still come from the
        // environment or the file; the missing port is reported by the settings check.
        assert!(Cli::try_parse_from(["bugaboo", "serve"]).is_ok());
        assert!(Cli::try_parse_from(["bugaboo", "serve", "--port", "47200"]).is_ok());
        assert!(
            Cli::try_parse_from(["bugaboo", "serve", "--port", "47200", "--web", "some/dir"])
                .is_ok()
        );
        assert!(Cli::try_parse_from(["bugaboo", "serve", "--unknown"]).is_err());
    }

    #[test]
    fn ops_10_command_line_values_become_settings() {
        let Ok(Cli {
            command: Command::Serve(args),
        }) = Cli::try_parse_from(["bugaboo", "serve", "--port", "1", "--tls", "off"])
        else {
            panic!("the command line should parse");
        };
        assert_eq!(
            args.settings(),
            [("port", "1".to_owned()), ("tls_mode", "off".to_owned())]
        );
    }

    #[cfg(unix)]
    #[test]
    fn sec_10_a_key_that_others_can_read_is_reported() {
        use std::os::unix::fs::PermissionsExt as _;
        let folder = tempfile::tempdir().unwrap();
        let key = folder.path().join("key.pem");
        std::fs::write(&key, "x").unwrap();
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o640)).unwrap();
        assert!(key_readable_warning(&key).is_some());
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(key_readable_warning(&key).is_none());
    }

    #[test]
    fn sec_11_hsts_lifetime_follows_who_made_the_certificate() {
        let settings = |args: &[&'static str]| {
            let sources = args
                .iter()
                .map(|pair| pair.split_once('=').unwrap())
                .map(|(k, v)| (k, v.to_owned()))
                .collect::<Vec<_>>();
            config::resolve(&Sources {
                file: None,
                env: &|_| None,
                cli: &sources,
                base: std::path::Path::new("exe"),
            })
            .unwrap()
        };
        let own = settings(&["port=1"]);
        assert_eq!(hsts_max_age(&own), Some(HSTS_MAX_AGE_SELF_SIGNED));
        let local = settings(&["port=1", "tls_mode=off"]);
        assert_eq!(hsts_max_age(&local), None);
        let proxied = settings(&[
            "port=1",
            "tls_mode=off",
            "trusted_proxies=10.0.0.5",
            "public_url=https://bugaboo.example.test",
        ]);
        assert_eq!(hsts_max_age(&proxied), Some(HSTS_MAX_AGE_TRUSTED));
        let provided = settings(&["port=1", "tls_mode=files", "tls_cert=a", "tls_key=b"]);
        assert_eq!(hsts_max_age(&provided), Some(HSTS_MAX_AGE_TRUSTED));
    }

    #[test]
    fn ops_08_the_database_line_never_shows_the_address() {
        let line = database_line(Some("postgres://user:secret@db.example.test/bugaboo"));
        assert!(!line.contains("secret") && !line.contains("db.example"));
        assert!(database_line(Some("sqlite:data.db")).contains("SQLite"));
        assert!(database_line(None).contains("built-in"));
    }
}
