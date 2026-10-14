//! The acceptance tests of the issues of the sample organization (M3 WP-11): they make the real
//! thing, a small sample of it, on SQLite and on PostgreSQL, and hold it against the plan of
//! `tools/dev/sample.json` (TEST-SAMPLE-DATA).

use std::path::Path;

use bb_dev_words::sample::Layout;
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;
use serde_json::Value;

use crate::{DEFAULT_SEED, Everything, Options, Sample, Target, make};

/// 2027-01-15 08:00:00 UTC, like the other tests.
const START: i64 = 1_800_000_000;
/// Few enough to be quick, enough for every project, type and kind of relation to be there.
const ISSUES: u32 = 150;

fn options() -> Options {
    Options {
        seed: DEFAULT_SEED,
        start: Timestamp::from_second(START).unwrap(),
    }
}

async fn made(target: &Target, folder: &Path) -> Result<Sample, Failure> {
    let mut everything = Everything::new(ISSUES)?;
    Ok(make(&options(), folder, target, &mut everything).await?)
}

async fn dump_of(sample: Sample) -> Result<String, Failure> {
    sample.verify_security_log().await?;
    let text = sample.dump().await?.to_json()?;
    sample.close().await;
    Ok(text)
}

#[tokio::test]
async fn the_issues_are_the_same_from_the_same_seed_on_both_databases() -> TestResult {
    each_database("the_issues_are_the_same_from_the_same_seed", async |db| {
        let (here, built_in) = (tempfile::tempdir()?, tempfile::tempdir()?);
        let on_this_database =
            dump_of(made(&Target::Address(db.url().to_owned()), here.path()).await?).await?;
        let on_the_built_in = dump_of(made(&Target::BuiltIn, built_in.path()).await?).await?;
        assert_eq!(
            on_this_database,
            on_the_built_in,
            "the same dump on {}",
            db.kind().code()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_sample_data_issues_match_the_plan() -> TestResult {
    each_database("test_sample_data_issues_match_the_plan", async |db| {
        let folder = tempfile::tempdir()?;
        let text =
            dump_of(made(&Target::Address(db.url().to_owned()), folder.path()).await?).await?;
        let dump: Value = serde_json::from_str(&text)?;
        let plan = Layout::bundled()?.issues;
        let problems = plan.problems(ISSUES, db.kind().code(), &dump);
        assert!(problems.is_empty(), "{problems:#?}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn the_sample_has_its_issues_made_by_people_over_many_months_and_nothing_more() -> TestResult
{
    let folder = tempfile::tempdir()?;
    let text = dump_of(made(&Target::BuiltIn, folder.path()).await?).await?;
    let dump: Value = serde_json::from_str(&text)?;
    let events = dump["events"].as_array().ok_or("no events")?;
    // The first issue event is long before the end of the sample, the last one close to it.
    let times: Vec<&str> = events
        .iter()
        .filter(|event| {
            event["entity"]
                .as_str()
                .is_some_and(|e| e.starts_with("issue#"))
        })
        .filter_map(|event| event["time"].as_str())
        .collect();
    let first: Timestamp = times.first().ok_or("no issue events")?.parse()?;
    let last: Timestamp = times.last().ok_or("no issue events")?.parse()?;
    let months = last.as_second() - first.as_second();
    assert!(months > 15 * 30 * 86_400, "{first} to {last}");
    assert!(last.as_second() < START);
    // Made by people through the use cases: each issue event has a person as its actor, and none
    // is made by the system.
    assert!(
        events
            .iter()
            .filter(|event| event["entity"]
                .as_str()
                .is_some_and(|e| e.starts_with("issue#")))
            .all(|event| event["actor"]
                .as_str()
                .is_some_and(|a| a.starts_with("person#")))
    );
    // No restricted issues, no releases: nothing of the sample is about them.
    assert!(events.iter().all(|event| {
        let entity = event["entity"].as_str().unwrap_or_default();
        !entity.starts_with("release#") && !entity.starts_with("restriction#")
    }));
    Ok(())
}
