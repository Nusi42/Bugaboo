//! META-SPEED: edit-time checks take under 3 seconds and commit-time checks under 10 seconds
//! on a quiet PC (architecture §10), so nobody is tempted to skip them.
//!
//! The check measures itself. It runs every active edit-time check on one representative tracked
//! file of a matching type (3 s each), and one full commit-time run over a sample of tracked
//! files (10 s in total). The numbers are written to `target/meta-speed.txt`.
//!
//! Wall-clock time alone fails whenever other builds run on the same PC, so every timing is
//! judged against a **reference workload** timed right before and right after it: a few small
//! processes plus some file and CPU work, like a typical check. The workload takes
//! [`REFERENCE_QUIET`] on a quiet PC. When it takes twice as long, the PC is twice as busy
//! (the load factor is 2) and the budget doubles too. Each timing is taken up to
//! [`ATTEMPTS`] times and the best attempt counts, which drops one-off spikes. A check that is
//! slow by itself stays slow in every attempt, so it still fails.
//!
//! Follow-up: the commit-time run reads the working tree, not the staged contents, because
//! `check staged` does the same for now; it moves to the index when the checks learn to read
//! staged files.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use sha2::{Digest as _, Sha256};

use super::{Check, Context, Finding};
use crate::catalog::Entry;
use crate::repo::Repo;
use crate::tools;

/// Where the measured numbers are written, relative to the repository root.
const REPORT_PATH: &str = "target/meta-speed.txt";
/// The scratch file of the reference workload, relative to the repository root.
const REFERENCE_FILE: &str = "target/meta-speed-reference.tmp";

/// The slowest an edit-time check may be on one file, on a quiet PC.
const EDIT_BUDGET: Duration = Duration::from_secs(3);
/// The slowest a whole commit-time run may be, on a quiet PC.
const COMMIT_BUDGET: Duration = Duration::from_secs(10);
/// How many tracked files the commit-time run covers.
const COMMIT_SAMPLE: usize = 50;

/// How long the reference workload takes on a quiet developer PC. The budgets above are
/// multiples of this: the edit-time budget is `EDIT_BUDGET / REFERENCE_QUIET` times the
/// reference, the commit-time budget `COMMIT_BUDGET / REFERENCE_QUIET` times. The workload
/// is fixed, so this number changes only together with the workload below.
const REFERENCE_QUIET: Duration = Duration::from_millis(90);
/// The reference workload starts this many small processes (a check starts tools the same way).
const REFERENCE_PROCESSES: usize = 2;
/// The reference workload writes, reads and hashes this many bytes, [`REFERENCE_ROUNDS`] times.
const REFERENCE_BYTES: usize = 256 * 1024;
const REFERENCE_ROUNDS: usize = 8;

/// A busy PC may stretch the budgets by at most this factor. Beyond it the time is never
/// accepted: a check that needs a minute for one file is slow whatever else is running.
const MAX_LOAD_FACTOR: f64 = 20.0;
/// How often a timing is taken at most. The best attempt counts; timings that pass are not
/// repeated.
const ATTEMPTS: usize = 3;

pub(crate) struct MetaSpeed;

/// What one measurement times.
enum Work<'a> {
    /// One edit-time check on one file.
    Edit { entry: &'a Entry, file: String },
    /// All commit-time checks that match, side by side as the commit hook runs them, on the
    /// sample.
    Commit {
        entries: Vec<&'a Entry>,
        sample: Vec<String>,
    },
}

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

        let mut measurements = Vec::new();
        let mut works = Vec::new();
        for entry in active.iter().filter(|entry| entry.runs_at("edit")) {
            let Some(file) = files.iter().find(|file| {
                entry
                    .matches_any(std::slice::from_ref(file))
                    .unwrap_or(false)
            }) else {
                continue;
            };
            measurements.push(Measurement::new(
                format!("{} on {file}", entry.id),
                EDIT_BUDGET,
            ));
            works.push(Work::Edit {
                entry,
                file: file.clone(),
            });
        }
        let sample = sample(&files, COMMIT_SAMPLE);
        let mut entries = Vec::new();
        for entry in active.iter().filter(|entry| entry.runs_at("commit")) {
            if entry.matches_any(&sample)? {
                entries.push(*entry);
            }
        }
        measurements.push(Measurement::new(
            format!("all commit-time checks on {} files", sample.len()),
            COMMIT_BUDGET,
        ));
        works.push(Work::Commit { entries, sample });

        let time_check = |entry: &Entry, files: &[String]| -> anyhow::Result<Duration> {
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
        collect(
            &mut measurements,
            |index, probe| match &works[index] {
                Work::Edit { entry, file } => time_check(entry, std::slice::from_ref(file)),
                Work::Commit { entries, sample } => {
                    let inner = Context {
                        repo,
                        catalog: context.catalog,
                        files: Some(sample),
                    };
                    let started = Instant::now();
                    crate::runner::run_entries(&registry, entries, &inner)?;
                    let took = started.elapsed();
                    // A load that comes and goes during the run shows in a reference taken
                    // right after it.
                    probe()?;
                    Ok(took)
                }
            },
            || reference_workload(repo),
        )?;

        let report: String = measurements.iter().fold(String::new(), |mut out, m| {
            let _ = writeln!(out, "{}", m.report_line());
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

/// One timing, with the runs of the reference workload taken right before, during (between
/// long steps) and right after it.
#[derive(Clone)]
struct Sample {
    took: Duration,
    references: Vec<Duration>,
}

impl Sample {
    /// The reference workload's time while this was timed: its mean.
    fn reference(&self) -> Option<Duration> {
        let count = u32::try_from(self.references.len())
            .ok()
            .filter(|n| *n > 0)?;
        Some(self.references.iter().sum::<Duration>() / count)
    }
}

/// What the samples of one measurement add up to.
struct Verdict {
    /// The fastest attempt.
    took: Duration,
    /// The reference workload's time in the calmest attempt.
    reference: Duration,
}

impl Verdict {
    /// How many times busier than a quiet PC the machine was: 1.0 or more, never more than
    /// [`MAX_LOAD_FACTOR`].
    fn load_factor(&self) -> f64 {
        (self.reference.as_secs_f64() / REFERENCE_QUIET.as_secs_f64()).clamp(1.0, MAX_LOAD_FACTOR)
    }

    /// The time a check may take on a PC that busy.
    fn allowed(&self, budget: Duration) -> Duration {
        budget.mul_f64(self.load_factor())
    }
}

/// One thing that is timed, the budget it has on a quiet PC, and the samples taken so far.
struct Measurement {
    what: String,
    budget: Duration,
    samples: Vec<Sample>,
}

impl Measurement {
    fn new(what: String, budget: Duration) -> Self {
        Self {
            what,
            budget,
            samples: Vec::new(),
        }
    }

    /// Both sides take their best case: the fastest attempt of the check against the calmest
    /// attempt of the reference. A spike in the check, or in the reference, then changes
    /// nothing; only a load that lasts through all attempts raises the allowance, and it slows
    /// the check as well.
    fn verdict(&self) -> Option<Verdict> {
        let took = self.samples.iter().map(|sample| sample.took).min()?;
        let reference = self.samples.iter().filter_map(Sample::reference).min()?;
        Some(Verdict { took, reference })
    }

    fn passes(&self) -> bool {
        self.verdict()
            .is_some_and(|verdict| verdict.took <= verdict.allowed(self.budget))
    }

    fn report_line(&self) -> String {
        match self.verdict() {
            Some(verdict) => format!(
                "{} ms of {} ms ({} ms when quiet, load factor {:.1}, reference {} ms): {}",
                verdict.took.as_millis(),
                verdict.allowed(self.budget).as_millis(),
                self.budget.as_millis(),
                verdict.load_factor(),
                verdict.reference.as_millis(),
                self.what
            ),
            None => format!("not timed: {}", self.what),
        }
    }
}

/// Times every measurement, up to [`ATTEMPTS`] times. Only measurements that have not passed
/// yet are repeated. `time` runs the work of one measurement and returns the time of the work
/// alone; it calls the probe between long steps, which runs the reference workload.
/// `reference` runs the reference workload.
fn collect(
    measurements: &mut [Measurement],
    mut time: impl FnMut(usize, &mut dyn FnMut() -> anyhow::Result<()>) -> anyhow::Result<Duration>,
    mut reference: impl FnMut() -> anyhow::Result<Duration>,
) -> anyhow::Result<()> {
    for _ in 0..ATTEMPTS {
        // The reference taken after one measurement is also the one before the next.
        let mut latest = reference()?;
        let mut repeated = false;
        for (index, measurement) in measurements.iter_mut().enumerate() {
            if measurement.passes() {
                continue;
            }
            repeated = true;
            let mut references = vec![latest];
            let took = time(index, &mut || {
                references.push(reference()?);
                Ok(())
            })?;
            latest = reference()?;
            references.push(latest);
            measurement.samples.push(Sample { took, references });
        }
        if !repeated {
            break;
        }
    }
    Ok(())
}

/// A fixed piece of work that a busy PC slows down in the same way as a check: it starts
/// small processes, writes, reads and hashes a file. Its time on a quiet PC is
/// [`REFERENCE_QUIET`].
fn reference_workload(repo: &Repo) -> anyhow::Result<Duration> {
    let started = Instant::now();
    for _ in 0..REFERENCE_PROCESSES {
        tools::run(Path::new("git"), &["--version"], repo.root())?;
    }
    let path = repo.path(REFERENCE_FILE);
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)
            .with_context(|| format!("could not create {}", folder.display()))?;
    }
    let block = vec![0x5a_u8; REFERENCE_BYTES];
    let mut digest = Sha256::new();
    for _ in 0..REFERENCE_ROUNDS {
        fs::write(&path, &block).with_context(|| format!("could not write {}", path.display()))?;
        let back = fs::read(&path).with_context(|| format!("could not read {}", path.display()))?;
        digest.update(&back);
    }
    std::hint::black_box(digest.finalize());
    // Best effort: a left-over scratch file in `target/` does no harm.
    let _ = fs::remove_file(&path);
    Ok(started.elapsed())
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
        .filter(|m| !m.passes())
        .filter_map(|m| {
            let best = m.verdict()?;
            Some(Finding::at(
                &m.what,
                format!(
                    "Took {} ms at best of {} attempts; {} ms are allowed ({} ms on a quiet PC). \
                     The reference workload took {} ms now and {} ms on a quiet PC, so the PC is \
                     {:.1} times as busy (load factor). A real slowness shows in every attempt. \
                     Make it faster, or run it at a later stage (for example push or ci) in tools/checks/catalog.toml.",
                    best.took.as_millis(),
                    m.samples.len(),
                    best.allowed(m.budget).as_millis(),
                    m.budget.as_millis(),
                    best.reference.as_millis(),
                    REFERENCE_QUIET.as_millis(),
                    best.load_factor(),
                ),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference time of a quiet PC, in milliseconds: the one definition, not a copy.
    fn quiet_ms() -> u64 {
        u64::try_from(REFERENCE_QUIET.as_millis()).unwrap()
    }

    /// A sample whose reference workload took `reference_ms` before and after.
    fn sample_of(took_ms: u64, reference_ms: u64) -> Sample {
        Sample {
            took: Duration::from_millis(took_ms),
            references: vec![Duration::from_millis(reference_ms); 2],
        }
    }

    fn verdict_of(reference_ms: u64) -> Verdict {
        Verdict {
            took: Duration::ZERO,
            reference: Duration::from_millis(reference_ms),
        }
    }

    fn times(count: usize, sample: Sample) -> Vec<Sample> {
        vec![sample; count]
    }

    fn measurement(what: &str, budget: Duration, samples: &[Sample]) -> Measurement {
        Measurement {
            what: what.to_owned(),
            budget,
            samples: samples.to_vec(),
        }
    }

    #[test]
    fn the_quiet_reference_is_the_one_definition_of_the_load_factor() {
        assert!((verdict_of(quiet_ms()).load_factor() - 1.0).abs() < 1e-9);
        assert!((verdict_of(4 * quiet_ms()).load_factor() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn good_example_a_quiet_pc_keeps_the_documented_budgets() {
        let fine = [
            measurement("edit", EDIT_BUDGET, &[sample_of(2_999, quiet_ms())]),
            measurement("commit", COMMIT_BUDGET, &[sample_of(10_000, quiet_ms())]),
        ];
        assert!(evaluate(&fine).is_empty());
    }

    #[test]
    fn good_example_a_normal_check_under_heavy_load_passes() {
        // Five times busier: a check that takes 2.9 s when quiet takes 14.5 s.
        let loaded = [
            measurement("edit", EDIT_BUDGET, &[sample_of(14_500, 5 * quiet_ms())]),
            measurement(
                "commit",
                COMMIT_BUDGET,
                &[sample_of(49_000, 5 * quiet_ms())],
            ),
        ];
        assert!(evaluate(&loaded).is_empty());
    }

    #[test]
    fn bad_example_a_slow_check_on_a_quiet_pc_fails() {
        // Sleeping twice the budget.
        let slow = [
            measurement("edit", EDIT_BUDGET, &times(3, sample_of(6_000, quiet_ms()))),
            measurement(
                "commit",
                COMMIT_BUDGET,
                &times(3, sample_of(20_000, quiet_ms())),
            ),
        ];
        assert_eq!(evaluate(&slow).len(), 2);
    }

    #[test]
    fn bad_example_a_slow_check_on_a_loaded_pc_fails() {
        // Four times busier, and the check needs twice what even that allows.
        let slow = [measurement(
            "edit",
            EDIT_BUDGET,
            &times(3, sample_of(24_000, 4 * quiet_ms())),
        )];
        let findings = evaluate(&slow);
        assert_eq!(findings.len(), 1);
        let message = &findings[0].message;
        assert!(message.contains("Took 24000 ms at best of 3 attempts"));
        assert!(message.contains("12000 ms are allowed (3000 ms on a quiet PC)"));
        assert!(message.contains(&format!(
            "reference workload took {} ms now and {} ms",
            4 * quiet_ms(),
            quiet_ms()
        )));
        assert!(message.contains("4.0 times as busy"));
    }

    #[test]
    fn the_best_attempt_drops_a_spike() {
        let spiked = [measurement(
            "edit",
            EDIT_BUDGET,
            &[
                sample_of(40_000, quiet_ms()),
                sample_of(2_000, quiet_ms()),
                sample_of(2_100, quiet_ms()),
            ],
        )];
        assert!(evaluate(&spiked).is_empty());
    }

    #[test]
    fn a_reference_spike_does_not_excuse_a_slow_check() {
        // The check is slow in every attempt; only one attempt saw a busy PC.
        let slow = [measurement(
            "edit",
            EDIT_BUDGET,
            &[
                sample_of(6_000, quiet_ms()),
                sample_of(6_000, quiet_ms()),
                sample_of(6_000, 10 * quiet_ms()),
            ],
        )];
        assert_eq!(evaluate(&slow).len(), 1);
    }

    #[test]
    fn a_load_that_lasts_through_all_attempts_raises_the_allowance() {
        let busy = [measurement(
            "edit",
            EDIT_BUDGET,
            &times(3, sample_of(6_000, 3 * quiet_ms())),
        )];
        assert!(evaluate(&busy).is_empty());
    }

    #[test]
    fn the_load_factor_has_a_floor_and_a_ceiling() {
        assert!((verdict_of(10).load_factor() - 1.0).abs() < 1e-9);
        assert!((verdict_of(1_000_000).load_factor() - MAX_LOAD_FACTOR).abs() < 1e-9);
        // Even a reference of an hour does not excuse a check that needs two minutes.
        let hopeless = [measurement(
            "edit",
            EDIT_BUDGET,
            &times(3, sample_of(120_000, 3_600_000)),
        )];
        assert_eq!(evaluate(&hopeless).len(), 1);
    }

    #[test]
    fn collect_repeats_only_what_has_not_passed_and_stops_after_the_attempts() {
        let mut measurements = [
            Measurement::new("fast".to_owned(), EDIT_BUDGET),
            Measurement::new("slow".to_owned(), EDIT_BUDGET),
        ];
        let mut calls = [0_usize; 2];
        collect(
            &mut measurements,
            |index, _| {
                calls[index] += 1;
                Ok(Duration::from_millis(if index == 0 { 100 } else { 9_000 }))
            },
            || Ok(REFERENCE_QUIET),
        )
        .unwrap();
        assert_eq!(calls, [1, ATTEMPTS]);
        assert_eq!(measurements[1].samples.len(), ATTEMPTS);
        assert_eq!(evaluate(&measurements).len(), 1);
    }

    #[test]
    fn collect_takes_the_reference_around_and_during_each_timing() {
        let mut measurements = [Measurement::new("commit".to_owned(), COMMIT_BUDGET)];
        let mut references = [100_u64, 200, 300].into_iter();
        collect(
            &mut measurements,
            |_, probe| {
                probe()?;
                Ok(Duration::from_millis(500))
            },
            || Ok(Duration::from_millis(references.next().unwrap_or(0))),
        )
        .unwrap();
        let taken = &measurements[0].samples[0];
        assert_eq!(taken.references.len(), 3);
        assert_eq!(taken.reference(), Some(Duration::from_millis(200)));
    }

    #[test]
    fn the_report_names_time_allowance_and_load() {
        let line =
            measurement("edit", EDIT_BUDGET, &[sample_of(900, 2 * quiet_ms())]).report_line();
        assert_eq!(
            line,
            format!(
                "900 ms of 6000 ms (3000 ms when quiet, load factor 2.0, reference {} ms): edit",
                2 * quiet_ms()
            )
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
