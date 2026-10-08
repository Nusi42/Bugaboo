//! LINT-DRY: no copy-pasted block of code above the size that `.jscpd.json` names
//! (coding-standards §1, DRY).
//!
//! `jscpd` reads the Rust and TypeScript that the `files` of the catalogue entry cover, without the
//! files that a generator writes (`generate::all_paths`), and finds a block of tokens that stands
//! twice. The imports (`use …;`, `import …;`) are left out of the comparison: a list of what a file
//! needs is no knowledge that is kept twice. A block is a finding unless the two files are named
//! in `tools/checks/dry-exemptions.toml` with a reason that names the follow-up that removes the
//! copy; an entry that no copy needs any more fails, so the file shrinks as the copies go.
//!
//! The size of a block (`minTokens`, `minLines`) is in `.jscpd.json`, the one place that says it.
//! The run is part of `cargo xtask ci` only, as the catalogue says: it reads every source file.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::{Value, json};

use super::exemptions::{self, Exemption as _, Used, names_a_follow_up};
use super::node_tool::run;
use super::{Check, Context, Finding};
use crate::generate::all_paths;
use crate::repo::Repo;

/// The settings of the tool that do not depend on the files: the size of a block, what to leave
/// out of the comparison.
const SETTINGS_PATH: &str = ".jscpd.json";
/// Where the run writes its configuration and its report, below the repository.
const WORK_FOLDER: &str = "target/lint-dry";
/// The report that the tool writes into [`WORK_FOLDER`].
const REPORT_FILE: &str = "jscpd-report.json";
/// The copies that stand for now, each with the follow-up that removes it.
const EXEMPTIONS_PATH: &str = "tools/checks/dry-exemptions.toml";
/// The format of the tool for a file ending.
const FORMATS: [(&str, &str); 3] = [("rs", "rust"), ("ts", "typescript"), ("tsx", "tsx")];

pub(crate) struct LintDry;

impl Check for LintDry {
    fn id(&self) -> &'static str {
        "LINT-DRY"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let generated = all_paths();
        let covered = context
            .everything()
            .selected(self.id())?
            .into_iter()
            .filter(|file| !generated.contains(&file.as_str()))
            .collect::<Vec<_>>();
        let clones = find_clones(repo, &covered)?;
        let exemptions: Exemptions = exemptions::load(repo, EXEMPTIONS_PATH)?;
        Ok(judge(&clones, &exemptions.exempt))
    }
}

/// The file of copies that stand for now.
#[derive(Deserialize)]
struct Exemptions {
    #[serde(default)]
    exempt: Vec<Exempt>,
}

/// Two files that hold the same block, and why that may stand for now.
#[derive(Deserialize)]
struct Exempt {
    files: [String; 2],
    reason: String,
}

impl exemptions::Exemption for Exempt {
    fn name(&self) -> String {
        format!("{} and {}", self.files[0], self.files[1])
    }

    fn reason(&self) -> &str {
        &self.reason
    }

    fn unused(&self) -> String {
        "is no copy any more. Remove it.".to_owned()
    }
}

/// A block that stands twice: where, and how big.
#[derive(Debug, PartialEq, Eq)]
struct Copied {
    first: (String, usize),
    second: (String, usize),
    lines: usize,
    tokens: usize,
}

/// The report of the tool.
#[derive(Deserialize)]
struct Report {
    duplicates: Vec<Found>,
}

#[derive(Deserialize)]
struct Found {
    #[serde(rename = "firstFile")]
    first: Place,
    #[serde(rename = "secondFile")]
    second: Place,
    lines: usize,
    tokens: usize,
}

#[derive(Deserialize)]
struct Place {
    name: String,
    start: usize,
}

/// Runs the tool on the folders and formats of the files that the check covers, and reads its
/// report.
fn find_clones(repo: &Repo, covered: &[String]) -> anyhow::Result<Vec<Copied>> {
    let root = repo.root();
    let folders: BTreeSet<&str> = covered
        .iter()
        .filter_map(|file| file.split('/').next())
        .collect();
    let formats: BTreeSet<&str> = covered
        .iter()
        .filter_map(|file| {
            let (_, ending) = file.rsplit_once('.')?;
            FORMATS
                .iter()
                .find(|(known, _)| *known == ending)
                .map(|(_, format)| *format)
        })
        .collect();
    if folders.is_empty() {
        return Ok(Vec::new());
    }
    let mut settings: Value = serde_json::from_str(&crate::source::read_text(repo, SETTINGS_PATH)?)
        .with_context(|| format!("{SETTINGS_PATH} is not valid"))?;
    let object = settings
        .as_object_mut()
        .with_context(|| format!("{SETTINGS_PATH} must be an object"))?;
    // The tool reads `path` from the folder of the configuration, and `output` from where it runs.
    let up = "../".repeat(Path::new(WORK_FOLDER).components().count());
    let mut ignore: Vec<Value> = object
        .get("ignore")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    ignore.extend(
        all_paths()
            .into_iter()
            .map(|path| json!(format!("**/{path}"))),
    );
    object.insert("ignore".to_owned(), Value::Array(ignore));
    object.insert(
        "path".to_owned(),
        json!(
            folders
                .iter()
                .map(|folder| format!("{up}{folder}"))
                .collect::<Vec<_>>()
        ),
    );
    object.insert("format".to_owned(), json!(formats));
    object.insert("output".to_owned(), json!(WORK_FOLDER));
    // Whole paths in the report, so two files of the same name are told apart.
    object.insert("absolute".to_owned(), json!(true));
    std::fs::create_dir_all(root.join(WORK_FOLDER))
        .with_context(|| format!("could not make {WORK_FOLDER}"))?;
    let config = format!("{WORK_FOLDER}/config.json");
    std::fs::write(root.join(&config), serde_json::to_string_pretty(&settings)?)
        .with_context(|| format!("could not write {config}"))?;
    let _ = std::fs::remove_file(root.join(WORK_FOLDER).join(REPORT_FILE));
    let result = run(root, "jscpd", &["--config", &config])?;
    let report = std::fs::read_to_string(root.join(WORK_FOLDER).join(REPORT_FILE))
        .with_context(|| format!("jscpd wrote no report:\n{}", result.output.trim()))?;
    clones_in(&report, root)
}

/// The blocks of a report, with the paths from the root of the repository, written with `/`.
fn clones_in(report: &str, root: &Path) -> anyhow::Result<Vec<Copied>> {
    let report: Report =
        serde_json::from_str(report).context("the report of jscpd is not valid")?;
    let root = plain(&root.to_string_lossy());
    let relative = |name: &str| {
        let name = plain(name);
        name.strip_prefix(&format!("{root}/"))
            .map_or(name.clone(), str::to_owned)
    };
    Ok(report
        .duplicates
        .into_iter()
        .map(|found| Copied {
            first: (relative(&found.first.name), found.first.start),
            second: (relative(&found.second.name), found.second.start),
            lines: found.lines,
            tokens: found.tokens,
        })
        .collect())
}

/// A path as the checks write it: `/` between the parts, and without the prefix that Windows puts
/// in front of a long path.
fn plain(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("//?/")
        .to_owned()
}

/// A finding for each block that no entry covers, and the findings about the entries.
fn judge(clones: &[Copied], exemptions: &[Exempt]) -> Vec<Finding> {
    let mut used = Used::default();
    let mut findings = Vec::new();
    for clone in clones {
        let index = exemptions.iter().position(|exempt| {
            let named = |file: &str| exempt.files.iter().any(|name| name == file);
            named(&clone.first.0) && named(&clone.second.0)
        });
        if let Some(index) = index {
            used.mark(index);
            continue;
        }
        findings.push(Finding::at(
            format!("{}:{}", clone.first.0, clone.first.1),
            format!(
                "{} lines ({} tokens) are the same as in {}:{}. Make the shared part one function, \
                 type or component and use it in both places (coding-standards §1, DRY). Only a \
                 copy that a follow-up removes is named in {EXEMPTIONS_PATH}, with its reason.",
                clone.lines, clone.tokens, clone.second.0, clone.second.1
            ),
        ));
    }
    findings.extend(exemptions::judge(EXEMPTIONS_PATH, exemptions, &used));
    for exempt in exemptions {
        if !names_a_follow_up(&exempt.reason) {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!(
                    "The entry `{}` must give a reason that names the follow-up that removes the copy (like M2C-F40).",
                    exempt.name()
                ),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::read_text;
    use crate::testutil::TempDir;

    /// A function of more than 100 tokens and 10 lines, so it is above the size of a block.
    const FUNCTION: &str = "pub fn total(values: &[u64]) -> u64 {
    let mut sum = 0;
    let mut count = 0;
    for value in values {
        if *value > 10 {
            sum += *value;
            count += 1;
        } else {
            sum += 1;
        }
    }
    let mut extra = 0;
    for value in values.iter().rev() {
        if value % 2 == 0 {
            extra += value / 2;
        } else {
            extra += value * 3 + 1;
        }
    }
    let mut tail = 0;
    for value in values.iter().skip(1) {
        if value % 3 == 0 {
            tail += value / 3;
        } else {
            tail += value * 5 + 2;
        }
    }
    sum + count * 2 + extra + tail + values.len() as u64
}
";

    /// A repository with the real settings of the check (but not the folders it leaves out: the
    /// examples live below `target`) and these files.
    fn repository(name: &str, files: &[(&str, &str)]) -> TempDir {
        let temp = TempDir::new(name);
        let real = Repo::discover().unwrap();
        let mut settings: Value =
            serde_json::from_str(&read_text(&real, SETTINGS_PATH).unwrap()).unwrap();
        settings.as_object_mut().unwrap().remove("ignore");
        temp.write(SETTINGS_PATH, settings.to_string());
        for (path, text) in files {
            temp.write(path, text);
        }
        temp
    }

    fn covered(files: &[&str]) -> Vec<String> {
        files.iter().map(|file| (*file).to_owned()).collect()
    }

    fn clones(temp: &TempDir, files: &[&str]) -> Vec<Copied> {
        let repo = Repo::at(temp.path().to_path_buf());
        find_clones(&repo, &covered(files)).unwrap()
    }

    #[test]
    fn bad_example_the_same_function_in_two_files_is_found() {
        let temp = repository(
            "lint-dry-bad",
            &[
                ("crates/a/src/lib.rs", FUNCTION),
                ("crates/b/src/lib.rs", FUNCTION),
            ],
        );
        let found = clones(&temp, &["crates/a/src/lib.rs", "crates/b/src/lib.rs"]);
        assert_eq!(found.len(), 1, "{found:?}");
        let findings = judge(&found, &[]);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("are the same as in"));
        let places = [found[0].first.0.as_str(), found[0].second.0.as_str()];
        assert!(places.contains(&"crates/a/src/lib.rs") && places.contains(&"crates/b/src/lib.rs"));
    }

    #[test]
    fn good_example_two_different_files_are_no_copy() {
        let other = "pub fn names(words: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for word in words {
        let trimmed = word.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut chars = trimmed.chars();
        let first = chars.next().map(|c| c.to_uppercase().to_string());
        out.push(format!(\"{}{}\", first.unwrap_or_default(), chars.as_str()));
    }
    out.sort();
    out.dedup();
    out
}
";
        let temp = repository(
            "lint-dry-good",
            &[
                ("crates/a/src/lib.rs", FUNCTION),
                ("crates/b/src/lib.rs", other),
            ],
        );
        let found = clones(&temp, &["crates/a/src/lib.rs", "crates/b/src/lib.rs"]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn imports_are_not_compared() {
        let uses = "use std::collections::BTreeMap;
use std::path::Path;
use std::fmt::Write;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;
use std::process::Command;
use std::fs::File;
use std::env::Args;
use std::net::IpAddr;
use std::ops::Range;
use std::rc::Rc;
";
        let temp = repository(
            "lint-dry-imports",
            &[
                (
                    "crates/a/src/lib.rs",
                    &format!(
                        "{uses}fn a() {{}}
"
                    ),
                ),
                (
                    "crates/b/src/lib.rs",
                    &format!(
                        "{uses}fn b() {{ let x = 1; }}
"
                    ),
                ),
            ],
        );
        let found = clones(&temp, &["crates/a/src/lib.rs", "crates/b/src/lib.rs"]);
        assert!(found.is_empty(), "{found:?}");
    }

    fn clone_of(first: &str, second: &str) -> Copied {
        Copied {
            first: (first.to_owned(), 1),
            second: (second.to_owned(), 1),
            lines: 12,
            tokens: 120,
        }
    }

    fn exempt(first: &str, second: &str, reason: &str) -> Exempt {
        Exempt {
            files: [first.to_owned(), second.to_owned()],
            reason: reason.to_owned(),
        }
    }

    #[test]
    fn a_named_copy_stands_and_an_entry_that_matches_nothing_fails() {
        let reason = "Both go when the picker is one component (M2C-F40).";
        let found = [clone_of("web/a.ts", "web/b.ts")];
        assert!(judge(&found, &[exempt("web/b.ts", "web/a.ts", reason)]).is_empty());
        let stale = judge(&[], &[exempt("web/a.ts", "web/b.ts", reason)]);
        assert!(
            stale
                .iter()
                .any(|f| f.message.contains("is no copy any more"))
        );
    }

    #[test]
    fn an_entry_needs_a_follow_up_in_its_reason() {
        let found = [clone_of("web/a.ts", "web/b.ts")];
        let findings = judge(
            &found,
            &[exempt(
                "web/a.ts",
                "web/b.ts",
                "Nobody minds this copy at all.",
            )],
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("names the follow-up"));
    }

    #[test]
    fn the_paths_of_a_report_are_those_of_the_repository() {
        let report = r#"{"duplicates":[{"firstFile":{"name":"\\\\?\\C:\\work\\repo\\crates\\a\\src\\lib.rs","start":3},
            "secondFile":{"name":"C:/work/repo/web/x.ts","start":9},"lines":12,"tokens":120}]}"#;
        let found = clones_in(report, Path::new("C:\\work\\repo")).unwrap();
        assert_eq!(found[0].first, ("crates/a/src/lib.rs".to_owned(), 3));
        assert_eq!(found[0].second, ("web/x.ts".to_owned(), 9));
    }

    #[test]
    fn the_copies_of_this_repository_are_all_named_or_gone() {
        let repo = Repo::discover().unwrap();
        let catalog = crate::catalog::Catalog::load(&repo).unwrap();
        let context = Context {
            repo: &repo,
            catalog: &catalog,
            files: None,
        };
        let findings = LintDry.run(&context).unwrap();
        assert!(
            findings.is_empty(),
            "{:?}",
            findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }
}
