//! DEV-PORTS: no default framework ports; every port comes from `tools/dev/ports.json` and is
//! strict (ADR-0023).

use std::fs;

use super::node_tool::Selection;
use super::{Check, Context, Finding};

const PORTS_FILE: &str = "tools/dev/ports.json";

/// Ports that other projects on the development PC already use (ADR-0023).
const DEFAULT_PORTS: [u32; 5] = [5173, 4173, 3000, 8080, 51204];

/// Our block of ports.
const ALLOWED: std::ops::RangeInclusive<u32> = 47200..=47299;

/// The tool configs and package files that could name a port.
const CONFIGS: Selection = Selection {
    folders: &["web"],
    files: &["package.json"],
    endings: &[
        "vite.config.ts",
        "vite.config.mts",
        "vite.config.js",
        "vite.config.mjs",
        "vitest.config.ts",
        "vitest.config.mts",
        "vitest.config.js",
        "vitest.config.mjs",
        "playwright.config.ts",
        "playwright.config.mts",
        "playwright.config.js",
        "playwright.config.mjs",
        "package.json",
    ],
};

pub(crate) struct DevPorts;

impl Check for DevPorts {
    fn id(&self) -> &'static str {
        "DEV-PORTS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let mut findings = Vec::new();
        match fs::read_to_string(root.join(PORTS_FILE)) {
            Ok(text) => findings.extend(check_ports_file(&text)),
            Err(_) => findings.push(Finding::at(PORTS_FILE, "The ports file is missing.")),
        }
        for path in CONFIGS.pick(context) {
            let text = fs::read_to_string(root.join(&path))?;
            findings.extend(check_config(&path, &text));
        }
        Ok(findings)
    }
}

fn check_ports_file(text: &str) -> Vec<Finding> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(text) else {
        return vec![Finding::at(
            PORTS_FILE,
            "The ports file must be a JSON object.",
        )];
    };
    let mut findings = Vec::new();
    let mut seen: Vec<(&str, u64)> = Vec::new();
    for (name, value) in &map {
        if name.starts_with('$') {
            continue;
        }
        let Some(port) = value.as_u64() else {
            findings.push(Finding::at(
                PORTS_FILE,
                format!("`{name}` must be a port number."),
            ));
            continue;
        };
        if !u32::try_from(port).is_ok_and(|port| ALLOWED.contains(&port)) {
            findings.push(Finding::at(
                PORTS_FILE,
                format!("`{name}` is {port}; use the block 47200-47299 (ADR-0023)."),
            ));
        }
        if let Some((other, _)) = seen.iter().find(|(_, used)| *used == port) {
            findings.push(Finding::at(
                PORTS_FILE,
                format!("`{name}` and `{other}` both use {port}."),
            ));
        }
        seen.push((name, port));
    }
    findings
}

fn check_config(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for number in numbers(text) {
        if DEFAULT_PORTS.contains(&number) {
            findings.push(Finding::at(
                path,
                format!(
                    "{number} is a default framework port. Read the port from {PORTS_FILE} instead."
                ),
            ));
        }
    }
    if path.ends_with("package.json") {
        if text.contains("--port") {
            findings.push(Finding::at(
                path,
                format!("Do not pass `--port` in scripts; the tool configs read {PORTS_FILE}."),
            ));
        }
        return findings;
    }
    let port_keys = text.matches("port:").count();
    if has_literal_port(text) {
        findings.push(Finding::at(
            path,
            format!(
                "A port is typed as a number. Use `readPorts()` so it comes from {PORTS_FILE}."
            ),
        ));
    }
    if port_keys > 0 && !text.contains("readPorts(") {
        findings.push(Finding::at(
            path,
            format!("This config sets a port but does not read {PORTS_FILE}; use `readPorts()`."),
        ));
    }
    if path.contains("playwright.config") {
        if text.contains("webServer") && !text.contains("reuseExistingServer: false") {
            findings.push(Finding::at(
                path,
                "Set `reuseExistingServer: false`, so a busy port fails loudly (strict port).",
            ));
        }
    } else if port_keys > text.matches("strictPort: true").count() {
        findings.push(Finding::at(
            path,
            "Every `port:` needs `strictPort: true` next to it, so a busy port fails loudly.",
        ));
    }
    findings
}

/// Every whole number in the text.
fn numbers(text: &str) -> Vec<u32> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty() && word.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|word| word.parse().ok())
        .collect()
}

/// `port: 1234`, `port = 1234` or `"port": 1234`.
fn has_literal_port(text: &str) -> bool {
    text.match_indices("port").any(|(index, _)| {
        let after = text[index + 4..]
            .trim_start_matches(['"', '\''])
            .trim_start();
        after
            .strip_prefix([':', '='])
            .is_some_and(|rest| rest.trim_start().starts_with(|c: char| c.is_ascii_digit()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Repo;

    const GOOD: &str = "import { readPorts } from './ports.ts';\nconst ports = readPorts();\nexport default { server: { port: ports.web, strictPort: true } };\n";

    #[test]
    fn good_example() {
        assert!(check_config("web/app/vite.config.ts", GOOD).is_empty());
        let repo = Repo::discover().unwrap();
        let ports = fs::read_to_string(repo.path(PORTS_FILE)).unwrap();
        assert!(check_ports_file(&ports).is_empty());
    }

    #[test]
    fn default_and_literal_ports_fail() {
        let bad = "export default { server: { port: 5173 } };\n";
        let messages: String = check_config("web/app/vite.config.ts", bad)
            .iter()
            .map(|finding| finding.message.clone())
            .collect();
        assert!(messages.contains("default framework port"));
        assert!(messages.contains("typed as a number"));
        assert!(messages.contains("strictPort"));
    }

    #[test]
    fn port_that_is_not_strict_fails() {
        let loose = GOOD.replace(", strictPort: true", "");
        assert!(!check_config("web/app/vite.config.ts", &loose).is_empty());
    }

    #[test]
    fn scripts_may_not_pass_a_port() {
        let bad = r#"{ "scripts": { "dev": "vite --port 47201" } }"#;
        assert!(!check_config("package.json", bad).is_empty());
    }

    #[test]
    fn ports_outside_our_block_fail() {
        let bad = r#"{ "web": 5173, "server": 47200 }"#;
        assert!(!check_ports_file(bad).is_empty());
        assert!(!check_ports_file(r#"{ "web": 47201, "server": 47201 }"#).is_empty());
    }
}
