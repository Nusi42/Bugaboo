//! The service on Linux: a systemd unit that runs Bugaboo as a dedicated user.
//!
//! Installing is a list of steps (`plan`) that is made first and only then carried out, so the
//! list can be tested without a Linux machine or root rights. Everything here is plain text and
//! plain data; the commands are started in `mod.rs`.

use std::path::{Path, PathBuf};

use bb_core::message::{self, Message};

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
    steps.push(run("chown", &["-R", &owner, &data]));
    // The settings file may hold a database password: the account that runs Bugaboo reads it,
    // nobody else does.
    steps.push(run("chown", &["root:bugaboo", &config]));
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
                "chown -R bugaboo:bugaboo /var/lib/bugaboo",
                "chown root:bugaboo /etc/bugaboo/bugaboo.toml",
                "chmod 640 /etc/bugaboo/bugaboo.toml",
                &format!("write {}", Path::new(UNIT_FOLDER).join(UNIT).display()),
                "systemctl daemon-reload",
                "systemctl enable --now bugaboo.service",
            ]
        );
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
