//! DEP-LICENSES: only licenses on the allow list ship (architecture §1, `deny.toml`).
//!
//! Rust crates are checked by `cargo deny`. npm packages are checked here against the very same
//! allow list, which is read from `deny.toml` so that the list exists only once.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::Context as _;
use serde_json::Value;

use super::{Check, Context, Finding};
use crate::tools;

const DENY_CONFIG: &str = "deny.toml";

pub(crate) struct DepLicenses;

impl Check for DepLicenses {
    fn id(&self) -> &'static str {
        "DEP-LICENSES"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let mut findings = rust_licenses(root)?;
        let lockfiles: Vec<String> = context
            .repo
            .all_files()?
            .into_iter()
            .filter(|file| {
                (file == "package-lock.json" || file.ends_with("/package-lock.json"))
                    && !file.split('/').any(|part| part == "node_modules")
            })
            .collect();
        findings.extend(npm_licenses(root, &lockfiles)?);
        Ok(findings)
    }
}

/// `cargo deny check licenses bans sources`, and no warnings either.
fn rust_licenses(root: &Path) -> anyhow::Result<Vec<Finding>> {
    // `license-not-encountered` is allowed: the allow list is the architecture's policy
    // (architecture §1), not a list of what happens to be used today.
    let output = tools::run_installed(
        "cargo-deny",
        &[
            "--workspace",
            "--config",
            DENY_CONFIG,
            "check",
            "-A",
            "license-not-encountered",
            "licenses",
            "bans",
            "sources",
        ],
        root,
    )?;
    let output = output.map(|mut output| {
        // cargo-deny exits 0 on warnings; this project treats warnings as errors.
        output.success &= !output.text.contains("warning[");
        output
    });
    Ok(tools::findings_from(
        output,
        "cargo deny found a license, ban or source problem. Replace the dependency, or ask for an \
         ADR before changing the allow list in deny.toml.",
    ))
}

/// The allowed SPDX identifiers in `deny.toml` (`[licenses] allow`).
fn allow_list(deny_toml: &str) -> anyhow::Result<BTreeSet<String>> {
    let value: toml::Value = toml::from_str(deny_toml).context("deny.toml is not valid TOML")?;
    let list = value
        .get("licenses")
        .and_then(|l| l.get("allow"))
        .and_then(toml::Value::as_array)
        .context("deny.toml has no [licenses] allow list")?;
    Ok(list
        .iter()
        .filter_map(toml::Value::as_str)
        .map(str::to_owned)
        .collect())
}

fn npm_licenses(root: &Path, lockfiles: &[String]) -> anyhow::Result<Vec<Finding>> {
    if lockfiles.is_empty() {
        return Ok(Vec::new());
    }
    let allow = allow_list(&fs::read_to_string(root.join(DENY_CONFIG))?)?;
    let mut findings = Vec::new();
    for lockfile in lockfiles {
        let text = fs::read_to_string(root.join(lockfile))
            .with_context(|| format!("could not read {lockfile}"))?;
        let lock: Value =
            serde_json::from_str(&text).with_context(|| format!("{lockfile} is not valid JSON"))?;
        let folder = Path::new(lockfile)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        let packages = lock.get("packages").and_then(Value::as_object);
        for (key, entry) in packages.into_iter().flatten() {
            // Only installed packages count: keys outside node_modules/ are our own workspace
            // packages, links are the same. Dev-only packages never ship, so they are exempt from
            // the allow list (they still need register rows), but not from the ban on the GPL
            // family (ADR-0035).
            let is_dependency = key.starts_with("node_modules/") || key.contains("/node_modules/");
            let dev_only = entry.get("dev").and_then(Value::as_bool) == Some(true);
            if !is_dependency || entry.get("link").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            let name = key.rsplit("node_modules/").next().unwrap_or(key);
            // The installed package.json is the truth; the lockfile's copy is the fallback when
            // `npm ci` has not run for this folder.
            let installed = fs::read_to_string(root.join(folder).join(key).join("package.json"))
                .ok()
                .and_then(|text| serde_json::from_str::<Value>(&text).ok());
            let license = installed
                .as_ref()
                .and_then(declared_license)
                .or_else(|| declared_license(entry));
            if dev_only {
                if let Some(expression) = license
                    && !satisfied_by(&expression, &|id| !is_gpl_family(id))
                {
                    findings.push(Finding::at(
                        format!("{lockfile}: {name}"),
                        format!(
                            "has the license `{expression}`; the GPL family is not allowed, not even in a tool for developers."
                        ),
                    ));
                }
                continue;
            }
            match license {
                None => findings.push(Finding::at(
                    format!("{lockfile}: {name}"),
                    "declares no readable license. Pick another package or get the license clarified.",
                )),
                Some(expression) if !satisfied(&expression, &allow) => findings.push(Finding::at(
                    format!("{lockfile}: {name}"),
                    format!(
                        "has the license `{expression}`, which is not on the allow list in {DENY_CONFIG}."
                    ),
                )),
                Some(_) => {}
            }
        }
    }
    Ok(findings)
}

/// The `license` field of a `package.json`, or its legacy `licenses` list, as text.
fn declared_license(json: &Value) -> Option<String> {
    json.get("license")
        .or_else(|| json.get("licenses"))
        .and_then(license_text)
}

/// The license of a `package.json` as an SPDX expression (old object and list forms included).
fn license_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(map) => map.get("type").and_then(Value::as_str).map(str::to_owned),
        Value::Array(list) => {
            let parts: Vec<String> = list.iter().filter_map(license_text).collect();
            (!parts.is_empty()).then(|| parts.join(" OR "))
        }
        _ => None,
    }
}

/// Whether an SPDX expression (`MIT`, `(MIT OR Apache-2.0)`, `A AND B`) is allowed.
/// `OR` needs one allowed side, `AND` needs both.
pub(crate) fn satisfied(expression: &str, allow: &BTreeSet<String>) -> bool {
    satisfied_by(expression, &|id| allow.contains(id))
}

/// Whether a license identifier belongs to the GPL family (GPL, LGPL, AGPL), which Bugaboo never
/// takes in, not even in a tool that only runs on a developer machine (ADR-0035): code under
/// another party's GPL could not go into an edition under other terms.
fn is_gpl_family(id: &str) -> bool {
    let id = id.to_ascii_uppercase();
    ["GPL", "LGPL", "AGPL"].iter().any(|family| {
        id.strip_prefix(family)
            .is_some_and(|rest| rest.starts_with('-'))
    })
}

/// Whether an SPDX expression can be met by identifiers for which `is_allowed` holds.
fn satisfied_by(expression: &str, is_allowed: &dyn Fn(&str) -> bool) -> bool {
    let spaced = expression.replace('(', " ( ").replace(')', " ) ");
    let tokens: Vec<&str> = spaced.split_whitespace().collect();
    let mut position = 0;
    let result = parse_or(&tokens, &mut position, is_allowed);
    result.is_some_and(|ok| ok) && position == tokens.len()
}

fn parse_or(tokens: &[&str], at: &mut usize, is_allowed: &dyn Fn(&str) -> bool) -> Option<bool> {
    let mut result = parse_and(tokens, at, is_allowed)?;
    while tokens
        .get(*at)
        .is_some_and(|t| t.eq_ignore_ascii_case("OR"))
    {
        *at += 1;
        let next = parse_and(tokens, at, is_allowed)?;
        result |= next;
    }
    Some(result)
}

fn parse_and(tokens: &[&str], at: &mut usize, is_allowed: &dyn Fn(&str) -> bool) -> Option<bool> {
    let mut result = parse_atom(tokens, at, is_allowed)?;
    while tokens
        .get(*at)
        .is_some_and(|t| t.eq_ignore_ascii_case("AND"))
    {
        *at += 1;
        let next = parse_atom(tokens, at, is_allowed)?;
        result &= next;
    }
    Some(result)
}

fn parse_atom(tokens: &[&str], at: &mut usize, is_allowed: &dyn Fn(&str) -> bool) -> Option<bool> {
    let token = *tokens.get(*at)?;
    *at += 1;
    if token == "(" {
        let inner = parse_or(tokens, at, is_allowed)?;
        (*tokens.get(*at)? == ")").then_some(())?;
        *at += 1;
        return Some(inner);
    }
    let mut id = token.to_owned();
    if tokens
        .get(*at)
        .is_some_and(|t| t.eq_ignore_ascii_case("WITH"))
    {
        id = format!("{id} WITH {}", tokens.get(*at + 1)?);
        *at += 2;
    }
    Some(is_allowed(&id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn allow() -> BTreeSet<String> {
        ["MIT", "Apache-2.0", "ISC", "Apache-2.0 WITH LLVM-exception"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    }

    #[test]
    fn expressions_are_evaluated() {
        let allow = allow();
        assert!(satisfied("MIT", &allow));
        assert!(satisfied("(MIT OR GPL-3.0-only)", &allow));
        assert!(satisfied("MIT AND ISC", &allow));
        assert!(satisfied("Apache-2.0 WITH LLVM-exception", &allow));
        assert!(!satisfied("GPL-3.0-only", &allow));
        assert!(!satisfied("MIT AND GPL-3.0-only", &allow));
        assert!(!satisfied("Apache-2.0 WITH Other-exception", &allow));
        assert!(!satisfied("(MIT", &allow));
        assert!(!satisfied("", &allow));
    }

    #[test]
    fn real_allow_list_is_read_from_deny_toml() {
        let text = include_str!("../../../deny.toml");
        assert!(allow_list(text).unwrap().contains("MIT"));
    }

    fn npm_repo(temp: &TempDir, license: &str) {
        temp.write(DENY_CONFIG, "[licenses]\nallow = [\"MIT\", \"ISC\"]\n");
        temp.write(
            "package-lock.json",
            format!(
                r#"{{"packages":{{"":{{"name":"x"}},"node_modules/left-pad":{{"version":"1.0.0","license":"{license}"}}}}}}"#
            ),
        );
    }

    #[test]
    fn good_example_npm() {
        let temp = TempDir::new("licenses-npm-good");
        npm_repo(&temp, "(MIT OR GPL-3.0-only)");
        let findings = npm_licenses(temp.path(), &["package-lock.json".into()]).unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn bad_example_npm() {
        let temp = TempDir::new("licenses-npm-bad");
        npm_repo(&temp, "GPL-3.0-only");
        let findings = npm_licenses(temp.path(), &["package-lock.json".into()]).unwrap();
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("GPL-3.0-only"));
    }

    #[test]
    fn workspace_packages_and_dev_only_packages_are_exempt() {
        let temp = TempDir::new("licenses-npm-exempt");
        temp.write(DENY_CONFIG, "[licenses]\nallow = [\"MIT\"]\n");
        temp.write(
            "package-lock.json",
            r#"{"packages":{"":{},
              "web/ui":{"license":"UNLICENSED"},
              "node_modules/tool":{"license":"MPL-2.0","dev":true},
              "node_modules/legacy":{"licenses":[{"type":"MIT"}]}}}"#,
        );
        let findings = npm_licenses(temp.path(), &["package-lock.json".into()]).unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn dev_only_packages_may_not_be_gpl() {
        let temp = TempDir::new("licenses-npm-dev-gpl");
        temp.write(
            DENY_CONFIG,
            "[licenses]
allow = [\"MIT\"]
",
        );
        temp.write(
            "package-lock.json",
            r#"{"packages":{
              "node_modules/a":{"license":"AGPL-3.0-or-later","dev":true},
              "node_modules/b":{"license":"(MIT OR GPL-3.0-only)","dev":true},
              "node_modules/c":{"license":"MPL-2.0","dev":true}}}"#,
        );
        let findings = npm_licenses(temp.path(), &["package-lock.json".into()]).unwrap();
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("AGPL-3.0-or-later"));
    }

    #[test]
    fn gpl_family_is_recognised_and_never_allowed() {
        for id in [
            "GPL-3.0-only",
            "LGPL-2.1-or-later",
            "AGPL-3.0-or-later",
            "GPL-2.0-with-classpath-exception",
        ] {
            assert!(is_gpl_family(id), "{id}");
        }
        for id in ["MIT", "MPL-2.0", "Apache-2.0"] {
            assert!(!is_gpl_family(id), "{id}");
        }
        let text = include_str!("../../../deny.toml");
        assert!(
            allow_list(text)
                .unwrap()
                .iter()
                .all(|id| !is_gpl_family(id))
        );
    }

    #[test]
    fn bad_example_legacy_licenses_field() {
        let temp = TempDir::new("licenses-npm-legacy");
        temp.write(DENY_CONFIG, "[licenses]\nallow = [\"MIT\"]\n");
        temp.write(
            "package-lock.json",
            r#"{"packages":{"node_modules/legacy":{"licenses":[{"type":"GPL-3.0-only"}]}}}"#,
        );
        let findings = npm_licenses(temp.path(), &["package-lock.json".into()]).unwrap();
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn installed_package_json_wins_over_the_lockfile() {
        let temp = TempDir::new("licenses-npm-installed");
        npm_repo(&temp, "MIT");
        temp.write(
            "node_modules/left-pad/package.json",
            r#"{"name":"left-pad","license":"AGPL-3.0-only"}"#,
        );
        let findings = npm_licenses(temp.path(), &["package-lock.json".into()]).unwrap();
        assert_eq!(findings.len(), 1);
    }

    const MANIFEST: &str = "[package]\nname = \"demo\"\nversion = \"0.0.0\"\nedition = \"2024\"\n";

    fn rust_repo(temp: &TempDir, license: &str) {
        temp.write(DENY_CONFIG, "[licenses]\nversion = 2\nallow = [\"MIT\"]\n");
        temp.write(
            "Cargo.toml",
            format!("{MANIFEST}license = \"{license}\"\n\n[workspace]\n"),
        );
        temp.write("src/main.rs", "fn main() {}\n");
    }

    #[test]
    fn bad_example_rust() {
        let temp = TempDir::new("licenses-rust-bad");
        rust_repo(&temp, "GPL-3.0-only");
        crate::testutil::assert_real_findings(&rust_licenses(temp.path()).unwrap());
    }

    #[test]
    fn good_example_rust() {
        let temp = TempDir::new("licenses-rust-good");
        rust_repo(&temp, "MIT");
        assert!(rust_licenses(temp.path()).unwrap().is_empty());
    }
}
