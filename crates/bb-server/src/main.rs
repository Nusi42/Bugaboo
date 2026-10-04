//! The `bugaboo` binary: command line, setup wizard, service install and wiring of all adapters.

#![forbid(unsafe_code)]

mod config;
mod data_folder;
mod log;
mod output;
mod text;

use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context as _;
use bb_api::tls::{TlsError, TlsSource, certificate_path, server_config};
use bb_api::{Incoming, ListenerOptions, Options, WebSource};
use clap::{Args, Parser, Subcommand};
use tokio::net::TcpListener;

use crate::config::{ConfigError, FILE_ENV, Settings, Sources, TlsMode};
use crate::data_folder::DataFolder;
use crate::text::t;

#[derive(Parser)]
#[command(name = "bugaboo", about = "Bugaboo: the issue tracker in one file")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the server.
    Serve(ServeArgs),
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
    let file = config::read_file(named)?;
    config::resolve(&Sources {
        file: file
            .as_ref()
            .map(|(path, text)| (path.as_path(), text.as_str())),
        env: &|name| std::env::var(name).ok(),
        cli: &args.settings(),
    })
}

async fn serve(args: ServeArgs) -> anyhow::Result<()> {
    let settings = load_settings(&args)?;
    log::init(settings.log_level);
    let source = args.web.map_or(WebSource::Embedded, WebSource::Folder);
    let data = DataFolder::prepare(&settings.data_dir)?;
    let listener_options = listener_options(&settings, &data)?;
    // Browsers reach us over HTTPS here, or at a proxy we trust to add it.
    let https = listener_options.tls.is_some() || !settings.trusted_proxies.is_empty();
    // Installed before listening, so a failure is reported instead of stopping silently.
    let shutdown = shutdown_signal()?;
    let address = SocketAddr::new(settings.address, settings.port);
    let tcp = TcpListener::bind(address)
        .await
        .with_context(|| t("server.port-busy", &[("port", &settings.port.to_string())]))?;
    let incoming = Incoming::start(tcp, listener_options)?;

    if !source.has_index().await {
        output::warning(bb_api::WEB_APP_MISSING);
    }
    if settings.tls == TlsMode::SelfSigned {
        let path = certificate_path(&data.tls());
        output::line(&t(
            "server.self-signed",
            &[("path", &path.display().to_string())],
        ));
    }
    output::line(&database_line(settings.database_url.as_deref()));
    let scheme = if settings.tls == TlsMode::Off {
        "http"
    } else {
        "https"
    };
    output::line(&t(
        "server.listening",
        &[("address", &format!("{scheme}://{address}"))],
    ));
    tracing::info!(listen = %address, https, "server started");

    let options = Options {
        web: source,
        https,
        data_folder: data.root().to_path_buf(),
        trusted_proxies: settings.trusted_proxies,
    };
    bb_api::serve(incoming, options, shutdown).await?;
    tracing::info!("server stopped");
    Ok(())
}

/// What the listener does with connections: HTTPS with the configured certificate, or plain
/// HTTP that only trusted proxies may use.
fn listener_options(settings: &Settings, data: &DataFolder) -> anyhow::Result<ListenerOptions> {
    let source = match &settings.tls {
        TlsMode::Off => {
            return Ok(ListenerOptions {
                tls: None,
                only_from: (!settings.trusted_proxies.is_empty())
                    .then(|| settings.trusted_proxies.clone()),
            });
        }
        TlsMode::SelfSigned => TlsSource::SelfSigned {
            folder: data.tls(),
            names: settings.tls_names.clone(),
        },
        TlsMode::Files { cert, key } => TlsSource::Files {
            cert: cert.clone(),
            key: key.clone(),
        },
    };
    let config = server_config(&source).map_err(|error| anyhow::anyhow!(tls_message(&error)))?;
    Ok(ListenerOptions {
        tls: Some(config),
        only_from: None,
    })
}

fn tls_message(error: &TlsError) -> String {
    let shown = |path: &std::path::Path| path.display().to_string();
    match error {
        TlsError::Read { path, source } => t(
            "tls.read-failed",
            &[("path", &shown(path)), ("reason", &source.to_string())],
        ),
        TlsError::Write { path, source } => t(
            "tls.write-failed",
            &[("path", &shown(path)), ("reason", &source.to_string())],
        ),
        TlsError::NoCertificate { path } => t("tls.no-certificate", &[("path", &shown(path))]),
        TlsError::NoKey { path } => t("tls.no-key", &[("path", &shown(path))]),
        TlsError::Unusable { reason } => t("tls.unusable", &[("reason", reason)]),
        TlsError::Generate { reason } => t("tls.generate-failed", &[("reason", reason)]),
    }
}

/// Which database is in use, without ever showing the address (it may hold a password).
fn database_line(url: Option<&str>) -> String {
    match url {
        None => t("server.database-built-in", &[]),
        Some(url) if url.starts_with("sqlite:") => t("server.database-sqlite", &[]),
        Some(_) => t("server.database-postgres", &[]),
    }
}

/// Completes when the system asks the server to stop: Ctrl+C, and on Unix SIGTERM (what
/// service managers send); on Windows also Ctrl+Break, closing the console and shutdown.
fn shutdown_signal() -> anyhow::Result<impl Future<Output = ()> + Send + 'static> {
    let failed = || t("server.signal-failed", &[]);
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

    #[test]
    fn ops_08_the_database_line_never_shows_the_address() {
        let line = database_line(Some("postgres://user:secret@db.example.test/bugaboo"));
        assert!(!line.contains("secret") && !line.contains("db.example"));
        assert!(database_line(Some("sqlite:data.db")).contains("SQLite"));
        assert!(database_line(None).contains("built-in"));
    }
}
