//! LINT-PATHS: code that ships and the tools never write down a path of the system
//! (coding-standards §2, "No fixed paths").
//!
//! A string that starts with a folder of the system (`/etc/…`, `/usr/…`, `/proc/…`, a drive such
//! as `C:\…`) is a guess about this computer. The program asks the operating system instead, or
//! stops with a plain message. Comments, the code behind `#[cfg(test)]`, and the files named in
//! [`LEFT_OUT`] are not read: a test may use such a path as data for a pure function.

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::rust_source::{is_test_module_file, without_tests};
use crate::source::read_text;

/// The folders directly under the root of a Unix system. A string that starts with a slash and
/// one of these names is a path of the system.
const SYSTEM_ROOTS: [&str; 20] = [
    "bin", "boot", "dev", "etc", "home", "lib", "lib64", "mnt", "opt", "proc", "root", "run",
    "sbin", "srv", "sys", "tmp", "usr", "var", "Users", "Windows",
];

/// The one list of files that are not read: tests, their fixtures and data. A part of a path.
const LEFT_OUT: [&str; 7] = [
    "/tests/",
    "/fixtures/",
    "/testdata/",
    "/e2e/",
    ".test.",
    ".spec.",
    "/testutil.rs",
];

pub(crate) struct LintPaths;

impl Check for LintPaths {
    fn id(&self) -> &'static str {
        "LINT-PATHS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for file in context.selected(self.id())? {
            if !is_left_out(&file) {
                findings.extend(findings_in(&file, &read_text(context.repo, &file)?));
            }
        }
        Ok(findings)
    }
}

fn is_left_out(path: &str) -> bool {
    let slashed = format!("/{path}");
    is_test_module_file(path) || LEFT_OUT.iter().any(|part| slashed.contains(part))
}

/// Whether the text after an opening quote starts with a path of the system.
fn starts_with_system_path(rest: &str) -> bool {
    let mut characters = rest.chars();
    match (characters.next(), characters.next()) {
        (Some('/'), _) => {
            let name: String = rest[1..]
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();
            SYSTEM_ROOTS.contains(&name.as_str())
                && matches!(
                    rest[1 + name.len()..].chars().next(),
                    None | Some('/' | '"' | '\'' | '`')
                )
        }
        (Some(drive), Some(':')) if drive.is_ascii_alphabetic() => {
            matches!(characters.next(), Some('\\' | '/'))
        }
        _ => false,
    }
}

/// The problems in one file. In Rust the code behind `#[cfg(test)]` is blanked out first.
fn findings_in(path: &str, text: &str) -> Vec<Finding> {
    let text = if has_extension(path, "rs") {
        without_tests(text)
    } else {
        text.to_owned()
    };
    let mut findings = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let code = code_of(line);
        for (at, quote) in code
            .char_indices()
            .filter(|(_, c)| matches!(c, '"' | '\'' | '`'))
        {
            let rest = &code[at + quote.len_utf8()..];
            if starts_with_system_path(rest) {
                let shown: String = rest.chars().take_while(|c| *c != quote).take(40).collect();
                findings.push(Finding::at(
                    format!("{path}:{}", number + 1),
                    format!(
                        "`{shown}` is a fixed path of the system (coding-standards §2, No fixed paths). Ask the operating system for it, and stop with a plain message when it does not answer."
                    ),
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bad(path: &str, text: &str) -> usize {
        findings_in(path, text).len()
    }

    #[test]
    fn bad_example_a_path_of_the_system_in_code() {
        assert_eq!(bad("crates/a/src/lib.rs", "let p = \"C:\\Windows\";\n"), 1);
        assert_eq!(bad("crates/a/src/lib.rs", "let p = \"/etc/passwd\";\n"), 1);
        assert_eq!(
            bad(
                "crates/a/src/lib.rs",
                "let p = Path::new(\"/proc/self\");\n"
            ),
            1
        );
        assert_eq!(bad("xtask/src/a.rs", "let p = \"/tmp\";\n"), 1);
        assert_eq!(bad("web/tooling/a.ts", "const p = '/usr/bin/node';\n"), 1);
        assert_eq!(bad("tools/dev/a.sh", "run \"D:/data\"\n"), 1);
    }

    #[test]
    fn bad_example_a_drive_written_with_escaped_backslashes() {
        let source = format!("let p = \"C:{0}{0}Windows\";", '\\');
        assert_eq!(bad("crates/a/src/lib.rs", &source), 1);
    }

    #[test]
    fn good_example_asking_the_system_and_paths_that_are_not_the_systems() {
        let good = "let folder = known_folders::get_known_folder_path(KnownFolder::System);\n\
                    let url = \"/api/issues\";\n\
                    let relative = \"etc/settings\";\n\
                    let text = \"Keep it in /etc/bugaboo, for example\";\n\
                    let home = \"/homepage\";\n\
                    let label = \"A: first\";\n\
                    // let p = \"/etc/passwd\";\n\
                    let temp = std::env::temp_dir();\n";
        assert_eq!(bad("crates/a/src/lib.rs", good), 0);
    }

    #[test]
    fn good_example_a_test_may_use_a_path_as_data() {
        let inline =
            "fn f() {}\n#[cfg(test)]\nmod tests {\n    const P: &str = \"/etc/passwd\";\n}\n";
        assert_eq!(bad("crates/a/src/lib.rs", inline), 0);
        assert!(is_left_out("crates/a/src/tests.rs"));
        assert!(is_left_out("crates/a/tests/x.rs"));
        assert!(is_left_out("tools/checks/fixtures/LINT-PATHS/bad/a.rs"));
        assert!(is_left_out("web/tooling/a.test.ts"));
        assert!(!is_left_out("crates/a/src/lib.rs"));
    }

    #[test]
    fn the_finding_names_the_file_and_the_line() {
        let found = findings_in(
            "crates/a/src/lib.rs",
            "fn f() {}\nlet p = \"/etc/hosts\";\n",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location.as_deref(), Some("crates/a/src/lib.rs:2"));
    }
}
