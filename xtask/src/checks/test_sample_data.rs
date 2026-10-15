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
//! The rules:
//!
//! 1. every run made the same dump, so the organization is the same from the same seed and on
//!    both databases;
//! 2. every e-mail address in a dump is under `bugaboo.example`, which can never reach a real
//!    inbox;
//! 3. every name is a given name and a family name of the list written for Bugaboo
//!    (`crates/bb-sample/data/names.toml`), so no real person is in it;
//! 4. no password, code-app secret or access key of `sign-in.txt` appears in any other file of a
//!    run;
//! 5. the dump has the eight groups and the four projects with their sub-projects of
//!    architecture §11 (their names are written once, in `tools/dev/sample.json`);
//! 6. every role of the mix of architecture §11 (2 Administrators, 4 Project leads, 30 Team
//!    members, 8 Contributors, 4 Viewers, 2 Guests) is in the dump and held by that many people;
//! 7. every project has a Project lead, its own or one from a project above it;
//! 8. the issues of the dump are the issues of the plan in `tools/dev/sample.json`, scaled to the
//!    number the run was asked to make and within its tolerance: per project, place and type,
//!    the relations of every kind with a chain deeper than four steps and one circle, the
//!    comments (some edited), tags, labels, values of every own field, sub-issues, one edit lock
//!    held and one draft waiting (the judging itself is `IssuePlan::problems` of `bb-dev-words`,
//!    which the tests of `bb-sample` ask as well)).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;

use super::{Check, Context, Finding};
use crate::dataset::load_layout;
use crate::source::read_text;
use bb_core::text::CROCKFORD;
use bb_dev_words::sample::{IssuePlan, Layout, OrganizationNames};

/// The list of names, written for Bugaboo. `bb-sample` reads the same file.
pub(crate) const NAMES_FILE: &str = "crates/bb-sample/data/names.toml";
/// The only domain a sample address may have. Written out here on purpose, apart from the program
/// that makes the addresses: the check is an independent witness, and fails if the program ever
/// leaves this domain.
const EMAIL_DOMAIN: &str = "bugaboo.example";
/// The mix of roles of the 50 people (architecture §11), by the key of the starting role. Written
/// out here on purpose, apart from the program that makes the people: the check is an independent
/// witness, and fails if the program ever leaves the mix.
const ROLE_MIX: [(&str, u64); 6] = [
    ("administrator", 2),
    ("project-lead", 4),
    ("team-member", 30),
    ("contributor", 8),
    ("viewer", 4),
    ("guest", 2),
];
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
        load_layout(context.repo.root())?;
        let names = read_text(context.repo, NAMES_FILE)?;
        if let Err(problem) = NameList::parse(&names) {
            findings.push(Finding::at(NAMES_FILE, problem));
        }
        let ci = read_text(context.repo, CI_SOURCE).unwrap_or_default();
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
    /// The name of the one file that may hold secrets, for the messages.
    sign_in_file: String,
    dump: String,
    /// What must not show up outside the sign-in file: the passwords, the secrets of the code
    /// apps as text, and the raw bytes those secrets stand for.
    needles: Vec<Vec<u8>>,
    /// Every other file of the run: the places a secret must not be.
    scanned: Vec<(String, Vec<u8>)>,
}

impl Run {
    /// Reads the folder of one run.
    pub(crate) fn read(label: &str, folder: &Path, layout: &Layout) -> anyhow::Result<Self> {
        let read = |name: &str| {
            fs::read_to_string(folder.join(name))
                .with_context(|| format!("{label}: could not read {name}"))
        };
        let mut scanned = Vec::new();
        collect(folder, folder, &layout.sign_in_file, &mut scanned)?;
        Ok(Self {
            label: label.to_owned(),
            sign_in_file: layout.sign_in_file.clone(),
            dump: read(&layout.dump_file)?,
            needles: needles_of(&read(&layout.sign_in_file)?),
            scanned,
        })
    }
}

/// What the text of a sign-in file says must stay inside it: lines of tab-separated columns
/// (address, password, role, name, secret of the code app, access key), `#` for comments. The secret of a code app is written
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
        // The access key of the few people who have one: `bb_<prefix>_<secret>`. A file may keep
        // the secret alone (the part after the second `_`), or the 32 bytes it stands for.
        if let Some(key) = columns.get(5).filter(|key| !key.is_empty()) {
            needles.push(key.as_bytes().to_vec());
            if let Some(secret) = key.rsplit_once('_').map(|(_, secret)| secret) {
                needles.push(secret.as_bytes().to_vec());
                needles.extend(crockford_decode(secret).filter(|raw| !raw.is_empty()));
            }
        }
    }
    needles
}

/// The bytes a base 32 text (RFC 4648, as code apps take it) stands for; `None` for any other
/// character.
fn base32_decode(text: &str) -> Option<Vec<u8>> {
    decode_base32(
        text.trim_end_matches('='),
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
    )
}

/// The bytes the secret of an access key stands for: Crockford's base 32 (`bb-auth` writes it in
/// lower case, which the decoding does not tell apart); `None` for any other character.
fn crockford_decode(text: &str) -> Option<Vec<u8>> {
    decode_base32(text, CROCKFORD)
}

/// The bytes a base 32 text stands for, written with the 32 symbols of `alphabet`.
fn decode_base32(text: &str, alphabet: &[u8; 32]) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let (mut bits, mut count) = (0_u32, 0_u32);
    for symbol in text.bytes() {
        let value = alphabet
            .iter()
            .position(|known| known.eq_ignore_ascii_case(&symbol))?;
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
fn collect(
    root: &Path,
    folder: &Path,
    sign_in_file: &str,
    found: &mut Vec<(String, Vec<u8>)>,
) -> anyhow::Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(folder)
        .with_context(|| format!("could not read {}", folder.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(root, &path, sign_in_file, found)?;
        } else if path != root.join(sign_in_file) {
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
pub(crate) fn problems(
    names: &NameList,
    organization: &OrganizationNames,
    issues: Option<(&IssuePlan, u32)>,
    runs: &[Run],
) -> Vec<String> {
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
        the_organization(organization, label, dump, &mut problems);
        if let Some((plan, asked)) = issues {
            problems.extend(plan.problems(asked, label, dump));
        }
    }
    for run in runs {
        for (file, bytes) in &run.scanned {
            if run.needles.iter().any(|needle| contains(bytes, needle)) {
                problems.push(format!(
                    "{}: {file} holds a password or secret. They belong only in the ignored file {}.",
                    run.label,
                    run.sign_in_file
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

/// Rules 5 to 7 for one dump.
fn the_organization(
    organization: &OrganizationNames,
    label: &str,
    dump: &Value,
    problems: &mut Vec<String>,
) {
    let listed = |key: &str, member: &str| -> Vec<String> {
        dump[key]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter_map(|item| item[member].as_str().map(str::to_owned))
            .collect()
    };
    let groups = listed("groups", "name");
    for name in organization.groups.values() {
        if !groups.contains(name) {
            problems.push(format!(
                "{label}: the group {name} is not in the dump. The sample has the groups of architecture §11."
            ));
        }
    }
    let roles = dump["roles"].as_array().map_or(&[][..], Vec::as_slice);
    for (key, wanted) in ROLE_MIX {
        let found = roles.iter().find(|role| role["starting"] == key);
        match found.map(|role| role["held_by"].as_u64()) {
            None => problems.push(format!(
                "{label}: the starting role {key} is not in the dump. The sample has the mix of roles of architecture §11."
            )),
            Some(held) if held != Some(wanted) => problems.push(format!(
                "{label}: the starting role {key} is held by {} people, not {wanted}. The sample has the mix of roles of architecture §11.",
                held.map_or_else(|| "no".to_owned(), |held| held.to_string())
            )),
            Some(_) => {}
        }
    }
    let projects = listed("projects", "short_name");
    for project in &organization.projects {
        let inside = project.sub_projects.iter().map(|sub| &sub.short_name);
        for short_name in std::iter::once(&project.short_name).chain(inside) {
            if !projects.contains(short_name) {
                problems.push(format!(
                    "{label}: the project {short_name} is not in the dump. The sample has the projects of architecture §11."
                ));
            }
        }
    }
    for project in dump["projects"].as_array().map_or(&[][..], Vec::as_slice) {
        let has_lead = project["leads"]
            .as_array()
            .is_some_and(|leads| !leads.is_empty());
        if !has_lead {
            problems.push(format!(
                "{label}: the project {} has no Project lead. Every project of the sample has one.",
                project["short_name"].as_str().unwrap_or("?")
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

    fn layout() -> Layout {
        load_layout(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()).unwrap()
    }

    fn names() -> NameList {
        let text = include_str!("../../../crates/bb-sample/data/names.toml");
        NameList::parse(text).unwrap()
    }

    fn organization() -> OrganizationNames {
        layout().organization
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
            .map(|label| Run::read(label, &folder.join(label), &layout()).unwrap())
            .collect()
    }

    fn found(case: &str) -> Vec<String> {
        problems(&names(), &organization(), None, &fixture("bad", case))
    }

    #[test]
    fn good_example() {
        assert_eq!(
            problems(
                &names(),
                &organization(),
                None,
                &fixture("good", "the-same-everywhere")
            ),
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
    fn bad_example_a_project_without_a_lead() {
        let found = found("a-project-without-a-lead");
        assert!(
            found
                .iter()
                .any(|p| p.contains("the project WEB has no Project lead")),
            "{found:?}"
        );
        // Every run says it, and nothing else is wrong.
        assert_eq!(found.len(), 2, "{found:?}");
    }

    #[test]
    fn bad_example_a_role_of_the_mix_is_missing_or_held_by_too_few() {
        let found = found("a-role-of-the-mix-is-missing");
        assert!(
            found
                .iter()
                .any(|p| p.contains("the starting role guest is not in the dump")),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|p| p.contains("contributor is held by 7 people, not 8")),
            "{found:?}"
        );
    }

    #[test]
    fn bad_example_a_group_or_a_project_of_the_organization_is_missing() {
        let found = found("a-group-is-missing");
        assert!(
            found
                .iter()
                .any(|p| p.contains("the group External is not in the dump")),
            "{found:?}"
        );
        let found = self::found("a-project-is-missing");
        assert!(
            found
                .iter()
                .any(|p| p.contains("the project BLOG is not in the dump")),
            "{found:?}"
        );
    }

    #[test]
    fn bad_example_an_access_key_outside_the_sign_in_file() {
        let found = found("an-access-key-outside-the-sign-in-file");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("data/logs/security-head.log"),
            "{found:?}"
        );
    }

    #[test]
    fn bad_example_no_dump_to_compare_and_no_people() {
        assert!(!problems(&names(), &organization(), None, &[]).is_empty());
        let empty = Run {
            label: "only".to_owned(),
            sign_in_file: layout().sign_in_file,
            dump: "{}".to_owned(),
            needles: Vec::new(),
            scanned: Vec::new(),
        };
        let found = problems(&names(), &organization(), None, &[empty]);
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
        collect(root, &fixtures, &layout().sign_in_file, &mut files).unwrap();
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
    fn the_needles_are_the_password_the_code_app_and_the_access_key_columns_only() {
        let text = "# header\nana@bugaboo.example\tPW-ONE\tcontributor\tAna Moreno\nben@bugaboo.example\tPW-TWO\tteam-member\tBen Hale\tMZXW6YTBOI\ncleo@bugaboo.example\tPW-THREE\tteam-member\tCleo Coder\t\tbb_k7q2x9mf_cpnqj\n";
        let needles = needles_of(text);
        assert_eq!(
            needles,
            [
                b"PW-ONE".to_vec(),
                b"PW-TWO".to_vec(),
                b"MZXW6YTBOI".to_vec(),
                b"foobar".to_vec(),
                b"PW-THREE".to_vec(),
                b"bb_k7q2x9mf_cpnqj".to_vec(),
                b"cpnqj".to_vec(),
                // The 25 bits of the five symbols are three bytes and a spare bit.
                vec![0x65, 0xAB, 0x79],
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
            sign_in_file: layout().sign_in_file,
            dump: "{}".to_owned(),
            needles: needles_of(sign_in),
            scanned: vec![("data/keys.bin".to_owned(), b"header foobar".to_vec())],
        };
        let found = problems(&names(), &organization(), None, &[leaking]);
        assert!(
            found.iter().any(|p| p.contains("data/keys.bin holds")),
            "{found:?}"
        );
    }

    #[test]
    fn bad_example_the_secret_part_or_the_bytes_of_an_access_key_in_a_file() {
        // `bb_<prefix>_<secret>`: a file may keep the secret alone, or the 32 bytes it stands for.
        // 52 symbols of the alphabet, made here and not written out: the secret scan would
        // (rightly) take a written key for a real one.
        let secret: String = (0..51)
            .map(|at| char::from(CROCKFORD[(at * 7 + 3) % 32]).to_ascii_lowercase())
            .chain(['0'])
            .collect();
        let sign_in = format!(
            "ana@bugaboo.example\tPW-ONE\tcontributor\tAna Moreno\t\tbb_FIXTURE1_{secret}\n"
        );
        let bytes = crockford_decode(&secret).unwrap();
        assert_eq!(bytes.len(), 32);
        for (file, content) in [
            ("data/log.txt", format!("key part {secret}").into_bytes()),
            ("data/keys.bin", [b"header ".as_slice(), &bytes].concat()),
        ] {
            let leaking = Run {
                label: "only".to_owned(),
                sign_in_file: layout().sign_in_file,
                dump: "{}".to_owned(),
                needles: needles_of(&sign_in),
                scanned: vec![(file.to_owned(), content)],
            };
            let found = problems(&names(), &organization(), None, &[leaking]);
            assert!(
                found.iter().any(|p| p.contains(&format!("{file} holds"))),
                "{file}: {found:?}"
            );
        }
    }

    #[test]
    fn the_secret_of_an_access_key_is_decoded_the_way_bb_auth_writes_it() {
        // Crockford's base 32 in lower case: no i, l, o or u.
        assert_eq!(crockford_decode("cpnqj").unwrap(), [0x65, 0xAB, 0x79]);
        assert!(crockford_decode("cpnuj").is_none());
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
