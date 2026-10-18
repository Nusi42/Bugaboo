//! The acceptance tests of the sample organization (M2 WP-06 and WP-13). They make the real thing:
//! 50 people, groups, roles, projects and assignments through the use cases, on SQLite and on
//! PostgreSQL.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use bb_app::access::StartingRole;
use bb_app::projects::ProjectStore as _;
use bb_app::roles::{CheckAccessQuery, check_access};
use bb_core::message::Message;
use bb_core::sign_in::SecurityAction;
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;
use serde_json::Value;

mod issues;
mod work;

use crate::{
    ACCESS_KEYS, DEFAULT_SEED, EMAIL_DOMAIN, LATER_SECONDS, Later, NameList, Options, Organization,
    PEOPLE, ROLE_MIX, STEP_SECONDS, Sample, SampleError, Step, Target, World, create,
};

/// One start moment for every test, so the dates are always the same.
const START: i64 = bb_dev_words::accounts::CLOCK_START;

fn options(seed: u64) -> Options {
    Options {
        seed,
        start: Timestamp::from_second(START).unwrap(),
    }
}

/// Makes the sample of `seed` in the folder, on the built-in database.
async fn built_in(seed: u64, folder: &Path) -> Result<Sample, Failure> {
    Ok(create(&options(seed), folder, &Target::BuiltIn, &mut Organization).await?)
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
        let here = create(
            &options(DEFAULT_SEED),
            first.path(),
            &Target::Address(db.url().to_owned()),
            &mut Organization,
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
            db.kind().code()
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
        let sample = create(
            &options(DEFAULT_SEED),
            folder.path(),
            &Target::Address(db.url().to_owned()),
            &mut Organization,
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
        assert_eq!(count(SecurityAction::StartingRolesCreated), 1);
        // The organization came from the same use cases: a security-log entry for each.
        assert_eq!(count(SecurityAction::GroupCreated), 8);
        assert_eq!(count(SecurityAction::ProjectCreated), 8);
        assert_eq!(count(SecurityAction::RoleCreated), 2);
        assert_eq!(
            dump.security_log_actions().take(2).collect::<Vec<_>>(),
            [
                SecurityAction::StartingRolesCreated.code(),
                SecurityAction::SetupCompleted.code(),
            ],
            "the starting roles, then the first administrator, come first"
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
        secrets.extend(made.access_key.clone());
    }
    assert!(
        secrets.len() >= PEOPLE + 5 + ACCESS_KEYS.len(),
        "a password each, five code apps and the access keys"
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
        // Address, password, role, name, the secret of the code app when there is one (empty
        // when only the access key is), and the access key when there is one.
        assert_eq!(columns[0], made.person.email);
        assert_eq!(columns[1], made.password);
        assert_eq!(columns[2], made.person.role.code());
        assert_eq!(columns[3], made.person.name);
        assert_eq!(
            columns.get(4).copied().filter(|secret| !secret.is_empty()),
            made.code_app_secret.as_deref()
        );
        assert_eq!(columns.get(5).copied(), made.access_key.as_deref());
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

/// Reads a text out of a JSON value by the names of the members, or fails with what was missing.
fn text<'a>(value: &'a Value, path: &[&str]) -> Result<&'a str, Failure> {
    let mut at = value;
    for name in path {
        at = at
            .get(name)
            .ok_or_else(|| format!("no {name} in {path:?}"))?;
    }
    Ok(at.as_str().ok_or_else(|| format!("{path:?} is not text"))?)
}

fn members<'a>(dump: &'a Value, key: &str) -> Result<&'a Vec<Value>, Failure> {
    Ok(dump[key]
        .as_array()
        .ok_or_else(|| format!("the dump has no {key}"))?)
}

/// The eight groups, each with members.
fn the_eight_groups(dump: &Value) -> Result<(), Failure> {
    let mut groups: BTreeMap<&str, usize> = BTreeMap::new();
    for group in members(dump, "groups")? {
        groups.insert(
            text(group, &["name"])?,
            group["members"].as_array().map_or(0, Vec::len),
        );
    }
    assert_eq!(
        groups.keys().copied().collect::<Vec<_>>(),
        [
            "Backend",
            "Docs",
            "External",
            "Frontend",
            "Management",
            "Mobile",
            "QA",
            "Support"
        ]
    );
    assert!(groups.values().all(|count| *count > 0), "{groups:?}");
    Ok(())
}

/// The six starting roles with the mix of the plan of the people, and the two custom roles.
fn the_roles(dump: &Value) -> Result<(), Failure> {
    let mut roles: BTreeMap<&str, (bool, u64)> = BTreeMap::new();
    for role in members(dump, "roles")? {
        roles.insert(
            text(role, &["name"])?,
            (
                role["two_step"] == true,
                role["held_by"].as_u64().unwrap_or(0),
            ),
        );
    }
    for role in StartingRole::ALL.iter().copied() {
        let name = Message::from(role.name()).render();
        let name = name.as_str();
        let wanted = ROLE_MIX
            .iter()
            .find(|(known, _)| *known == role)
            .map(|(_, count)| *count);
        assert_eq!(
            roles
                .get(name)
                .map(|(_, held)| usize::try_from(*held).unwrap_or(0)),
            wanted,
            "{name}"
        );
    }
    assert_eq!(roles.get("Support agent"), Some(&(false, 3)));
    assert_eq!(roles.get("Security reviewer"), Some(&(true, 5)));
    Ok(())
}

/// Four projects with their sub-projects, a lead for each.
fn the_projects(dump: &Value) -> Result<(), Failure> {
    let projects = members(dump, "projects")?;
    let mut shorts = Vec::new();
    for project in projects {
        shorts.push(text(project, &["short_name"])?);
    }
    assert_eq!(
        shorts,
        ["AND", "APP", "BLOG", "DOC", "IOS", "OPS", "SHOP", "WEB"]
    );
    for project in projects {
        assert!(
            project["leads"]
                .as_array()
                .is_some_and(|leads| !leads.is_empty()),
            "{} has no lead",
            project["short_name"]
        );
    }
    Ok(())
}

/// One No access, on a sub-project, rows for the whole organization, on projects and through
/// groups; Mobile app hides Cost center from Contributors, Website keeps the starting rules.
fn the_rows_and_levels(dump: &Value) -> Result<(), Failure> {
    let rows = members(dump, "assignments")?;
    let no_access: Vec<&Value> = rows.iter().filter(|row| row["role"].is_null()).collect();
    assert_eq!(no_access.len(), 1);
    assert_eq!(no_access[0]["project"], "AND");
    assert!(
        rows.iter()
            .any(|row| row["project"].is_null() && !row["role"].is_null())
    );
    assert!(
        rows.iter()
            .any(|row| row["who"] == "External" && row["project"] == "WEB")
    );
    let levels = members(dump, "levels")?;
    let on = |project: &str, role: &str| {
        levels
            .iter()
            .filter(|level| level["project"] == project && level["role"] == role)
            .count()
    };
    assert_eq!(on("APP", "Contributor"), 1);
    assert_eq!(on("APP", "Guest"), 2);
    assert_eq!(on("WEB", "Contributor") + on("WEB", "Guest"), 0);
    Ok(())
}

#[tokio::test]
async fn the_sample_holds_the_groups_roles_and_projects_of_architecture_11() -> TestResult {
    each_database(
        "the_sample_holds_the_groups_roles_and_projects_of_architecture_11",
        async |db| {
            let folder = tempfile::tempdir()?;
            let sample = create(
                &options(DEFAULT_SEED),
                folder.path(),
                &Target::Address(db.url().to_owned()),
                &mut Organization,
            )
            .await?;
            let dump: Value = serde_json::from_str(&sample.dump().await?.to_json()?)?;
            sample.close().await;

            the_eight_groups(&dump)?;
            the_roles(&dump)?;
            the_projects(&dump)?;
            the_rows_and_levels(&dump)?;
            Ok(())
        },
    )
    .await
}

/// Makes the organization, then asks *Check access* about every person in the organization and in
/// every project and notes which roles it names, and what it says about Ana on Website.
#[derive(Default)]
struct CheckingAccess {
    /// For each role, the addresses of the people it was named for.
    held: BTreeMap<String, BTreeSet<String>>,
    /// The answer for Ana on Website, as the browser gets it.
    ana_on_website: Option<Value>,
}

impl Later for CheckingAccess {
    async fn after_people(&mut self, world: &World<'_>) -> Result<(), SampleError> {
        Organization.after_people(world).await?;
        let failed = |error: &dyn std::fmt::Display| SampleError::step(Step::Assignment, &error);
        let projects = world.storage.project_tree().await?;
        let website = projects
            .iter()
            .find(|project| project.short_name.as_str() == "WEB")
            .map(|project| project.id);
        let mut places = vec![None];
        places.extend(projects.iter().map(|project| Some(project.id)));
        for made in world.people {
            for place in &places {
                let answer = check_access(
                    &world.services.ctx(world.request()),
                    CheckAccessQuery {
                        issue_type: None,
                        session_token: world.administrator_session.to_owned(),
                        person: made.id,
                        place: *place,
                    },
                )
                .await
                .map_err(|error| failed(&error))?
                .masked_value()
                .map_err(|error| failed(&error))?;
                for permission in answer["permissions"].as_array().into_iter().flatten() {
                    for reason in permission["reasons"].as_array().into_iter().flatten() {
                        if reason["kind"] == "given_by_role"
                            && let Some(role) = reason["role"]["name"].as_str()
                        {
                            self.held
                                .entry(role.to_owned())
                                .or_default()
                                .insert(made.person.email.clone());
                        }
                    }
                }
                if made.person.name == "Ana Moreno" && *place == website {
                    self.ana_on_website = Some(answer);
                }
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn the_sample_shows_the_role_mix_through_check_access() -> TestResult {
    each_database(
        "the_sample_shows_the_role_mix_through_check_access",
        async |db| {
            let folder = tempfile::tempdir()?;
            let mut checking = CheckingAccess::default();
            let sample = create(
                &options(DEFAULT_SEED),
                folder.path(),
                &Target::Address(db.url().to_owned()),
                &mut checking,
            )
            .await?;
            sample.close().await;

            // Every role of the mix is held by the right number of people.
            let counts: BTreeMap<&str, usize> = checking
                .held
                .iter()
                .map(|(role, people)| (role.as_str(), people.len()))
                .collect();
            assert_eq!(
                counts,
                BTreeMap::from([
                    ("Administrator", 2),
                    ("Project lead", 4),
                    ("Team member", 30),
                    ("Contributor", 8),
                    ("Viewer", 4),
                    ("Guest", 2),
                    ("Support agent", 3),
                    ("Security reviewer", 5),
                ])
            );

            // Ana is a Contributor on Website because the group External has the role there;
            // her Cost center is what the exit demo hides.
            let ana = checking
                .ana_on_website
                .ok_or("no answer for Ana on Website")?;
            let see = ana["permissions"]
                .as_array()
                .and_then(|all| all.iter().find(|p| p["permission"] == "see-projects"))
                .ok_or("no see-projects")?;
            assert_eq!(see["allowed"], true);
            assert_eq!(
                text(&see["reasons"][0], &["row", "who", "group", "name"])?,
                "External"
            );
            assert_eq!(text(&see["reasons"][0], &["role", "name"])?, "Contributor");
            let cost_center = ana["fields"]
                .as_array()
                .and_then(|all| all.iter().find(|f| f["subject"] == "cost-center"))
                .ok_or("no cost-center")?;
            assert_eq!(cost_center["level"], "can-see");
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn everything_is_dated_before_the_start_moment_and_inside_the_time_reserved() -> TestResult {
    let folder = tempfile::tempdir()?;
    let text = dump_of(built_in(DEFAULT_SEED, folder.path()).await?).await?;
    let dump: Value = serde_json::from_str(&text)?;
    let start = Timestamp::from_second(START)?;
    let first = start.as_second() - LATER_SECONDS - i64::try_from(PEOPLE)? * STEP_SECONDS;
    let mut last = i64::MIN;
    for event in members(&dump, "events")? {
        let time = event["time"]
            .as_str()
            .ok_or("no time")?
            .parse::<Timestamp>()?
            .as_second();
        assert!((first..=start.as_second()).contains(&time), "{time}");
        last = last.max(time);
    }
    // The organization is made in far less than the time reserved for it.
    assert!(last < start.as_second() - LATER_SECONDS / 2);
    Ok(())
}
