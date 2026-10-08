//! A person who guesses addresses must not be able to tell from the time an answer takes whether
//! an account exists (AUTH-01, threat model: "finding out which e-mail addresses have accounts").
//!
//! The test is statistical and tolerant: it compares the middle of many timings of a wrong
//! password for an account that exists with a password for one that does not. Both do the same
//! Argon2id work, so the middle values are close; if the unknown account were answered at once,
//! it would be many times faster.

use std::time::{Duration, Instant};

use bb_testing::{TestResult, each_database};

use super::rig::{PASSWORD, Rig};

/// How many tries of each kind. Odd, so the middle is one measurement.
const TRIES: usize = 15;

fn middle(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}

#[tokio::test(flavor = "multi_thread")]
async fn auth_01_an_unknown_account_takes_as_long_as_a_wrong_password() -> TestResult {
    each_database("auth_01_timing", async |db| {
        // No limit gets in the way of the measurement.
        let rig = Rig::start(db, 100_000).await?;
        rig.admin("ada@example.test").await?;
        // A first round warms up the thread pool and the memory of the hashing.
        for _ in 0..3 {
            let _ = rig.sign_in("ada@example.test", "Wrong-Password-1").await;
            let _ = rig.sign_in("warmup@example.test", "Wrong-Password-1").await;
        }
        let mut known = Vec::new();
        let mut unknown = Vec::new();
        for number in 0..TRIES {
            let start = Instant::now();
            assert!(
                rig.sign_in("ada@example.test", "Wrong-Password-1")
                    .await
                    .is_err()
            );
            known.push(start.elapsed());
            let start = Instant::now();
            assert!(
                rig.sign_in(&format!("nobody{number}@example.test"), PASSWORD)
                    .await
                    .is_err()
            );
            unknown.push(start.elapsed());
        }
        let (known, unknown) = (middle(known), middle(unknown));
        let ratio = unknown.as_secs_f64() / known.as_secs_f64();
        // The work is the same, so the middles are close. Hashing 19 MiB takes milliseconds;
        // an answer that skipped it would be far outside this range.
        assert!(
            known > Duration::from_millis(1),
            "the hashing work was not done: {known:?}"
        );
        assert!(
            (0.5..=2.0).contains(&ratio),
            "unknown {unknown:?} against known {known:?} (ratio {ratio:.2})"
        );
        Ok(())
    })
    .await
}
