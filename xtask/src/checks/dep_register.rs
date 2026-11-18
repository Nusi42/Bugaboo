//! DEP-REGISTER: every direct dependency has a register row, and every row a dependency
//! (COMP-02, coding-standards §5).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::Context as _;

use super::dep_licenses::is_npm_lockfile;
use super::{Check, Context, Finding};
use crate::metadata::{self, Package};

/// The register, relative to the repository root.
const REGISTER: &str = "docs/third-party/README.md";

/// The `package.json` sections that list direct dependencies.
const NPM_SECTIONS: [&str; 4] = [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
];

pub(crate) struct DepRegister;

impl Check for DepRegister {
    fn id(&self) -> &'static str {
        "DEP-REGISTER"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let rows = read_register(root)?;
        let rust = rust_dependencies(&metadata::load(root)?);
        let files = context.repo.all_files()?;
        let npm = npm_dependencies(root, &files)?;
        let mut findings = compare(&rows, &rust, &npm);
        let locked = Locked {
            rust: metadata::load_direct_versions(root)?,
            npm: npm_lock_versions(root, &files)?,
        };
        findings.extend(compare_versions(&rows, &locked));
        Ok(findings)
    }
}

/// One row of a register table: the cells after the first, as written (backticks and links
/// removed from the first cell only).
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub(crate) struct Row {
    /// The text of the Version column (empty in the tables of bundled data).
    pub(crate) version: String,
    /// The text of the License column.
    pub(crate) license: String,
    /// The text of the Purpose column (in *Accepted risks*: *Why accepted*).
    pub(crate) purpose: String,
    /// The text of the *Ships?* column (empty in *Accepted risks*).
    pub(crate) ships: String,
    /// The text of the *VEX justification* column (only in *Accepted risks*).
    pub(crate) justification: String,
    /// The cells in the order of the table, the first included.
    pub(crate) cells: Vec<String>,
}

/// The names in the register tables, split by the kind of table (`Crate`, `Package`, `Data` or
/// `Advisory` in the first column). Each maps a name to its row.
#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct Rows {
    pub(crate) rust: BTreeMap<String, Row>,
    pub(crate) npm: BTreeMap<String, Row>,
    /// Bundled data files, by path.
    pub(crate) data: BTreeMap<String, Row>,
    /// Accepted risks, by advisory ID.
    pub(crate) risks: BTreeMap<String, Row>,
}

/// Name -> the versions the lockfiles pin for direct dependencies (Rust: only what workspace
/// members depend on, so a transitive duplicate cannot satisfy a row, M0-F10).
type Versions = BTreeMap<String, BTreeSet<String>>;

struct Locked {
    rust: Versions,
    npm: Versions,
}

/// Reads the register of `root`.
///
/// # Errors
/// When the file cannot be read.
pub(crate) fn read_register(root: &Path) -> anyhow::Result<Rows> {
    let register = fs::read_to_string(root.join(REGISTER))
        .with_context(|| format!("could not read {REGISTER}"))?;
    Ok(parse_register(&register))
}

/// Which table a row belongs to, by the name of the first column.
#[derive(Clone, Copy)]
enum Table {
    Rust,
    Npm,
    Data,
    Risks,
}

/// The name in a first cell: without backticks, and for a link `[text](address)` its text.
fn plain_name(cell: &str) -> String {
    let cell = cell.trim().trim_matches('`');
    match (cell.strip_prefix('['), cell.find("](")) {
        (Some(inner), Some(end)) => inner[..end - 1].to_owned(),
        _ => cell.to_owned(),
    }
}

/// The columns the rows are read by, as the header of a table names them (in lower case).
const VERSION: &[&str] = &["version"];
const LICENSE: &[&str] = &["license"];
const PURPOSE: &[&str] = &["purpose", "why accepted"];
const SHIPS: &[&str] = &["ships?"];
const JUSTIFICATION: &[&str] = &["vex justification"];

pub(crate) fn parse_register(markdown: &str) -> Rows {
    let mut rows = Rows::default();
    let mut section: Option<Table> = None;
    // The header cells of the table being read, in lower case: columns are found by name, so a
    // table with its columns in another order reads the same.
    let mut header: Vec<String> = Vec::new();
    for line in markdown.lines().map(str::trim) {
        if !line.starts_with('|') {
            section = None;
            continue;
        }
        let cells: Vec<String> = line
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_owned())
            .collect();
        let first = plain_name(cells.first().map_or("", String::as_str));
        let started = match first.as_str() {
            "Crate" => Some(Table::Rust),
            "Package" => Some(Table::Npm),
            "Data" => Some(Table::Data),
            "Advisory" => Some(Table::Risks),
            _ => None,
        };
        if let Some(table) = started {
            section = Some(table);
            header = cells.iter().map(|cell| cell.to_lowercase()).collect();
            continue;
        }
        if first.is_empty() || first.starts_with("---") || first.starts_with(":-") {
            continue;
        }
        let Some(table) = section else { continue };
        let named = |names: &[&str]| {
            header
                .iter()
                .position(|column| names.contains(&column.as_str()))
                .and_then(|at| cells.get(at))
                .map_or_else(String::new, |text| text.trim_matches('`').to_owned())
        };
        let row = Row {
            version: named(VERSION),
            license: named(LICENSE),
            purpose: named(PURPOSE),
            ships: named(SHIPS),
            justification: named(JUSTIFICATION),
            cells: cells.clone(),
        };
        match table {
            Table::Rust => rows.rust.insert(first, row),
            Table::Npm => rows.npm.insert(first, row),
            Table::Data => rows.data.insert(first, row),
            Table::Risks => rows.risks.insert(first, row),
        };
    }
    rows
}

/// External crates the workspace members list in their `Cargo.toml` (normal, dev and build).
fn rust_dependencies(packages: &[Package]) -> BTreeSet<String> {
    packages
        .iter()
        .flat_map(|package| &package.dependencies)
        .filter(|dependency| dependency.path.is_none())
        .map(|dependency| dependency.name.clone())
        .collect()
}

/// Packages the repository's `package.json` files list, except its own workspace packages.
fn npm_dependencies(root: &Path, files: &[String]) -> anyhow::Result<BTreeSet<String>> {
    let mut manifests = Vec::new();
    for file in files {
        let is_package_json = file == "package.json" || file.ends_with("/package.json");
        if is_package_json && !file.split('/').any(|part| part == "node_modules") {
            let text = fs::read_to_string(root.join(file))
                .with_context(|| format!("could not read {file}"))?;
            let json: serde_json::Value =
                serde_json::from_str(&text).with_context(|| format!("{file} is not valid JSON"))?;
            manifests.push(json);
        }
    }
    let own: BTreeSet<&str> = manifests
        .iter()
        .filter_map(|json| json.get("name").and_then(serde_json::Value::as_str))
        .collect();
    let mut names = BTreeSet::new();
    for json in &manifests {
        for section in NPM_SECTIONS {
            if let Some(map) = json.get(section).and_then(serde_json::Value::as_object) {
                names.extend(
                    map.keys()
                        .filter(|name| !own.contains(name.as_str()))
                        .cloned(),
                );
            }
        }
    }
    Ok(names)
}

/// Versions of top-level packages (`node_modules/<name>`) in every `package-lock.json`.
fn npm_lock_versions(root: &Path, files: &[String]) -> anyhow::Result<Versions> {
    let mut versions = Versions::new();
    for file in files {
        if !is_npm_lockfile(file) {
            continue;
        }
        let text = fs::read_to_string(root.join(file))
            .with_context(|| format!("could not read {file}"))?;
        let json: serde_json::Value =
            serde_json::from_str(&text).with_context(|| format!("{file} is not valid JSON"))?;
        let packages = json.get("packages").and_then(serde_json::Value::as_object);
        for (key, entry) in packages.into_iter().flatten() {
            if let (Some(name), Some(version)) = (
                key.strip_prefix("node_modules/"),
                entry.get("version").and_then(serde_json::Value::as_str),
            ) {
                versions
                    .entry(name.to_owned())
                    .or_default()
                    .insert(version.to_owned());
            }
        }
    }
    Ok(versions)
}

/// The Version column must be a version the lockfile really has.
fn compare_versions(rows: &Rows, locked: &Locked) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (kind, in_register, in_lock) in [
        ("Rust crate", &rows.rust, &locked.rust),
        ("npm package", &rows.npm, &locked.npm),
    ] {
        for (name, row) in in_register {
            let version = &row.version;
            let Some(have) = in_lock.get(name) else {
                continue; // not locked: the name comparison reports it if it matters
            };
            // Build metadata (`1.1.6+spec-1.1.0`) is not part of the version people read.
            if !have
                .iter()
                .any(|v| v.split('+').next() == Some(version.as_str()))
            {
                let have: Vec<&str> = have.iter().map(String::as_str).collect();
                findings.push(Finding::at(
                    name,
                    format!(
                        "{kind}: the register says {version} but the lockfile has {}. \
                         Update the Version column in {REGISTER}.",
                        have.join(", ")
                    ),
                ));
            }
        }
    }
    findings
}

fn compare(rows: &Rows, rust: &BTreeSet<String>, npm: &BTreeSet<String>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (kind, in_register, in_code) in [
        ("Rust crate", &rows.rust, rust),
        ("npm package", &rows.npm, npm),
    ] {
        let in_register: BTreeSet<String> = in_register.keys().cloned().collect();
        for name in in_code.difference(&in_register) {
            findings.push(Finding::at(
                name,
                format!(
                    "{kind} is a direct dependency but has no row in {REGISTER}. Add one (purpose, license, health, alternatives)."
                ),
            ));
        }
        for name in in_register.difference(in_code) {
            findings.push(Finding::at(
                name,
                format!(
                    "{kind} has a row in {REGISTER} but nothing depends on it. Remove the row or add the dependency."
                ),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    const REGISTER_TEXT: &str = "\
## Tooling (npm)

| Package | Version |
|---------|---------|
| marked | 18.0.14 |

## Rust

| Crate | Version |
|-------|---------|
| anyhow | 1.0.104 |
| `serde` | 1.0.229 |
";

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    #[test]
    fn columns_are_read_by_their_header_not_their_place() {
        let reordered = "\
| Crate | Purpose | Ships? | License | Version |
|---|---|---|---|---|
| serde | Reads things | yes (bb-core) | MIT | 1.0.2 |

| Advisory | VEX justification | Why accepted | Package |
|---|---|---|---|
| [A-1](https://example.test) | code_not_present | Only tools. | picomatch |
";
        let rows = parse_register(reordered);
        let serde = &rows.rust["serde"];
        assert_eq!(serde.version, "1.0.2");
        assert_eq!(serde.license, "MIT");
        assert_eq!(serde.purpose, "Reads things");
        assert_eq!(serde.ships, "yes (bb-core)");
        let risk = &rows.risks["A-1"];
        assert_eq!(risk.purpose, "Only tools.");
        assert_eq!(risk.justification, "code_not_present");
        // A column the table does not have reads as empty, not as another column.
        assert_eq!(risk.ships, "");
    }

    #[test]
    fn register_tables_are_read() {
        let rows = parse_register(REGISTER_TEXT);
        assert_eq!(
            rows.rust.keys().cloned().collect::<BTreeSet<_>>(),
            set(&["anyhow", "serde"])
        );
        assert_eq!(rows.rust["anyhow"].version, "1.0.104");
        assert_eq!(rows.npm["marked"].version, "18.0.14");
    }

    #[test]
    fn good_example() {
        let rows = parse_register(REGISTER_TEXT);
        assert!(compare(&rows, &set(&["anyhow", "serde"]), &set(&["marked"])).is_empty());
    }

    #[test]
    fn bad_example_dependency_without_row() {
        let rows = parse_register(REGISTER_TEXT);
        let findings = compare(
            &rows,
            &set(&["anyhow", "serde", "tokio"]),
            &set(&["marked"]),
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("no row"));
    }

    #[test]
    fn bad_example_row_without_dependency() {
        let rows = parse_register(REGISTER_TEXT);
        let findings = compare(&rows, &set(&["anyhow"]), &set(&["marked", "react"]));
        assert_eq!(findings.len(), 2);
    }

    fn locked(rust: &[(&str, &str)], npm: &[(&str, &str)]) -> Locked {
        let build = |list: &[(&str, &str)]| {
            let mut map = Versions::new();
            for (name, version) in list {
                map.entry((*name).to_owned())
                    .or_default()
                    .insert((*version).to_owned());
            }
            map
        };
        Locked {
            rust: build(rust),
            npm: build(npm),
        }
    }

    #[test]
    fn good_example_versions_match_the_lockfiles() {
        let rows = parse_register(REGISTER_TEXT);
        let lock = locked(
            &[
                ("anyhow", "1.0.104"),
                ("serde", "1.0.229"),
                ("serde", "0.9.0"),
            ],
            &[("marked", "18.0.14")],
        );
        assert!(compare_versions(&rows, &lock).is_empty());
    }

    #[test]
    fn bad_example_register_version_differs_from_the_lockfile() {
        let rows = parse_register(REGISTER_TEXT);
        let lock = locked(
            &[("anyhow", "1.0.105"), ("serde", "1.0.229")],
            &[("marked", "19.0.0")],
        );
        assert_eq!(compare_versions(&rows, &lock).len(), 2);
    }

    #[test]
    fn the_npm_lockfile_is_read() {
        let temp = TempDir::new("register-npm-lock");
        temp.write(
            "package-lock.json",
            r#"{"packages":{"":{},"node_modules/x":{"version":"2.0.0"},"node_modules/x/node_modules/y":{"version":"3"}}}"#,
        );
        let npm = npm_lock_versions(temp.path(), &["package-lock.json".to_owned()]).unwrap();
        assert!(npm["x"].contains("2.0.0"));
        assert!(!npm.contains_key("y"));
    }

    #[test]
    fn npm_dependencies_skip_our_own_packages() {
        let temp = TempDir::new("register-npm");
        temp.write(
            "package.json",
            r#"{"name":"root","devDependencies":{"@bugaboo/ui":"*","vite":"8.3.2"}}"#,
        );
        temp.write(
            "web/ui/package.json",
            r#"{"name":"@bugaboo/ui","dependencies":{"react":"19.3.0"}}"#,
        );
        temp.write(
            "node_modules/x/package.json",
            r#"{"name":"x","dependencies":{"ignored":"1"}}"#,
        );
        let files = [
            "package.json".to_owned(),
            "web/ui/package.json".to_owned(),
            "node_modules/x/package.json".to_owned(),
        ];
        let names = npm_dependencies(temp.path(), &files).unwrap();
        assert_eq!(names, set(&["react", "vite"]));
    }
}
