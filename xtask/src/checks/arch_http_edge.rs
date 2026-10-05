//! ARCH-HTTP-EDGE: only `bb-api` and `bb-mcp` use HTTP crates, and only `bb-server` and
//! `bb-sample` wire the adapters (architecture §4, dependency rule 4).
//!
//! ARCH-DEPS keeps crates to the edges of the module map. This check adds the two rules about the
//! edge of the system by crate names, so they hold even if the map is edited:
//!
//! - an HTTP crate (a server, a client, a WebSocket or an OpenAPI helper) is a normal or build
//!   dependency of `bb-api` and `bb-mcp` only. Tests may use them anywhere, and `xtask` is the
//!   developer's tool, not part of the product;
//! - the adapter crates (they implement the ports of `bb-app`) are normal dependencies of the
//!   wiring crates only: `bb-server`, and the developer program `bb-sample` that makes the sample
//!   organization through the same use cases (M2 WP-06). Every other crate knows only the ports.

use super::{Check, Context, Finding};
use crate::metadata::{self, Package};

/// Crates that speak or serve HTTP.
const HTTP_CRATES: [&str; 25] = [
    "axum",
    "axum-core",
    "axum-extra",
    "hyper",
    "hyper-util",
    "h2",
    "http",
    "http-body",
    "http-body-util",
    "tower",
    "tower-http",
    "tower-service",
    "tower-layer",
    "tower_governor",
    "utoipa-axum",
    "reqwest",
    "ureq",
    "isahc",
    "surf",
    "tungstenite",
    "tokio-tungstenite",
    "actix-web",
    "warp",
    "rocket",
    "poem",
];

/// The crates that may use them.
const HTTP_CRATES_ALLOWED_IN: [&str; 3] = ["bb-api", "bb-mcp", "xtask"];

/// Crates that implement the ports of `bb-app`.
const ADAPTERS: [&str; 4] = ["bb-store", "bb-files", "bb-ai", "bb-notify"];

/// The only crates that may depend on the adapters: the program, and the developer tool that
/// makes the sample organization (not shipped before OPS-12).
const WIRING_CRATES: [&str; 2] = ["bb-server", "bb-sample"];

pub(crate) struct ArchHttpEdge;

impl Check for ArchHttpEdge {
    fn id(&self) -> &'static str {
        "ARCH-HTTP-EDGE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&metadata::load(context.repo.root())?))
    }
}

fn evaluate(packages: &[Package]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for package in packages {
        let name = package.name.as_str();
        for dependency in &package.dependencies {
            let shipped = dependency.kind.as_deref() != Some("dev");
            if shipped
                && HTTP_CRATES.contains(&dependency.name.as_str())
                && !HTTP_CRATES_ALLOWED_IN.contains(&name)
            {
                findings.push(Finding::at(
                    name,
                    format!(
                        "depends on the HTTP crate `{}`. Only bb-api and bb-mcp speak HTTP; move this code there.",
                        dependency.name
                    ),
                ));
            }
            if shipped
                && ADAPTERS.contains(&dependency.name.as_str())
                && !WIRING_CRATES.contains(&name)
            {
                findings.push(Finding::at(
                    name,
                    format!(
                        "depends on the adapter `{}`. Only {} wire adapters to ports; depend on the port in bb-app instead.",
                        dependency.name,
                        WIRING_CRATES.join(" and ")
                    ),
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::fixtures::package;

    fn with(name: &str, deps: &[(&str, Option<&str>)]) -> Package {
        let mut built = package(name, &[]);
        for (dep, kind) in deps {
            let mut one = package(name, &[dep]).dependencies;
            one[0].kind = kind.map(str::to_owned);
            built.dependencies.append(&mut one);
        }
        built
    }

    #[test]
    fn bad_example() {
        let packages = [
            with("bb-app", &[("axum", None)]),
            with("bb-core", &[("reqwest", Some("build"))]),
            with("bb-jobs", &[("bb-store", None)]),
        ];
        let found: Vec<String> = evaluate(&packages)
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found[0].contains("HTTP crate `axum`"));
        assert!(found[1].contains("`reqwest`"));
        assert!(found[2].contains("adapter `bb-store`"));
        assert!(found[2].contains("bb-server and bb-sample"), "{}", found[2]);
    }

    #[test]
    fn bad_example_the_sample_tool_does_not_get_to_speak_http() {
        let packages = [with("bb-sample", &[("axum", None), ("bb-store", None)])];
        let found = evaluate(&packages);
        assert_eq!(found.len(), 1, "only the HTTP crate is refused");
        assert!(found[0].message.contains("HTTP crate `axum`"));
    }

    #[test]
    fn bad_example_a_crate_that_only_looks_like_the_sample_tool_wires_nothing() {
        let packages = [with("bb-sample-extras", &[("bb-store", None)])];
        assert_eq!(evaluate(&packages).len(), 1);
    }

    #[test]
    fn good_example() {
        let packages = [
            with("bb-api", &[("axum", None), ("tower", None)]),
            with("bb-mcp", &[("axum", None)]),
            with("bb-app", &[("tower", Some("dev"))]),
            with("bb-server", &[("bb-store", None), ("bb-api", None)]),
            with("bb-sample", &[("bb-store", None), ("bb-app", None)]),
            with("bb-store", &[("bb-testing", Some("dev"))]),
        ];
        assert!(evaluate(&packages).is_empty());
    }

    #[test]
    fn the_real_workspace_follows_the_rules() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let found = evaluate(&metadata::load(root).unwrap());
        let messages: Vec<&String> = found.iter().map(|f| &f.message).collect();
        assert!(found.is_empty(), "{messages:?}");
    }
}
