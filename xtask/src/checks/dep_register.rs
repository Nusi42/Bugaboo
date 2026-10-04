//! DEP-REGISTER: every direct dependency has a register row, and every row a dependency
//! (COMP-02, coding-standards §5).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::Context as _;

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
        let register = fs::read_to_string(root.join(REGISTER))
            .with_context(|| format!("could not read {REGISTER}"))?;
        let rows = parse_register(&register);
        let rust = rust_dependencies(&metadata::load(root)?);
        let npm = npm_dependencies(root, &context.repo.all_files()?)?;
        Ok(compare(&rows, &rust, &npm))
    }
}

/// The names in the register tables, split by the kind of table (`Crate` or `Package` column).
#[derive(Default, Debug, PartialEq, Eq)]
struct Rows {
    rust: BTreeSet<String>,
    npm: BTreeSet<String>,
}

fn parse_register(markdown: &str) -> Rows {
    let mut rows = Rows::default();
    let mut section: Option<bool> = None; // Some(true) = Rust table, Some(false) = npm table
    for line in markdown.lines().map(str::trim) {
        if !line.starts_with('|') {
            section = None;
            continue;
        }
        let first = line
            .trim_matches('|')
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('`')
            .to_owned();
        match first.as_str() {
            "Crate" => section = Some(true),
            "Package" => section = Some(false),
            name if name.is_empty() || name.starts_with("---") || name.starts_with(":-") => {}
            name => match section {
                Some(true) => {
                    rows.rust.insert(name.to_owned());
                }
                Some(false) => {
                    rows.npm.insert(name.to_owned());
                }
                None => {}
            },
        }
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

fn compare(rows: &Rows, rust: &BTreeSet<String>, npm: &BTreeSet<String>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (kind, in_register, in_code) in [
        ("Rust crate", &rows.rust, rust),
        ("npm package", &rows.npm, npm),
    ] {
        for name in in_code.difference(in_register) {
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
    fn register_tables_are_read() {
        let rows = parse_register(REGISTER_TEXT);
        assert_eq!(rows.rust, set(&["anyhow", "serde"]));
        assert_eq!(rows.npm, set(&["marked"]));
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
