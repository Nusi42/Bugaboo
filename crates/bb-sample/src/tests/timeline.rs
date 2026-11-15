//! The acceptance test of the plan in the sample (M8 WP-12, PLAN-01, PLAN-11 to PLAN-15,
//! TEST-SAMPLE-DATA): *Mobile app 2.0* with a due date, its dated issues with estimates and
//! dependencies, Lea Fischer's usual work days, work hours and share, the public holidays of one
//! German state, no away time, and one issue with Lea's own estimate. They make the real thing on
//! SQLite and PostgreSQL and read the canonical dump.

use bb_dev_words::sample::Layout;
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;
use serde_json::Value;

use crate::{DEFAULT_SEED, Everything, Options, Sample, Target, create};

const START: i64 = bb_dev_words::accounts::CLOCK_START;
/// Few enough to be quick, enough for every kind of work to be there.
const ISSUES: u32 = 150;

/// The dump of the sample.
async fn made(target: &Target, folder: &std::path::Path) -> Result<Value, Failure> {
    let options = Options {
        seed: DEFAULT_SEED,
        start: Timestamp::from_second(START)?,
    };
    let mut everything = Everything::new(ISSUES)?;
    let sample: Sample = Box::pin(create(&options, folder, target, &mut everything)).await?;
    sample.verify_security_log().await?;
    let text = sample.dump().await?.to_json()?;
    sample.close().await;
    Ok(serde_json::from_str(&text)?)
}

#[tokio::test]
async fn plan_12_the_sample_plans_mobile_app_2_0_with_lea() -> TestResult {
    Box::pin(each_database(
        "plan_12_the_sample_plans_mobile_app_2_0_with_lea",
        async |db| {
            let folder = tempfile::tempdir()?;
            let dump = Box::pin(made(&Target::Address(db.url().to_owned()), folder.path())).await?;
            // TEST-SAMPLE-DATA: the plan judges the part of the dump on the goal.
            let plan = Layout::bundled()?.issues;
            let problems = plan.problems(ISSUES, "sample", &dump);
            assert!(problems.is_empty(), "{problems:?}");
            let timeline = &dump["issues"]["work"]["timeline"];
            assert_eq!(timeline["goal"]["title"], "Mobile app 2.0");
            assert_eq!(timeline["person"]["name"], "Lea Fischer");
            assert_eq!(timeline["person"]["share_percent"], 100);
            assert_eq!(timeline["person"]["away_times"], 0);
            assert_eq!(timeline["holidays"], "de-by");
            // Lea's own estimate is the only one, and the issue is one of hers.
            let own = timeline["own_estimates"].as_array().ok_or("no estimates")?;
            assert_eq!(own.len(), 1);
            let issue = own[0]["issue"].as_str().ok_or("no issue")?;
            let hers = timeline["issues"].as_array().ok_or("no issues")?;
            assert!(
                hers.iter()
                    .any(|each| each["title"] == issue && each["person"] == "Lea Fischer")
            );
            Ok(())
        },
    ))
    .await
}
