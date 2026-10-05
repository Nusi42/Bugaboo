//! The service on Linux: a systemd unit that runs Bugaboo as a dedicated user.
//!
//! Installing is a list of steps (`plan`) that is made first and only then carried out, so the
//! list can be tested without a Linux machine or root rights. Everything here is plain text and
//! plain data; the commands are started in `mod.rs`.

use std::path::{Path, PathBuf};

use bb_core::message::{self, Message};

use crate::data_folder::SUB_FOLDERS;

/// The name of the unit, and so the name of the service for `systemctl`.
pub(super) const UNIT: &str = "bugaboo.service";
/// The account that runs Bugaboo: no password, no shell, no home folder.
pub(super) const USER: &str = "bugaboo";
/// Where systemd looks for units that an administrator installs.
pub(super) const UNIT_FOLDER: &str = "/etc/systemd/system";
/// The shell of an account that cannot sign in.
const NO_LOGIN: &str = "/usr/sbin/nologin";

/// One thing to do, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Step {
    /// Start a program.
    Run {
        program: &'static str,
        arguments: Vec<String>,
    },
    /// Make the data folder with its sub-folders.
    PrepareDataFolder(PathBuf),
    /// Write a text file.
    Write { path: PathBuf, contents: String },
}

/// What the unit needs to know about this installation.
pub(super) struct Installation<'a> {
    /// The program, in full.
    pub(super) program: &'a Path,
    /// The settings file, in full.
    pub(super) config: &'a Path,
    /// The data folder, in full.
    pub(super) data_dir: &'a Path,
    /// The files directly in the data folder that Bugaboo made, which the account takes over.
    pub(super) known_files: &'a [PathBuf],
}

fn run(program: &'static str, arguments: &[&str]) -> Step {
    Step::Run {
        program,
        arguments: arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect(),
    }
}

fn path_argument(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// A word of a unit line, quoted so that spaces and special characters stay in it. systemd reads
/// `"…"` with C escapes, and `%` starts a specifier, so it is written twice.
fn quote(value: &str) -> String {
    let mut quoted = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '%' => quoted.push_str("%%"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

/// The text of the unit file. Bugaboo may write only to its data folder, never gains rights, and
/// is started again when it fails.
pub(super) fn unit(installation: &Installation<'_>) -> String {
    let name = Message::from(message::APP_NAME).render();
    let program = quote(&path_argument(installation.program));
    let config = quote(&path_argument(installation.config));
    let data = quote(&path_argument(installation.data_dir));
    format!(
        "[Unit]\n\
         Description={name}\n\
         After=network.target\n\
         \n\
         [Service]\n\
         User={USER}\n\
         Group={USER}\n\
         ExecStart={program} serve --config {config}\n\
         Restart=on-failure\n\
         NoNewPrivileges=yes\n\
         ProtectSystem=strict\n\
         ReadWritePaths={data}\n\
         PrivateTmp=yes\n\
         \n\
         [Install]\n\
         WantedBy=multi-user.target\n"
    )
}

/// The real path of `path` when it exists, with every link in it followed (`..` and links in
/// the middle included); a path that is not there yet stays as it is.
pub(super) fn real_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Why the folder of the settings file is not trusted.
#[derive(Debug, PartialEq, Eq)]
enum Untrusted {
    /// The group or everybody may write in it, so they could replace the file.
    OthersCanWrite,
    /// It belongs to somebody but root, who could replace the file.
    NotRoots,
}

/// Whether a folder with this mode and owner (user number) may hold the settings file.
fn trust_of(mode: u32, owner: u32) -> Option<Untrusted> {
    if mode & 0o022 != 0 {
        Some(Untrusted::OthersCanWrite)
    } else if owner != 0 {
        Some(Untrusted::NotRoots)
    } else {
        None
    }
}

/// Linux: the reason the folder is not trusted, if it is not.
#[cfg(unix)]
fn untrusted(folder: &Path) -> Option<Untrusted> {
    use std::os::unix::fs::MetadataExt as _;
    let data = std::fs::metadata(folder).ok()?;
    trust_of(data.mode(), data.uid())
}

#[cfg(not(unix))]
fn untrusted(_folder: &Path) -> Option<Untrusted> {
    None
}

/// Whether the settings file may be given to the account of the service: it is a file and no
/// link (a `chmod` would follow a link to whatever it points to), it is not inside the data
/// folder (the service could replace it there; both are compared by their real paths, so `..`
/// and links do not hide it), and the folder that holds it belongs to root and cannot be changed
/// by other accounts (they could replace the file or put a link in its place).
pub(super) fn check_settings_file(config: &Path, data_dir: &Path) -> Result<(), Message> {
    let shown = config.display();
    if std::fs::symlink_metadata(config).is_ok_and(|data| data.file_type().is_symlink()) {
        return Err(message::service_settings_is_link(shown));
    }
    let config = real_path(config);
    if config.starts_with(real_path(data_dir)) {
        return Err(message::service_settings_in_data(config.display()));
    }
    let Some(folder) = config.parent() else {
        return Ok(());
    };
    match untrusted(folder) {
        Some(Untrusted::OthersCanWrite) => {
            Err(message::service_settings_folder_open(folder.display()))
        }
        Some(Untrusted::NotRoots) => Err(message::service_settings_folder_owner(folder.display())),
        None => Ok(()),
    }
}

/// The steps that install the service. The account is made only when it does not exist yet.
pub(super) fn plan(installation: &Installation<'_>, user_exists: bool) -> Vec<Step> {
    let owner = format!("{USER}:{USER}");
    let mut steps = Vec::new();
    if !user_exists {
        steps.push(run(
            "useradd",
            &["--system", "--no-create-home", "--shell", NO_LOGIN, USER],
        ));
    }
    let data = path_argument(installation.data_dir);
    let config = path_argument(installation.config);
    steps.push(Step::PrepareDataFolder(installation.data_dir.to_path_buf()));
    // Only what Bugaboo made changes owner: the folder itself, its known files and its own
    // sub-folders. Never the whole folder with whatever else lies in it.
    // `-h` changes a link itself and never the file it points to, `-P` keeps the walk of `-R`
    // from stepping through links.
    steps.push(run("chown", &["-h", &owner, &data]));
    for file in installation.known_files {
        steps.push(run("chown", &["-h", &owner, &path_argument(file)]));
    }
    for name in SUB_FOLDERS {
        // Plain text, because this plan is for Linux whichever system the test runs on.
        let folder = format!("{}/{name}", data.trim_end_matches('/'));
        steps.push(run("chown", &["-h", "-R", "-P", &owner, &folder]));
    }
    // The settings file may hold a database password: the account that runs Bugaboo reads it,
    // nobody else does. `check_settings_file` made sure it is no link.
    steps.push(run("chown", &["-h", "root:bugaboo", &config]));
    steps.push(run("chmod", &["640", &config]));
    steps.push(Step::Write {
        path: Path::new(UNIT_FOLDER).join(UNIT),
        contents: unit(installation),
    });
    steps.push(run("systemctl", &["daemon-reload"]));
    steps.push(run("systemctl", &["enable", "--now", UNIT]));
    steps
}

/// The steps that remove the service. The account and the data stay.
pub(super) fn removal() -> Vec<Step> {
    vec![
        run("systemctl", &["disable", "--now", UNIT]),
        Step::Run {
            program: "rm",
            arguments: vec![path_argument(&Path::new(UNIT_FOLDER).join(UNIT))],
        },
        run("systemctl", &["daemon-reload"]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installation() -> Installation<'static> {
        Installation {
            program: Path::new("/opt/bugaboo/bugaboo"),
            config: Path::new("/etc/bugaboo/bugaboo.toml"),
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
    fn ops_04_paths_with_spaces_and_special_characters_stay_in_one_word() {
        assert_eq!(quote("/srv/my data"), "\"/srv/my data\"");
        assert_eq!(quote("50%"), "\"50%%\"");
        assert_eq!(quote("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }

    #[test]
    fn ops_04_installing_makes_the_user_the_folder_the_unit_and_starts_it_in_that_order() {
        let installation = installation();
        let steps = plan(&installation, false);
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
                "useradd --system --no-create-home --shell /usr/sbin/nologin bugaboo",
                "prepare /var/lib/bugaboo",
                "chown -h bugaboo:bugaboo /var/lib/bugaboo",
                "chown -h -R -P bugaboo:bugaboo /var/lib/bugaboo/files",
                "chown -h -R -P bugaboo:bugaboo /var/lib/bugaboo/keys",
                "chown -h -R -P bugaboo:bugaboo /var/lib/bugaboo/backups",
                "chown -h -R -P bugaboo:bugaboo /var/lib/bugaboo/tls",
                "chown -h -R -P bugaboo:bugaboo /var/lib/bugaboo/logs",
                "chown -h root:bugaboo /etc/bugaboo/bugaboo.toml",
                "chmod 640 /etc/bugaboo/bugaboo.toml",
                &format!("write {}", Path::new(UNIT_FOLDER).join(UNIT).display()),
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
        let chowns: Vec<String> = plan(&installation, true)
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
                    ["files", "keys", "backups", "tls", "logs"]
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
        for step in plan(&installation, true) {
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

    #[test]
    fn sec_16_the_folder_of_the_settings_file_must_be_root_s_and_closed() {
        assert_eq!(trust_of(0o755, 0), None);
        assert_eq!(trust_of(0o700, 0), None);
        assert_eq!(trust_of(0o775, 0), Some(Untrusted::OthersCanWrite));
        assert_eq!(trust_of(0o754, 0), None, "only writing counts, not reading");
        assert_eq!(trust_of(0o777, 0), Some(Untrusted::OthersCanWrite));
        assert_eq!(trust_of(0o755, 1000), Some(Untrusted::NotRoots));
        // Somebody else's folder that is open as well is named for what is worse.
        assert_eq!(trust_of(0o777, 1000), Some(Untrusted::OthersCanWrite));
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
        let new = Path::new("/var/lib/not-there-yet/bugaboo");
        assert_eq!(real_path(new), new);
    }

    #[test]
    fn ops_04_an_account_that_exists_is_not_made_again() {
        let steps = plan(&installation(), true);
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
        let steps = removal();
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
}
