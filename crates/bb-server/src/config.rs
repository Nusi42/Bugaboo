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

use crate::text::t;

/// Every start-up setting, once. The environment variable of `port` is `BUGABOO_PORT`.
const KEYS: [&str; 10] = [
    "address",
    "port",
    "data_dir",
    "database_url",
    "tls_mode",
    "tls_cert",
    "tls_key",
    "tls_names",
    "trusted_proxies",
    "log_level",
];
/// Settings that name files or folders: relative paths in the settings file are relative to it.
const PATH_KEYS: [&str; 3] = ["data_dir", "tls_cert", "tls_key"];
const ENV_PREFIX: &str = "BUGABOO_";
/// The environment variable that names the settings file.
pub(crate) const FILE_ENV: &str = "BUGABOO_CONFIG";
/// Read from the current folder when it exists and no other file is named.
pub(crate) const DEFAULT_FILE: &str = "bugaboo.toml";

const DEFAULT_ADDRESS: &str = "127.0.0.1";
const DEFAULT_DATA_DIR: &str = "bugaboo-data";
const DEFAULT_TLS_MODE: &str = "self-signed";
const DEFAULT_LOG_LEVEL: &str = "info";

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
    pub(crate) tls: TlsMode,
    pub(crate) tls_names: Vec<String>,
    pub(crate) trusted_proxies: Vec<IpAddr>,
    pub(crate) log_level: LogLevel,
}

/// Where the settings come from.
pub(crate) struct Sources<'a> {
    /// The settings file: its path and its text.
    pub(crate) file: Option<(&'a Path, &'a str)>,
    /// Looks up an environment variable.
    pub(crate) env: &'a dyn Fn(&str) -> Option<String>,
    /// Values from the command line, by setting name.
    pub(crate) cli: &'a [(&'static str, String)],
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
    TlsModeInvalid(String),
    TlsFilesMissing,
    TlsFilesUnused,
    TlsOffPublic(IpAddr),
    ProxyInvalid(String),
    LogLevelInvalid(String),
}

impl ConfigError {
    /// The explanation, in plain words.
    pub(crate) fn message(&self) -> String {
        let shown = |path: &Path| path.display().to_string();
        match self {
            Self::FileUnreadable { path, reason } => t(
                "config.file-unreadable",
                &[("path", &shown(path)), ("reason", reason)],
            ),
            Self::FileInvalid { path, reason } => t(
                "config.file-invalid",
                &[("path", &shown(path)), ("reason", reason)],
            ),
            Self::UnknownKey { path, name } => t(
                "config.file-unknown-key",
                &[
                    ("path", &shown(path)),
                    ("name", name),
                    ("known", &KEYS.join(", ")),
                ],
            ),
            Self::WrongType { path, name } => t(
                "config.file-wrong-type",
                &[("path", &shown(path)), ("name", name)],
            ),
            Self::PortMissing => t("config.port-missing", &[]),
            Self::PortInvalid(value) => t("config.port-invalid", &[("value", value)]),
            Self::AddressInvalid(value) => t("config.address-invalid", &[("value", value)]),
            Self::DataDirEmpty => t("config.data-dir-empty", &[]),
            Self::DatabaseInvalid => t("config.database-invalid", &[]),
            Self::TlsModeInvalid(value) => t("config.tls-mode-invalid", &[("value", value)]),
            Self::TlsFilesMissing => t("config.tls-files-missing", &[]),
            Self::TlsFilesUnused => t("config.tls-files-unused", &[]),
            Self::TlsOffPublic(address) => t(
                "config.tls-off-public",
                &[("address", &address.to_string())],
            ),
            Self::ProxyInvalid(value) => t("config.proxy-invalid", &[("value", value)]),
            Self::LogLevelInvalid(value) => t("config.log-level-invalid", &[("value", value)]),
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
pub(crate) fn read_file(named: Option<PathBuf>) -> Result<Option<(PathBuf, String)>, ConfigError> {
    let (path, required) = match named {
        Some(path) => (path, true),
        None => (PathBuf::from(DEFAULT_FILE), false),
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
    settings_from(&values)
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

fn settings_from(values: &BTreeMap<&'static str, String>) -> Result<Settings, ConfigError> {
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

    let data_dir = get("data_dir").unwrap_or(DEFAULT_DATA_DIR);
    if data_dir.trim().is_empty() {
        return Err(ConfigError::DataDirEmpty);
    }

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
        data_dir: PathBuf::from(data_dir),
        database_url,
        tls,
        tls_names: list(get("tls_names").unwrap_or_default()),
        trusted_proxies,
        log_level,
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
        })
    }

    #[test]
    fn ops_10_only_a_port_is_needed_and_the_rest_is_safe_by_default() {
        let settings = resolve_with(None, &[], &[("port", "47200")]).unwrap();
        assert_eq!(settings.port, 47200);
        assert_eq!(settings.address, "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(settings.tls, TlsMode::SelfSigned);
        assert_eq!(settings.data_dir, PathBuf::from(DEFAULT_DATA_DIR));
        assert!(settings.trusted_proxies.is_empty());
        assert_eq!(settings.log_level, LogLevel::Info);
    }

    #[test]
    fn ops_10_there_is_no_default_port() {
        assert_eq!(
            resolve(&Sources {
                file: None,
                env: &no_env,
                cli: &[]
            })
            .unwrap_err(),
            ConfigError::PortMissing
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
        let file = "port = 8443\ntrusted_proxies = [\"10.0.0.5\", \"10.0.0.6\"]\ntls_names = [\"bugaboo.example.test\"]\ntls_mode = \"off\"\naddress = \"0.0.0.0\"";
        let settings = resolve_with(Some(file), &[], &[]).unwrap();
        assert_eq!(settings.trusted_proxies.len(), 2);
        assert_eq!(settings.tls_names, ["bugaboo.example.test"]);
        let from_env = resolve_with(
            None,
            &[
                ("BUGABOO_PORT", "1"),
                ("BUGABOO_TRUSTED_PROXIES", "10.0.0.1, ::1"),
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
        ];
        assert!(resolve_with(None, &[], &cli).is_ok());
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
        let missing = read_file(Some(PathBuf::from("no/such/bugaboo.toml"))).unwrap_err();
        assert!(matches!(missing, ConfigError::FileUnreadable { .. }));
    }

    #[test]
    fn ops_10_environment_names_follow_the_setting_names() {
        assert_eq!(env_name("tls_mode"), "BUGABOO_TLS_MODE");
        assert_eq!(env_name("port"), "BUGABOO_PORT");
    }
}
