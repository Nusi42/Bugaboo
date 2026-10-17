//! The tests of the settings of the server.

use super::*;

fn no_env(_: &str) -> Option<String> {
    None
}

fn resolve_with(
    file: Option<&str>,
    env: &[(&str, &str)],
    cli: &[(SettingName, &str)],
) -> Result<Settings, ConfigError> {
    let lookup = |name: &str| {
        env.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| (*value).to_owned())
    };
    let cli: Vec<(SettingName, String)> = cli
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
    let settings = resolve_with(None, &[], &[(SettingName::Port, "47200")]).unwrap();
    assert_eq!(settings.port, 47200);
    assert_eq!(settings.address, "127.0.0.1".parse::<IpAddr>().unwrap());
    assert_eq!(settings.tls, TlsMode::SelfSigned);
    assert_eq!(settings.data_dir, Path::new("exe").join(DEFAULT_DATA_DIR));
    assert_eq!(settings.max_connections, DEFAULT_MAX_CONNECTIONS);
    assert_eq!(settings.header_timeout, DEFAULT_HEADER_TIMEOUT);
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
    let sources = |cli: &'static [(SettingName, String)]| Sources {
        file: None,
        env: &no_env,
        cli,
        base: Path::new("exe"),
    };
    assert!(resolve_without_port(&sources(&[])).is_ok());
    // A port that is given is still checked.
    let wrong = [(SettingName::Port, "http".to_owned())];
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
    let bad = [(SettingName::DatabaseUrl, "mysql://x".to_owned())];
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
        &[(SettingName::Port, "1003")],
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
            &[
                (SettingName::Port, "47200"),
                (SettingName::TlsMode, "off"),
                (SettingName::Address, address),
            ],
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::TlsOffPublic(_)), "{address}");
        assert!(error.message().contains("cannot be switched off"));
    }
}

#[test]
fn sec_10_tls_off_is_allowed_on_this_computer_or_behind_trusted_proxies() {
    for address in ["127.0.0.1", "::1", "127.0.0.2", "::ffff:127.0.0.1"] {
        let cli = [
            (SettingName::Port, "1"),
            (SettingName::TlsMode, "off"),
            (SettingName::Address, address),
        ];
        assert!(resolve_with(None, &[], &cli).is_ok(), "{address}");
    }
    let cli = [
        (SettingName::Port, "1"),
        (SettingName::TlsMode, "off"),
        (SettingName::Address, "0.0.0.0"),
        (SettingName::TrustedProxies, "10.0.0.5"),
        (SettingName::PublicUrl, "https://bugaboo.example.test"),
    ];
    assert!(resolve_with(None, &[], &cli).is_ok());
}

#[test]
fn auth_02_the_public_address_is_given_or_comes_from_this_computer() {
    let local = resolve_with(None, &[], &[(SettingName::Port, "47200")]).unwrap();
    assert_eq!(local.public_url, "https://localhost:47200");
    let plain = resolve_with(
        None,
        &[],
        &[(SettingName::Port, "47200"), (SettingName::TlsMode, "off")],
    )
    .unwrap();
    assert_eq!(plain.public_url, "http://localhost:47200");
    let given = resolve_with(
        None,
        &[],
        &[
            (SettingName::Port, "47200"),
            (
                SettingName::PublicUrl,
                " https://Bugaboo.example.test:8443/ ",
            ),
        ],
    )
    .unwrap();
    assert_eq!(given.public_url, "https://bugaboo.example.test:8443");
}

#[test]
fn auth_02_a_server_behind_a_proxy_must_be_told_its_public_address() {
    // Even on this computer: people reach it through the address of the proxy.
    let cli = [
        (SettingName::Port, "1"),
        (SettingName::TlsMode, "off"),
        (SettingName::TrustedProxies, "10.0.0.5"),
    ];
    let error = resolve_with(None, &[], &cli).unwrap_err();
    assert!(matches!(error, ConfigError::PublicUrlNeededBehindProxy));
    assert!(error.message().contains("public_url"));
    let with = [
        (SettingName::Port, "1"),
        (SettingName::TlsMode, "off"),
        (SettingName::TrustedProxies, "10.0.0.5"),
        (SettingName::PublicUrl, "https://bugaboo.example.test"),
    ];
    let settings = resolve_with(None, &[], &with).unwrap();
    assert_eq!(settings.public_url, "https://bugaboo.example.test");
}

#[test]
fn auth_02_a_server_on_the_network_must_be_told_its_public_address() {
    let error = resolve_with(
        None,
        &[],
        &[
            (SettingName::Port, "47200"),
            (SettingName::Address, "192.168.1.20"),
        ],
    )
    .unwrap_err();
    assert!(matches!(error, ConfigError::PublicUrlNeeded(_)));
    assert!(error.message().contains("public_url"));
    for bad in [
        "bugaboo.example.test",
        "https://example.test/app",
        "ftp://x.test",
    ] {
        let error = resolve_with(
            None,
            &[],
            &[(SettingName::Port, "1"), (SettingName::PublicUrl, bad)],
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::PublicUrlInvalid(_)), "{bad}");
        assert!(error.message().contains("web address"));
    }
}

#[test]
fn ops_10_bad_values_get_plain_explanations() {
    let cases: [(&[(SettingName, &str)], &str); 8] = [
        (&[(SettingName::Port, "0")], "not a number from 1 to 65535"),
        (
            &[(SettingName::Port, "70000")],
            "not a number from 1 to 65535",
        ),
        (
            &[(SettingName::Port, "http")],
            "not a number from 1 to 65535",
        ),
        (
            &[
                (SettingName::Port, "1"),
                (SettingName::Address, "localhost"),
            ],
            "not an IP address",
        ),
        (
            &[(SettingName::Port, "1"), (SettingName::TlsMode, "maybe")],
            "is not known",
        ),
        (
            &[(SettingName::Port, "1"), (SettingName::TlsMode, "files")],
            "needs both",
        ),
        (
            &[(SettingName::Port, "1"), (SettingName::TlsCert, "a.pem")],
            "only used with HTTPS mode files",
        ),
        (
            &[
                (SettingName::Port, "1"),
                (SettingName::DatabaseUrl, "mysql://x"),
            ],
            "must start with",
        ),
    ];
    for (cli, expected) in cases {
        let error = resolve_with(None, &[], cli).unwrap_err();
        assert!(error.message().contains(expected), "{cli:?}: {error}");
    }
    for (key, value, expected) in [
        (
            SettingName::TrustedProxies,
            "proxy.example",
            "not an IP address",
        ),
        (SettingName::LogLevel, "loud", "log level"),
        (SettingName::DataDir, " ", "empty"),
        (SettingName::MaxConnections, "0", "whole number"),
        (SettingName::HeaderTimeoutSeconds, "soon", "whole number"),
    ] {
        let error = resolve_with(None, &[], &[(SettingName::Port, "1"), (key, value)]).unwrap_err();
        assert!(error.message().contains(expected), "{key}: {error}");
    }
}

#[test]
fn ops_10_the_settings_file_is_checked_too() {
    let unknown =
        resolve_with(Some("colour = \"red\""), &[], &[(SettingName::Port, "1")]).unwrap_err();
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
        (SettingName::Port, "1"),
        (SettingName::MaxConnections, "50"),
        (SettingName::HeaderTimeoutSeconds, "5"),
    ];
    let settings = resolve_with(None, &[], &cli).unwrap();
    assert_eq!(settings.max_connections, 50);
    assert_eq!(settings.header_timeout, Duration::from_secs(5));
}

#[test]
fn sec_14_rate_limits_have_defaults_and_can_be_changed() {
    let settings = resolve_with(None, &[], &[(SettingName::Port, "1")]).unwrap();
    assert_eq!(settings.limits, LimitsConfig::default());
    let cli = [
        (SettingName::Port, "1"),
        (SettingName::RateLimitPerMinute, "50"),
        (SettingName::SignInAttemptsPerMinute, "3"),
    ];
    let settings = resolve_with(None, &[], &cli).unwrap();
    assert_eq!(settings.limits.requests_per_minute, 50);
    assert_eq!(settings.limits.sign_in_attempts_per_minute, 3);
    for bad in ["0", "many", "99999999999"] {
        let cli = [
            (SettingName::Port, "1"),
            (SettingName::RateLimitPerMinute, bad),
        ];
        let error = resolve_with(None, &[], &cli).unwrap_err();
        assert!(matches!(error, ConfigError::NumberInvalid { .. }), "{bad}");
    }
}

#[test]
fn ntf_02_the_wait_for_the_changes_of_one_e_mail_has_a_default_and_can_be_changed() {
    let settings = resolve_with(None, &[], &[(SettingName::Port, "1")]).unwrap();
    assert_eq!(settings.mail_batch, bb_core::notification::mail_batch());
    let cli = [
        (SettingName::Port, "1"),
        (SettingName::MailBatchSeconds, "2"),
    ];
    let settings = resolve_with(None, &[], &cli).unwrap();
    assert_eq!(settings.mail_batch, jiff::SignedDuration::from_secs(2));
    for bad in ["0", "soon", "-5"] {
        let cli = [
            (SettingName::Port, "1"),
            (SettingName::MailBatchSeconds, bad),
        ];
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
    assert_eq!(env_name(SettingName::TlsMode), "BUGABOO_TLS_MODE");
    assert_eq!(env_name(SettingName::Port), "BUGABOO_PORT");
}
