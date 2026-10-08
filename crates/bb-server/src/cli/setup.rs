//! `bugaboo setup` (OPS-03): asks a few questions in the console and saves the answers in the
//! settings file `bugaboo.toml`. Everything else is set up in the browser, with the setup code
//! that the first start prints (AUTH-09).
//!
//! The questions are the start-up settings of `config.rs`, by the same names: the answers are
//! checked by the same code that reads the file when the server starts, so the wizard can never
//! write a file that the server then refuses. With `--non-interactive` nothing is asked: the
//! options and the defaults are used (for scripts).

use std::io::{self, BufRead as _};
use std::net::{IpAddr, TcpListener};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, anyhow};
use bb_core::message::{self, Message};
use clap::Args;

use super::{program_folder, text};
use crate::config::{
    self, ConfigError, DEFAULT_ADDRESS, DEFAULT_DATA_DIR, DEFAULT_FILE, DEFAULT_TLS_MODE,
    SettingName, Sources, TlsModeName,
};
use crate::output;

/// The port the wizard suggests first (ADR-0023: Bugaboo has no default port; this is only a
/// suggestion that the person can change), and how many ports after it are tried when it is busy.
const SUGGESTED_PORT: u16 = 8443;
const SUGGESTED_PORT_TRIES: u16 = 100;

#[derive(Args)]
pub(crate) struct SetupArgs {
    #[arg(long, help = text(message::CLI_HELP_CONFIG))]
    config: Option<PathBuf>,
    #[arg(long, help = text(message::CLI_HELP_NON_INTERACTIVE))]
    non_interactive: bool,
    #[arg(long, help = text(message::CLI_HELP_OVERWRITE))]
    overwrite: bool,
    #[arg(long, help = text(message::CLI_HELP_DATA_DIR))]
    data_dir: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_ADDRESS))]
    address: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_PORT))]
    port: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_TLS))]
    tls: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_TLS_CERT))]
    tls_cert: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_TLS_KEY))]
    tls_key: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_PUBLIC_URL))]
    public_url: Option<String>,
    #[arg(long, help = text(message::CLI_HELP_DATABASE_URL))]
    database_url: Option<String>,
}

/// Where questions are asked and answers are read: the console, or a script in the tests.
pub(super) trait Terminal {
    /// Asks `question` and returns the answer, or the default when the answer is empty. `None`
    /// when the input has ended.
    fn ask(&mut self, question: &str, default: Option<&str>) -> Option<String>;
    /// Tells something.
    fn say(&mut self, text: &str);
}

/// The console of the person who runs the command.
struct Console;

impl Terminal for Console {
    fn ask(&mut self, question: &str, default: Option<&str>) -> Option<String> {
        output::line(question);
        output::prompt(&default.map_or_else(String::new, |default| format!("[{default}] ")));
        let mut line = String::new();
        match io::stdin().lock().read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => {
                let answer = line.trim();
                Some(if answer.is_empty() {
                    default.unwrap_or_default().to_owned()
                } else {
                    answer.to_owned()
                })
            }
        }
    }

    fn say(&mut self, text: &str) {
        output::line(text);
    }
}

/// What can be asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Question {
    DataDir,
    Address,
    Port,
    Tls,
    TlsCert,
    TlsKey,
    PublicUrl,
    Database,
}

/// The answers so far, by the name of the start-up setting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Answers {
    data_dir: Option<String>,
    address: Option<String>,
    port: Option<String>,
    tls_mode: Option<String>,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    public_url: Option<String>,
    database_url: Option<String>,
}

impl Answers {
    fn slot(&mut self, question: Question) -> &mut Option<String> {
        match question {
            Question::DataDir => &mut self.data_dir,
            Question::Address => &mut self.address,
            Question::Port => &mut self.port,
            Question::Tls => &mut self.tls_mode,
            Question::TlsCert => &mut self.tls_cert,
            Question::TlsKey => &mut self.tls_key,
            Question::PublicUrl => &mut self.public_url,
            Question::Database => &mut self.database_url,
        }
    }

    /// The answers as start-up settings, in the order they are written to the file.
    fn settings(&self) -> Vec<(SettingName, String)> {
        [
            (SettingName::DataDir, &self.data_dir),
            (SettingName::Address, &self.address),
            (SettingName::Port, &self.port),
            (SettingName::TlsMode, &self.tls_mode),
            (SettingName::TlsCert, &self.tls_cert),
            (SettingName::TlsKey, &self.tls_key),
            (SettingName::PublicUrl, &self.public_url),
            (SettingName::DatabaseUrl, &self.database_url),
        ]
        .into_iter()
        // An empty answer is an answer that is no setting: the built-in database.
        .filter_map(|(name, value)| value.clone().filter(|value| !value.is_empty()).map(|value| (name, value)))
        .collect()
    }

    fn address_is_local(&self) -> bool {
        self.address
            .as_deref()
            .and_then(|address| address.trim().parse::<IpAddr>().ok())
            .is_none_or(|address| address.to_canonical().is_loopback())
    }
}

impl SetupArgs {
    fn answers(&self) -> Answers {
        Answers {
            data_dir: self.data_dir.clone(),
            address: self.address.clone(),
            port: self.port.clone(),
            tls_mode: self.tls.clone(),
            tls_cert: self.tls_cert.clone(),
            tls_key: self.tls_key.clone(),
            public_url: self.public_url.clone(),
            database_url: self.database_url.clone(),
        }
    }
}

/// `bugaboo setup`.
pub(super) fn run(args: &SetupArgs) -> anyhow::Result<()> {
    let target = args
        .config
        .clone()
        .unwrap_or_else(|| program_folder().join(DEFAULT_FILE));
    execute(args, &mut Console, &target, &port_is_free)
}

/// Whether nothing listens on the port now.
fn port_is_free(address: IpAddr, port: u16) -> bool {
    TcpListener::bind((address, port)).is_ok()
}

fn execute(
    args: &SetupArgs,
    terminal: &mut dyn Terminal,
    target: &Path,
    port_is_free: &dyn Fn(IpAddr, u16) -> bool,
) -> anyhow::Result<()> {
    let interactive = !args.non_interactive;
    // Setup saves the address in the settings file, which is a safe place; the command line it
    // came from is not (M1-F62).
    if args
        .database_url
        .as_deref()
        .is_some_and(config::has_password)
    {
        terminal.say(
            &message::setup_database_url_password(
                target.display(),
                config::env_name(SettingName::DatabaseUrl),
            )
            .render(),
        );
    }
    if target.exists() && !args.overwrite {
        if !interactive {
            return Err(anyhow!(message::setup_exists(target.display()).render()));
        }
        let question = message::setup_ask_replace(target.display()).render();
        let yes = Message::from(message::SETUP_ANSWER_YES).render();
        let answer = terminal.ask(&question, None).ok_or_else(input_ended)?;
        if !answer.trim().eq_ignore_ascii_case(&yes) {
            terminal.say(&Message::from(message::SETUP_REPLACE_REFUSED).render());
            return Ok(());
        }
    }
    if interactive {
        terminal.say(&Message::from(message::SETUP_WELCOME).render());
    }
    let answers = collect(args.answers(), interactive, terminal, port_is_free)?;
    save(target, &answers.settings())?;
    terminal.say(&message::setup_saved(target.display()).render());
    terminal.say(&Message::from(message::SETUP_NEXT).render());
    terminal.say(&Message::from(message::SETUP_SERVICE_HINT).render());
    Ok(())
}

fn input_ended() -> anyhow::Error {
    anyhow!(Message::from(message::SETUP_INPUT_ENDED).render())
}

/// Asks what the options did not answer, until the answers make settings that the server accepts.
/// Without questions (scripts) the defaults fill in, and a mistake is an error.
fn collect(
    mut answers: Answers,
    interactive: bool,
    terminal: &mut dyn Terminal,
    port_is_free: &dyn Fn(IpAddr, u16) -> bool,
) -> anyhow::Result<Answers> {
    let order = [
        Question::DataDir,
        Question::Address,
        Question::Port,
        Question::Tls,
        Question::TlsCert,
        Question::TlsKey,
        Question::PublicUrl,
        Question::Database,
    ];
    loop {
        for question in order {
            if answers.slot(question).is_some() || !applies(question, &answers) {
                continue;
            }
            let given = answer(question, &answers, interactive, terminal, port_is_free)?;
            *answers.slot(question) = given;
        }
        let settings = answers.settings();
        let checked = config::resolve(&Sources {
            file: None,
            env: &|_| None,
            cli: &settings,
            base: &program_folder(),
        });
        let Err(error) = checked else {
            return Ok(answers);
        };
        if !interactive {
            return Err(error.into());
        }
        terminal.say(&message::setup_not_usable(error.message()).render());
        for question in questions_to_repeat(&error) {
            *answers.slot(*question) = None;
        }
    }
}

/// Whether a question makes sense given the answers before it.
fn applies(question: Question, answers: &Answers) -> bool {
    match question {
        Question::TlsCert | Question::TlsKey => answers
            .tls_mode
            .as_deref()
            .is_some_and(|mode| mode.trim().eq_ignore_ascii_case(TlsModeName::Files.code())),
        // A server that listens on the network cannot guess the address people type.
        Question::PublicUrl => !answers.address_is_local(),
        _ => true,
    }
}

/// The questions that a refused answer is asked again for.
fn questions_to_repeat(error: &ConfigError) -> &'static [Question] {
    match error {
        ConfigError::PortMissing | ConfigError::PortInvalid(_) => &[Question::Port],
        ConfigError::AddressInvalid(_) => &[Question::Address],
        ConfigError::DataDirEmpty => &[Question::DataDir],
        ConfigError::DatabaseInvalid => &[Question::Database],
        ConfigError::PublicUrlInvalid(_)
        | ConfigError::PublicUrlNeeded(_)
        | ConfigError::PublicUrlNeededBehindProxy => &[Question::PublicUrl],
        ConfigError::TlsModeInvalid(_)
        | ConfigError::TlsFilesMissing
        | ConfigError::TlsFilesUnused => &[Question::Tls, Question::TlsCert, Question::TlsKey],
        // Plain HTTP on a network address: either the address or the HTTPS mode must change.
        ConfigError::TlsOffPublic(_) => &[Question::Address, Question::Tls],
        _ => &[],
    }
}

/// The answer to one question: asked, or taken from the defaults when nobody is asked.
fn answer(
    question: Question,
    answers: &Answers,
    interactive: bool,
    terminal: &mut dyn Terminal,
    port_is_free: &dyn Fn(IpAddr, u16) -> bool,
) -> anyhow::Result<Option<String>> {
    let default = default_for(question, answers, port_is_free);
    if !interactive {
        return Ok(match (question, default) {
            // There is no default port; the settings check says so.
            (Question::Port | Question::PublicUrl | Question::TlsCert | Question::TlsKey, _)
            | (_, None) => None,
            (_, Some(default)) => Some(given(question, &default)),
        });
    }
    let words = match question {
        Question::DataDir => message::SETUP_ASK_DATA_DIR,
        Question::Address => message::SETUP_ASK_ADDRESS,
        Question::Port => message::SETUP_ASK_PORT,
        Question::Tls => message::SETUP_ASK_TLS,
        Question::TlsCert => message::SETUP_ASK_TLS_CERT,
        Question::TlsKey => message::SETUP_ASK_TLS_KEY,
        Question::PublicUrl => message::SETUP_ASK_PUBLIC_URL,
        Question::Database => message::SETUP_ASK_DATABASE,
    };
    let question_text = Message::from(words).render();
    loop {
        let reply = terminal
            .ask(&question_text, default.as_deref())
            .ok_or_else(input_ended)?;
        if question == Question::Port
            && let Some(busy) = busy_port(&reply, answers, port_is_free)
        {
            terminal.say(&message::server_port_busy(busy).render());
            continue;
        }
        if let Some(error) = malformed(question, &reply) {
            terminal.say(&message::setup_not_usable(error.message()).render());
            continue;
        }
        return Ok(Some(given(question, &reply)));
    }
}

/// Whether the reply is malformed by itself, whatever the other answers are. Mistakes that only
/// show in combination (plain HTTP on a network address) are found when all questions are
/// answered.
fn malformed(question: Question, reply: &str) -> Option<ConfigError> {
    let name = match question {
        Question::DataDir => SettingName::DataDir,
        Question::Address => SettingName::Address,
        Question::Port => SettingName::Port,
        Question::Tls => SettingName::TlsMode,
        Question::PublicUrl => SettingName::PublicUrl,
        Question::Database => SettingName::DatabaseUrl,
        Question::TlsCert | Question::TlsKey => return None,
    };
    let reply = given(question, reply);
    if reply.is_empty() {
        return None;
    }
    // Everything else the settings need is filled in with something that is valid.
    let mut settings = vec![(SettingName::Port, "1".to_owned())];
    settings.retain(|(setting, _)| *setting != name);
    settings.push((name, reply));
    let error = config::resolve(&Sources {
        file: None,
        env: &|_| None,
        cli: &settings,
        base: &program_folder(),
    })
    .err()?;
    matches!(
        error,
        ConfigError::AddressInvalid(_)
            | ConfigError::PortInvalid(_)
            | ConfigError::DataDirEmpty
            | ConfigError::DatabaseInvalid
            | ConfigError::TlsModeInvalid(_)
            | ConfigError::PublicUrlInvalid(_)
    )
    .then_some(error)
}

/// The reply as an answer. The built-in database is an empty answer, which is no setting; paths are made absolute,
/// because the file is read from other folders (a service starts in `System32` or `/`).
fn given(question: Question, reply: &str) -> String {
    let reply = reply.trim();
    match question {
        Question::Database if reply == Message::from(message::SETUP_DATABASE_BUILT_IN).render() => {
            String::new()
        }
        Question::DataDir | Question::TlsCert | Question::TlsKey if !reply.is_empty() => {
            absolute(reply)
        }
        _ => reply.to_owned(),
    }
}

fn absolute(path: &str) -> String {
    std::path::absolute(path)
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .into_owned()
}

/// The port the reply names, if something is listening on it already.
fn busy_port(
    reply: &str,
    answers: &Answers,
    port_is_free: &dyn Fn(IpAddr, u16) -> bool,
) -> Option<u16> {
    let port = reply.trim().parse::<u16>().ok()?;
    let address = answers
        .address
        .as_deref()
        .and_then(|address| address.trim().parse::<IpAddr>().ok())?;
    (!port_is_free(address, port)).then_some(port)
}

fn default_for(
    question: Question,
    answers: &Answers,
    port_is_free: &dyn Fn(IpAddr, u16) -> bool,
) -> Option<String> {
    match question {
        Question::DataDir => Some(
            program_folder()
                .join(DEFAULT_DATA_DIR)
                .to_string_lossy()
                .into_owned(),
        ),
        Question::Address => Some(DEFAULT_ADDRESS.to_owned()),
        Question::Port => suggested_port(answers, port_is_free).map(|port| port.to_string()),
        Question::Tls => Some(DEFAULT_TLS_MODE.code().to_owned()),
        Question::Database => Some(Message::from(message::SETUP_DATABASE_BUILT_IN).render()),
        Question::TlsCert | Question::TlsKey | Question::PublicUrl => None,
    }
}

/// The first free port from the suggested one on, on the address that was chosen.
fn suggested_port(answers: &Answers, port_is_free: &dyn Fn(IpAddr, u16) -> bool) -> Option<u16> {
    let address = answers
        .address
        .as_deref()
        .and_then(|address| address.trim().parse::<IpAddr>().ok())?;
    (SUGGESTED_PORT..SUGGESTED_PORT + SUGGESTED_PORT_TRIES)
        .find(|port| port_is_free(address, *port))
}

/// Writes the settings file: all at once (a new file next to it replaces the old one) and, where
/// the system has such a thing, readable by its owner only, because a database address can hold
/// a password.
fn save(path: &Path, settings: &[(SettingName, String)]) -> anyhow::Result<()> {
    let mut table = toml::Table::new();
    for (name, value) in settings {
        let entry = match (*name, value.parse::<i64>()) {
            (SettingName::Port, Ok(number)) => toml::Value::Integer(number),
            _ => toml::Value::String(value.trim().to_owned()),
        };
        table.insert(name.code().to_owned(), entry);
    }
    let contents = toml::to_string(&table).context("the settings could not be written")?;
    if let Some(folder) = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
    {
        std::fs::create_dir_all(folder).with_context(|| format!("{}", folder.display()))?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".new");
    let temporary = PathBuf::from(temporary);
    write_private(&temporary, contents.as_bytes())
        .and_then(|()| std::fs::rename(&temporary, path))
        .with_context(|| format!("{}", path.display()))
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)
}

/// On Windows the file takes the access list of its folder.
#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    std::fs::write(path, bytes)
}

#[cfg(test)]
#[path = "../tests/setup.rs"]
mod tests;
