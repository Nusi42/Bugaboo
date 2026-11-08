//! The acceptance tests of AI and agents in the sample (M7 WP-13, AI-30 to AI-38, TEST-SAMPLE-DATA):
//! AI is on with the agreement confirmed, two agents of one Team member hold keys and work in every
//! state, no agent holds a role or is assigned to anything, a standing rule over new bugs proposes
//! and ends within 90 days, and the keys are written to `sign-in.txt` and nowhere else. They make
//! the real thing on SQLite and PostgreSQL and read the canonical dump.

use bb_dev_words::sample::Layout;
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;
use serde_json::Value;

use crate::{DEFAULT_SEED, Everything, Options, Sample, Target, create};

const START: i64 = bb_dev_words::accounts::CLOCK_START;
/// Few enough to be quick, enough for every kind of work to be there.
const ISSUES: u32 = 150;

/// The sample, its dump and the text of `sign-in.txt`.
async fn made(target: &Target, folder: &std::path::Path) -> Result<(Value, String), Failure> {
    let options = Options {
        seed: DEFAULT_SEED,
        start: Timestamp::from_second(START)?,
    };
    let mut everything = Everything::new(ISSUES)?;
    let sample: Sample = Box::pin(create(&options, folder, target, &mut everything)).await?;
    sample.verify_security_log().await?;
    let text = sample.dump().await?.to_json()?;
    let sign_in = sample.sign_in_text();
    sample.close().await;
    Ok((serde_json::from_str(&text)?, sign_in))
}

#[tokio::test]
async fn ai_30_the_sample_has_two_agents_of_one_person_in_every_state_of_agent_work() -> TestResult
{
    Box::pin(each_database(
        "ai_30_the_sample_has_two_agents_of_one_person",
        async |db| {
            let folder = tempfile::tempdir()?;
            let (dump, sign_in) =
                Box::pin(made(&Target::Address(db.url().to_owned()), folder.path())).await?;
            // TEST-SAMPLE-DATA: the plan judges the part of the dump on AI and agents.
            let plan = Layout::bundled()?.issues;
            let problems = plan.problems(ISSUES, "sample", &dump);
            assert!(problems.is_empty(), "{problems:?}");
            let ai = &dump["issues"]["work"]["ai"];
            assert_eq!(ai["agents"].as_array().map(Vec::len), Some(2));
            assert_eq!(ai["agents"][0]["person"], ai["agents"][1]["person"]);
            // Both agents wrote under their own name: the events say "agent of person".
            let by_agents = dump["events"]
                .as_array()
                .ok_or("no events")?
                .iter()
                .filter(|event| {
                    event["actor"]
                        .as_str()
                        .is_some_and(|actor| actor.starts_with("agent#"))
                })
                .count();
            assert!(by_agents > 10, "{by_agents} events of agents");
            // The keys are in sign-in.txt as comment lines, and nowhere in the dump.
            let mark = format!("{}	", Layout::bundled()?.issues.work.ai.agent_key_mark);
            let keys: Vec<&str> = sign_in
                .lines()
                .filter(|line| line.starts_with(&mark))
                .collect();
            assert_eq!(keys.len(), 2);
            for line in keys {
                let columns: Vec<&str> = line.trim_start_matches(&mark).split('\t').collect();
                assert_eq!(columns.len(), 3, "{columns:?}");
                assert!(columns[2].starts_with("bb_"));
                assert!(!dump.to_string().contains(columns[2]));
            }
            Ok(())
        },
    ))
    .await
}

#[tokio::test]
async fn ai_32_the_standing_rule_of_the_sample_proposes_only_and_matches_issues() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (dump, _) = Box::pin(made(&Target::BuiltIn, folder.path())).await?;
    let rules = dump["issues"]["work"]["ai"]["rules"]
        .as_array()
        .ok_or("no rules")?;
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["fields"], serde_json::json!([]));
    assert!(rules[0]["matches"].as_u64().unwrap_or(0) > 0);
    Ok(())
}
