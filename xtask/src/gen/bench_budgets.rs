//! `cargo xtask gen bench-budgets`: the budgets of `tools/checks/perf-bench.toml` as a TypeScript
//! constant, so the benchmarks and the speed tests read one number and nobody writes it twice.

use super::Generated;
use crate::bench_budgets::{BUDGETS_PATH, Budgets, OUTPUT, typescript};
use crate::repo::Repo;
use crate::source::read_text;

/// The file this generator writes.
pub(crate) const FILE: &str = OUTPUT;

pub(crate) fn generate(repo: &Repo) -> anyhow::Result<Vec<Generated>> {
    let budgets = Budgets::parse(&read_text(repo, BUDGETS_PATH)?)?;
    Ok(vec![Generated::new(FILE, typescript(&budgets))])
}
