//! The command line of `bugaboo`: what it can do and how each command finds its settings.
//!
//! - `serve` starts the server (`serve.rs`);
//! - `setup` asks a few questions and writes the settings file (`setup.rs`, OPS-03);
//! - `service install` / `service uninstall` run Bugaboo as a service (`service.rs`);
//! - `security-log verify`, `rotate-key` and `keys export` look after the secrets (`secrets.rs`,
//!   M1-F19, M1-F20, M1-F22);
//! - `openapi` prints the description of the API.
//!
//! Every text a person reads, the help of each option included, comes from the message catalog.

mod renewal;
mod secrets;
mod serve;
mod service;
mod setup;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bb_core::message::{self, Message, MessageKey};
use clap::{Args, Parser, Subcommand};

use crate::config::{self, ConfigError, FILE_ENV, Settings, Sources};
use crate::output;

/// The text of a message without placeholders, for the help texts of the command line.
fn text(key: MessageKey) -> String {
    Message::from(key).render()
}

#[derive(Parser)]
#[command(name = "bugaboo", about = text(message::SERVER_ABOUT))]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = text(message::CLI_ABOUT_SERVE))]
    Serve(ServeArgs),
    #[command(about = text(message::CLI_ABOUT_OPENAPI))]
    Openapi,
    #[command(about = text(message::CLI_ABOUT_SETUP))]
    Setup(setup::SetupArgs),
    #[command(about = text(message::CLI_ABOUT_SERVICE))]
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    #[command(name = "security-log", about = text(message::CLI_ABOUT_SECURITY_LOG))]
    SecurityLog {
        #[command(subcommand)]
        command: SecurityLogCommand,
    },
    #[command(name = "rotate-key", about = text(message::CLI_ABOUT_ROTATE_KEY))]
    RotateKey(StoreArgs),
    #[command(about = text(message::CLI_ABOUT_KEYS))]
    Keys {
        #[command(subcommand)]
        command: KeysCommand,
    },
}

#[derive(Subcommand)]
enum ServiceCommand {
    #[command(about = text(message::CLI_ABOUT_SERVICE_INSTALL))]
    Install(ConfigArgs),
    #[command(about = text(message::CLI_ABOUT_SERVICE_UNINSTALL))]
    Uninstall,
    /// Started by the Windows service manager, never by a person.
    #[command(name = "run", hide = true)]
    Run(ConfigArgs),
}

#[derive(Subcommand)]
enum SecurityLogCommand {
    #[command(about = text(message::CLI_ABOUT_SECURITY_LOG_VERIFY))]
    Verify(StoreArgs),
}

#[derive(Subcommand)]
enum KeysCommand {
    #[command(about = text(message::CLI_ABOUT_KEYS_EXPORT))]
    Export {
        #[command(flatten)]
        store: StoreArgs,
        /// The folder the key files are copied into.
        #[arg(help = text(message::CLI_HELP_KEYS_FOLDER))]
        folder: PathBuf,
    },
}

/// Start-up settings can also come from `bugaboo.toml` or `BUGABOO_…` variables; values given
/// here win. There is deliberately no default port (ADR-0023).
#[derive(Args)]
pub(crate) struct ServeArgs {
    #[arg(long, help = text(message::CLI_HELP_CONFIG))]
    config: Option<PathBuf>,
    #[arg(long, help = text(message::CLI_HELP_PORT))]
    port: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_ADDRESS))]
    address: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_DATA_DIR))]
    data_dir: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_TLS))]
    tls: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_WEB))]
    web: Option<PathBuf>,
}

impl ServeArgs {
    /// The settings given on the command line, by setting name.
    fn settings(&self) -> Vec<(&'static str, String)> {
        given([
            ("port", &self.port),
            ("address", &self.address),
            ("data_dir", &self.data_dir),
            ("tls_mode", &self.tls),
        ])
    }
}

/// The settings file, for the commands that need nothing else.
#[derive(Args, Clone, Default)]
pub(crate) struct ConfigArgs {
    #[arg(long, help = text(message::CLI_HELP_CONFIG))]
    config: Option<PathBuf>,
}

/// What a command that works on the stored data (and does not listen) needs to find it.
#[derive(Args, Clone, Default)]
pub(crate) struct StoreArgs {
    #[arg(long, help = text(message::CLI_HELP_CONFIG))]
    config: Option<PathBuf>,
    #[arg(long, help = text(message::CLI_HELP_DATA_DIR))]
    data_dir: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_DATABASE_URL))]
    database_url: Option<String>,
}

impl StoreArgs {
    fn settings(&self) -> Vec<(&'static str, String)> {
        given([
            ("data_dir", &self.data_dir),
            ("database_url", &self.database_url),
        ])
    }
}

fn given<const N: usize>(
    options: [(&'static str, &Option<String>); N],
) -> Vec<(&'static str, String)> {
    options
        .into_iter()
        .filter_map(|(name, value)| value.clone().map(|value| (name, value)))
        .collect()
}

/// Runs the command line and says how it went.
pub(crate) async fn run() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Serve(args) => serve::serve(args).await,
        Command::Openapi => {
            output::raw(bb_api::openapi::json());
            Ok(())
        }
        Command::Setup(args) => setup::run(&args),
        Command::Service { command } => match command {
            ServiceCommand::Install(args) => service::install(&args),
            ServiceCommand::Uninstall => service::uninstall(),
            ServiceCommand::Run(args) => service::run(&args),
        },
        Command::SecurityLog {
            command: SecurityLogCommand::Verify(args),
        } => secrets::verify_security_log(&args).await,
        Command::RotateKey(args) => secrets::rotate_key(&args).await,
        Command::Keys {
            command: KeysCommand::Export { store, folder },
        } => secrets::export_keys(&store, &folder).await,
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            output::error(&format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

/// The settings of a command: the settings file, the environment and the options given.
fn load_settings(
    config: Option<&Path>,
    given: &[(&'static str, String)],
    listening: bool,
) -> Result<Settings, ConfigError> {
    let named = config
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os(FILE_ENV).map(PathBuf::from));
    let base = program_folder();
    let file = config::read_file(named, &base)?;
    let sources = Sources {
        file: file
            .as_ref()
            .map(|(path, text)| (path.as_path(), text.as_str())),
        env: &|name| std::env::var(name).ok(),
        cli: given,
        base: &base,
    };
    if listening {
        config::resolve(&sources)
    } else {
        config::resolve_without_port(&sources)
    }
}

/// The folder of the program. Defaults are relative to it, not to the current folder: a
/// Windows service starts in `System32`, a systemd unit in `/`.
pub(crate) fn program_folder() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
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
    fn ops_03_every_command_of_this_package_parses() {
        for line in [
            "bugaboo setup",
            "bugaboo setup --non-interactive --port 8443 --tls off --overwrite",
            "bugaboo service install",
            "bugaboo service install --config bugaboo.toml",
            "bugaboo service uninstall",
            "bugaboo security-log verify",
            "bugaboo security-log verify --data-dir data",
            "bugaboo rotate-key",
            "bugaboo keys export backup-folder",
        ] {
            let words: Vec<&str> = line.split(' ').collect();
            assert!(Cli::try_parse_from(words).is_ok(), "{line}");
        }
        assert!(Cli::try_parse_from(["bugaboo", "keys", "export"]).is_err());
        assert!(Cli::try_parse_from(["bugaboo", "security-log"]).is_err());
    }

    #[test]
    fn ops_03_commands_that_do_not_listen_need_no_port() {
        let args = StoreArgs {
            data_dir: Some("data".to_owned()),
            ..StoreArgs::default()
        };
        assert_eq!(args.settings(), [("data_dir", "data".to_owned())]);
    }
}
