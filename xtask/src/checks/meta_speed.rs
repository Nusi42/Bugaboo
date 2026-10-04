//! META-SPEED: edit-time checks take under 3 seconds and commit-time checks under 10 seconds
//! (architecture §10), so nobody is tempted to skip them.
//!
//! The check measures itself, after one warm-up pass. It runs every active edit-time check on one representative tracked
//! file of a matching type (3 s each), and one full commit-time run over a sample of tracked
//! files (10 s in total). The numbers are written to `target/meta-speed.txt`.
//!
//! Follow-up: the commit-time run reads the working tree, not the staged contents, because
//! `check staged` does the same for now; it moves to the index when the checks learn to read
//! staged files.

use std::fmt::Write as _;
use std::fs;
use std::time::{Duration, Instant};

use anyhow::Context as _;

use super::{Check, Context, Finding};
use crate::catalog::Entry;

/// Where the measured numbers are written, relative to the repository root.
const REPORT_PATH: &str = "target/meta-speed.txt";

/// The slowest an edit-time check may be on one file.
const EDIT_BUDGET: Duration = Duration::from_secs(3);
/// The slowest a whole commit-time run may be.
const COMMIT_BUDGET: Duration = Duration::from_secs(10);
/// How many tracked files the commit-time run covers.
const COMMIT_SAMPLE: usize = 50;

pub(crate) struct MetaSpeed;

impl Check for MetaSpeed {
    fn id(&self) -> &'static str {
        "META-SPEED"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let files = repo.all_files()?;
        let registry = super::registry();
        let active: Vec<&Entry> = context
            .catalog
            .entries()
            .iter()
            .filter(|entry| entry.is_active() && entry.id != self.id())
            .collect();
        let time = |entry: &Entry, files: &[String]| -> anyhow::Result<Duration> {
            let Some(check) = registry.iter().find(|check| check.id() == entry.id) else {
                return Ok(Duration::ZERO);
            };
            let inner = Context {
                repo,
                catalog: context.catalog,
                files: Some(files),
            };
            let started = Instant::now();
            check.run(&inner)?;
            Ok(started.elapsed())
        };

        let measure = || -> anyhow::Result<Vec<Measurement>> {
            let mut measurements = Vec::new();
            for entry in active.iter().filter(|entry| entry.runs_at("edit")) {
                let Some(file) = files.iter().find(|file| {
                    entry
                        .matches_any(std::slice::from_ref(file))
                        .unwrap_or(false)
                }) else {
                    continue;
                };
                measurements.push(Measurement {
                    what: format!("{} on {file}", entry.id),
                    took: time(entry, std::slice::from_ref(file))?,
                    budget: EDIT_BUDGET,
                });
            }
            let sample = sample(&files, COMMIT_SAMPLE);
            let mut total = Duration::ZERO;
            for entry in active.iter().filter(|entry| entry.runs_at("commit")) {
                if entry.matches_any(&sample)? {
                    total += time(entry, &sample)?;
                }
            }
            measurements.push(Measurement {
                what: format!("all commit-time checks on {} files", sample.len()),
                took: total,
                budget: COMMIT_BUDGET,
            });
            Ok(measurements)
        };
        // One warm-up pass, so cold file caches do not count against the checks.
        measure()?;
        let measurements = measure()?;

        let report: String = measurements.iter().fold(String::new(), |mut out, m| {
            let _ = writeln!(
                out,
                "{} ms of {} ms: {}",
                m.took.as_millis(),
                m.budget.as_millis(),
                m.what
            );
            out
        });
        let path = repo.path(REPORT_PATH);
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)
                .with_context(|| format!("could not create {}", folder.display()))?;
        }
        fs::write(&path, report).with_context(|| format!("could not write {}", path.display()))?;
        Ok(evaluate(&measurements))
    }
}

/// One timing and the budget it has to stay within.
struct Measurement {
    what: String,
    took: Duration,
    budget: Duration,
}

/// Up to `count` files spread evenly over the list.
fn sample(files: &[String], count: usize) -> Vec<String> {
    if files.len() <= count {
        return files.to_vec();
    }
    (0..count)
        .map(|n| files[n * files.len() / count].clone())
        .collect()
}

fn evaluate(measurements: &[Measurement]) -> Vec<Finding> {
    measurements
        .iter()
        .filter(|m| m.took > m.budget)
        .map(|m| {
            Finding::at(
                &m.what,
                format!(
                    "Took {} ms; the budget is {} ms. Make it faster, or run it at a later stage (for example push or ci) in tools/checks/catalog.toml.",
                    m.took.as_millis(),
                    m.budget.as_millis()
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measurement(what: &str, millis: u64, budget: Duration) -> Measurement {
        Measurement {
            what: what.to_owned(),
            took: Duration::from_millis(millis),
            budget,
        }
    }

    #[test]
    fn good_example() {
        let fine = [
            measurement("edit", 2_999, EDIT_BUDGET),
            measurement("commit", 10_000, COMMIT_BUDGET),
        ];
        assert!(evaluate(&fine).is_empty());
    }

    #[test]
    fn bad_example() {
        let slow = [
            measurement("edit", 3_001, EDIT_BUDGET),
            measurement("commit", 10_001, COMMIT_BUDGET),
        ];
        let findings = evaluate(&slow);
        assert_eq!(findings.len(), 2);
        assert!(
            findings[0]
                .message
                .contains("Took 3001 ms; the budget is 3000 ms")
        );
    }

    #[test]
    fn the_sample_is_spread_over_all_files() {
        let files: Vec<String> = (0..200).map(|n| n.to_string()).collect();
        let picked = sample(&files, 50);
        assert_eq!(picked.len(), 50);
        assert_eq!(picked[0], "0");
        assert_eq!(picked[49], "196");
        assert_eq!(sample(&files[..3], 50).len(), 3);
    }
}
