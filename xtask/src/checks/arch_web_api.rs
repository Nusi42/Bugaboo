//! ARCH-WEB-API: the browser app calls the server only through `@bugaboo/api-client`
//! (ADR-0008, architecture §4 rule 5).
//!
//! Outside `web/packages/api-client/` no TypeScript file may send requests by itself: no
//! `fetch(`, `XMLHttpRequest`, `WebSocket`, `EventSource` or `sendBeacon(`, no HTTP library
//! (including the ones the client is built on), and no import from inside the client package (only
//! its public entry). The routes themselves are written as the client's typed addresses
//! (`api.client.GET("/api/v1/status")`), which the compiler checks against the OpenAPI document. The Playwright tests (`e2e/`) drive a
//! real browser and the tooling folder reads files; neither is part of the app.

use super::web_source::{WebSource, code_of, read_web_sources};
use super::{Check, Context, Finding};

/// The one place that may talk to the server.
const CLIENT_PACKAGE: &str = "web/packages/api-client/";
/// Test drivers and tooling, which are not the app.
const EXEMPT_PARTS: [&str; 2] = ["/e2e/", "web/tooling/"];

/// Calls that send requests by themselves.
const CALLS: [&str; 3] = ["XMLHttpRequest", "new WebSocket(", "new EventSource("];
/// HTTP libraries; the client package is the only one that may use the first two.
const MODULES: [&str; 8] = [
    "openapi-fetch",
    "openapi-react-query",
    "axios",
    "ky",
    "got",
    "node-fetch",
    "undici",
    "superagent",
];
const CLIENT_NAME: &str = "@bugaboo/api-client";

pub(crate) struct ArchWebApi;

impl Check for ArchWebApi {
    fn id(&self) -> &'static str {
        "ARCH-WEB-API"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&read_web_sources(context.repo)?))
    }
}

fn evaluate(sources: &[WebSource]) -> Vec<Finding> {
    sources
        .iter()
        .filter(|(path, _)| {
            !path.starts_with(CLIENT_PACKAGE) && !EXEMPT_PARTS.iter().any(|p| path.contains(p))
        })
        .flat_map(|(path, text)| scan(path, text))
        .collect()
}

fn scan(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let code = code_of(line);
        if let Some(problem) = problem_in(code) {
            findings.push(Finding::at(
                format!("{path}:{}", index + 1),
                format!("{problem} The app talks to the server only through `{CLIENT_NAME}`."),
            ));
        }
    }
    findings
}

fn problem_in(code: &str) -> Option<String> {
    if calls_fetch(code) {
        return Some("`fetch(` sends a request by itself.".to_owned());
    }
    if code.contains("sendBeacon(") {
        return Some("`sendBeacon(` sends a request by itself.".to_owned());
    }
    if let Some(call) = CALLS.iter().find(|call| code.contains(**call)) {
        return Some(format!("`{call}` talks to the server by itself."));
    }
    if let Some(module) = imported(code) {
        if MODULES.contains(&module) {
            return Some(format!("`{module}` is an HTTP library."));
        }
        if module
            .strip_prefix(CLIENT_NAME)
            .is_some_and(|rest| rest.starts_with('/'))
        {
            return Some(format!(
                "`{module}` reaches into the client package; import from `{CLIENT_NAME}`."
            ));
        }
    }
    None
}

/// `fetch(` as a call of the global function (also `window.fetch(`, `globalThis.fetch(`), not a
/// method of something else (`client.fetch(`) and not a longer name (`prefetch(`).
fn calls_fetch(code: &str) -> bool {
    code.match_indices("fetch(").any(|(at, _)| {
        let before = &code[..at];
        match before.chars().last() {
            None => true,
            Some('.') => ["window.", "globalThis.", "self."]
                .iter()
                .any(|global| before.ends_with(global)),
            Some(c) => !(c.is_alphanumeric() || c == '_' || c == '$'),
        }
    })
}

/// The module named by `import … from "x"` or `import "x"` or `import("x")`.
fn imported(code: &str) -> Option<&str> {
    let after = code
        .split_once(" from ")
        .map(|(_, rest)| rest)
        .or_else(|| code.trim_start().strip_prefix("import "))
        .or_else(|| code.split_once("import(").map(|(_, rest)| rest))?;
    let rest = after.trim_start().trim_start_matches('(');
    let quote = rest
        .chars()
        .next()
        .filter(|c| matches!(c, '"' | '\'' | '`'))?;
    let inner = &rest[1..];
    inner.split_once(quote).map(|(name, _)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(path: &str, text: &str) -> Vec<String> {
        evaluate(&[(path.to_owned(), text.to_owned())])
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn bad_example() {
        for bad in [
            "const r = await fetch(\"/x\");",
            "window.fetch(url)",
            "const x = new XMLHttpRequest();",
            "new WebSocket(url)",
            "navigator.sendBeacon(u)",
            "import axios from \"axios\";",
            "import createClient from \"openapi-fetch\";",
            "import { x } from \"@bugaboo/api-client/src/client.ts\";",
        ] {
            assert!(!run("web/app/src/a.ts", bad).is_empty(), "{bad}");
        }
    }

    #[test]
    fn good_example() {
        for good in [
            "import { createApi } from \"@bugaboo/api-client\";",
            "const data = await api.client.GET(\"/api/v1/status\");",
            "client.fetch(request);",
            "prefetch(route);",
            "// fetch(\"/api\") is how it used to be",
            "const url = \"https://example.test/page\";",
        ] {
            let found = run("web/app/src/a.ts", good);
            assert!(found.is_empty(), "{good}: {found:?}");
        }
    }

    #[test]
    fn the_client_package_and_test_drivers_are_not_checked() {
        let bad = "await fetch(\"/api/v1/status\");";
        assert!(run("web/packages/api-client/src/client.ts", bad).is_empty());
        assert!(run("web/app/e2e/hello.spec.ts", bad).is_empty());
        assert!(run("web/tooling/ports.ts", bad).is_empty());
        assert!(!run("web/packages/ui/src/a.ts", bad).is_empty());
    }
}
