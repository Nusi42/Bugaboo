//! The acceptance tests of the people working together (M4 WP-10, M3-F111): what each role did in
//! the small sample is what the role may do, the files have their previews, and no notification
//! tells a Guest of an issue they did not report. They make the real thing on SQLite and PostgreSQL
//! and read the canonical dump (TEST-SAMPLE-DATA holds the counts against the plan).

use std::collections::BTreeMap;

use bb_core::fields::Field;
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;
use serde_json::Value;

use crate::{DEFAULT_SEED, Everything, Options, Sample, Target, create};

const START: i64 = bb_dev_words::accounts::CLOCK_START;
/// Few enough to be quick, enough for every kind of work to be there.
const ISSUES: u32 = 150;

async fn dump_of(target: &Target, folder: &std::path::Path) -> Result<Value, Failure> {
    let options = Options {
        seed: DEFAULT_SEED,
        start: Timestamp::from_second(START)?,
    };
    let mut everything = Everything::new(ISSUES)?;
    let sample: Sample = create(&options, folder, target, &mut everything).await?;
    sample.verify_security_log().await?;
    let text = sample.dump().await?.to_json()?;
    sample.close().await;
    Ok(serde_json::from_str(&text)?)
}

/// The role of each person by the label the dump gives them (`person#1` is the first person).
fn roles_of(dump: &Value) -> Result<BTreeMap<String, String>, Failure> {
    dump["people"]
        .as_array()
        .ok_or("no people")?
        .iter()
        .enumerate()
        .map(|(at, person)| {
            Ok((
                format!("person#{}", at + 1),
                person["role"].as_str().ok_or("no role")?.to_owned(),
            ))
        })
        .collect()
}

/// Whether the event changes the field: `added` for a value that had none before, else for one
/// that had a value before.
fn has_change(event: &Value, field: &str, added: bool) -> bool {
    event["changes"].as_array().is_some_and(|changes| {
        changes
            .iter()
            .any(|change| change["field"] == field && change["before"].is_null() == added)
    })
}

#[tokio::test]
async fn m3_f111_every_role_does_only_what_the_role_may() -> TestResult {
    each_database(
        "m3_f111_every_role_does_only_what_the_role_may",
        async |db| {
            let folder = tempfile::tempdir()?;
            let dump = dump_of(&Target::Address(db.url().to_owned()), folder.path()).await?;
            let roles = roles_of(&dump)?;
            let events = dump["events"].as_array().ok_or("no events")?;
            // Who made each issue.
            let mut made_by: BTreeMap<&str, &str> = BTreeMap::new();
            for event in events {
                let entity = event["entity"].as_str().unwrap_or_default();
                if entity.starts_with("issue#") && has_change(event, Field::Title.code(), true) {
                    made_by.insert(entity, event["actor"].as_str().unwrap_or_default());
                }
            }
            let mut seen: BTreeMap<String, usize> = BTreeMap::new();
            for event in events {
                let actor = event["actor"].as_str().unwrap_or_default();
                let Some(role) = roles.get(actor) else {
                    continue;
                };
                let entity = event["entity"].as_str().unwrap_or_default();
                let kind = entity.split('#').next().unwrap_or_default();
                match role.as_str() {
                    // Contributors and Guests only touch issues they made themselves, and comment under them.
                    "contributor" | "guest" => {
                        if matches!(kind, "issue") {
                            assert_eq!(
                                made_by.get(entity),
                                Some(&actor),
                                "{role} {actor} changed {entity} that they did not make"
                            );
                        }
                        // A Guest never sets a status, a priority or an assignment.
                        if role == "guest" && kind == "issue" {
                            for field in [Field::Status, Field::Priority, Field::AssignedTo] {
                                assert!(
                                    !has_change(event, field.code(), false),
                                    "a Guest changed {}",
                                    field.code()
                                );
                            }
                        }
                    }
                    // Viewers only follow: they write nothing else but their own reading, their
                    // settings and signing in. (A follower has the ID of its issue, so the dump
                    // labels it like the issue; its change is the field `follower`.)
                    "viewer" => assert!(
                        has_change(event, "follower", true)
                            || matches!(kind, "person" | "setting" | "session"),
                        "a Viewer wrote {entity}: {event}"
                    ),
                    _ => {}
                }
                *seen.entry(role.clone()).or_default() += 1;
            }
            for role in ["contributor", "guest", "viewer"] {
                assert!(
                    seen.get(role).copied().unwrap_or(0) > 0,
                    "{role} did nothing"
                );
            }
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn no_notification_tells_a_guest_of_an_issue_they_did_not_report() -> TestResult {
    each_database(
        "no_notification_tells_a_guest_of_an_issue_they_did_not_report",
        async |db| {
            let folder = tempfile::tempdir()?;
            let dump = dump_of(&Target::Address(db.url().to_owned()), folder.path()).await?;
            let work = &dump["issues"]["work"]["notifications"];
            assert_eq!(
                work["to_guests_about_others"].as_u64(),
                Some(0),
                "a Guest was told of an issue they did not report"
            );
            assert!(
                work["to"]["guest"].as_u64().unwrap_or(0) > 0,
                "a Guest has notifications about their own issues"
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn att_04_the_files_of_the_sample_are_pictures_and_pdfs_that_all_have_a_preview() -> TestResult
{
    let folder = tempfile::tempdir()?;
    let dump = dump_of(&Target::BuiltIn, folder.path()).await?;
    let files = &dump["issues"]["work"]["files"];
    let total = files["total"].as_u64().unwrap_or(0);
    assert!(total > 0);
    assert_eq!(files["previews_ready"].as_u64(), Some(total), "{files}");
    assert_eq!(files["without_preview"].as_u64(), Some(0), "{files}");
    // The pictures and the pages are in the files folder of the data folder, and the previews too.
    let in_store = std::fs::read_dir(folder.path().join(bb_files::FILES_FOLDER))?.count();
    assert!(in_store > 0, "the files folder is empty");
    Ok(())
}

#[tokio::test]
async fn wf_04_the_project_with_a_workflow_of_its_own_asks_for_the_person_who_starts_work()
-> TestResult {
    let folder = tempfile::tempdir()?;
    let dump = dump_of(&Target::BuiltIn, folder.path()).await?;
    let work = &dump["issues"]["work"];
    let plan = bb_dev_words::sample::Layout::bundled()?
        .issues
        .work
        .own_workflow;
    assert_eq!(
        work["own_workflow_projects"],
        serde_json::json!([plan.project])
    );
    let own = work["workflows"]
        .as_array()
        .ok_or("no workflows")?
        .iter()
        .find(|workflow| workflow["name"] == plan.name.as_str())
        .ok_or("the workflow of the project is missing")?;
    assert_eq!(own["asks"].as_u64(), Some(1));
    // The issues of that project went through the change that asks: the person who started the
    // work was named in the same change.
    assert!(
        work["asked_for_who"].as_u64().unwrap_or(0) > 0,
        "no status change asked for who does the work"
    );
    Ok(())
}
