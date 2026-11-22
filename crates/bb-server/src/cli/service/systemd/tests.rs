use super::*;

fn installation() -> Installation<'static> {
    Installation {
        program: Path::new("/opt/bugaboo/bugaboo"),
        config: Path::new("/etc/bugaboo/bugaboo.toml"),
        unit_folder: Path::new("/etc/units"),
        data_dir: Path::new("/var/lib/bugaboo"),
        known_files: &[],
    }
}

#[test]
fn ops_04_the_unit_runs_the_program_as_its_own_user_with_little_power() {
    let text = unit(&installation());
    assert!(text.contains("User=bugaboo\nGroup=bugaboo\n"), "{text}");
    assert!(
        text.contains(
            "ExecStart=\"/opt/bugaboo/bugaboo\" serve --config \"/etc/bugaboo/bugaboo.toml\"\n"
        ),
        "{text}"
    );
    assert!(text.contains("ReadWritePaths=\"/var/lib/bugaboo\"\n"));
    for line in [
        "NoNewPrivileges=yes",
        "ProtectSystem=strict",
        "PrivateTmp=yes",
        "Restart=on-failure",
        "WantedBy=multi-user.target",
        "Description=Bugaboo",
    ] {
        assert!(text.contains(line), "{line}");
    }
}

#[test]
fn ops_04_the_unit_holds_the_hardening_lines() {
    let text = unit(&installation());
    for line in [
        "ProtectHome=yes",
        "PrivateDevices=yes",
        "CapabilityBoundingSet=",
        "RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX",
        "NoNewPrivileges=yes",
        "ProtectSystem=strict",
        "ReadWritePaths=\"/var/lib/bugaboo\"",
        "PrivateTmp=yes",
        "Restart=on-failure",
    ] {
        assert!(
            text.lines().any(|written| written == line),
            "{line} is missing:
{text}"
        );
    }
    // An empty bounding set takes every capability away: the line ends after `=`.
    assert!(
        text.contains(
            "
CapabilityBoundingSet=
"
        ),
        "{text}"
    );
}

#[test]
fn ops_04_a_stopped_upgrade_does_not_restart_the_unit() {
    let text = unit(&installation());
    assert!(
        text.lines()
            .any(|line| line == format!("RestartPreventExitStatus={UPGRADE_STOPPED}")),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|line| line == format!("RestartSec={}", RESTART_DELAY.as_secs())),
        "{text}"
    );
}

#[test]
fn ops_04_service_status_reads_the_state_systemd_names() {
    use super::super::State;
    let show = |load: &str, active: &str, sub: &str, status: &str| {
        format!(
            "LoadState={load}
ActiveState={active}
SubState={sub}
ExecMainStatus={status}
"
        )
    };
    assert_eq!(
        state_of(&show("not-found", "inactive", "dead", "0")),
        Some(State::NotInstalled)
    );
    assert_eq!(
        state_of(&show("loaded", "active", "running", "0")),
        Some(State::Running)
    );
    assert_eq!(
        state_of(&show("loaded", "activating", "start", "0")),
        Some(State::Starting)
    );
    assert_eq!(
        state_of(&show("loaded", "activating", "auto-restart", "1")),
        Some(State::Restarting)
    );
    assert_eq!(
        state_of(&show("loaded", "deactivating", "stop-sigterm", "0")),
        Some(State::Stopping)
    );
    assert_eq!(
        state_of(&show("loaded", "inactive", "dead", "0")),
        Some(State::Stopped)
    );
    assert_eq!(
        state_of(&show("loaded", "failed", "failed", "65")),
        Some(State::StoppedAtUpgrade)
    );
    assert_eq!(
        state_of(&show("loaded", "failed", "failed", "1")),
        Some(State::Failed(1))
    );
    // A process killed by a signal names the signal, not an exit code of that number; a stop
    // that systemd asked for with a signal is a clean stop.
    let ended = |code: &str, status: &str, result: &str| {
        format!(
            "LoadState=loaded\nActiveState=failed\nSubState=failed\nExecMainCode={code}\nExecMainStatus={status}\nResult={result}\n"
        )
    };
    assert_eq!(state_of(&ended("2", "9", "signal")), Some(State::Killed(9)));
    assert_eq!(
        state_of(&ended("3", "11", "core-dump")),
        Some(State::Killed(11))
    );
    assert_eq!(state_of(&ended("2", "15", "success")), Some(State::Stopped));
    assert_eq!(
        state_of(&ended("1", "9", "exit-code")),
        Some(State::Failed(9))
    );
    // An answer that names nothing known is not taken for a running service.
    assert_eq!(state_of(""), None);
}

#[test]
fn ops_04_paths_with_spaces_and_special_characters_stay_in_one_word() {
    assert_eq!(quote("/srv/my data"), "\"/srv/my data\"");
    assert_eq!(quote("50%"), "\"50%%\"");
    assert_eq!(quote("a\"b\\c"), "\"a\\\"b\\\\c\"");
}

#[test]
fn ops_04_installing_makes_the_user_the_folder_the_unit_and_starts_it_in_that_order() {
    let installation = installation();
    let steps = plan(&installation, Some(Path::new("/shells/nologin")));
    let kinds: Vec<String> = steps
        .iter()
        .map(|step| match step {
            Step::Run { program, arguments } => format!("{program} {}", arguments.join(" ")),
            Step::PrepareDataFolder(path) => format!("prepare {}", path.display()),
            Step::Write { path, .. } => format!("write {}", path.display()),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "useradd --system --no-create-home --shell /shells/nologin bugaboo",
            "prepare /var/lib/bugaboo",
            "chown -h bugaboo:bugaboo /var/lib/bugaboo",
            "chown -h -R -P bugaboo:bugaboo /var/lib/bugaboo/files",
            "chown -h -R -P bugaboo:bugaboo /var/lib/bugaboo/exports",
            "chown -h -R -P bugaboo:bugaboo /var/lib/bugaboo/keys",
            "chown -h -R -P bugaboo:bugaboo /var/lib/bugaboo/backups",
            "chown -h -R -P bugaboo:bugaboo /var/lib/bugaboo/tls",
            "chown -h -R -P bugaboo:bugaboo /var/lib/bugaboo/logs",
            "chown -h root:bugaboo /etc/bugaboo/bugaboo.toml",
            "chmod 640 /etc/bugaboo/bugaboo.toml",
            &format!("write {}", installation.unit_folder.join(UNIT).display()),
            "systemctl daemon-reload",
            "systemctl enable --now bugaboo.service",
        ]
    );
}

#[test]
fn sec_16_only_what_bugaboo_made_changes_owner_and_never_the_whole_folder() {
    let files = [PathBuf::from("/var/lib/bugaboo/bugaboo.db")];
    let installation = Installation {
        known_files: &files,
        ..installation()
    };
    let chowns: Vec<String> = plan(&installation, None)
        .iter()
        .filter_map(|step| match step {
            Step::Run {
                program: "chown",
                arguments,
            } => Some(arguments.join(" ")),
            _ => None,
        })
        .collect();
    // The folder itself, without its content, and a known file without anything below it.
    assert!(chowns.contains(&"-h bugaboo:bugaboo /var/lib/bugaboo".to_owned()));
    assert!(chowns.contains(&"-h bugaboo:bugaboo /var/lib/bugaboo/bugaboo.db".to_owned()));
    // Nothing recurses over the folder as a whole, where somebody's own files may lie.
    assert!(
        !chowns
            .iter()
            .any(|line| line == "-h -R -P bugaboo:bugaboo /var/lib/bugaboo")
    );
    assert!(
        chowns
            .iter()
            .filter(|line| line.contains("-R"))
            .all(|line| {
                ["files", "exports", "keys", "backups", "tls", "logs"]
                    .iter()
                    .any(|name| line.ends_with(&format!("/var/lib/bugaboo/{name}")))
            })
    );
}

#[test]
fn sec_16_no_chown_follows_a_link() {
    let files = [PathBuf::from("/var/lib/bugaboo/bugaboo.db")];
    let installation = Installation {
        known_files: &files,
        ..installation()
    };
    for step in plan(&installation, None) {
        if let Step::Run {
            program: "chown",
            arguments,
        } = step
        {
            // `-h` changes a link itself and never what it points to; with `-R`, `-P` keeps
            // the walk from stepping through links.
            assert_eq!(arguments[0], "-h", "{arguments:?}");
            if arguments.iter().any(|argument| argument == "-R") {
                assert!(
                    arguments.iter().any(|argument| argument == "-P"),
                    "{arguments:?}"
                );
            }
        }
    }
}

#[test]
fn sec_16_a_settings_file_inside_the_data_folder_is_refused() {
    let inside = check_settings_file(
        Path::new("/var/lib/bugaboo/bugaboo.toml"),
        Path::new("/var/lib/bugaboo"),
    )
    .unwrap_err();
    assert!(
        inside.render().contains("inside the data folder"),
        "{}",
        inside.render()
    );
}

#[cfg(unix)]
#[test]
fn sec_16_a_settings_file_that_is_a_link_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let real = temp.path().join("real.toml");
    std::fs::write(&real, "port = 1").unwrap();
    let link = temp.path().join("link.toml");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let refused = check_settings_file(&link, Path::new("/var/lib/bugaboo")).unwrap_err();
    assert!(
        refused.render().contains("is a link"),
        "{}",
        refused.render()
    );
}

#[cfg(unix)]
#[test]
fn sec_16_a_settings_file_in_a_folder_others_can_change_or_root_does_not_own_is_refused() {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
    let temp = tempfile::tempdir().unwrap();
    let folder = temp.path().join("etc");
    std::fs::create_dir(&folder).unwrap();
    let config = folder.join("bugaboo.toml");
    std::fs::write(&config, "port = 1").unwrap();
    let data = Path::new("/var/lib/bugaboo");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
    // The test's folder belongs to the account that runs the test: fine only for root.
    let is_root = std::fs::metadata(&folder).unwrap().uid() == 0;
    let answer = check_settings_file(&config, data);
    if is_root {
        answer.unwrap();
    } else {
        let refused = answer.unwrap_err();
        assert!(
            refused.render().contains("does not belong"),
            "{}",
            refused.render()
        );
    }
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o775)).unwrap();
    let refused = check_settings_file(&config, data).unwrap_err();
    assert!(
        refused.render().contains("other accounts"),
        "{}",
        refused.render()
    );
}

#[test]
fn sec_16_a_data_folder_given_by_a_path_that_climbs_is_still_the_same_folder() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("bugaboo");
    std::fs::create_dir(&data).unwrap();
    std::fs::create_dir(temp.path().join("other")).unwrap();
    let config = data.join("bugaboo.toml");
    std::fs::write(&config, "port = 1").unwrap();
    // `other/../bugaboo` is the data folder, and the file in it is inside it.
    let climbing = temp.path().join("other").join("..").join("bugaboo");
    let refused = check_settings_file(&config, &climbing).unwrap_err();
    assert!(
        refused.render().contains("inside the data folder"),
        "{}",
        refused.render()
    );
    // And the file named the same way is inside the plain folder.
    let climbing_config = temp
        .path()
        .join("other")
        .join("..")
        .join("bugaboo")
        .join("bugaboo.toml");
    let refused = check_settings_file(&climbing_config, &data).unwrap_err();
    assert!(
        refused.render().contains("inside the data folder"),
        "{}",
        refused.render()
    );
}

#[cfg(unix)]
#[test]
fn sec_16_a_data_folder_that_is_a_link_is_planned_with_its_real_path() {
    let temp = tempfile::tempdir().unwrap();
    let real = temp.path().join("disk").join("bugaboo");
    std::fs::create_dir_all(&real).unwrap();
    let link = temp.path().join("bugaboo");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert_eq!(real_path(&link), std::fs::canonicalize(&real).unwrap());
    // The plan and the unit name the folder the link leads to, so that `chown -h` changes
    // the folder and not the link.
    let resolved = real_path(&link);
    let installation = Installation {
        data_dir: &resolved,
        ..installation()
    };
    let text = unit(&installation);
    assert!(!text.contains(link.to_str().unwrap()), "{text}");
    assert!(text.contains(resolved.to_str().unwrap()), "{text}");
}

#[test]
fn sec_16_a_path_that_is_not_there_yet_stays_as_it_is() {
    // Only made absolute, which a path of Linux already is there.
    let new = Path::new("/var/lib/not-there-yet/bugaboo");
    assert_eq!(real_path(new), std::path::absolute(new).unwrap());
}

#[test]
fn ops_04_an_account_that_exists_is_not_made_again() {
    let steps = plan(&installation(), None);
    assert!(!steps.iter().any(|step| matches!(
        step,
        Step::Run {
            program: "useradd",
            ..
        }
    )));
}

#[test]
fn ops_04_removing_stops_and_forgets_the_unit_and_leaves_the_data() {
    let steps = removal(Path::new("/etc/units"));
    assert_eq!(steps.len(), 3);
    assert!(matches!(
        &steps[0],
        Step::Run { program: "systemctl", arguments } if arguments[0] == "disable"
    ));
    assert!(
        steps
            .iter()
            .all(|step| !matches!(step, Step::PrepareDataFolder(_)))
    );
}

#[test]
fn ops_04_the_unit_goes_into_the_folder_systemd_names() {
    let folder = std::env::temp_dir();
    let installation = Installation {
        unit_folder: &folder,
        ..installation()
    };
    let written = plan(&installation, None)
        .into_iter()
        .find_map(|step| match step {
            Step::Write { path, .. } => Some(path),
            _ => None,
        });
    assert_eq!(written, Some(folder.join(UNIT)));
    let removed = removal(&folder);
    assert!(matches!(
        &removed[1],
        Step::Run { program: "rm", arguments } if arguments == &[path_argument(&folder.join(UNIT))]
    ));
}

#[test]
fn ops_04_systemd_runs_the_computer_in_any_state_but_off_or_unknown() {
    for state in ["running\n", "degraded\n", "starting\n", "maintenance\n"] {
        assert!(is_running(state), "{state}");
    }
    for state in ["offline\n", "unknown\n", "", "Failed to connect to bus\n"] {
        assert!(!is_running(state), "{state}");
    }
}

#[test]
fn ops_04_when_systemd_names_no_folder_the_message_says_so_and_does_not_guess() {
    let said = Message::from(message::SERVICE_UNIT_FOLDER_UNKNOWN).render();
    assert!(said.contains("does not guess"), "{said}");
    assert!(said.contains("systemd-path"), "{said}");
}

#[cfg(unix)]
#[test]
fn ops_04_an_answer_that_root_does_not_own_alone_is_refused() {
    use std::os::unix::fs::PermissionsExt as _;
    let temp = tempfile::tempdir().unwrap();
    let folder = temp.path().join("units");
    std::fs::create_dir(&folder).unwrap();
    // Open to the group: refused for every account, root included.
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o775)).unwrap();
    let said = trusted(folder.clone()).unwrap_err().render();
    assert!(said.contains("does not use it"), "{said}");
    assert!(said.contains(&folder.display().to_string()), "{said}");
}

#[cfg(unix)]
#[test]
fn ops_04_a_path_that_cannot_be_looked_at_is_not_trusted() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("not-there");
    assert_eq!(untrusted(&missing, None), Some(Untrusted::Unreadable));
    assert!(trusted(missing).is_err());
}

#[test]
fn ops_04_the_install_refuses_a_place_the_hardened_unit_hides() {
    // systemd is asked with the unit's own line, through a short-lived service.
    let (program, arguments) = sees_command(Path::new("/usr/bin/test"), Path::new("/srv/b b"));
    assert_eq!(program, "systemd-run");
    assert!(arguments.contains(&"--wait".to_owned()), "{arguments:?}");
    let property = format!("--property={PROTECT_HOME}");
    assert!(arguments.contains(&property), "{arguments:?}");
    assert!(
        unit(&installation())
            .lines()
            .any(|line| line == PROTECT_HOME),
        "the unit and the question hold the same line"
    );
    assert_eq!(
        arguments[arguments.len() - 3..],
        ["/usr/bin/test", "-e", "/srv/b b"]
    );

    // A folder standing for a home folder that the service does not see.
    let place = tempfile::tempdir().unwrap();
    let home = place.path().join("home");
    let open = place.path().join("open");
    std::fs::create_dir_all(home.join("ana")).unwrap();
    std::fs::create_dir_all(&open).unwrap();
    let asked = std::cell::RefCell::new(Vec::new());
    let sees = |path: &Path| {
        asked.borrow_mut().push(path.to_path_buf());
        !path.starts_with(&home)
    };
    let program = open.join("bugaboo");
    let config = open.join("bugaboo.toml");
    assert_eq!(first_hidden(&[&program, &config], sees), None);
    // A data folder the install would make is asked by the nearest folder that is there.
    let data = home.join("ana/data");
    assert_eq!(
        first_hidden(&[&program, &config, &data], sees),
        Some(data.as_path())
    );
    assert!(asked.borrow().contains(&home.join("ana")), "{asked:?}");
    assert!(!asked.borrow().contains(&data), "{asked:?}");
}
