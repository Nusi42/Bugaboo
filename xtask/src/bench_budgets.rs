//! The budgets of the benchmarks (PERF-01, PERF-BENCH): `tools/checks/perf-bench.toml` is their
//! only home. This file reads it, lists the scenarios of `web/tooling/bench/scenarios.ts` (the one
//! place where scenarios are registered), says what is wrong with the two together, writes the
//! TypeScript constant the speed tests and the benchmarks import, and judges a run against the
//! budgets.

use std::collections::BTreeMap;

use std::fmt::Write as _;

use anyhow::Context as _;
use bb_dev_words::database::DatabaseKind;
use serde::Deserialize;

/// Where the budgets are, relative to the repository root.
pub(crate) const BUDGETS_PATH: &str = "tools/checks/perf-bench.toml";
/// Where the scenarios are registered.
pub(crate) const SCENARIOS_PATH: &str = "web/tooling/bench/scenarios.ts";
/// The TypeScript the budgets are written to.
pub(crate) const OUTPUT: &str = "web/tooling/bench/budgets.generated.ts";

/// The settings file.
#[derive(Debug, Deserialize)]
pub(crate) struct Budgets {
    budgets: BTreeMap<String, Budget>,
    /// Scenarios that are over their budget on one database and are accepted for now.
    #[serde(default)]
    known_gaps: BTreeMap<String, KnownGap>,
}

/// A scenario over its budget that the maintainer accepted: the one place that says so, with the
/// follow-up that closes it and the reason. The budget itself stays as it is.
#[derive(Debug, Deserialize)]
struct KnownGap {
    database: String,
    /// The most the scenario may take at the 95th percentile while it is a known gap; over it the
    /// run fails, so the gap cannot grow unseen.
    p95_ms_at_most: f64,
    follow_up: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct Budget {
    p95_ms: f64,
}

/// A scenario as registered: its name and the budget it is held to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Scenario {
    pub(crate) name: String,
    pub(crate) budget: String,
}

/// What a scenario took in a run, as the driver writes it.
#[derive(Debug, Deserialize)]
pub(crate) struct Measured {
    pub(crate) budget: String,
    #[serde(default)]
    pub(crate) p50: f64,
    pub(crate) p95: f64,
    #[serde(default)]
    pub(crate) p99: f64,
}

impl Budgets {
    /// Reads the settings.
    ///
    /// # Errors
    /// The text is not TOML of this shape.
    pub(crate) fn parse(text: &str) -> anyhow::Result<Self> {
        toml::from_str(text).with_context(|| format!("{BUDGETS_PATH} is not valid"))
    }

    fn gap_of(&self, scenario: &str, database: &str) -> Option<&KnownGap> {
        self.known_gaps
            .get(scenario)
            .filter(|gap| gap.database == database)
    }

    /// The most a budget allows at the 95th percentile.
    pub(crate) fn p95_ms(&self, name: &str) -> Option<f64> {
        self.budgets.get(name).map(|budget| budget.p95_ms)
    }
}

/// The scenarios a registry file lists: each is a `name: "…"` followed by its `budget: "…"`.
pub(crate) fn scenarios(source: &str) -> Vec<(Scenario, usize)> {
    let quoted = |line: &str, key: &str| {
        let rest = line
            .trim()
            .strip_prefix(key)?
            .trim_start()
            .strip_prefix('"')?;
        rest.split('"').next().map(str::to_owned)
    };
    let mut found = Vec::new();
    let mut name: Option<(String, usize)> = None;
    for (at, line) in source.lines().enumerate() {
        if let Some(each) = quoted(line, "name:") {
            name = Some((each, at + 1));
        } else if let Some(budget) = quoted(line, "budget:")
            && let Some((name, line)) = name.take()
        {
            found.push((Scenario { name, budget }, line));
        }
    }
    found
}

/// What is wrong with the budgets and the scenarios, each in a sentence with its place.
pub(crate) fn problems(budgets: &Budgets, listed: &[(Scenario, usize)]) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (name, budget) in &budgets.budgets {
        if budget.p95_ms.is_nan() || budget.p95_ms <= 0.0 {
            found.push((
                BUDGETS_PATH.to_owned(),
                format!("The budget {name} must be more than 0 milliseconds."),
            ));
        }
    }
    if listed.is_empty() {
        found.push((
            SCENARIOS_PATH.to_owned(),
            "No scenario is registered: each needs a name and a budget.".to_owned(),
        ));
    }
    for (at, (scenario, line)) in listed.iter().enumerate() {
        let place = format!("{SCENARIOS_PATH}:{line}");
        if budgets.p95_ms(&scenario.budget).is_none() {
            found.push((
                place.clone(),
                format!(
                    "The scenario {} has no budget: {} is not in {BUDGETS_PATH}. Every scenario names its budget there.",
                    scenario.name, scenario.budget
                ),
            ));
        }
        if listed[..at]
            .iter()
            .any(|(each, _)| each.name == scenario.name)
        {
            found.push((
                place,
                format!("The scenario {} is registered twice.", scenario.name),
            ));
        }
    }
    for (name, gap) in &budgets.known_gaps {
        if !DatabaseKind::ALL
            .iter()
            .any(|kind| kind.code() == gap.database)
        {
            found.push((
                BUDGETS_PATH.to_owned(),
                format!(
                    "The known gap {name} names the database {}, which does not exist.",
                    gap.database
                ),
            ));
        }
        if gap.p95_ms_at_most.is_nan() || gap.p95_ms_at_most <= 0.0 {
            found.push((
                BUDGETS_PATH.to_owned(),
                format!("The known gap {name} must allow more than 0 milliseconds."),
            ));
        }
        let budget = listed
            .iter()
            .find(|(each, _)| &each.name == name)
            .and_then(|(each, _)| budgets.p95_ms(&each.budget));
        if budget.is_some_and(|allowed| gap.p95_ms_at_most <= allowed) {
            found.push((
                BUDGETS_PATH.to_owned(),
                format!("The known gap {name} must allow more than the budget of the scenario."),
            ));
        }
        if !listed.iter().any(|(each, _)| &each.name == name) {
            found.push((
                BUDGETS_PATH.to_owned(),
                format!("The known gap {name} is not a registered scenario."),
            ));
        }
    }
    found
}

/// The TypeScript constant of the budgets, already formatted.
pub(crate) fn typescript(budgets: &Budgets) -> String {
    let mut text = String::from(
        "// Generated by `cargo xtask gen bench-budgets`. Do not edit by hand.\n\n\
         /** The most a request may take at the 95th percentile, in milliseconds (tools/checks/perf-bench.toml). */\n\
         export const budgets = {\n",
    );
    for (name, budget) in &budgets.budgets {
        let _ = writeln!(text, "  {name}: {{ p95Ms: {} }},", budget.p95_ms);
    }
    text.push_str("} as const;\n\n/** The name of a budget. */\nexport type BudgetName = keyof typeof budgets;\n");
    text
}

/// What a run is judged to be: the sentences that fail it, and the known gaps it showed.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Judged {
    pub(crate) failures: Vec<String>,
    pub(crate) known: Vec<String>,
}

/// The scenarios of a run on `database` that are over their budget. A scenario that
/// `known_gaps` names for this database is told as a known gap and fails the run only over the
/// limit of its entry.
pub(crate) fn over(budgets: &Budgets, database: &str, run: &BTreeMap<String, Measured>) -> Judged {
    let mut judged = Judged::default();
    for (name, measured) in run {
        match budgets.p95_ms(&measured.budget) {
            None => judged.failures.push(format!(
                "The scenario {name} has no budget: {} is not in {BUDGETS_PATH}.",
                measured.budget
            )),
            Some(allowed) if measured.p95 > allowed => {
                let sentence = format!(
                    "The scenario {name} took {:.1} ms at the 95th percentile, over its budget of {allowed} ms ({}).",
                    measured.p95, measured.budget
                );
                match budgets.gap_of(name, database) {
                    Some(gap) if measured.p95 <= gap.p95_ms_at_most => judged.known.push(format!(
                        "{sentence} Known gap {}: {}",
                        gap.follow_up, gap.reason
                    )),
                    Some(gap) => judged.failures.push(format!(
                        "{sentence} It is over the {} ms that the known gap {} allows.",
                        gap.p95_ms_at_most, gap.follow_up
                    )),
                    None => judged.failures.push(sentence),
                }
            }
            Some(_) => {
                if let Some(gap) = budgets.gap_of(name, database) {
                    judged.known.push(format!(
                        "The scenario {name} meets its budget again: remove the known gap {} from {BUDGETS_PATH}.",
                        gap.follow_up
                    ));
                }
            }
        }
    }
    judged
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOML: &str = "[budgets.page]\np95_ms = 100\n[budgets.search]\np95_ms = 300\n";
    const SOURCE: &str = "  {\n    name: \"issue-page\",\n    budget: \"page\",\n  },\n  {\n    name: \"search\",\n    budget: \"search\",\n  },\n";

    fn budgets() -> Budgets {
        Budgets::parse(TOML).unwrap()
    }

    #[test]
    fn the_scenarios_are_read_from_the_registry_with_their_lines() {
        let listed = scenarios(SOURCE);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].0.name, "issue-page");
        assert_eq!(listed[0].0.budget, "page");
        assert_eq!(listed[1].1, 6);
    }

    #[test]
    fn a_good_file_passes() {
        assert!(problems(&budgets(), &scenarios(SOURCE)).is_empty());
    }

    #[test]
    fn a_scenario_without_a_budget_fails() {
        let source = SOURCE.replace("budget: \"search\"", "budget: \"nothing\"");
        let found = problems(&budgets(), &scenarios(&source));
        assert_eq!(found.len(), 1);
        assert!(found[0].1.contains("has no budget"), "{}", found[0].1);
    }

    #[test]
    fn a_budget_of_zero_and_a_twice_registered_scenario_fail() {
        let zero = Budgets::parse(
            "[budgets.page]
p95_ms = 0
[budgets.search]
p95_ms = 1
",
        )
        .unwrap();
        assert_eq!(problems(&zero, &scenarios(SOURCE)).len(), 1);
        let twice = SOURCE.replace("issue-page", "search");
        assert_eq!(problems(&budgets(), &scenarios(&twice)).len(), 1);
    }

    #[test]
    fn a_result_over_a_budget_fails_with_the_scenario_the_number_and_the_budget() {
        let run = BTreeMap::from([
            (
                "issue-page".to_owned(),
                Measured {
                    budget: "page".to_owned(),
                    p50: 0.0,
                    p95: 123.4,
                    p99: 0.0,
                },
            ),
            (
                "search".to_owned(),
                Measured {
                    budget: "search".to_owned(),
                    p50: 0.0,
                    p95: 299.0,
                    p99: 0.0,
                },
            ),
        ]);
        let found = over(&budgets(), "sqlite", &run);
        assert_eq!(
            found.failures,
            [
                "The scenario issue-page took 123.4 ms at the 95th percentile, over its budget of 100 ms (page)."
            ]
        );
        assert!(found.known.is_empty());
    }

    #[test]
    fn a_known_gap_is_told_but_fails_only_the_database_it_names() {
        let gap = Budgets::parse(&format!(
            "{TOML}[known_gaps.search]
database = \"sqlite\"
p95_ms_at_most = 700
follow_up = \"M5-F89\"
reason = \"slow\"
"
        ))
        .unwrap();
        let run = BTreeMap::from([(
            "search".to_owned(),
            Measured {
                budget: "search".to_owned(),
                p50: 0.0,
                p95: 560.0,
                p99: 0.0,
            },
        )]);
        let took = |p95: f64| {
            BTreeMap::from([(
                "search".to_owned(),
                Measured {
                    budget: "search".to_owned(),
                    p50: 0.0,
                    p95,
                    p99: 0.0,
                },
            )])
        };
        let past = over(&gap, "sqlite", &took(701.0));
        assert!(past.failures[0].contains("allows"), "{:?}", past.failures);
        let again = over(&gap, "sqlite", &took(250.0));
        assert!(again.failures.is_empty() && again.known[0].contains("meets its budget again"));
        let nowhere = Budgets::parse(&format!(
            "{TOML}[known_gaps.search]
database = \"oracle\"
p95_ms_at_most = 700
follow_up = \"M5-F89\"
reason = \"x\"
"
        ))
        .unwrap();
        assert_eq!(problems(&nowhere, &scenarios(SOURCE)).len(), 1);
        let low = Budgets::parse(&format!(
            "{TOML}[known_gaps.search]
database = \"sqlite\"
p95_ms_at_most = 300
follow_up = \"M5-F89\"
reason = \"x\"
"
        ))
        .unwrap();
        let found = problems(&low, &scenarios(SOURCE));
        assert!(found[0].1.contains("more than the budget"), "{found:?}");
        let on_sqlite = over(&gap, "sqlite", &run);
        assert!(on_sqlite.failures.is_empty());
        assert!(on_sqlite.known[0].contains("Known gap M5-F89: slow"));
        assert_eq!(over(&gap, "postgres", &run).failures.len(), 1);
        assert!(problems(&gap, &scenarios(SOURCE)).is_empty());
        let stray = Budgets::parse(&format!(
            "{TOML}[known_gaps.nothing]
database = \"sqlite\"
p95_ms_at_most = 700
follow_up = \"M5-F89\"
reason = \"x\"
"
        ))
        .unwrap();
        assert_eq!(problems(&stray, &scenarios(SOURCE)).len(), 1);
    }

    #[test]
    fn the_typescript_lists_every_budget_once() {
        let text = typescript(&budgets());
        assert!(text.contains("page: { p95Ms: 100 },"), "{text}");
        assert!(text.contains("search: { p95Ms: 300 },"), "{text}");
    }
}
