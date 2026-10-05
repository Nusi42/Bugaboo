//! TEST-SAMPLE-DATA: the sample organization builds from its seed, is the same on both
//! databases, and contains only fictional data (architecture §11, ADR-0023).
//!
//! `cargo xtask dataset sample --check` makes the organization four times with the `bb-sample`
//! program (twice on SQLite, twice on PostgreSQL, one seed) into one folder per run, and hands the
//! folder to [`problems`]. Like the database parity record, that run decides: `check all` does not
//! make the organization (it takes a few seconds and a database), it makes sure the pieces are in
//! place and `cargo xtask ci` runs the real check.
//!
//! One folder per run, as `bb-sample` leaves it:
//!
//! ```text
//! <run>/dump.json    the canonical dump (IDs, hashes and secrets left out by design)
//! <run>/sign-in.txt  e-mail address, password, role, name and code-app secret of every person: the
//!                    only file that may hold a secret
//! <run>/output.txt   what the program said; <run>/data/ the data folder it made
//! ```
//!
//! The rules (people only; WP-13 adds projects, groups and roles):
//!
//! 1. every run made the same dump, so the organization is the same from the same seed and on
//!    both databases;
//! 2. every e-mail address in a dump is under `bugaboo.example`, which can never reach a real
//!    inbox;
//! 3. every name is a given name and a family name of the list written for Bugaboo
//!    (`crates/bb-sample/data/names.toml`), so no real person is in it;
//! 4. no password or code-app secret of `sign-in.txt` appears in any other file of a run.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;

use super::{Check, Context, Finding};

/// The list of names, written for Bugaboo. `bb-sample` reads the same file.
pub(crate) const NAMES_FILE: &str = "crates/bb-sample/data/names.toml";
/// The only domain a sample address may have (`bb-sample` has the same constant; a test keeps both
/// in step).
const EMAIL_DOMAIN: &str = "bugaboo.example";
/// The file of a run that may hold secrets.
const SIGN_IN_FILE: &str = "sign-in.txt";
/// The canonical dump of a run.
const DUMP_FILE: &str = "dump.json";
/// The step of `cargo xtask ci` that runs the real check.
const CI_SOURCE: &str = "xtask/src/ci.rs";
const CI_STEP: &str = r#"&["dataset", "sample", "--check"]"#;

pub(crate) struct TestSampleData;

impl Check for TestSampleData {
    fn id(&self) -> &'static str {
        "TEST-SAMPLE-DATA"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        let names = fs::read_to_string(context.repo.path(NAMES_FILE))
            .with_context(|| format!("could not read {NAMES_FILE}"))?;
        if let Err(problem) = NameList::parse(&names) {
            findings.push(Finding::at(NAMES_FILE, problem));
        }
        let ci = fs::read_to_string(context.repo.path(CI_SOURCE)).unwrap_or_default();
        if !ci.contains(CI_STEP) {
            findings.push(Finding::at(
                CI_SOURCE,
                format!(
                    "`cargo xtask ci` has to run `{CI_STEP}`, so the sample organization is made twice on each database and compared."
                ),
            ));
        }
        Ok(findings)
    }
}

/// Given names and family names of `names.toml`.
#[derive(Deserialize)]
pub(crate) struct NameList {
    first: Vec<String>,
    last: Vec<String>,
}

impl NameList {
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let list: Self =
            toml::from_str(text).map_err(|error| format!("not a list of names: {error}"))?;
        if list.first.is_empty() || list.last.is_empty() {
            return Err("the list has no first names or no last names".to_owned());
        }
        Ok(list)
    }

    fn contains(&self, name: &str) -> bool {
        name.split_once(' ').is_some_and(|(first, last)| {
            self.first.iter().any(|known| known == first)
                && self.last.iter().any(|known| known == last)
        })
    }
}

/// What one run of the program left in its folder.
pub(crate) struct Run {
    pub(crate) label: String,
    dump: String,
    /// What must not show up outside the sign-in file: the passwords, the secrets of the code
    /// apps as text, and the raw bytes those secrets stand for.
    needles: Vec<Vec<u8>>,
    /// Every other file of the run: the places a secret must not be.
    scanned: Vec<(String, Vec<u8>)>,
}

impl Run {
    /// Reads the folder of one run.
    pub(crate) fn read(label: &str, folder: &Path) -> anyhow::Result<Self> {
        let read = |name: &str| {
            fs::read_to_string(folder.join(name))
                .with_context(|| format!("{label}: could not read {name}"))
        };
        let mut scanned = Vec::new();
        collect(folder, folder, &mut scanned)?;
        Ok(Self {
            label: label.to_owned(),
            dump: read(DUMP_FILE)?,
            needles: needles_of(&read(SIGN_IN_FILE)?),
            scanned,
        })
    }
}

/// What the text of a sign-in file says must stay inside it: lines of tab-separated columns
/// (address, password, role, name, secret), `#` for comments. The secret of a code app is written
/// in base 32; a file that stores it as the 20 bytes it stands for leaks it as well, so those
/// bytes are looked for too.
fn needles_of(text: &str) -> Vec<Vec<u8>> {
    let mut needles = Vec::new();
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        let columns: Vec<&str> = line.split('\t').collect();
        if let Some(password) = columns.get(1).filter(|password| !password.is_empty()) {
            needles.push(password.as_bytes().to_vec());
        }
        if let Some(secret) = columns.get(4).filter(|secret| !secret.is_empty()) {
            needles.push(secret.as_bytes().to_vec());
            needles.extend(base32_decode(secret).filter(|raw| !raw.is_empty()));
        }
    }
    needles
}

/// The bytes a base 32 text (RFC 4648, as code apps take it) stands for; `None` for any other
/// character.
fn base32_decode(text: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bytes = Vec::new();
    let (mut bits, mut count) = (0_u32, 0_u32);
    for symbol in text.trim_end_matches('=').bytes() {
        let value = ALPHABET
            .iter()
            .position(|known| *known == symbol.to_ascii_uppercase())?;
        bits = (bits << 5) | u32::try_from(value).ok()?;
        count += 5;
        if count >= 8 {
            count -= 8;
            bytes.push(u8::try_from((bits >> count) & 0xFF).ok()?);
        }
    }
    Some(bytes)
}

/// Every file below `folder` except the sign-in file of the run itself, with its bytes.
fn collect(root: &Path, folder: &Path, found: &mut Vec<(String, Vec<u8>)>) -> anyhow::Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(folder)
        .with_context(|| format!("could not read {}", folder.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(root, &path, found)?;
        } else if path != root.join(SIGN_IN_FILE) {
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace('\\', "/");
            found.push((
                name,
                fs::read(&path).with_context(|| format!("could not read {}", path.display()))?,
            ));
        }
    }
    Ok(())
}

/// What is wrong with the runs of one seed, in plain words; empty when the organization is the
/// same every time and fictional through and through.
pub(crate) fn problems(names: &NameList, runs: &[Run]) -> Vec<String> {
    let mut problems = Vec::new();
    if runs.len() < 2 {
        problems.push("Fewer than two runs: nothing to compare the dumps with.".to_owned());
    }
    let mut dumps: Vec<(&str, Value)> = Vec::new();
    for run in runs {
        match serde_json::from_str::<Value>(&run.dump) {
            Ok(dump) => dumps.push((&run.label, dump)),
            Err(error) => problems.push(format!("{}: the dump is not JSON: {error}.", run.label)),
        }
    }
    if let Some((first_label, first)) = dumps.first() {
        for (label, dump) in &dumps[1..] {
            if let Some(place) = difference(first, dump, "") {
                problems.push(format!(
                    "The dump of {label} differs from the dump of {first_label} (first at {place}). The same seed has to make the same organization on every run and on both databases."
                ));
            }
        }
    }
    for (label, dump) in &dumps {
        fictional(names, label, dump, &mut problems);
    }
    for run in runs {
        for (file, bytes) in &run.scanned {
            if run.needles.iter().any(|needle| contains(bytes, needle)) {
                problems.push(format!(
                    "{}: {file} holds a password or secret. They belong only in the ignored file {SIGN_IN_FILE}.",
                    run.label
                ));
            }
        }
    }
    problems
}

/// Rules 2 and 3 for one dump.
fn fictional(names: &NameList, label: &str, dump: &Value, problems: &mut Vec<String>) {
    let people = dump["people"].as_array().map_or(&[][..], Vec::as_slice);
    if people.is_empty() {
        problems.push(format!("{label}: the dump has no people."));
    }
    let mut check_address = |address: &str| {
        let under = address
            .rsplit_once('@')
            .is_some_and(|(_, domain)| domain == EMAIL_DOMAIN);
        if !under {
            problems.push(format!(
                "{label}: the address {address} is not under {EMAIL_DOMAIN}. Only addresses of the reserved example domain can never reach a real inbox."
            ));
        }
    };
    for person in people {
        check_address(person["email"].as_str().unwrap_or(""));
    }
    for event in dump["events"].as_array().map_or(&[][..], Vec::as_slice) {
        for change in event["changes"].as_array().map_or(&[][..], Vec::as_slice) {
            if change["field"] == "email" {
                check_address(change["after"].as_str().unwrap_or(""));
            }
        }
    }
    for person in people {
        let name = person["name"].as_str().unwrap_or("");
        if !names.contains(name) {
            problems.push(format!(
                "{label}: the name {name} is not a given name and a family name of {NAMES_FILE}. Only names from the list written for Bugaboo are used."
            ));
        }
    }
}

/// Where two JSON values first differ, as a path like `/people/3/email`.
fn difference(left: &Value, right: &Value, path: &str) -> Option<String> {
    match (left, right) {
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            keys.into_iter()
                .find_map(|key| match (a.get(key), b.get(key)) {
                    (Some(x), Some(y)) => difference(x, y, &format!("{path}/{key}")),
                    _ => Some(format!("{path}/{key}")),
                })
        }
        (Value::Array(a), Value::Array(b)) => {
            (0..a.len().max(b.len())).find_map(|at| match (a.get(at), b.get(at)) {
                (Some(x), Some(y)) => difference(x, y, &format!("{path}/{at}")),
                _ => Some(format!("{path}/{at}")),
            })
        }
        _ => (left != right).then(|| {
            if path.is_empty() {
                "/".to_owned()
            } else {
                path.to_owned()
            }
        }),
    }
}

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && bytes.windows(needle.len()).any(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: &str = "../tools/checks/fixtures/TEST-SAMPLE-DATA";

    fn names() -> NameList {
        let text = include_str!("../../../crates/bb-sample/data/names.toml");
        NameList::parse(text).unwrap()
    }

    /// The runs of one fixture: a folder with one sub-folder per run, as `bb-sample` leaves them.
    fn fixture(kind: &str, case: &str) -> Vec<Run> {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURES)
            .join(kind)
            .join(case);
        let mut labels: Vec<String> = fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        labels.sort();
        labels
            .iter()
            .map(|label| Run::read(label, &folder.join(label)).unwrap())
            .collect()
    }

    fn found(case: &str) -> Vec<String> {
        problems(&names(), &fixture("bad", case))
    }

    #[test]
    fn good_example() {
        assert_eq!(
            problems(&names(), &fixture("good", "the-same-everywhere")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn bad_example_an_address_outside_the_example_domain() {
        let found = found("address-outside-the-example-domain");
        assert!(
            found.iter().any(|p| p.contains("ana.moreno@gmail.test")),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|p| p.contains("is not under bugaboo.example"))
        );
    }

    #[test]
    fn bad_example_two_dumps_that_differ() {
        let found = found("dumps-that-differ");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("differs"), "{found:?}");
        assert!(
            found[0].contains("/people/1/email"),
            "names the place: {found:?}"
        );
    }

    #[test]
    fn bad_example_a_name_that_is_not_on_the_list() {
        let found = found("name-not-on-the-list");
        assert!(
            found.iter().any(|p| p.contains("Zaphod Beeblebrox")),
            "{found:?}"
        );
    }

    #[test]
    fn bad_example_a_secret_outside_the_sign_in_file() {
        let found = found("secret-outside-the-sign-in-file");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("data/logs/security-head.log"),
            "{found:?}"
        );
        assert!(found[0].contains("holds a password or secret"));
    }

    #[test]
    fn bad_example_a_secret_in_the_data_of_the_postgresql_database() {
        let found = found("secret-in-the-postgres-dump");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("postgres-1: postgres.sql holds"),
            "{found:?}"
        );
    }

    #[test]
    fn bad_example_no_dump_to_compare_and_no_people() {
        assert!(!problems(&names(), &[]).is_empty());
        let empty = Run {
            label: "only".to_owned(),
            dump: "{}".to_owned(),
            needles: Vec::new(),
            scanned: Vec::new(),
        };
        let found = problems(&names(), &[empty]);
        assert!(found.iter().any(|p| p.contains("Fewer than two runs")));
        assert!(found.iter().any(|p| p.contains("no people")));
    }

    /// A fixture file that Git ignores is missing from every fresh clone, and the bad example that
    /// needed it then finds nothing: so none of them may be ignored (`*.db` is, for one).
    #[test]
    fn no_fixture_file_is_ignored_by_git() {
        use std::io::Write as _;
        use std::process::{Command, Stdio};

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let mut files = Vec::new();
        let fixtures = root.join("tools/checks/fixtures/TEST-SAMPLE-DATA");
        collect(root, &fixtures, &mut files).unwrap();
        let mut child = Command::new("git")
            .args(["check-ignore", "--stdin"])
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        for (name, _) in &files {
            writeln!(stdin, "{name}").unwrap();
        }
        drop(stdin);
        let ignored = String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap();
        assert!(ignored.trim().is_empty(), "ignored by Git: {ignored}");
    }

    #[test]
    fn the_needles_are_the_password_and_the_code_app_column_only() {
        let text = "# header\nana@bugaboo.example\tPW-ONE\tcontributor\tAna Moreno\nben@bugaboo.example\tPW-TWO\tteam_member\tBen Hale\tMZXW6YTBOI\n";
        let needles = needles_of(text);
        assert_eq!(
            needles,
            [
                b"PW-ONE".to_vec(),
                b"PW-TWO".to_vec(),
                b"MZXW6YTBOI".to_vec(),
                b"foobar".to_vec()
            ]
        );
    }

    #[test]
    fn base_32_is_decoded_the_way_code_apps_read_it() {
        // RFC 4648 test vectors.
        assert_eq!(base32_decode("MZXW6YTBOI").unwrap(), b"foobar");
        assert_eq!(base32_decode("mzxw6ytboi======").unwrap(), b"foobar");
        assert_eq!(base32_decode("").unwrap(), b"");
        assert!(base32_decode("not base 32!").is_none());
    }

    #[test]
    fn bad_example_the_raw_bytes_of_a_code_app_secret_in_a_file() {
        // `MZXW6YTBOI` is the text of `foobar`; a file may keep the bytes instead of the text.
        let sign_in = "ana@bugaboo.example\tPW-ONE\tcontributor\tAna Moreno\tMZXW6YTBOI\n";
        let leaking = Run {
            label: "only".to_owned(),
            dump: "{}".to_owned(),
            needles: needles_of(sign_in),
            scanned: vec![("data/keys.bin".to_owned(), b"header foobar".to_vec())],
        };
        let found = problems(&names(), &[leaking]);
        assert!(
            found.iter().any(|p| p.contains("data/keys.bin holds")),
            "{found:?}"
        );
    }

    #[test]
    fn the_domain_is_the_one_bb_sample_makes_its_addresses_with() {
        let roster = include_str!("../../../crates/bb-sample/src/roster.rs");
        assert!(
            roster.contains(&format!(
                "pub const EMAIL_DOMAIN: &str = \"{EMAIL_DOMAIN}\";"
            )),
            "bb-sample and this check name different domains"
        );
    }

    #[test]
    fn the_real_names_file_and_ci_step_are_in_place() {
        let repo = crate::repo::Repo::discover().unwrap();
        let catalog = crate::catalog::Catalog::load(&repo).unwrap();
        let context = Context {
            repo: &repo,
            catalog: &catalog,
            files: None,
        };
        let findings = TestSampleData.run(&context).unwrap();
        let messages: Vec<&String> = findings.iter().map(|f| &f.message).collect();
        assert!(findings.is_empty(), "{messages:?}");
    }
}
