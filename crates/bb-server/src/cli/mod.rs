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

use bb_core::files::PreviewKind;
use bb_core::message::{self, Message, MessageKey};
use clap::{Args, Parser, Subcommand};

use crate::config::{self, ConfigError, DEFAULT_FILE, FILE_ENV, SettingName, Settings, Sources};
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
    /// Started by the server, never by a person: creates the preview of one file in a process of
    /// its own, with limits on pixels, time, size and memory (ADR-0032 §4). The file comes on the
    /// standard input and the preview leaves on the standard output.
    #[command(name = "preview", hide = true)]
    Preview(PreviewArgs),
}

/// What `bugaboo preview` is told: the kind of preview. The rest comes on its standard streams.
#[derive(Args)]
pub(crate) struct PreviewArgs {
    /// `image` or `pdf`.
    #[arg(value_parser = preview_kind)]
    kind: PreviewKind,
}

/// The kind of a preview, as the server names it on the command line.
fn preview_kind(word: &str) -> Result<PreviewKind, String> {
    PreviewKind::from_code(word).ok_or_else(|| word.to_owned())
}

#[derive(Subcommand)]
enum ServiceCommand {
    #[command(about = text(message::CLI_ABOUT_SERVICE_INSTALL))]
    Install(InstallArgs),
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
    fn settings(&self) -> Vec<(SettingName, String)> {
        given([
            (SettingName::Port, &self.port),
            (SettingName::Address, &self.address),
            (SettingName::DataDir, &self.data_dir),
            (SettingName::TlsMode, &self.tls),
        ])
    }
}

/// The settings file, for the commands that need nothing else.
#[derive(Args, Clone, Default)]
pub(crate) struct ConfigArgs {
    #[arg(long, help = text(message::CLI_HELP_CONFIG))]
    config: Option<PathBuf>,
}

/// What `bugaboo service install` needs: the settings file and whether a data folder with only
/// the person's own certificate may be taken over (M1-F66).
#[derive(Args, Clone, Default)]
pub(crate) struct InstallArgs {
    #[command(flatten)]
    config: ConfigArgs,
    #[arg(long, help = text(message::CLI_HELP_TAKE_OVER_DATA_FOLDER))]
    take_over_data_folder: bool,
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
    fn settings(&self) -> Vec<(SettingName, String)> {
        given([
            (SettingName::DataDir, &self.data_dir),
            (SettingName::DatabaseUrl, &self.database_url),
        ])
    }
}

fn given<const N: usize>(
    options: [(SettingName, &Option<String>); N],
) -> Vec<(SettingName, String)> {
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
        // The child process of a preview: its exit code is its answer (`PreviewRefusal`).
        Command::Preview(args) => {
            let code = bb_files::preview::run(args.kind, &crate::ALLOCATOR);
            return u8::try_from(code).map_or(ExitCode::FAILURE, ExitCode::from);
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

/// The warning for a database address with a password on the command line, or `None` when the
/// command line holds none. Other people on the computer can read the command line (in the list
/// of running programs), so the warning names the places that are safe: the settings file and the
/// environment variable. It never repeats the address.
fn password_warning(given: &[(SettingName, String)], file: &str) -> Option<String> {
    given
        .iter()
        .any(|(name, value)| *name == SettingName::DatabaseUrl && config::has_password(value))
        .then(|| {
            message::cli_database_url_password(file, config::env_name(SettingName::DatabaseUrl))
                .render()
        })
}

/// The settings of a command: the settings file, the environment and the options given.
fn load_settings(
    config: Option<&Path>,
    given: &[(SettingName, String)],
    listening: bool,
) -> Result<Settings, ConfigError> {
    let named = config
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os(FILE_ENV).map(PathBuf::from));
    if let Some(warning) = password_warning(
        given,
        &named.as_ref().map_or_else(
            || DEFAULT_FILE.to_owned(),
            |path| path.display().to_string(),
        ),
    ) {
        output::warning(&warning);
    }
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
            [
                (SettingName::Port, "1".to_owned()),
                (SettingName::TlsMode, "off".to_owned())
            ]
        );
    }

    #[test]
    fn ops_03_every_command_of_this_package_parses() {
        for line in [
            "bugaboo setup",
            "bugaboo setup --non-interactive --port 8443 --tls off --overwrite",
            "bugaboo service install",
            "bugaboo service install --config bugaboo.toml",
            "bugaboo service install --take-over-data-folder",
            "bugaboo service install --config bugaboo.toml --take-over-data-folder",
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
        assert_eq!(args.settings(), [(SettingName::DataDir, "data".to_owned())]);
    }

    #[test]
    fn ops_10_a_database_url_with_a_password_on_the_command_line_warns() {
        let given = |url: &str| vec![(SettingName::DatabaseUrl, url.to_owned())];
        let warning = password_warning(
            &given("postgres://bugaboo:Hunter2-Secret@db.example.test/bugaboo"),
            "bugaboo.toml",
        )
        .unwrap_or_default();
        // It names the safe places, in plain words ...
        assert!(warning.contains("bugaboo.toml"), "{warning}");
        assert!(warning.contains("BUGABOO_DATABASE_URL"), "{warning}");
        assert!(warning.contains("command line"), "{warning}");
        // ... and never repeats the password or the address.
        assert!(!warning.contains("Hunter2") && !warning.contains("db.example"));
        let in_query = password_warning(
            &given("postgres://db.example.test/bugaboo?user=a&password=Hunter2-Secret"),
            "settings.toml",
        )
        .unwrap_or_default();
        assert!(in_query.contains("settings.toml") && !in_query.contains("Hunter2"));
        // No password, no warning: a user name alone, a SQLite file, no address, other options.
        for quiet in [
            "postgres://bugaboo@db.example.test/bugaboo",
            "postgres://bugaboo:@db.example.test/bugaboo",
            "postgresql://db.example.test/bugaboo?sslmode=require",
            "sqlite:data/bugaboo.db",
        ] {
            assert!(
                password_warning(&given(quiet), "bugaboo.toml").is_none(),
                "{quiet}"
            );
        }
        assert!(password_warning(&[(SettingName::Port, "1".to_owned())], "bugaboo.toml").is_none());
    }
}
