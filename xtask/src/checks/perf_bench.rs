//! PERF-BENCH: the latency budgets hold on the large dataset (PERF-01, architecture §11).
//!
//! This is the part that runs at commit time: `tools/checks/perf-bench.toml` is usable and every
//! scenario of `web/tooling/bench/scenarios.ts` names a budget in it. The numbers are judged by
//! the nightly run (`cargo xtask bench`, `cargo xtask nightly`), which fails a scenario over its
//! budget with the scenario, the number and the budget.

use super::{Check, Context, Finding};
use crate::bench_budgets::{BUDGETS_PATH, Budgets, SCENARIOS_PATH, problems, scenarios};
use crate::source::read_text;

pub(crate) struct PerfBench;

impl Check for PerfBench {
    fn id(&self) -> &'static str {
        "PERF-BENCH"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        evaluate(
            &read_text(repo, BUDGETS_PATH)?,
            &read_text(repo, SCENARIOS_PATH)?,
        )
    }
}

/// What is wrong with a settings file and a registry of scenarios.
fn evaluate(budgets: &str, registry: &str) -> anyhow::Result<Vec<Finding>> {
    let budgets = Budgets::parse(budgets)?;
    Ok(problems(&budgets, &scenarios(registry))
        .into_iter()
        .map(|(place, message)| Finding::at(place, message))
        .collect())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// A fixture folder: `tools/checks/fixtures/PERF-BENCH/<kind>`, with its own files.
    fn run_fixture(kind: &str) -> Vec<Finding> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tools/checks/fixtures/PERF-BENCH")
            .join(kind);
        let read = |name: &str| std::fs::read_to_string(root.join(name)).unwrap();
        evaluate(&read("perf-bench.toml"), &read("scenarios.ts")).unwrap()
    }

    #[test]
    fn bad_example() {
        let found = run_fixture("bad");
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains("has no budget"),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn good_example() {
        assert!(run_fixture("good").is_empty());
    }
}
