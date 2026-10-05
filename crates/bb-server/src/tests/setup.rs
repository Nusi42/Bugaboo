//! The setup wizard (OPS-03): the questions, the checks, the file it writes.

use std::collections::VecDeque;
use std::net::IpAddr;
use std::path::Path;

use bb_testing::TestResult;
use clap::Parser as _;

use super::*;
use crate::config::{Settings, TlsMode, resolve};

/// A person at the console: answers the questions in turn and remembers what was said.
struct Script {
    replies: VecDeque<&'static str>,
    questions: Vec<(String, Option<String>)>,
    said: Vec<String>,
}

impl Script {
    fn new(replies: &[&'static str]) -> Self {
        Self {
            replies: replies.iter().copied().collect(),
            questions: Vec::new(),
            said: Vec::new(),
        }
    }

    fn asked(&self, word: &str) -> bool {
        self.questions
            .iter()
            .any(|(question, _)| question.contains(word))
    }
}

impl Terminal for Script {
    fn ask(&mut self, question: &str, default: Option<&str>) -> Option<String> {
        self.questions
            .push((question.to_owned(), default.map(str::to_owned)));
        let reply = self.replies.pop_front()?;
        Some(if reply.is_empty() {
            default.unwrap_or_default().to_owned()
        } else {
            reply.to_owned()
        })
    }

    fn say(&mut self, text: &str) {
        self.said.push(text.to_owned());
    }
}

/// The command line of the wizard on its own.
#[derive(clap::Parser)]
struct Only {
    #[command(flatten)]
    setup: SetupArgs,
}

fn args(line: &str) -> SetupArgs {
    let mut words = vec!["setup"];
    words.extend(line.split_whitespace());
    Only::parse_from(words).setup
}

/// Every port is free, as on a fresh computer.
fn all_free(_: IpAddr, _: u16) -> bool {
    true
}

fn written(target: &Path) -> Result<Settings, Box<dyn std::error::Error + Send + Sync>> {
    let text = std::fs::read_to_string(target)?;
    Ok(resolve(&Sources {
        file: Some((target, &text)),
        env: &|_| None,
        cli: &[],
        base: Path::new("program"),
    })?)
}

#[test]
fn ops_10_a_database_url_with_a_password_on_the_setup_command_line_warns() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    let line = "--non-interactive --port 8443 --database-url postgres://bugaboo:Hunter2-Secret@db.example.test/bugaboo";
    let mut terminal = Script::new(&[]);
    execute(&args(line), &mut terminal, &target, &all_free)?;
    let said = terminal.said.join(
        "
",
    );
    // It says where the address went and where to put it next time, and never repeats it.
    assert!(said.contains("command line"), "{said}");
    assert!(said.contains("BUGABOO_DATABASE_URL"), "{said}");
    assert!(said.contains(&target.display().to_string()), "{said}");
    assert!(
        !said.contains("Hunter2") && !said.contains("db.example"),
        "{said}"
    );
    // The address itself is saved in the settings file.
    assert!(std::fs::read_to_string(&target)?.contains("db.example.test"));

    // No password, no warning.
    let quiet = folder.path().join("quiet.toml");
    let mut terminal = Script::new(&[]);
    let plain =
        "--non-interactive --port 8443 --database-url postgres://bugaboo@db.example.test/bugaboo";
    execute(&args(plain), &mut terminal, &quiet, &all_free)?;
    assert!(
        !terminal
            .said
            .join(
                "
"
            )
            .contains("command line")
    );
    Ok(())
}

#[test]
fn ops_03_the_defaults_make_a_file_the_server_accepts() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    // Enter for every question: all defaults, the suggested port included.
    let mut terminal = Script::new(&["", "", "", "", ""]);
    execute(&args(""), &mut terminal, &target, &all_free)?;

    let settings = written(&target)?;
    assert_eq!(settings.port, 8443);
    assert_eq!(settings.address, "127.0.0.1".parse::<IpAddr>()?);
    assert_eq!(settings.tls, TlsMode::SelfSigned);
    assert!(settings.database_url.is_none(), "the built-in database");
    assert!(settings.data_dir.is_absolute());
    // The questions that do not apply were not asked.
    assert!(!terminal.asked("certificate file") && !terminal.asked("web address will people"));
    // The person is told where the answers went and what happens at the first start.
    let said = terminal.said.join("\n");
    assert!(said.contains(&target.display().to_string()), "{said}");
    assert!(said.contains("setup code") && said.contains("bugaboo service install"));
    Ok(())
}

#[test]
fn ops_03_a_busy_port_is_not_accepted_and_the_next_free_one_is_suggested() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    // 8443 and 8444 are taken: the suggestion is 8445. The person first types a busy port.
    let busy = |_: IpAddr, port: u16| port != 8443 && port != 8444 && port != 9000;
    let mut terminal = Script::new(&["", "", "9000", "9001", "", ""]);
    execute(&args(""), &mut terminal, &target, &busy)?;
    let port_question = terminal
        .questions
        .iter()
        .find(|(question, _)| question.contains("port"))
        .ok_or("no port question")?;
    assert_eq!(port_question.1.as_deref(), Some("8445"));
    assert!(
        terminal.said.iter().any(|text| text.contains("Port 9000")),
        "{:?}",
        terminal.said
    );
    assert_eq!(written(&target)?.port, 9001);
    Ok(())
}

#[test]
fn ops_03_a_mistake_is_explained_and_only_that_question_is_asked_again() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    // A bad address is explained at once and asked again; nothing before it is asked twice.
    let mut terminal = Script::new(&["", "banana", "127.0.0.1", "", "", ""]);
    execute(&args(""), &mut terminal, &target, &all_free)?;
    assert!(
        terminal
            .said
            .iter()
            .any(|text| text.contains("That does not work") && text.contains("banana")),
        "{:?}",
        terminal.said
    );
    let address_questions = terminal
        .questions
        .iter()
        .filter(|(question, _)| question.contains("open Bugaboo"))
        .count();
    assert_eq!(address_questions, 2);
    assert_eq!(
        terminal
            .questions
            .iter()
            .filter(|(question, _)| question.contains("keep its data"))
            .count(),
        1
    );
    assert_eq!(written(&target)?.address, "127.0.0.1".parse::<IpAddr>()?);
    Ok(())
}

#[test]
fn ops_03_plain_http_on_the_network_is_refused_and_the_address_or_the_mode_is_asked_again()
-> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    // The mistake is only in the combination, so it is found when all questions are answered.
    let mut terminal = Script::new(&[
        "",
        "0.0.0.0",
        "",
        "off",
        "https://bugaboo.example.test",
        "",
        "127.0.0.1",
        "off",
    ]);
    execute(&args(""), &mut terminal, &target, &all_free)?;
    assert!(
        terminal
            .said
            .iter()
            .any(|text| text.contains("cannot be switched off")),
        "{:?}",
        terminal.said
    );
    let settings = written(&target)?;
    assert_eq!(settings.tls, TlsMode::Off);
    assert_eq!(settings.address, "127.0.0.1".parse::<IpAddr>()?);
    Ok(())
}

#[test]
fn ops_03_a_certificate_of_your_own_asks_for_its_files() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    let mut terminal = Script::new(&["", "", "", "files", "cert.pem", "key.pem", ""]);
    execute(&args(""), &mut terminal, &target, &all_free)?;
    assert!(terminal.asked("certificate file") && terminal.asked("private key file"));
    let settings = written(&target)?;
    let TlsMode::Files { cert, key } = settings.tls else {
        return Err("not HTTPS mode files".into());
    };
    // Paths are written in full: a service does not start in the folder of the person.
    assert!(cert.is_absolute() && key.is_absolute());
    assert!(cert.ends_with("cert.pem") && key.ends_with("key.pem"));
    Ok(())
}

#[test]
fn ops_03_a_server_on_the_network_is_asked_for_its_public_address() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    let mut terminal = Script::new(&["", "0.0.0.0", "", "", "https://bugaboo.example.test", ""]);
    execute(&args(""), &mut terminal, &target, &all_free)?;
    assert!(terminal.asked("web address will people"));
    assert_eq!(written(&target)?.public_url, "https://bugaboo.example.test");
    Ok(())
}

#[test]
fn ops_03_a_database_address_is_kept_and_the_built_in_one_is_no_setting() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    let mut terminal = Script::new(&["", "", "", "", "sqlite:other.db"]);
    execute(&args(""), &mut terminal, &target, &all_free)?;
    assert_eq!(
        written(&target)?.database_url.as_deref(),
        Some("sqlite:other.db")
    );
    let text = std::fs::read_to_string(&target)?;
    assert!(text.contains("database_url"), "{text}");

    let second = folder.path().join("built-in.toml");
    execute(
        &args(""),
        &mut Script::new(&["", "", "", "", "built-in"]),
        &second,
        &all_free,
    )?;
    assert!(!std::fs::read_to_string(&second)?.contains("database_url"));
    Ok(())
}

#[test]
fn ops_03_the_input_ending_saves_nothing() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    let mut terminal = Script::new(&["", ""]);
    let error = execute(&args(""), &mut terminal, &target, &all_free)
        .err()
        .ok_or("the wizard finished without answers")?;
    assert!(format!("{error:#}").contains("nothing was saved"));
    assert!(!target.exists());
    Ok(())
}

#[test]
fn ops_03_an_existing_file_is_replaced_only_when_the_person_agrees() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    std::fs::write(&target, "port = 1\n")?;

    let mut refused = Script::new(&["no"]);
    execute(&args(""), &mut refused, &target, &all_free)?;
    assert_eq!(std::fs::read_to_string(&target)?, "port = 1\n");
    assert!(
        refused
            .said
            .iter()
            .any(|text| text.contains("Nothing was changed"))
    );

    let mut agreed = Script::new(&["yes", "", "", "", "", ""]);
    execute(&args(""), &mut agreed, &target, &all_free)?;
    assert_eq!(written(&target)?.port, 8443);
    Ok(())
}

#[test]
fn ops_03_scripts_ask_nothing_and_need_a_port() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    let mut terminal = Script::new(&[]);
    // No port: there is no default port, so this says so instead of choosing one.
    let error = execute(
        &args("--non-interactive"),
        &mut terminal,
        &target,
        &all_free,
    )
    .err()
    .ok_or("a port was made up")?;
    assert!(format!("{error:#}").contains("No port is set"));
    assert!(terminal.questions.is_empty() && !target.exists());

    execute(
        &args("--non-interactive --port 47200 --tls off --data-dir somewhere"),
        &mut terminal,
        &target,
        &all_free,
    )?;
    let settings = written(&target)?;
    assert_eq!(settings.port, 47200);
    assert_eq!(settings.tls, TlsMode::Off);
    assert!(settings.data_dir.ends_with("somewhere"));
    assert!(terminal.questions.is_empty(), "nobody was asked anything");
    Ok(())
}

#[test]
fn ops_03_scripts_do_not_replace_a_file_without_the_option() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    std::fs::write(&target, "port = 1\n")?;
    let error = execute(
        &args("--non-interactive --port 47200"),
        &mut Script::new(&[]),
        &target,
        &all_free,
    )
    .err()
    .ok_or("the file was replaced")?;
    assert!(format!("{error:#}").contains("--overwrite"));
    assert_eq!(std::fs::read_to_string(&target)?, "port = 1\n");

    execute(
        &args("--non-interactive --overwrite --port 47200"),
        &mut Script::new(&[]),
        &target,
        &all_free,
    )?;
    assert_eq!(written(&target)?.port, 47200);
    Ok(())
}

#[test]
fn ops_03_a_script_with_an_unusable_answer_fails_with_the_reason() -> TestResult {
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    let error = execute(
        &args("--non-interactive --port 47200 --tls off --address 0.0.0.0"),
        &mut Script::new(&[]),
        &target,
        &all_free,
    )
    .err()
    .ok_or("plain HTTP on the network was accepted")?;
    assert!(format!("{error:#}").contains("cannot be switched off"));
    assert!(!target.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn ops_03_the_file_can_be_read_by_its_owner_only() -> TestResult {
    use std::os::unix::fs::PermissionsExt as _;
    let folder = tempfile::tempdir()?;
    let target = folder.path().join("bugaboo.toml");
    execute(
        &args("--non-interactive --port 47200 --database-url postgres://user:secret@db/x"),
        &mut Script::new(&[]),
        &target,
        &all_free,
    )?;
    assert_eq!(
        std::fs::metadata(&target)?.permissions().mode() & 0o777,
        0o600
    );
    Ok(())
}
