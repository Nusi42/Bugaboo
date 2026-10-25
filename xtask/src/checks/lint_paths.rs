//! LINT-PATHS: code that ships and the tools never write down a path of the system
//! (coding-standards §2, "No fixed paths").
//!
//! A string that starts with a folder of the system (`/etc/…`, `/usr/…`, `/proc/…`, a drive such
//! as `C:\…` or `\\?\C:\…`) is a guess about this computer. The program asks the operating system
//! instead, or stops with a plain message. In scripts (`.sh`, `.cmd`, `.ps1`) a path also counts
//! after a space, `=` or `(`, because a script writes paths without quotes. Comments, the code
//! behind `#[cfg(test)]`, and the files named in [`LEFT_OUT`] are not read: a test may use such a
//! path as data for a pure function.
//!
//! What it does not see: a path built from parts (`Path::new("/").join("etc")`, `concat!`), since
//! it reads the start of one text. Review catches those.

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::rust_source::{is_test_module_file, without_tests};
use crate::source::read_text;

/// The folders directly under the root of a Unix system (and of macOS). A text that starts with
/// a slash and one of these names is a path of the system.
const SYSTEM_ROOTS: [&str; 27] = [
    "bin",
    "boot",
    "dev",
    "etc",
    "home",
    "lib",
    "lib64",
    "mnt",
    "opt",
    "proc",
    "root",
    "run",
    "sbin",
    "srv",
    "sys",
    "tmp",
    "usr",
    "var",
    "media",
    "nix",
    "snap",
    "Users",
    "Windows",
    "Library",
    "System",
    "private",
    "Applications",
];

/// The one list of files that are not read, each with why. A part of a path.
const LEFT_OUT: [(&str, &str); 8] = [
    ("/tests/", "tests use paths as data"),
    ("/fixtures/", "examples of the checks"),
    ("/testdata/", "test data"),
    ("/e2e/", "browser tests"),
    (".test.", "tests use paths as data"),
    (".spec.", "tests use paths as data"),
    ("/testutil.rs", "helpers of tests"),
    (
        "/sec_private/patterns.rs",
        "SEC-PRIVATE searches text for the start of a profile path of a person: a detection pattern, not a path that is used",
    ),
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
    is_test_module_file(path) || LEFT_OUT.iter().any(|(part, _)| slashed.contains(part))
}

/// Whether the text starts with a path of the system.
fn starts_with_system_path(rest: &str) -> bool {
    // `\\?\C:\…`, the long form of a drive path.
    let rest = match rest.trim_start_matches('\\').strip_prefix('?') {
        Some(long) => long.trim_start_matches(['\\', '?']),
        None => rest,
    };
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
                    None | Some('/' | '"' | '\'' | '`' | ' ' | ')')
                )
        }
        (Some(drive), Some(':')) if drive.is_ascii_alphabetic() => {
            matches!(characters.next(), Some('\\' | '/'))
        }
        _ => false,
    }
}

/// Whether the file is a script, where paths are written without quotes.
fn is_script(path: &str) -> bool {
    ["sh", "cmd", "ps1"]
        .iter()
        .any(|ending| has_extension(path, ending))
}

/// Whether a line of a script is a comment (`#`, `rem`, `::`).
fn is_script_comment(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with('#')
        || line.starts_with("::")
        || line
            .get(..4)
            .is_some_and(|word| word.eq_ignore_ascii_case("rem "))
}

/// The line of a script without a trailing `#` comment (a `#` after a space, outside quotes).
fn without_trailing_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut previous = ' ';
    for (at, c) in line.char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            None if matches!(c, '"' | '\'') => quote = Some(c),
            None if c == '#' && previous.is_whitespace() => return &line[..at],
            _ => {}
        }
        previous = c;
    }
    line
}

/// PowerShell text without its `<# … #>` blocks (new lines stay, so line numbers still fit).
fn without_block_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<#") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("#>").map_or(after.len(), |at| at + 2);
        out.extend(after[..end].chars().filter(|c| *c == '\n'));
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// The places in a line where a path may start: after a quote, and in a script also at the start
/// and after a space, `=` or `(`.
fn starts_in(code: &str, script: bool) -> Vec<usize> {
    let mut starts = Vec::new();
    if script {
        starts.push(0);
    }
    for (at, c) in code.char_indices() {
        if matches!(c, '"' | '\'' | '`') || (script && matches!(c, ' ' | '\t' | '=' | '(')) {
            starts.push(at + c.len_utf8());
        }
    }
    starts
}

/// The problems in one file. In Rust the code behind `#[cfg(test)]` is blanked out first.
fn findings_in(path: &str, text: &str) -> Vec<Finding> {
    let text = if has_extension(path, "rs") {
        without_tests(text)
    } else {
        text.to_owned()
    };
    let text = if has_extension(path, "ps1") {
        without_block_comments(&text)
    } else {
        text
    };
    let script = is_script(path);
    let mut findings = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if script && is_script_comment(line) {
            continue;
        }
        let code = if script && !has_extension(path, "cmd") {
            without_trailing_comment(line)
        } else {
            code_of(line)
        };
        for at in starts_in(code, script) {
            let rest = &code[at..];
            if starts_with_system_path(rest) {
                let shown: String = rest
                    .chars()
                    .take_while(|c| !matches!(c, '"' | '\'' | '`' | ' ' | ')'))
                    .take(40)
                    .collect();
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
    fn bad_example_more_roots_and_the_long_form_of_a_drive() {
        assert_eq!(bad("crates/a/src/lib.rs", "let p = \"/Library/x\";\n"), 1);
        assert_eq!(bad("crates/a/src/lib.rs", "let p = \"/snap/bin\";\n"), 1);
        assert_eq!(bad("crates/a/src/lib.rs", "let p = \"/nix/store\";\n"), 1);
        let long = format!("let p = \"{0}{0}?{0}C:{0}x\";", '\\');
        assert_eq!(bad("crates/a/src/lib.rs", &long), 1);
    }

    #[test]
    fn bad_example_a_script_writes_a_path_without_quotes() {
        assert_eq!(bad("tools/dev/a.sh", "cp x /usr/local/bin\n"), 1);
        assert_eq!(bad("tools/dev/a.sh", "DIR=/var/lib/x\n"), 1);
        assert_eq!(bad("tools/dev/a.sh", "(cd /opt/x && ls)\n"), 1);
        assert_eq!(bad("tools/dev/a.cmd", "set P=C:\\Windows\\System32\n"), 1);
        assert_eq!(bad("tools/dev/a.cmd", "call D:\\x\\y.bat\n"), 1);
        assert_eq!(bad("tools/dev/a.ps1", "$p = (Get-Item C:/data)\n"), 1);
        assert_eq!(bad("tools/dev/a.ps1", "& /usr/bin/pwsh -c x\n"), 1);
    }

    #[test]
    fn good_example_a_script_with_comments_and_paths_of_the_repository() {
        let script = "# cp x /usr/local/bin\nrem set P=C:\\Windows\n:: /etc/x\ncd tools/dev\necho see /about\n";
        assert_eq!(bad("tools/dev/a.sh", script), 0);
    }

    #[test]
    fn good_example_trailing_and_block_comments_in_scripts() {
        let sh = "cp x y # then /etc/x\nrun \"a # b\" ok\n";
        assert_eq!(bad("tools/dev/a.sh", sh), 0);
        let ps = "<#\n  Copies to /usr/local/bin\n  and C:/data\n#>\nGet-Item x # was /etc/y\n";
        assert_eq!(bad("tools/dev/a.ps1", ps), 0);
        // After the block, code counts again, with the right line.
        let after = "<# note #>\nGet-Item /etc/x\n";
        let found = findings_in("tools/dev/a.ps1", after);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location.as_deref(), Some("tools/dev/a.ps1:2"));
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
    fn a_detection_pattern_is_on_the_one_left_out_list_and_nowhere_else() {
        assert!(is_left_out("xtask/src/checks/sec_private/patterns.rs"));
        assert_eq!(
            bad("crates/a/src/lib.rs", "if p.ends_with(\"/var\") {}\n"),
            1
        );
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
