//! The lines of the replay list that wait in the database, on both databases (ADR-0044, decision
//! 8): a line written twice waits once, the lines are read oldest first, and the lines the file
//! holds are forgotten, also more of them than one statement can name; and the people who are not
//! active are found for the last-administrator rule.

use bb_app::auth::AccountStore as _;
use bb_app::people::StandingStore as _;
use bb_app::ports::ChangeStore;
use bb_core::backup::replay::{ReplayKind, ReplayLine};
use bb_core::id::{IdSource, UserId};
use bb_core::person::{PersonStanding, Standing};
use bb_testing::{TestResult, each_database};
use jiff::{SignedDuration, Timestamp};

use super::*;
use crate::test_support::{at, event, fixture, ids, person};

fn line(moment: Timestamp, id: UserId) -> ReplayLine {
    ReplayLine {
        moment,
        kind: ReplayKind::SwitchOff,
        id: id.to_string(),
        choice: None,
    }
}

async fn wait(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    lines: Vec<ReplayLine>,
) -> Result<(), StoreError> {
    ChangeStore::record_change(storage, &event(source, 3), move |tx| {
        Box::pin(async move { AuthStorage::wait_to_replay(tx, &lines).await })
    })
    .await
}

#[tokio::test]
async fn priv_04_waiting_lines_are_kept_once_read_in_order_and_forgotten() -> TestResult {
    each_database("priv_04_replay_waiting", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let ada = UserId::generate(&mut source);
        let late = line(at(20), ada);
        let early = line(at(10), ada);
        wait(storage, &mut source, vec![late.clone(), early.clone()]).await?;
        // A line that waits already stays once, in the same change or a later one.
        wait(storage, &mut source, vec![early.clone(), early.clone()]).await?;
        let waiting = storage.waiting_replay_lines().await?;
        assert_eq!(waiting, [early.to_string(), late.to_string()]);
        storage.replay_lines_written(&[early.to_string()]).await?;
        assert_eq!(storage.waiting_replay_lines().await?, [late.to_string()]);
        // More lines than one statement may name are forgotten in parts.
        let many = (0..STATEMENT_ITEMS + 2)
            .map(|second| {
                let moment =
                    at(100).checked_add(SignedDuration::from_secs(i64::try_from(second)?))?;
                Ok(line(moment, ada))
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error + Send + Sync>>>()?;
        wait(storage, &mut source, many).await?;
        let all = storage.waiting_replay_lines().await?;
        assert_eq!(all.len(), STATEMENT_ITEMS + 3);
        storage.replay_lines_written(&all).await?;
        assert!(storage.waiting_replay_lines().await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_the_people_who_are_not_active_are_found() -> TestResult {
    each_database("priv_04_not_active", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let [ada, bob, cid] = [
            person(&mut source, "ada@example.test"),
            person(&mut source, "bob@example.test"),
            person(&mut source, "cid@example.test"),
        ];
        let (bob_id, cid_id, ada_id) = (bob.id, cid.id, ada.id);
        let not_active = ChangeStore::record_change(storage, &event(&mut source, 1), move |tx| {
            Box::pin(async move {
                for one in [&ada, &bob, &cid] {
                    AuthStorage::insert_person(tx, one, "hash", at(1)).await?;
                }
                for (whom, standing) in
                    [(bob_id, Standing::SwitchedOff), (cid_id, Standing::Removed)]
                {
                    AuthStorage::set_standing(
                        tx,
                        &PersonStanding {
                            person: whom,
                            standing,
                            since: at(2),
                            by: Some(ada_id),
                            label: (standing == Standing::Removed).then(|| "K7Q2".to_owned()),
                            hold_name: None,
                            hold_reason: None,
                            hold_until: None,
                        },
                    )
                    .await?;
                }
                crate::auth::standing::not_active_in(tx).await
            })
        })
        .await?;
        assert_eq!(not_active, [bob_id, cid_id].into_iter().collect());
        Ok(())
    })
    .await
}
