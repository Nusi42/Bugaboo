//! DEV-PORTS: no default framework ports; every port comes from `tools/dev/ports.json` and is
//! strict (ADR-0023).

use std::fs;

use super::node_tool::Selection;
use super::{Check, Context, Finding};

const PORTS_FILE: &str = "tools/dev/ports.json";

/// Ports that other projects on the development PC already use (ADR-0023).
const DEFAULT_PORTS: [u32; 5] = [5173, 4173, 3000, 8080, 51204];

/// All our ports: the blocks of every work folder (ADR-0023). Free on the development PC and
/// below the dynamic range of Windows (49152 and up).
const ALLOWED: std::ops::RangeInclusive<u32> = 47200..=48099;

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

/// Rust and TOML files that could name a port. This check's own list of default ports is not
/// scanned.
const OTHER: Selection = Selection {
    folders: &["crates", "xtask", "tools", ".cargo"],
    files: &[
        "Cargo.toml",
        "deny.toml",
        "clippy.toml",
        "rustfmt.toml",
        "rust-toolchain.toml",
    ],
    endings: &[".rs", ".toml"],
};

const THIS_FILE: &str = "xtask/src/checks/dev_ports.rs";

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
        for path in OTHER.pick(context) {
            if path != THIS_FILE {
                let text = fs::read_to_string(root.join(&path))?;
                findings.extend(default_port_findings(&path, &text));
            }
        }
        Ok(findings)
    }
}

/// The ports file: blocks of `stride` ports from `first`, one block for each work folder, and the
/// offset of every port inside a block. Every port of every block must be in [`ALLOWED`], and no
/// two ports of a block may be the same.
fn check_ports_file(text: &str) -> Vec<Finding> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(text) else {
        return vec![Finding::at(
            PORTS_FILE,
            "The ports file must be a JSON object.",
        )];
    };
    let number = |key: &str| map.get(key).and_then(serde_json::Value::as_u64);
    let (Some(first), Some(stride), Some(blocks)) =
        (number("first"), number("stride"), number("blocks"))
    else {
        return vec![Finding::at(
            PORTS_FILE,
            "The ports file needs the numbers `first`, `stride` and `blocks`.",
        )];
    };
    let Some(serde_json::Value::Object(offsets)) = map.get("offsets") else {
        return vec![Finding::at(
            PORTS_FILE,
            "The ports file needs `offsets`: the offset of every port inside a block.",
        )];
    };
    let mut findings = Vec::new();
    let last = first + stride.saturating_mul(blocks).saturating_sub(1);
    let in_range = |port: u64| u32::try_from(port).is_ok_and(|port| ALLOWED.contains(&port));
    if stride == 0 || blocks == 0 || !in_range(first) || !in_range(last) {
        findings.push(Finding::at(
            PORTS_FILE,
            format!(
                "The blocks run from {first} to {last}; they must lie inside {}-{} (ADR-0023).",
                ALLOWED.start(),
                ALLOWED.end()
            ),
        ));
    }
    let mut seen: Vec<(&str, u64)> = Vec::new();
    for (name, value) in offsets {
        let Some(offset) = value.as_u64() else {
            findings.push(Finding::at(
                PORTS_FILE,
                format!("`{name}` must be an offset (a number)."),
            ));
            continue;
        };
        if offset >= stride {
            findings.push(Finding::at(
                PORTS_FILE,
                format!("`{name}` is {offset}; an offset must be below the stride {stride}."),
            ));
        }
        if let Some((other, _)) = seen.iter().find(|(_, used)| *used == offset) {
            findings.push(Finding::at(
                PORTS_FILE,
                format!("`{name}` and `{other}` both use the offset {offset}."),
            ));
        }
        seen.push((name, offset));
    }
    findings
}

fn check_config(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = default_port_findings(path, text);
    if path.ends_with("package.json") {
        if text.contains("--port") {
            findings.push(Finding::at(
                path,
                format!("Do not pass `--port` in scripts; the tool configs read {PORTS_FILE}."),
            ));
        }
        return findings;
    }
    let mut problem = |message: String| findings.push(Finding::at(path, message));
    if !text.contains("readPorts(") {
        problem(format!(
            "This config must read its ports from {PORTS_FILE} with `readPorts()`."
        ));
    }
    if has_literal_port(text) {
        problem(format!(
            "A port is typed as a number. Use `readPorts()` so it comes from {PORTS_FILE}."
        ));
    }
    if path.contains("playwright.config") {
        if text.contains("webServer") && !text.contains("reuseExistingServer: false") {
            problem(
                "Set `reuseExistingServer: false`, so a busy port fails loudly (strict port)."
                    .to_owned(),
            );
        }
    } else {
        let blocks: &[&str] = if path.contains("vite.config") {
            &["server", "preview"]
        } else {
            &["api"]
        };
        for block in blocks {
            let Some(body) = block_body(text, block) else {
                problem(format!(
                    "Add `{block}: {{ port: ports.<name>, strictPort: true }}`; every dev server needs its port from {PORTS_FILE}."
                ));
                continue;
            };
            if !body.contains("port: ports.") {
                problem(format!(
                    "The `{block}` block must set `port: ports.<name>`, read from {PORTS_FILE}."
                ));
            }
            if !body.contains("strictPort: true") {
                problem(format!(
                    "The `{block}` block must set `strictPort: true`, so a busy port fails loudly."
                ));
            }
        }
    }
    findings
}

/// The text between the braces of `name: { … }`, if the block exists.
fn block_body<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let start = text.find(&format!("{name}: {{"))? + name.len() + 3;
    let mut depth = 1_usize;
    for (offset, character) in text[start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..start + offset]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Words that put a line in "talking about ports or addresses" context.
const ADDRESS_WORDS: [&str; 11] = [
    "port",
    "://",
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
    "socketaddr",
    "ipv4addr",
    "ipv6addr",
    "tcplistener",
    "bind(",
    "listen(",
];

/// Lines that name a default framework port. A number counts only in port context: a line that
/// talks about ports or addresses, a `host:NNNN` inside a string, or the number after an address
/// array such as `([127, 0, 0, 1], 8080)`. So an unrelated number such as `timeout: 3000` is fine.
fn default_port_findings(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        let about_ports = ADDRESS_WORDS.iter().any(|word| lower.contains(word));
        let in_line = numbers(line);
        let found = DEFAULT_PORTS.iter().find(|port| {
            let number = port.to_string();
            in_line.contains(port)
                && (about_ports
                    || colon_port_in_string(line, &number)
                    || after_address_array(line, &number))
        });
        let Some(number) = found else {
            continue;
        };
        findings.push(Finding::at(
            format!("{path}:{}", index + 1),
            format!(
                "{number} is a default framework port. Read the port from {PORTS_FILE} instead."
            ),
        ));
    }
    findings
}

/// `"host:3000"`: the number follows a colon that follows a host character, inside quotes.
fn colon_port_in_string(line: &str, number: &str) -> bool {
    line.match_indices(&format!(":{number}"))
        .any(|(index, matched)| {
            let after = line[index + matched.len()..].chars().next();
            let before = line[..index].chars().next_back();
            let in_string = line[..index].matches('"').count() % 2 == 1;
            in_string
                && before.is_some_and(|c| c.is_ascii_alphanumeric() || c == ']')
                && after.is_none_or(|c| !c.is_ascii_digit())
        })
}

/// `([127, 0, 0, 1], 8080)`: the number right after the closing bracket of an address array.
fn after_address_array(line: &str, number: &str) -> bool {
    line.match_indices(number)
        .any(|(index, _)| line[..index].trim_end().ends_with("],"))
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

    const GOOD: &str = "import { readPorts } from './ports.ts';
const ports = readPorts();
export default { server: { port: ports.web, strictPort: true }, preview: { port: ports.webPreview, strictPort: true } };
";

    #[test]
    fn good_example() {
        assert!(check_config("web/app/vite.config.ts", GOOD).is_empty());
        let repo = Repo::discover().unwrap();
        let ports = fs::read_to_string(repo.path(PORTS_FILE)).unwrap();
        assert!(check_ports_file(&ports).is_empty());
    }

    fn messages(path: &str, text: &str) -> String {
        check_config(path, text)
            .iter()
            .map(|finding| finding.message.clone())
            .collect()
    }

    #[test]
    fn bad_example_default_and_literal_ports_fail() {
        let bad = "export default { server: { port: 5173 } };
";
        let text = messages("web/app/vite.config.ts", bad);
        assert!(text.contains("default framework port"));
        assert!(text.contains("typed as a number"));
        assert!(text.contains("strictPort"));
        assert!(text.contains("readPorts"));
    }

    #[test]
    fn missing_preview_block_fails() {
        let no_preview = "import { readPorts } from './p.ts';
const ports = readPorts();
export default { server: { port: ports.web, strictPort: true } };
";
        assert!(messages("web/app/vite.config.ts", no_preview).contains("preview"));
    }

    #[test]
    fn block_without_a_port_from_ports_fails() {
        let no_port = GOOD.replace("port: ports.webPreview, ", "");
        assert!(messages("web/app/vite.config.ts", &no_port).contains("preview"));
        let literal = GOOD.replace("port: ports.webPreview", "port: 47299");
        assert!(messages("web/app/vite.config.ts", &literal).contains("preview"));
    }

    #[test]
    fn config_without_read_ports_fails() {
        assert!(
            messages(
                "web/app/vitest.config.ts",
                "export default {};
"
            )
            .contains("readPorts")
        );
    }

    #[test]
    fn port_that_is_not_strict_fails() {
        let loose = GOOD.replace(", strictPort: true", "");
        assert!(!check_config("web/app/vite.config.ts", &loose).is_empty());
    }

    #[test]
    fn unrelated_numbers_are_not_ports() {
        let ok = format!(
            "{GOOD}const timeout = 3000;
"
        );
        assert!(check_config("web/app/vite.config.ts", &ok).is_empty());
    }

    #[test]
    fn rust_and_toml_default_ports_fail() {
        assert!(
            !default_port_findings("crates/bb-server/src/main.rs", "let port = 8080;").is_empty()
        );
        assert!(!default_port_findings("x.toml", "url = \"http://localhost:3000\"").is_empty());
        assert!(default_port_findings("x.rs", "let retries = 3000;").is_empty());
    }

    #[test]
    fn rust_addresses_with_default_ports_fail() {
        for bad in [
            "let address = \"0.0.0.0:3000\";",
            "let a = SocketAddr::from(([127, 0, 0, 1], 8080));",
            "let l = tokio::net::TcpListener::bind((\"::\", 5173)).await?;",
            "let a = ([0, 0, 0, 0], 4173).into();",
            "let url = \"api.local:51204/path\";",
        ] {
            assert!(
                !default_port_findings("crates/x/src/lib.rs", bad).is_empty(),
                "{bad}"
            );
        }
    }

    #[test]
    fn rust_numbers_that_are_not_ports_pass() {
        for ok in [
            "let timeout = Duration::from_millis(3000);",
            "let sizes = [8080, 3000];",
            "const RETRY_MS: u64 = 5173;",
        ] {
            assert!(
                default_port_findings("crates/x/src/lib.rs", ok).is_empty(),
                "{ok}"
            );
        }
    }

    #[test]
    fn scripts_may_not_pass_a_port() {
        let bad = r#"{ "scripts": { "dev": "vite --port 47201" } }"#;
        assert!(!check_config("package.json", bad).is_empty());
    }

    #[test]
    fn ports_outside_our_range_or_inside_each_other_fail() {
        let file = |first: u64, stride: u64, blocks: u64, offsets: &str| {
            format!(
                r#"{{"first":{first},"stride":{stride},"blocks":{blocks},"offsets":{offsets}}}"#
            )
        };
        let good = file(47200, 100, 9, r#"{"web":1,"server":0}"#);
        assert!(check_ports_file(&good).is_empty());
        // Blocks that start in front of the range, or run past it into the dynamic ports.
        assert!(!check_ports_file(&file(5000, 100, 9, r#"{"web":1}"#)).is_empty());
        assert!(!check_ports_file(&file(47200, 100, 10, r#"{"web":1}"#)).is_empty());
        assert!(!check_ports_file(&file(47200, 10000, 9, r#"{"web":1}"#)).is_empty());
        // An offset outside its block, two ports on one offset, a port that is no number.
        assert!(!check_ports_file(&file(47200, 100, 9, r#"{"web":100}"#)).is_empty());
        assert!(!check_ports_file(&file(47200, 100, 9, r#"{"web":1,"server":1}"#)).is_empty());
        assert!(!check_ports_file(&file(47200, 100, 9, r#"{"web":"x"}"#)).is_empty());
        // The old flat form is refused with a pointer to the new one.
        assert!(!check_ports_file(r#"{ "web": 47201, "server": 47200 }"#).is_empty());
        assert!(!check_ports_file("[]").is_empty());
    }
}
