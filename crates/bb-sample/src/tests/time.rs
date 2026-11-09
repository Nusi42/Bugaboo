//! The acceptance tests of the working hours and the time of the sample (M6 WP-12, TIME-01 to
//! TIME-13, TEST-SAMPLE-DATA): the canonical dump holds the counts of the plan, some days broke a
//! working-time rule and some were changed with a reason, *HR* (Hana, through the group *Payroll*)
//! sees the working hours of others and a Team member does not. They make the real thing on SQLite
//! and PostgreSQL and read it back through the use cases, as each person would.

use bb_app::auth::{SignIn, sign_in};
use bb_app::working_hours::{PeopleOfHoursQuery, WorkDaysQuery, people_of_hours, work_days};
use bb_core::id::UserId;
use bb_testing::{Failure, TestResult, each_database};
use jiff::{Timestamp, civil::Date};
use serde_json::Value;

use crate::{
    DEFAULT_SEED, Everything, Later, LeftBehind, Options, SampleError, Step, Target, World, create,
};

const START: i64 = bb_dev_words::accounts::CLOCK_START;
/// Few enough to be quick, enough for every kind of work to be there.
const ISSUES: u32 = 150;
/// How many people the page of people of working hours holds: more than the sample has.
const PAGE: u32 = 100;
/// How many days back from the start of the sample the days are read.
const BACK_DAYS: i64 = 60;

/// What the people of the sample see of working hours.
#[derive(Default)]
struct Seen {
    /// The days that carry a warning, of the days of the first people.
    warned: usize,
    /// The days that show *Changed*.
    changed: usize,
    /// The days Hana reads of Ben.
    hana_reads: usize,
    /// The names Hana finds in *Working hours of others*.
    hana_finds: Vec<String>,
    /// Whether Ben is refused the days of Ana, and the list of people.
    ben_refused: bool,
}

/// The sample with everything, and then some people reading working hours.
struct Reading {
    everything: Everything,
    seen: Seen,
}

fn person_named<'a>(world: &'a World<'_>, name: &str) -> Option<&'a crate::MadePerson> {
    world.people.iter().find(|made| made.person.name == name)
}

async fn token_of(world: &World<'_>, made: &crate::MadePerson) -> Result<String, SampleError> {
    Ok(sign_in(
        &world.services.ctx(world.request()),
        SignIn {
            email: made.person.email.clone(),
            password: made.password.clone(),
            previous_session: None,
        },
    )
    .await
    .map_err(|error| SampleError::step(Step::SigningIn, &error))?
    .session_token)
}

fn query(token: String, person: Option<UserId>, last: Date) -> WorkDaysQuery {
    WorkDaysQuery {
        session_token: token,
        person,
        first: last
            .checked_sub(jiff::Span::new().days(BACK_DAYS))
            .unwrap_or(last),
        last,
    }
}

impl Later for Reading {
    fn history_seconds(&self) -> i64 {
        self.everything.history_seconds()
    }

    fn left_behind(&self) -> LeftBehind {
        self.everything.left_behind()
    }

    async fn after_people(&mut self, world: &World<'_>) -> Result<(), SampleError> {
        Box::pin(self.everything.after_people(world)).await?;
        let step = |error: &dyn std::fmt::Display| SampleError::step(Step::Time, &error);
        let last = world.start.to_zoned(jiff::tz::TimeZone::UTC).date();
        // The days of the first people, each reading their own.
        for made in world
            .people
            .iter()
            .filter(|made| !made.person.code_app)
            .take(6)
        {
            let token = token_of(world, made).await?;
            let page = work_days(
                &world.services.ctx(world.request()),
                query(token, None, last),
            )
            .await
            .map_err(|error| step(&error))?
            .into_value();
            for day in &page.days {
                self.seen.warned += usize::from(!day.warnings.is_empty());
                self.seen.changed += usize::from(day.changed);
            }
        }
        let (Some(hana), Some(ben), Some(ana)) = (
            person_named(world, "Hana Weiss"),
            person_named(world, "Ben Hale"),
            person_named(world, "Ana Moreno"),
        ) else {
            return Err(SampleError::PersonMissing);
        };
        let ctx = world.services.ctx(world.request());
        let token = token_of(world, hana).await?;
        let read = work_days(&ctx, query(token.clone(), Some(ben.id), last))
            .await
            .map_err(|error| step(&error))?
            .into_value();
        self.seen.hana_reads = read.days.len();
        let found = people_of_hours(
            &ctx,
            PeopleOfHoursQuery {
                session_token: token,
                search: None,
                cursor: None,
                limit: PAGE,
            },
        )
        .await
        .map_err(|error| step(&error))?
        .masked_value()
        .map_err(|error| step(&error))?;
        self.seen.hana_finds = found["items"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter_map(|item| item["name"].as_str().map(str::to_owned))
            .collect();
        let token = token_of(world, ben).await?;
        let refused_days = work_days(&ctx, query(token.clone(), Some(ana.id), last)).await;
        let refused_people = people_of_hours(
            &ctx,
            PeopleOfHoursQuery {
                session_token: token,
                search: None,
                cursor: None,
                limit: PAGE,
            },
        )
        .await;
        self.seen.ben_refused = refused_days.is_err() && refused_people.is_err();
        Ok(())
    }
}

async fn read_back(target: &Target, folder: &std::path::Path) -> Result<(Reading, Value), Failure> {
    let options = Options {
        seed: DEFAULT_SEED,
        start: Timestamp::from_second(START)?,
    };
    let mut reading = Reading {
        everything: Everything::new(ISSUES)?,
        seen: Seen::default(),
    };
    let sample = Box::pin(create(&options, folder, target, &mut reading)).await?;
    let dump = serde_json::from_str(&sample.dump().await?.to_json()?)?;
    sample.close().await;
    Ok((reading, dump))
}

#[tokio::test]
async fn time_01_the_sample_has_working_hours_entries_and_changed_days_that_follow_the_plan()
-> TestResult {
    each_database("time_01_the_sample_time", async |db| {
        let folder = tempfile::tempdir()?;
        let (reading, dump) = Box::pin(read_back(
            &Target::Address(db.url().to_owned()),
            folder.path(),
        ))
        .await?;
        // The counts of the dump are the plan's, as TEST-SAMPLE-DATA judges them.
        let plan = bb_dev_words::sample::Layout::bundled()?.issues;
        assert_eq!(
            plan.problems(ISSUES, "time", &dump),
            Vec::<String>::new(),
            "{}",
            dump["issues"]["work"]["time"]
        );
        // Some days broke a working-time rule and some were changed afterwards with a reason.
        let seen = &reading.seen;
        assert!(seen.warned > 0, "no day of the first people has a warning");
        assert!(seen.changed > 0, "no day of the first people shows Changed");
        // Hana (HR) reads the days of others and finds them; Ben (a Team member) is refused.
        assert!(seen.hana_reads > 0, "Hana reads no day of Ben");
        assert!(
            seen.hana_finds.iter().any(|name| name == "Ana Moreno"),
            "{:?}",
            seen.hana_finds
        );
        assert!(seen.ben_refused, "Ben may read the working hours of others");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_02_the_notifications_of_the_sample_are_made_from_every_event() -> TestResult {
    each_database("time_02_the_sample_notifications", async |db| {
        let folder = tempfile::tempdir()?;
        let options = Options {
            seed: DEFAULT_SEED,
            start: Timestamp::from_second(START)?,
        };
        let mut everything = Everything::new(ISSUES)?;
        let sample = Box::pin(create(
            &options,
            folder.path(),
            &Target::Address(db.url().to_owned()),
            &mut everything,
        ))
        .await?;
        // The first page that lists notifications must not read the events of the working hours:
        // that took seconds, and under load more than the 30 seconds a request has.
        let waiting = sample
            .database
            .outbox_events(bb_core::notification::CONSUMER, 1)
            .await?;
        sample.close().await;
        assert!(
            waiting.is_empty(),
            "the notifications are behind the events"
        );
        Ok(())
    })
    .await
}
