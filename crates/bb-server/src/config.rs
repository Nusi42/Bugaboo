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
    DEFAULT_MAX_CONNECTIONS, DEFAULT_REQUESTS_PER_MINUTE, DEFAULT_SIGN_IN_ATTEMPTS_PER_MINUTE,
    DEFAULT_SIGN_IN_FAILURES_PER_MINUTE, LimitsConfig,
};

use bb_core::message::{self, Message};

/// Every start-up setting, once. The environment variable of `port` is `BUGABOO_PORT`.
const KEYS: [&str; 16] = [
    "address",
    "port",
    "data_dir",
    "database_url",
    "public_url",
    "tls_mode",
    "tls_cert",
    "tls_key",
    "tls_names",
    "trusted_proxies",
    "log_level",
    "max_connections",
    "header_timeout_seconds",
    "rate_limit_per_minute",
    "sign_in_attempts_per_minute",
    "sign_in_failures_per_minute",
];
/// Settings that name files or folders: relative paths in the settings file are relative to it.
const PATH_KEYS: [&str; 3] = ["data_dir", "tls_cert", "tls_key"];
const ENV_PREFIX: &str = "BUGABOO_";
/// The environment variable that names the settings file.
pub(crate) const FILE_ENV: &str = "BUGABOO_CONFIG";
/// Read from the current folder when it exists and no other file is named.
pub(crate) const DEFAULT_FILE: &str = "bugaboo.toml";

pub(crate) const DEFAULT_ADDRESS: &str = "127.0.0.1";
pub(crate) const DEFAULT_DATA_DIR: &str = "bugaboo-data";
pub(crate) const DEFAULT_TLS_MODE: &str = "self-signed";
const DEFAULT_LOG_LEVEL: &str = "info";
const DEFAULT_HEADER_TIMEOUT_SECONDS: u64 = 30;

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

/// How much the server writes to its log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
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
}

/// Where the settings come from.
pub(crate) struct Sources<'a> {
    /// The settings file: its path and its text.
    pub(crate) file: Option<(&'a Path, &'a str)>,
    /// Looks up an environment variable.
    pub(crate) env: &'a dyn Fn(&str) -> Option<String>,
    /// Values from the command line, by setting name.
    pub(crate) cli: &'a [(&'static str, String)],
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
    NumberInvalid { name: &'static str, value: String },
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
            Self::UnknownKey { path, name } => {
                message::config_file_unknown_key(KEYS.join(", "), name, shown(path)).render()
            }
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
                message::config_number_invalid(name, value).render()
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

/// The name of the environment variable for a setting.
pub(crate) fn env_name(key: &str) -> String {
    format!("{ENV_PREFIX}{}", key.to_ascii_uppercase())
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
    let mut values: BTreeMap<&'static str, String> = BTreeMap::new();
    if let Some((path, text)) = sources.file {
        values.extend(from_file(path, text)?);
    }
    for key in KEYS {
        if let Some(value) = (sources.env)(&env_name(key)).filter(|value| !value.is_empty()) {
            values.insert(key, value);
        }
    }
    for (key, value) in sources.cli {
        values.insert(key, value.clone());
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
            cli.push(("port", UNUSED_PORT.to_owned()));
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

fn from_file(path: &Path, text: &str) -> Result<BTreeMap<&'static str, String>, ConfigError> {
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
        let Some(&key) = KEYS.iter().find(|key| **key == name) else {
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
    text.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

fn settings_from(
    values: &BTreeMap<&'static str, String>,
    base: &Path,
) -> Result<Settings, ConfigError> {
    let get = |key: &str| values.get(key).map(String::as_str);
    let port_text = get("port").ok_or(ConfigError::PortMissing)?;
    let port = port_text
        .trim()
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| ConfigError::PortInvalid(port_text.to_owned()))?;

    let address_text = get("address").unwrap_or(DEFAULT_ADDRESS);
    let address = address_text
        .trim()
        .parse::<IpAddr>()
        .map_err(|_| ConfigError::AddressInvalid(address_text.to_owned()))?;

    let data_dir = match get("data_dir") {
        Some(given) if given.trim().is_empty() => return Err(ConfigError::DataDirEmpty),
        Some(given) => PathBuf::from(given),
        None => base.join(DEFAULT_DATA_DIR),
    };
    let max_connections = whole_number(values, "max_connections", DEFAULT_MAX_CONNECTIONS as u64)?;
    let header_seconds = whole_number(
        values,
        "header_timeout_seconds",
        DEFAULT_HEADER_TIMEOUT_SECONDS,
    )?;

    let limits = LimitsConfig {
        requests_per_minute: rate_limit(
            values,
            "rate_limit_per_minute",
            DEFAULT_REQUESTS_PER_MINUTE,
        )?,
        sign_in_attempts_per_minute: rate_limit(
            values,
            "sign_in_attempts_per_minute",
            DEFAULT_SIGN_IN_ATTEMPTS_PER_MINUTE,
        )?,
        sign_in_failures_per_minute: rate_limit(
            values,
            "sign_in_failures_per_minute",
            DEFAULT_SIGN_IN_FAILURES_PER_MINUTE,
        )?,
    };

    let database_url = get("database_url").map(str::to_owned);
    if let Some(url) = &database_url
        && !["sqlite:", "postgres://", "postgresql://"]
            .iter()
            .any(|scheme| url.starts_with(scheme))
    {
        return Err(ConfigError::DatabaseInvalid);
    }

    let trusted_proxies = list(get("trusted_proxies").unwrap_or_default())
        .into_iter()
        .map(|item| {
            item.parse::<IpAddr>()
                .map_err(|_| ConfigError::ProxyInvalid(item))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let tls = tls_from(
        get("tls_mode").unwrap_or(DEFAULT_TLS_MODE),
        get("tls_cert"),
        get("tls_key"),
    )?;
    // Secure by default (SEC-10): plain HTTP only on this computer, or when a proxy that we
    // trust adds the encryption.
    if tls == TlsMode::Off && !address.to_canonical().is_loopback() && trusted_proxies.is_empty() {
        return Err(ConfigError::TlsOffPublic(address));
    }

    let public_url = public_url_from(get("public_url"), address, port, &tls, &trusted_proxies)?;

    let log_text = get("log_level").unwrap_or(DEFAULT_LOG_LEVEL);
    let log_level = match log_text.trim().to_ascii_lowercase().as_str() {
        "error" => LogLevel::Error,
        "warn" => LogLevel::Warn,
        "info" => LogLevel::Info,
        "debug" => LogLevel::Debug,
        _ => return Err(ConfigError::LogLevelInvalid(log_text.to_owned())),
    };

    Ok(Settings {
        address,
        port,
        data_dir,
        database_url,
        public_url,
        tls,
        tls_names: list(get("tls_names").unwrap_or_default()),
        trusted_proxies,
        log_level,
        max_connections: usize::try_from(max_connections).unwrap_or(usize::MAX),
        header_timeout: Duration::from_secs(header_seconds),
        limits,
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
    values: &BTreeMap<&'static str, String>,
    name: &'static str,
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
    values: &BTreeMap<&'static str, String>,
    name: &'static str,
    default: u64,
) -> Result<u64, ConfigError> {
    let Some(text) = values.get(name) else {
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
    match mode.trim().to_ascii_lowercase().as_str() {
        "files" => match (cert, key) {
            (Some(cert), Some(key)) => Ok(TlsMode::Files {
                cert: PathBuf::from(cert),
                key: PathBuf::from(key),
            }),
            _ => Err(ConfigError::TlsFilesMissing),
        },
        other @ ("self-signed" | "off") => {
            if cert.is_some() || key.is_some() {
                return Err(ConfigError::TlsFilesUnused);
            }
            Ok(if other == "off" {
                TlsMode::Off
            } else {
                TlsMode::SelfSigned
            })
        }
        _ => Err(ConfigError::TlsModeInvalid(mode.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_env(_: &str) -> Option<String> {
        None
    }

    fn resolve_with(
        file: Option<&str>,
        env: &[(&str, &str)],
        cli: &[(&'static str, &str)],
    ) -> Result<Settings, ConfigError> {
        let lookup = |name: &str| {
            env.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        };
        let cli: Vec<(&'static str, String)> = cli
            .iter()
            .map(|(key, value)| (*key, (*value).to_owned()))
            .collect();
        resolve(&Sources {
            file: file.map(|text| (Path::new("conf/bugaboo.toml"), text)),
            env: &lookup,
            cli: &cli,
            base: Path::new("exe"),
        })
    }

    #[test]
    fn ops_10_only_a_port_is_needed_and_the_rest_is_safe_by_default() {
        let settings = resolve_with(None, &[], &[("port", "47200")]).unwrap();
        assert_eq!(settings.port, 47200);
        assert_eq!(settings.address, "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(settings.tls, TlsMode::SelfSigned);
        assert_eq!(settings.data_dir, Path::new("exe").join(DEFAULT_DATA_DIR));
        assert_eq!(settings.max_connections, DEFAULT_MAX_CONNECTIONS);
        assert_eq!(settings.header_timeout, Duration::from_secs(30));
        assert!(settings.trusted_proxies.is_empty());
        assert_eq!(settings.log_level, LogLevel::Info);
    }

    #[test]
    fn ops_10_there_is_no_default_port() {
        assert_eq!(
            resolve(&Sources {
                file: None,
                env: &no_env,
                cli: &[],
                base: Path::new("exe"),
            })
            .unwrap_err(),
            ConfigError::PortMissing
        );
    }

    #[test]
    fn ops_10_commands_that_do_not_listen_need_no_port_but_still_check_the_rest() {
        let sources = |cli: &'static [(&'static str, String)]| Sources {
            file: None,
            env: &no_env,
            cli,
            base: Path::new("exe"),
        };
        assert!(resolve_without_port(&sources(&[])).is_ok());
        // A port that is given is still checked.
        let wrong = [("port", "http".to_owned())];
        assert!(matches!(
            resolve_without_port(&Sources {
                file: None,
                env: &no_env,
                cli: &wrong,
                base: Path::new("exe"),
            }),
            Err(ConfigError::PortInvalid(_))
        ));
        // Other mistakes are still reported.
        let bad = [("database_url", "mysql://x".to_owned())];
        assert_eq!(
            resolve_without_port(&Sources {
                file: None,
                env: &no_env,
                cli: &bad,
                base: Path::new("exe"),
            })
            .unwrap_err(),
            ConfigError::DatabaseInvalid
        );
    }

    #[test]
    fn ops_10_command_line_beats_environment_beats_file() {
        let file = "port = 1001\naddress = \"::1\"\ndata_dir = \"from-file\"";
        let settings = resolve_with(
            Some(file),
            &[("BUGABOO_PORT", "1002"), ("BUGABOO_ADDRESS", "127.0.0.2")],
            &[("port", "1003")],
        )
        .unwrap();
        assert_eq!(settings.port, 1003);
        assert_eq!(settings.address, "127.0.0.2".parse::<IpAddr>().unwrap());
        // Relative paths in the file are relative to the file.
        assert_eq!(settings.data_dir, Path::new("conf").join("from-file"));
    }

    #[test]
    fn ops_10_lists_and_numbers_work_in_the_file_and_the_environment() {
        let file = "port = 8443\ntrusted_proxies = [\"10.0.0.5\", \"10.0.0.6\"]\ntls_names = [\"bugaboo.example.test\"]\ntls_mode = \"off\"\naddress = \"0.0.0.0\"
public_url = \"https://bugaboo.example.test\"";
        let settings = resolve_with(Some(file), &[], &[]).unwrap();
        assert_eq!(settings.trusted_proxies.len(), 2);
        assert_eq!(settings.tls_names, ["bugaboo.example.test"]);
        let from_env = resolve_with(
            None,
            &[
                ("BUGABOO_PORT", "1"),
                ("BUGABOO_TRUSTED_PROXIES", "10.0.0.1, ::1"),
                ("BUGABOO_PUBLIC_URL", "https://bugaboo.example.test"),
            ],
            &[],
        )
        .unwrap();
        assert_eq!(from_env.trusted_proxies.len(), 2);
    }

    #[test]
    fn sec_10_tls_off_is_refused_on_a_public_address() {
        for address in ["0.0.0.0", "192.168.1.20", "::", "2001:db8::1"] {
            let error = resolve_with(
                None,
                &[],
                &[("port", "47200"), ("tls_mode", "off"), ("address", address)],
            )
            .unwrap_err();
            assert!(matches!(error, ConfigError::TlsOffPublic(_)), "{address}");
            assert!(error.message().contains("cannot be switched off"));
        }
    }

    #[test]
    fn sec_10_tls_off_is_allowed_on_this_computer_or_behind_trusted_proxies() {
        for address in ["127.0.0.1", "::1", "127.0.0.2", "::ffff:127.0.0.1"] {
            let cli = [("port", "1"), ("tls_mode", "off"), ("address", address)];
            assert!(resolve_with(None, &[], &cli).is_ok(), "{address}");
        }
        let cli = [
            ("port", "1"),
            ("tls_mode", "off"),
            ("address", "0.0.0.0"),
            ("trusted_proxies", "10.0.0.5"),
            ("public_url", "https://bugaboo.example.test"),
        ];
        assert!(resolve_with(None, &[], &cli).is_ok());
    }

    #[test]
    fn auth_02_the_public_address_is_given_or_comes_from_this_computer() {
        let local = resolve_with(None, &[], &[("port", "47200")]).unwrap();
        assert_eq!(local.public_url, "https://localhost:47200");
        let plain = resolve_with(None, &[], &[("port", "47200"), ("tls_mode", "off")]).unwrap();
        assert_eq!(plain.public_url, "http://localhost:47200");
        let given = resolve_with(
            None,
            &[],
            &[
                ("port", "47200"),
                ("public_url", " https://Bugaboo.example.test:8443/ "),
            ],
        )
        .unwrap();
        assert_eq!(given.public_url, "https://bugaboo.example.test:8443");
    }

    #[test]
    fn auth_02_a_server_behind_a_proxy_must_be_told_its_public_address() {
        // Even on this computer: people reach it through the address of the proxy.
        let cli = [
            ("port", "1"),
            ("tls_mode", "off"),
            ("trusted_proxies", "10.0.0.5"),
        ];
        let error = resolve_with(None, &[], &cli).unwrap_err();
        assert!(matches!(error, ConfigError::PublicUrlNeededBehindProxy));
        assert!(error.message().contains("public_url"));
        let with = [
            ("port", "1"),
            ("tls_mode", "off"),
            ("trusted_proxies", "10.0.0.5"),
            ("public_url", "https://bugaboo.example.test"),
        ];
        let settings = resolve_with(None, &[], &with).unwrap();
        assert_eq!(settings.public_url, "https://bugaboo.example.test");
    }

    #[test]
    fn auth_02_a_server_on_the_network_must_be_told_its_public_address() {
        let error =
            resolve_with(None, &[], &[("port", "47200"), ("address", "192.168.1.20")]).unwrap_err();
        assert!(matches!(error, ConfigError::PublicUrlNeeded(_)));
        assert!(error.message().contains("public_url"));
        for bad in [
            "bugaboo.example.test",
            "https://example.test/app",
            "ftp://x.test",
        ] {
            let error = resolve_with(None, &[], &[("port", "1"), ("public_url", bad)]).unwrap_err();
            assert!(matches!(error, ConfigError::PublicUrlInvalid(_)), "{bad}");
            assert!(error.message().contains("web address"));
        }
    }

    #[test]
    fn ops_10_bad_values_get_plain_explanations() {
        let cases: [(&[(&'static str, &str)], &str); 8] = [
            (&[("port", "0")], "not a number from 1 to 65535"),
            (&[("port", "70000")], "not a number from 1 to 65535"),
            (&[("port", "http")], "not a number from 1 to 65535"),
            (
                &[("port", "1"), ("address", "localhost")],
                "not an IP address",
            ),
            (&[("port", "1"), ("tls_mode", "maybe")], "is not known"),
            (&[("port", "1"), ("tls_mode", "files")], "needs both"),
            (
                &[("port", "1"), ("tls_cert", "a.pem")],
                "only used with HTTPS mode files",
            ),
            (
                &[("port", "1"), ("database_url", "mysql://x")],
                "must start with",
            ),
        ];
        for (cli, expected) in cases {
            let error = resolve_with(None, &[], cli).unwrap_err();
            assert!(error.message().contains(expected), "{cli:?}: {error}");
        }
        for (key, value, expected) in [
            ("trusted_proxies", "proxy.example", "not an IP address"),
            ("log_level", "loud", "log level"),
            ("data_dir", " ", "empty"),
            ("max_connections", "0", "whole number"),
            ("header_timeout_seconds", "soon", "whole number"),
        ] {
            let error = resolve_with(None, &[], &[("port", "1"), (key, value)]).unwrap_err();
            assert!(error.message().contains(expected), "{key}: {error}");
        }
    }

    #[test]
    fn ops_10_the_settings_file_is_checked_too() {
        let unknown = resolve_with(Some("colour = \"red\""), &[], &[("port", "1")]).unwrap_err();
        assert!(unknown.message().contains("colour"));
        assert!(unknown.message().contains("tls_mode"));
        let broken = resolve_with(Some("port = ["), &[], &[]).unwrap_err();
        assert!(matches!(broken, ConfigError::FileInvalid { .. }));
        let wrong = resolve_with(Some("port = true"), &[], &[]).unwrap_err();
        assert!(matches!(wrong, ConfigError::WrongType { .. }));
    }

    #[test]
    fn ops_10_a_named_settings_file_must_exist() {
        let missing = read_file(
            Some(PathBuf::from("no/such/bugaboo.toml")),
            Path::new("exe"),
        )
        .unwrap_err();
        assert!(matches!(missing, ConfigError::FileUnreadable { .. }));
    }

    #[test]
    fn ops_10_limits_can_be_changed() {
        let cli = [
            ("port", "1"),
            ("max_connections", "50"),
            ("header_timeout_seconds", "5"),
        ];
        let settings = resolve_with(None, &[], &cli).unwrap();
        assert_eq!(settings.max_connections, 50);
        assert_eq!(settings.header_timeout, Duration::from_secs(5));
    }

    #[test]
    fn sec_14_rate_limits_have_defaults_and_can_be_changed() {
        let settings = resolve_with(None, &[], &[("port", "1")]).unwrap();
        assert_eq!(settings.limits, LimitsConfig::default());
        let cli = [
            ("port", "1"),
            ("rate_limit_per_minute", "50"),
            ("sign_in_attempts_per_minute", "3"),
        ];
        let settings = resolve_with(None, &[], &cli).unwrap();
        assert_eq!(settings.limits.requests_per_minute, 50);
        assert_eq!(settings.limits.sign_in_attempts_per_minute, 3);
        for bad in ["0", "many", "99999999999"] {
            let cli = [("port", "1"), ("rate_limit_per_minute", bad)];
            let error = resolve_with(None, &[], &cli).unwrap_err();
            assert!(matches!(error, ConfigError::NumberInvalid { .. }), "{bad}");
        }
    }

    #[test]
    fn ops_10_the_default_settings_file_is_next_to_the_program() {
        let missing = read_file(None, Path::new("no/such/folder")).unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn ops_10_environment_names_follow_the_setting_names() {
        assert_eq!(env_name("tls_mode"), "BUGABOO_TLS_MODE");
        assert_eq!(env_name("port"), "BUGABOO_PORT");
    }
}
