//! A person's change while the upkeep of the database (the statistics, ADR-0017, M5-F168) holds
//! SQLite's write lock longer than a writer waits for it: it is tried again and lands once, and one
//! that stays busy past the last try is refused in plain words and leaves nothing. PostgreSQL has
//! no lock of the whole database, so there the same changes land at once, without a second try.

use std::time::Duration;

use bb_app::projects::CreateProject;
use bb_core::ErrorCode;
use bb_store::Dialect;
use bb_testing::{TestResult, each_database};

use super::rig::{Rig, refused};

/// How long a writer of the rig waits for SQLite's write lock: much shorter than the program's,
/// so the test does not wait for it.
const BUSY_TIMEOUT: Duration = Duration::from_millis(200);

/// How long the upkeep holds the lock at most in the test of a change that lands: on SQLite it lets
/// go as soon as the change was tried again; PostgreSQL never tries again.
const HOLD_AT_MOST: Duration = Duration::from_secs(1);

/// How often the upkeep of the test looks whether the change was tried again.
const LOOK_EVERY: Duration = Duration::from_millis(10);

const ADMIN: &str = "ada@example.test";

/// *Create project*, asked by the administrator: a change of one transaction with its event.
async fn create(rig: &Rig, session: &str, short_name: &str) -> Result<(), bb_app::auth::AppError> {
    rig.api()
        .create_project(
            Rig::info(),
            CreateProject {
                session_token: session.to_owned(),
                parent: None,
                short_name: short_name.to_owned(),
                name: format!("Project {short_name}"),
            },
        )
        .await
        .map(|_| ())
}

#[tokio::test]
async fn adr_0017_a_write_during_the_statistics_refresh_is_tried_again_and_lands() -> TestResult {
    each_database("adr_0017_busy_write_lands", async |db| {
        let rig = Rig::start_with_busy_timeout(db, BUSY_TIMEOUT).await?;
        let admin = rig.admin(ADMIN).await?;
        // The events of one such change while nothing holds the lock.
        let before = rig.event_count().await?;
        create(&rig, &admin.session_token, "ONE").await?;
        let one = rig.event_count().await? - before;
        let before = rig.event_count().await?;
        let held = rig.database.hold_lock_as_upkeep_for_tests().await?;
        let database = &rig.database;
        let release = async move {
            // PostgreSQL never tries again: there the wait ends at its limit.
            let _waited = tokio::time::timeout(HOLD_AT_MOST, async {
                while database.writes_tried_again() == 0 {
                    tokio::time::sleep(LOOK_EVERY).await;
                }
            })
            .await;
            drop(held);
        };
        let (landed, ()) = tokio::join!(create(&rig, &admin.session_token, "TWO"), release);
        landed?;
        assert_eq!(
            rig.event_count().await? - before,
            one,
            "the change and its event are written once"
        );
        let tried_again = rig.database.writes_tried_again();
        match rig.database.dialect() {
            Dialect::Sqlite => assert!(tried_again > 0, "SQLite was busy: a second try"),
            Dialect::Postgres => assert_eq!(tried_again, 0, "PostgreSQL has no such lock"),
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0017_a_write_that_stays_busy_is_refused_in_plain_words_after_the_last_try()
-> TestResult {
    each_database("adr_0017_busy_write_refused", async |db| {
        let rig = Rig::start_with_busy_timeout(db, BUSY_TIMEOUT).await?;
        let admin = rig.admin(ADMIN).await?;
        let before = rig.event_count().await?;
        let held = rig.database.hold_lock_as_upkeep_for_tests().await?;
        let answer = create(&rig, &admin.session_token, "ONE").await;
        drop(held);
        match rig.database.dialect() {
            Dialect::Sqlite => {
                // *Bugaboo is not available right now. Please try again in a moment.*
                assert_eq!(refused(answer), Some(ErrorCode::Unavailable));
                assert_eq!(rig.event_count().await?, before, "nothing was written");
                // The same change, made again once the upkeep is done, lands.
                create(&rig, &admin.session_token, "ONE").await?;
            }
            Dialect::Postgres => answer?,
        }
        assert!(rig.event_count().await? > before);
        Ok(())
    })
    .await
}
