//! Start-up settings (OPS-10): the few things the server must know before it can open its
//! database. Everything else is a setting in the database, changed in the browser.
//!
//! Sources, strongest first: command line, environment variables (`BUGABOO_<NAME>`), the
//! settings file (`bugaboo.toml`), built-in defaults. Every value is checked before anything
//! starts, and every problem is explained in plain words (`messages/en.json`, `config.*`).

use std::collections::BTreeMap;
use std::fmt;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bb_api::{
    DEFAULT_ACCESS_KEY_REQUESTS_PER_MINUTE, DEFAULT_HEADER_TIMEOUT, DEFAULT_MAX_CONNECTIONS,
    DEFAULT_REQUESTS_PER_MINUTE, DEFAULT_SIGN_IN_ATTEMPTS_PER_MINUTE,
    DEFAULT_SIGN_IN_FAILURES_PER_MINUTE, LimitsConfig,
};

use bb_core::message::{self, Message};
use bb_core::notification::mail_batch;
use bb_core::settings::list_items;
use bb_core::words;

words! {
    /// Every start-up setting, once. The word is the name in the settings file and on the command
    /// line; the environment variable of `port` is `BUGABOO_PORT`.
    pub(crate) enum SettingName {
        Address => "address",
        Port => "port",
        DataDir => "data_dir",
        DatabaseUrl => "database_url",
        PublicUrl => "public_url",
        TlsMode => "tls_mode",
        TlsCert => "tls_cert",
        TlsKey => "tls_key",
        TlsNames => "tls_names",
        TrustedProxies => "trusted_proxies",
        LogLevel => "log_level",
        MaxConnections => "max_connections",
        HeaderTimeoutSeconds => "header_timeout_seconds",
        RateLimitPerMinute => "rate_limit_per_minute",
        SignInAttemptsPerMinute => "sign_in_attempts_per_minute",
        SignInFailuresPerMinute => "sign_in_failures_per_minute",
        AccessKeyRequestsPerMinute => "access_key_requests_per_minute",
        MailBatchSeconds => "mail_batch_seconds",
    }
}

words! {
    /// How the server gets its HTTPS certificate, as the setting `tls_mode` names it.
    pub(crate) enum TlsModeName {
        SelfSigned => "self-signed",
        Files => "files",
        Off => "off",
    }
}

/// Settings that name files or folders: relative paths in the settings file are relative to it.
const PATH_KEYS: [SettingName; 3] = [
    SettingName::DataDir,
    SettingName::TlsCert,
    SettingName::TlsKey,
];
const ENV_PREFIX: &str = "BUGABOO_";
/// The environment variable that names the settings file.
pub(crate) const FILE_ENV: &str = "BUGABOO_CONFIG";
/// Read from the current folder when it exists and no other file is named.
pub(crate) const DEFAULT_FILE: &str = "bugaboo.toml";

pub(crate) const DEFAULT_ADDRESS: &str = "127.0.0.1";
pub(crate) const DEFAULT_DATA_DIR: &str = "bugaboo-data";
pub(crate) const DEFAULT_TLS_MODE: TlsModeName = TlsModeName::SelfSigned;
const DEFAULT_LOG_LEVEL: LogLevel = LogLevel::Info;

/// How the server gets its HTTPS certificate (SEC-10).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TlsMode {
    /// A certificate made by the server, kept in the data folder.
    SelfSigned,
    /// A certificate and key the owner provides.
    Files { cert: PathBuf, key: PathBuf },
    /// Plain HTTP: only on this computer or behind trusted proxies.
    Off,
}

words! {
    /// How much the server writes to its log, as the setting `log_level` names it.
    pub(crate) enum LogLevel {
        Error => "error",
        Warn => "warn",
        Info => "info",
        Debug => "debug",
    }
}

/// The checked start-up settings.
#[derive(Debug)]
pub(crate) struct Settings {
    pub(crate) address: IpAddr,
    pub(crate) port: u16,
    pub(crate) data_dir: PathBuf,
    /// Never written to a log: it may contain a password.
    pub(crate) database_url: Option<String>,
    /// The origin people reach the server at: passkeys are tied to its host name (AUTH-02).
    pub(crate) public_url: String,
    pub(crate) tls: TlsMode,
    pub(crate) tls_names: Vec<String>,
    pub(crate) trusted_proxies: Vec<IpAddr>,
    pub(crate) log_level: LogLevel,
    pub(crate) max_connections: usize,
    pub(crate) header_timeout: Duration,
    pub(crate) limits: LimitsConfig,
    /// How long changes to one issue are collected into one e-mail (M4-Q6).
    pub(crate) mail_batch: jiff::SignedDuration,
}

/// Where the settings come from.
pub(crate) struct Sources<'a> {
    /// The settings file: its path and its text.
    pub(crate) file: Option<(&'a Path, &'a str)>,
    /// Looks up an environment variable.
    pub(crate) env: &'a dyn Fn(&str) -> Option<String>,
    /// Values from the command line, by setting name.
    pub(crate) cli: &'a [(SettingName, String)],
    /// The folder of the program. Defaults for the data folder and the settings file are
    /// relative to it, not to the current folder (a Windows service starts in System32).
    pub(crate) base: &'a Path,
}

/// What is wrong with the start-up settings.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConfigError {
    FileUnreadable { path: PathBuf, reason: String },
    FileInvalid { path: PathBuf, reason: String },
    UnknownKey { path: PathBuf, name: String },
    WrongType { path: PathBuf, name: String },
    PortMissing,
    PortInvalid(String),
    AddressInvalid(String),
    DataDirEmpty,
    DatabaseInvalid,
    PublicUrlInvalid(String),
    PublicUrlNeeded(IpAddr),
    PublicUrlNeededBehindProxy,
    TlsModeInvalid(String),
    TlsFilesMissing,
    TlsFilesUnused,
    TlsOffPublic(IpAddr),
    ProxyInvalid(String),
    LogLevelInvalid(String),
    NumberInvalid { name: SettingName, value: String },
}

impl ConfigError {
    /// The explanation, in plain words.
    pub(crate) fn message(&self) -> String {
        let shown = |path: &Path| path.display().to_string();
        match self {
            Self::FileUnreadable { path, reason } => {
                message::config_file_unreadable(shown(path), reason).render()
            }
            Self::FileInvalid { path, reason } => {
                message::config_file_invalid(shown(path), reason).render()
            }
            Self::UnknownKey { path, name } => message::config_file_unknown_key(
                SettingName::ALL
                    .iter()
                    .map(|key| key.code())
                    .collect::<Vec<_>>()
                    .join(", "),
                name,
                shown(path),
            )
            .render(),
            Self::WrongType { path, name } => {
                message::config_file_wrong_type(name, shown(path)).render()
            }
            Self::PortMissing => Message::from(message::CONFIG_PORT_MISSING).render(),
            Self::PortInvalid(value) => message::config_port_invalid(value).render(),
            Self::AddressInvalid(value) => message::config_address_invalid(value).render(),
            Self::DataDirEmpty => Message::from(message::CONFIG_DATA_DIR_EMPTY).render(),
            Self::DatabaseInvalid => Message::from(message::CONFIG_DATABASE_INVALID).render(),
            Self::PublicUrlInvalid(value) => message::config_public_url_invalid(value).render(),
            Self::PublicUrlNeeded(address) => message::config_public_url_needed(address).render(),
            Self::PublicUrlNeededBehindProxy => {
                Message::from(message::CONFIG_PUBLIC_URL_PROXY).render()
            }
            Self::TlsModeInvalid(value) => message::config_tls_mode_invalid(value).render(),
            Self::TlsFilesMissing => Message::from(message::CONFIG_TLS_FILES_MISSING).render(),
            Self::TlsFilesUnused => Message::from(message::CONFIG_TLS_FILES_UNUSED).render(),
            Self::TlsOffPublic(address) => message::config_tls_off_public(address).render(),
            Self::ProxyInvalid(value) => message::config_proxy_invalid(value).render(),
            Self::LogLevelInvalid(value) => message::config_log_level_invalid(value).render(),
            Self::NumberInvalid { name, value } => {
                message::config_number_invalid(name.code(), value).render()
            }
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for ConfigError {}

/// Whether a database address holds a password: `postgres://user:password@host/db`, or a
/// `password=` in its query (`?password=…`). Only the form is looked at; the password itself is
/// never copied anywhere.
pub(crate) fn has_password(url: &str) -> bool {
    let Some((_, rest)) = url.split_once("://") else {
        return false;
    };
    let (before_query, query) = rest.split_once('?').unwrap_or((rest, ""));
    let authority = before_query.split('/').next().unwrap_or_default();
    let in_authority = authority
        .rsplit_once('@')
        .and_then(|(user_info, _)| user_info.split_once(':'))
        .is_some_and(|(_, password)| !password.is_empty());
    let in_query = query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .any(|(name, value)| name.eq_ignore_ascii_case("password") && !value.is_empty());
    in_authority || in_query
}

/// The name of the environment variable for a setting.
pub(crate) fn env_name(key: SettingName) -> String {
    format!("{ENV_PREFIX}{}", key.code().to_ascii_uppercase())
}

/// Reads the settings file text, if there is one: the named file must exist, the default file
/// is optional.
pub(crate) fn read_file(
    named: Option<PathBuf>,
    base: &Path,
) -> Result<Option<(PathBuf, String)>, ConfigError> {
    let (path, required) = match named {
        Some(path) => (path, true),
        None => (base.join(DEFAULT_FILE), false),
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some((path, text))),
        Err(error) if !required && error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ConfigError::FileUnreadable {
            path,
            reason: error.to_string(),
        }),
    }
}

/// Combines the sources and checks the result.
pub(crate) fn resolve(sources: &Sources<'_>) -> Result<Settings, ConfigError> {
    let mut values: BTreeMap<SettingName, String> = BTreeMap::new();
    if let Some((path, text)) = sources.file {
        values.extend(from_file(path, text)?);
    }
    for key in SettingName::ALL.iter().copied() {
        if let Some(value) = (sources.env)(&env_name(key)).filter(|value| !value.is_empty()) {
            values.insert(key, value);
        }
    }
    for (key, value) in sources.cli {
        values.insert(*key, value.clone());
    }
    settings_from(&values, sources.base)
}

/// The port that [`resolve_without_port`] uses when no source names one. Nothing listens on it.
const UNUSED_PORT: &str = "1";

/// Like [`resolve`], for commands that do not listen (`security-log verify`, `rotate-key`): they
/// need no port, so a missing one is not an error.
pub(crate) fn resolve_without_port(sources: &Sources<'_>) -> Result<Settings, ConfigError> {
    match resolve(sources) {
        Err(ConfigError::PortMissing) => {
            let mut cli = sources.cli.to_vec();
            cli.push((SettingName::Port, UNUSED_PORT.to_owned()));
            resolve(&Sources {
                file: sources.file,
                env: sources.env,
                cli: &cli,
                base: sources.base,
            })
        }
        other => other,
    }
}

fn from_file(path: &Path, text: &str) -> Result<BTreeMap<SettingName, String>, ConfigError> {
    let table: toml::Table =
        text.parse()
            .map_err(|error: toml::de::Error| ConfigError::FileInvalid {
                path: path.to_path_buf(),
                reason: error.message().to_owned(),
            })?;
    let folder = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty());
    let mut values = BTreeMap::new();
    for (name, value) in &table {
        let Some(key) = SettingName::from_code(name) else {
            return Err(ConfigError::UnknownKey {
                path: path.to_path_buf(),
                name: name.clone(),
            });
        };
        let wrong_type = || ConfigError::WrongType {
            path: path.to_path_buf(),
            name: name.clone(),
        };
        let mut text = scalar(value).ok_or_else(wrong_type)?;
        if let (true, Some(folder)) = (PATH_KEYS.contains(&key), folder)
            && Path::new(&text).is_relative()
        {
            text = folder.join(&text).to_string_lossy().into_owned();
        }
        values.insert(key, text);
    }
    Ok(values)
}

/// Text and numbers as they are; a list of them joined by commas (same form as in the
/// environment).
fn scalar(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::String(text) => Some(text.clone()),
        toml::Value::Integer(number) => Some(number.to_string()),
        toml::Value::Array(items) => items
            .iter()
            .map(|item| match item {
                toml::Value::String(_) | toml::Value::Integer(_) => scalar(item),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(|items| items.join(",")),
        _ => None,
    }
}

fn list(text: &str) -> Vec<String> {
    list_items(text).map(str::to_owned).collect()
}

fn settings_from(
    values: &BTreeMap<SettingName, String>,
    base: &Path,
) -> Result<Settings, ConfigError> {
    let get = |key: SettingName| values.get(&key).map(String::as_str);
    let port_text = get(SettingName::Port).ok_or(ConfigError::PortMissing)?;
    let port = port_text
        .trim()
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| ConfigError::PortInvalid(port_text.to_owned()))?;

    let address_text = get(SettingName::Address).unwrap_or(DEFAULT_ADDRESS);
    let address = address_text
        .trim()
        .parse::<IpAddr>()
        .map_err(|_| ConfigError::AddressInvalid(address_text.to_owned()))?;

    let data_dir = match get(SettingName::DataDir) {
        Some(given) if given.trim().is_empty() => return Err(ConfigError::DataDirEmpty),
        Some(given) => PathBuf::from(given),
        None => base.join(DEFAULT_DATA_DIR),
    };
    let max_connections = whole_number(
        values,
        SettingName::MaxConnections,
        DEFAULT_MAX_CONNECTIONS as u64,
    )?;
    let header_seconds = whole_number(
        values,
        SettingName::HeaderTimeoutSeconds,
        DEFAULT_HEADER_TIMEOUT.as_secs(),
    )?;

    let limits = LimitsConfig {
        requests_per_minute: rate_limit(
            values,
            SettingName::RateLimitPerMinute,
            DEFAULT_REQUESTS_PER_MINUTE,
        )?,
        sign_in_attempts_per_minute: rate_limit(
            values,
            SettingName::SignInAttemptsPerMinute,
            DEFAULT_SIGN_IN_ATTEMPTS_PER_MINUTE,
        )?,
        sign_in_failures_per_minute: rate_limit(
            values,
            SettingName::SignInFailuresPerMinute,
            DEFAULT_SIGN_IN_FAILURES_PER_MINUTE,
        )?,
        access_key_requests_per_minute: rate_limit(
            values,
            SettingName::AccessKeyRequestsPerMinute,
            DEFAULT_ACCESS_KEY_REQUESTS_PER_MINUTE,
        )?,
    };

    let mail_batch_seconds = whole_number(
        values,
        SettingName::MailBatchSeconds,
        mail_batch().as_secs().unsigned_abs(),
    )?;
    let mail_batch = i64::try_from(mail_batch_seconds)
        .map(jiff::SignedDuration::from_secs)
        .map_err(|_| ConfigError::NumberInvalid {
            name: SettingName::MailBatchSeconds,
            value: mail_batch_seconds.to_string(),
        })?;

    let database_url = get(SettingName::DatabaseUrl).map(str::to_owned);
    if let Some(url) = &database_url
        && !["sqlite:", "postgres://", "postgresql://"]
            .iter()
            .any(|scheme| url.starts_with(scheme))
    {
        return Err(ConfigError::DatabaseInvalid);
    }

    let trusted_proxies = list(get(SettingName::TrustedProxies).unwrap_or_default())
        .into_iter()
        .map(|item| {
            item.parse::<IpAddr>()
                .map_err(|_| ConfigError::ProxyInvalid(item))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let tls = tls_from(
        get(SettingName::TlsMode).unwrap_or(DEFAULT_TLS_MODE.code()),
        get(SettingName::TlsCert),
        get(SettingName::TlsKey),
    )?;
    // Secure by default (SEC-10): plain HTTP only on this computer, or when a proxy that we
    // trust adds the encryption.
    if tls == TlsMode::Off && !address.to_canonical().is_loopback() && trusted_proxies.is_empty() {
        return Err(ConfigError::TlsOffPublic(address));
    }

    let public_url = public_url_from(
        get(SettingName::PublicUrl),
        address,
        port,
        &tls,
        &trusted_proxies,
    )?;

    let log_level = match get(SettingName::LogLevel) {
        None => DEFAULT_LOG_LEVEL,
        Some(text) => LogLevel::from_code(&text.trim().to_ascii_lowercase())
            .ok_or_else(|| ConfigError::LogLevelInvalid(text.to_owned()))?,
    };

    Ok(Settings {
        address,
        port,
        data_dir,
        database_url,
        public_url,
        tls,
        tls_names: list(get(SettingName::TlsNames).unwrap_or_default()),
        trusted_proxies,
        log_level,
        max_connections: usize::try_from(max_connections).unwrap_or(usize::MAX),
        header_timeout: Duration::from_secs(header_seconds),
        limits,
        mail_batch,
    })
}

/// The origin people reach the server at. Given as `public_url`, or, when the server only
/// listens on this computer, `localhost` with the port. A server that listens on a network
/// address cannot guess what name people use, so it must be told.
fn public_url_from(
    given: Option<&str>,
    address: IpAddr,
    port: u16,
    tls: &TlsMode,
    trusted_proxies: &[IpAddr],
) -> Result<String, ConfigError> {
    if let Some(text) = given {
        return bb_auth::public_origin(text)
            .ok_or_else(|| ConfigError::PublicUrlInvalid(text.to_owned()));
    }
    // People reach a server behind a proxy through the proxy's address, whatever it listens on.
    if !trusted_proxies.is_empty() {
        return Err(ConfigError::PublicUrlNeededBehindProxy);
    }
    if !address.to_canonical().is_loopback() {
        return Err(ConfigError::PublicUrlNeeded(address));
    }
    let scheme = if *tls == TlsMode::Off {
        "http"
    } else {
        "https"
    };
    Ok(format!("{scheme}://localhost:{port}"))
}

/// A rate limit: a whole number of at least 1, or `default` when the setting is not given.
fn rate_limit(
    values: &BTreeMap<SettingName, String>,
    name: SettingName,
    default: u32,
) -> Result<u32, ConfigError> {
    let number = whole_number(values, name, u64::from(default))?;
    u32::try_from(number).map_err(|_| ConfigError::NumberInvalid {
        name,
        value: number.to_string(),
    })
}

/// A whole number of at least 1, or `default` when the setting is not given.
fn whole_number(
    values: &BTreeMap<SettingName, String>,
    name: SettingName,
    default: u64,
) -> Result<u64, ConfigError> {
    let Some(text) = values.get(&name) else {
        return Ok(default);
    };
    text.trim()
        .parse::<u64>()
        .ok()
        .filter(|number| *number >= 1)
        .ok_or_else(|| ConfigError::NumberInvalid {
            name,
            value: text.clone(),
        })
}

fn tls_from(mode: &str, cert: Option<&str>, key: Option<&str>) -> Result<TlsMode, ConfigError> {
    let Some(name) = TlsModeName::from_code(&mode.trim().to_ascii_lowercase()) else {
        return Err(ConfigError::TlsModeInvalid(mode.to_owned()));
    };
    match name {
        TlsModeName::Files => match (cert, key) {
            (Some(cert), Some(key)) => Ok(TlsMode::Files {
                cert: PathBuf::from(cert),
                key: PathBuf::from(key),
            }),
            _ => Err(ConfigError::TlsFilesMissing),
        },
        TlsModeName::SelfSigned | TlsModeName::Off => {
            if cert.is_some() || key.is_some() {
                return Err(ConfigError::TlsFilesUnused);
            }
            Ok(if name == TlsModeName::Off {
                TlsMode::Off
            } else {
                TlsMode::SelfSigned
            })
        }
    }
}

#[cfg(test)]
mod tests;
