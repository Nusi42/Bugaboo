//! SEC-ADVISORIES: no known vulnerability in a locked dependency (COMP-03, architecture §12).
//!
//! Rust crates are checked by `cargo deny check advisories` against the `RustSec` database;
//! accepted advisories are the `ignore` list in `deny.toml`. npm packages are checked by
//! `npm audit` against the lockfile, the tools for developers included: they run on the
//! maintainer's computer; accepted advisories are the list in `tools/checks/npm-advisories.toml`.
//! Every accepted advisory has a reason that names the register (META-SUPPRESS counts them).
//! Both scans need the network, so a computer that cannot reach it gets a finding, never a
//! silent pass. The two reports are read by pure functions, which the examples feed with saved
//! output.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

use super::dep_licenses::npm_lockfiles;
use super::{Check, Context, Finding};
use crate::tools;

/// The accepted npm advisories, relative to the repository root.
const NPM_ACCEPTED: &str = "tools/checks/npm-advisories.toml";

pub(crate) struct SecAdvisories;

impl Check for SecAdvisories {
    fn id(&self) -> &'static str {
        "SEC-ADVISORIES"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let mut findings = rust_advisories(root)?;
        let accepted = accepted_npm(root)?;
        for lockfile in npm_lockfiles(context)? {
            findings.extend(npm_advisories(root, &lockfile, &accepted)?);
        }
        Ok(findings)
    }
}

/// `cargo deny check advisories`: vulnerabilities, unsound crates and yanked versions.
fn rust_advisories(root: &Path) -> anyhow::Result<Vec<Finding>> {
    Ok(
        match tools::cargo_deny(root, &["--format", "json"], &["advisories"])? {
            Ok(output) => deny_findings(output.success, &output.text),
            Err(missing) => vec![missing],
        },
    )
}

/// The findings in the JSON lines that `cargo deny --format json` printed: one for each
/// diagnostic that is an error or a warning, one for each error of the program itself (an
/// advisory database that could not be fetched), and one when it failed without naming a problem.
fn deny_findings(success: bool, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for line in text.lines() {
        let Ok(entry) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let fields = &entry["fields"];
        match entry["type"].as_str() {
            Some("diagnostic")
                if matches!(fields["severity"].as_str(), Some("error" | "warning")) =>
            {
                let message = fields["message"].as_str().unwrap_or("");
                let advisory = &fields["advisory"];
                let place = match advisory["package"].as_str() {
                    Some(package) => format!(
                        "{package} ({})",
                        advisory["id"].as_str().unwrap_or("no advisory ID")
                    ),
                    None => fields["code"].as_str().unwrap_or("cargo deny").to_owned(),
                };
                findings.push(Finding::at(
                    place,
                    format!(
                        "{message}. Update the crate (`cargo update -p <crate>`), or, when no fix \
                         exists and Bugaboo is not affected, accept it in deny.toml with a reason \
                         and a row in docs/third-party/README.md."
                    ),
                ));
            }
            Some("log") if fields["level"].as_str() == Some("ERROR") => {
                findings.push(Finding::general(format!(
                    "cargo deny could not check the advisories (is the network reachable?): {}",
                    fields["message"].as_str().unwrap_or("")
                )));
            }
            _ => {}
        }
    }
    if findings.is_empty() && !success {
        findings.push(Finding::general(format!(
            "cargo deny failed without naming a problem:\n{text}"
        )));
    }
    findings
}

/// The IDs of the accepted npm advisories, read from [`NPM_ACCEPTED`] (none when it is missing).
fn accepted_npm(root: &Path) -> anyhow::Result<BTreeSet<String>> {
    match fs::read_to_string(root.join(NPM_ACCEPTED)) {
        Ok(text) => accepted_ids(&text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeSet::new()),
        Err(error) => Err(error.into()),
    }
}

/// The IDs in the text of the file of accepted npm advisories (`accepted = [{ id, reason }]`).
fn accepted_ids(text: &str) -> anyhow::Result<BTreeSet<String>> {
    let value: toml::Value = toml::from_str(text)?;
    Ok(value
        .get("accepted")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("id").and_then(toml::Value::as_str))
        .map(str::to_owned)
        .collect())
}

/// `npm audit` for one lockfile, read as JSON from its standard output (warnings go elsewhere).
fn npm_advisories(
    root: &Path,
    lockfile: &str,
    accepted: &BTreeSet<String>,
) -> anyhow::Result<Vec<Finding>> {
    let folder = root.join(
        Path::new(lockfile)
            .parent()
            .unwrap_or_else(|| Path::new("")),
    );
    let output = tools::run(
        Path::new(crate::steps::npm()),
        &["audit", "--json"],
        &folder,
    )?;
    Ok(npm_findings(
        lockfile,
        output.success,
        &output.stdout,
        accepted,
    ))
}

/// The findings in the JSON that `npm audit --json` printed. A report that cannot be read, one
/// without the list of vulnerabilities, one whose own total disagrees with the list, and a failure
/// with a total of zero are findings too, so a changed report never passes by being empty.
fn npm_findings(
    lockfile: &str,
    success: bool,
    stdout: &str,
    accepted: &BTreeSet<String>,
) -> Vec<Finding> {
    let general = |message: String| vec![Finding::at(lockfile, message)];
    let Ok(report) = serde_json::from_str::<Value>(stdout) else {
        return general(format!(
            "npm audit printed no report (is the network reachable?):\n{stdout}"
        ));
    };
    if let Some(error) = report.get("error") {
        let summary = error["summary"].as_str().unwrap_or("");
        return general(format!("npm audit could not check the packages: {summary}"));
    }
    let Some(packages) = report["vulnerabilities"].as_object() else {
        return general("the report of npm audit has no list of vulnerabilities".to_owned());
    };
    let total = report["metadata"]["vulnerabilities"]["total"].as_u64();
    if total != Some(packages.len() as u64) {
        return general(format!(
            "the report of npm audit names {} vulnerable packages but counts {total:?}",
            packages.len()
        ));
    }
    if !success && packages.is_empty() {
        return general("npm audit failed without naming a vulnerability".to_owned());
    }
    let accepted_packages = accepted_packages(packages, accepted);
    packages
        .iter()
        .filter(|(name, _)| !accepted_packages.contains(name.as_str()))
        .map(|(name, entry)| {
            let severity = entry["severity"].as_str().unwrap_or("?");
            Finding::at(
                format!("{lockfile}: {name}"),
                format!(
                    "has a known {severity} vulnerability. Update the package (`npm update {name}`), \
                     change the version pinned in package.json, or add an override when its parent \
                     pins the old version. When no fix exists and Bugaboo is not affected, accept \
                     it in {NPM_ACCEPTED} with a reason and a row in docs/third-party/README.md."
                ),
            )
        })
        .collect()
}

/// The packages whose every advisory is accepted: each `via` is an advisory (by its GHSA ID in
/// its URL) that is accepted, or the name of another package that is accepted itself.
fn accepted_packages<'a>(
    packages: &'a serde_json::Map<String, Value>,
    accepted: &BTreeSet<String>,
) -> BTreeSet<&'a str> {
    let mut result = BTreeSet::new();
    loop {
        let before = result.len();
        for (name, entry) in packages {
            let via = entry["via"].as_array().map_or(&[][..], Vec::as_slice);
            let all = !via.is_empty()
                && via.iter().all(|item| match item.as_str() {
                    Some(other) => result.contains(other),
                    None => item["url"]
                        .as_str()
                        .and_then(|url| url.rsplit('/').next())
                        .is_some_and(|id| accepted.contains(id)),
                });
            if all {
                result.insert(name.as_str());
            }
        }
        if result.len() == before {
            return result;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADVISORY: &str = r#"{"fields":{"advisory":{"id":"RUSTSEC-2023-0071","package":"rsa"},"code":"vulnerability","message":"Marvin Attack","severity":"error"},"type":"diagnostic"}"#;
    const SUMMARY: &str = r#"{"fields":{"advisories":{"errors":0,"warnings":0}},"type":"summary"}"#;
    const OFFLINE: &str = r#"{"fields":{"level":"ERROR","message":"failed to fetch advisory database: network unreachable"},"type":"log"}"#;

    #[test]
    fn bad_example_rust() {
        let findings = deny_findings(false, &format!("{ADVISORY}\n{SUMMARY}"));
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Marvin Attack"));
    }

    #[test]
    fn good_example_rust() {
        assert!(deny_findings(true, SUMMARY).is_empty());
    }

    #[test]
    fn an_advisory_database_that_could_not_be_fetched_is_a_finding() {
        let findings = deny_findings(false, &format!("{OFFLINE}\n{SUMMARY}"));
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0]
                .message
                .contains("failed to fetch advisory database")
        );
    }

    #[test]
    fn a_failure_without_a_diagnostic_is_a_finding() {
        assert_eq!(deny_findings(false, "something odd").len(), 1);
    }

    fn none() -> BTreeSet<String> {
        BTreeSet::new()
    }

    const VITE: &str = r#"{"vulnerabilities":{"vite":{"name":"vite","severity":"high","via":[{"url":"https://github.com/advisories/GHSA-aaaa-bbbb-cccc"}]}},"metadata":{"vulnerabilities":{"total":1}}}"#;

    #[test]
    fn bad_example_npm() {
        let findings = npm_findings("package-lock.json", false, VITE, &none());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("high"));
    }

    #[test]
    fn good_example_npm() {
        let report = r#"{"vulnerabilities":{},"metadata":{"vulnerabilities":{"total":0}}}"#;
        assert!(npm_findings("package-lock.json", true, report, &none()).is_empty());
    }

    #[test]
    fn an_accepted_advisory_passes_and_so_does_a_package_that_only_carries_it() {
        let accepted = accepted_ids(
            "accepted = [\n  { id = \"GHSA-aaaa-bbbb-cccc\", reason = \"x §\" },\n]\n",
        )
        .unwrap();
        assert!(npm_findings("package-lock.json", false, VITE, &accepted).is_empty());
        let chain = r#"{"vulnerabilities":{"vite":{"severity":"high","via":[{"url":"https://github.com/advisories/GHSA-aaaa-bbbb-cccc"}]},"tool":{"severity":"high","via":["vite"]}},"metadata":{"vulnerabilities":{"total":2}}}"#;
        assert!(npm_findings("package-lock.json", false, chain, &accepted).is_empty());
    }

    #[test]
    fn an_audit_that_could_not_run_is_a_finding() {
        let offline = r#"{"error":{"code":"ENOTFOUND","summary":"request failed"}}"#;
        assert_eq!(
            npm_findings("package-lock.json", false, offline, &none()).len(),
            1
        );
        assert_eq!(
            npm_findings("package-lock.json", false, "", &none()).len(),
            1
        );
    }

    #[test]
    fn a_report_that_does_not_add_up_is_a_finding() {
        let without_list = r#"{"metadata":{"vulnerabilities":{"total":3}}}"#;
        assert_eq!(
            npm_findings("package-lock.json", false, without_list, &none()).len(),
            1
        );
        let disagrees = r#"{"vulnerabilities":{},"metadata":{"vulnerabilities":{"total":3}}}"#;
        assert_eq!(
            npm_findings("package-lock.json", false, disagrees, &none()).len(),
            1
        );
        let failed = r#"{"vulnerabilities":{},"metadata":{"vulnerabilities":{"total":0}}}"#;
        assert_eq!(
            npm_findings("package-lock.json", false, failed, &none()).len(),
            1
        );
    }
}
