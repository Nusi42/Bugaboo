//! The acceptance tests of the sample organization (M2 WP-06). They make the real thing: 50
//! people through the use cases, on SQLite and on PostgreSQL.

use std::path::Path;

use bb_core::sign_in::SecurityAction;
use bb_testing::{TestResult, each_database};
use jiff::Timestamp;
use serde_json::Value;

use crate::{
    DEFAULT_SEED, EMAIL_DOMAIN, NameList, NothingMore, Options, PEOPLE, Sample, Target, make,
};

/// 2027-01-15 08:00:00 UTC: one start moment for every test, so the dates are always the same.
const START: i64 = 1_800_000_000;

fn options(seed: u64) -> Options {
    Options {
        seed,
        start: Timestamp::from_second(START).unwrap(),
    }
}

type Failure = Box<dyn std::error::Error + Send + Sync>;

/// Makes the sample of `seed` in the folder, on the built-in database.
async fn built_in(seed: u64, folder: &Path) -> Result<Sample, Failure> {
    Ok(make(&options(seed), folder, &Target::BuiltIn, &mut NothingMore).await?)
}

/// The canonical dump as the text of `dump.json`, and the sample closed.
async fn dump_of(sample: Sample) -> Result<String, Failure> {
    sample.verify_security_log().await?;
    let text = sample.dump().await?.to_json()?;
    sample.close().await;
    Ok(text)
}

#[tokio::test]
async fn the_sample_is_the_same_from_the_same_seed_on_both_databases() -> TestResult {
    each_database("the_sample_is_the_same_from_the_same_seed", async |db| {
        let (first, second, other) = (
            tempfile::tempdir()?,
            tempfile::tempdir()?,
            tempfile::tempdir()?,
        );
        // On the database of this run, and twice on the built-in one: the three dumps are one.
        let here = make(
            &options(DEFAULT_SEED),
            first.path(),
            &Target::Address(db.url().to_owned()),
            &mut NothingMore,
        )
        .await?;
        let on_this_database = dump_of(here).await?;
        let built_in_once = dump_of(built_in(DEFAULT_SEED, second.path()).await?).await?;
        let built_in_twice = dump_of(built_in(DEFAULT_SEED, other.path()).await?).await?;
        assert_eq!(
            built_in_once, built_in_twice,
            "the same seed, the same dump"
        );
        assert_eq!(
            on_this_database,
            built_in_once,
            "the same dump on {}",
            db.kind().name()
        );
        // The dump is not empty or trivially equal: it has the people and what happened.
        let dump: Value = serde_json::from_str(&on_this_database)?;
        assert_eq!(dump["people"].as_array().map(Vec::len), Some(PEOPLE));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn another_seed_makes_another_organization() -> TestResult {
    let (first, second) = (tempfile::tempdir()?, tempfile::tempdir()?);
    let forty_two = dump_of(built_in(42, first.path()).await?).await?;
    let forty_three = dump_of(built_in(43, second.path()).await?).await?;
    assert_ne!(forty_two, forty_three);
    Ok(())
}

#[tokio::test]
async fn every_sample_address_is_under_bugaboo_example_and_every_name_is_from_the_list()
-> TestResult {
    let folder = tempfile::tempdir()?;
    let text = dump_of(built_in(DEFAULT_SEED, folder.path()).await?).await?;
    let dump: Value = serde_json::from_str(&text)?;
    let names = NameList::bundled()?;
    let people = dump["people"].as_array().ok_or("no people")?;
    assert_eq!(people.len(), PEOPLE);
    for person in people {
        let email = person["email"].as_str().ok_or("no address")?;
        assert!(
            email.ends_with(&format!("@{EMAIL_DOMAIN}")),
            "{email} is not under {EMAIL_DOMAIN}"
        );
        let name = person["name"].as_str().ok_or("no name")?;
        assert!(names.contains_full_name(name), "{name} is not on the list");
    }
    // The addresses in the events (the invitations) are the same ones.
    for event in dump["events"].as_array().ok_or("no events")? {
        for change in event["changes"].as_array().ok_or("no changes")? {
            if change["field"] == "email" {
                let email = change["after"].as_str().ok_or("no address")?;
                assert!(email.ends_with(&format!("@{EMAIL_DOMAIN}")), "{email}");
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn every_sample_change_has_its_event_and_the_security_log_verifies() -> TestResult {
    each_database("every_sample_change_has_its_event", async |db| {
        let folder = tempfile::tempdir()?;
        let sample = make(
            &options(DEFAULT_SEED),
            folder.path(),
            &Target::Address(db.url().to_owned()),
            &mut NothingMore,
        )
        .await?;
        let entries = sample.verify_security_log().await?;
        let dump = sample.dump().await?;
        sample.close().await;
        assert_eq!(usize::try_from(entries)?, dump.security_log_entries());

        // Everybody has the event of making their account, made by themselves (they accepted an
        // invitation, or used the setup code), and every other person was invited by the first.
        let events: Vec<(&str, &str)> = dump.event_entities().collect();
        for number in 1..=PEOPLE {
            let person = format!("person#{number}");
            assert!(
                events
                    .iter()
                    .any(|(entity, actor)| *entity == person && *actor == person),
                "{person} has no event of their own"
            );
        }
        let invitations = events
            .iter()
            .filter(|(entity, actor)| entity.starts_with("invitation#") && *actor == "person#1")
            .count();
        assert_eq!(
            invitations,
            PEOPLE - 1,
            "an invitation for each other person"
        );

        // The security log has one entry for each of these, in the order they happened.
        let count = |action: SecurityAction| {
            dump.security_log_actions()
                .filter(|found| *found == action.code())
                .count()
        };
        assert_eq!(count(SecurityAction::SetupCompleted), 1);
        assert_eq!(count(SecurityAction::InvitationCreated), PEOPLE - 1);
        assert_eq!(count(SecurityAction::InvitationAccepted), PEOPLE - 1);
        assert_eq!(count(SecurityAction::TwoStepEnabled), 5);
        assert_eq!(
            dump.security_log_actions().next(),
            Some(SecurityAction::SetupCompleted.code()),
            "the first administrator comes first"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn no_password_or_secret_is_written_outside_the_sign_in_text() -> TestResult {
    let folder = tempfile::tempdir()?;
    let sample = built_in(DEFAULT_SEED, folder.path()).await?;
    let mut secrets: Vec<String> = Vec::new();
    for made in sample.people() {
        secrets.push(made.password.clone());
        secrets.extend(made.code_app_secret.clone());
    }
    assert!(
        secrets.len() >= PEOPLE + 5,
        "a password each and five code apps"
    );
    let sign_in = sample.sign_in_text();
    for secret in &secrets {
        assert!(
            sign_in.contains(secret),
            "the sign-in text has every secret"
        );
    }
    let dump = sample.dump().await?.to_json()?;
    sample.close().await;

    // The dump, the database, the keys and the log files: nothing of it holds a secret.
    let mut places = vec![("the dump".to_owned(), dump.into_bytes())];
    collect(folder.path(), &mut places)?;
    assert!(places.len() > 3, "the database and the keys were looked at");
    for (place, bytes) in &places {
        for secret in &secrets {
            assert!(
                !bytes
                    .windows(secret.len())
                    .any(|window| window == secret.as_bytes()),
                "{place} holds a password or secret"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn the_sign_in_text_has_a_line_for_every_person_with_the_columns_the_tests_read() -> TestResult
{
    let folder = tempfile::tempdir()?;
    let sample = built_in(DEFAULT_SEED, folder.path()).await?;
    let text = sample.sign_in_text();
    let lines: Vec<Vec<&str>> = text
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.split(char::from(9)).collect())
        .collect();
    assert_eq!(lines.len(), PEOPLE);
    for (columns, made) in lines.iter().zip(sample.people()) {
        // Address, password, role, name, and the secret of the code app when there is one.
        assert_eq!(columns[0], made.person.email);
        assert_eq!(columns[1], made.password);
        assert_eq!(columns[2], made.person.role.code());
        assert_eq!(columns[3], made.person.name);
        assert_eq!(columns.get(4).copied(), made.code_app_secret.as_deref());
    }
    assert!(text.starts_with("# "), "the file explains itself first");
    sample.close().await;
    Ok(())
}

/// Every file below `folder` with its bytes.
fn collect(folder: &Path, found: &mut Vec<(String, Vec<u8>)>) -> Result<(), Failure> {
    for entry in std::fs::read_dir(folder)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, found)?;
        } else {
            found.push((path.display().to_string(), std::fs::read(&path)?));
        }
    }
    Ok(())
}
