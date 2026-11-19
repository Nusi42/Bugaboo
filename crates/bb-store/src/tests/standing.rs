//! The standing of a person on both databases (PRIV-04): a standing is written, read with the person,
//! replaced and cleared; a page of people holds the people its caller asks for; a label belongs to
//! one removed person; the legal-hold field is cleared on its own.

use bb_app::auth::{AccountStore as _, PeopleOf, SearchIn};
use bb_app::people::StandingStore;
use bb_app::ports::ChangeStore;
use bb_core::id::{IdSource, UserId};
use bb_core::person::{HoldReason, PersonStanding, Standing};
use bb_testing::{TestResult, each_database};
use jiff::civil::date;

use super::*;
use crate::test_support::{at, event, fixture, ids, person};

fn standing(person: UserId, by: UserId, standing: Standing) -> PersonStanding {
    PersonStanding {
        person,
        standing,
        since: at(5),
        by,
        label: None,
        hold_name: None,
        hold_reason: None,
        hold_until: None,
    }
}

async fn write(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    standing: &PersonStanding,
) -> Result<(), StoreError> {
    let stored = standing.clone();
    ChangeStore::record_change(storage, &event(source, 5), move |tx| {
        Box::pin(async move { AuthStorage::set_standing(tx, &stored).await })
    })
    .await
}

#[tokio::test]
async fn priv_04_a_standing_is_read_with_the_person_and_picks_leave_them_out() -> TestResult {
    each_database("priv_04_standing", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let [ada, bob, cid] = [
            person(&mut source, "ada@example.test"),
            person(&mut source, "bob@example.test"),
            person(&mut source, "cid@example.test"),
        ];
        for one in [&ada, &bob, &cid] {
            let stored = one.clone();
            ChangeStore::record_change(storage, &event(&mut source, 1), move |tx| {
                Box::pin(
                    async move { AuthStorage::insert_person(tx, &stored, "hash", at(1)).await },
                )
            })
            .await?;
        }
        let off = standing(bob.id, ada.id, Standing::SwitchedOff);
        write(storage, &mut source, &off).await?;
        let removed = PersonStanding {
            label: Some("K7Q2".to_owned()),
            hold_name: Some("Cid Former".to_owned()),
            hold_reason: Some(HoldReason::WorkDays),
            hold_until: Some(date(2029, 3, 31)),
            ..standing(cid.id, ada.id, Standing::Removed)
        };
        write(storage, &mut source, &removed).await?;
        assert_eq!(storage.standing(bob.id).await?, Some(off.clone()));
        assert_eq!(storage.standing(cid.id).await?, Some(removed.clone()));
        assert_eq!(storage.standing(ada.id).await?, None);
        let known = storage.people_of(&[ada.id, bob.id, cid.id]).await?;
        let of = |id: UserId| {
            known
                .iter()
                .find(|known| known.person.id == id)
                .and_then(|known| known.standing.clone())
        };
        assert_eq!(of(ada.id), None);
        assert_eq!(of(bob.id), Some(off.clone()));
        assert_eq!(of(cid.id), Some(removed.clone()));
        let page = |whom| storage.people_page(None, SearchIn::Names, whom, None, 50);
        let ids_of = |found: Vec<bb_core::person::KnownPerson>| -> Vec<UserId> {
            found.into_iter().map(|known| known.person.id).collect()
        };
        assert_eq!(ids_of(page(PeopleOf::Active).await?), [ada.id]);
        let mut not_removed = ids_of(page(PeopleOf::NotRemoved).await?);
        not_removed.sort();
        let mut expected = vec![ada.id, bob.id];
        expected.sort();
        assert_eq!(not_removed, expected);
        assert_eq!(storage.former_labels().await?, ["K7Q2"]);
        // A label belongs to one removed person.
        let twin = PersonStanding {
            label: Some("K7Q2".to_owned()),
            ..standing(ada.id, bob.id, Standing::Removed)
        };
        assert!(write(storage, &mut source, &twin).await.is_err());
        // The legal-hold field is cleared on its own; switching on clears only a switched-off
        // person.
        let (bob_id, cid_id) = (bob.id, cid.id);
        let cleared = ChangeStore::record_change(storage, &event(&mut source, 6), move |tx| {
            Box::pin(async move {
                Ok((
                    AuthStorage::clear_legal_hold(tx, cid_id).await?,
                    AuthStorage::clear_standing(tx, cid_id).await?,
                    AuthStorage::clear_standing(tx, bob_id).await?,
                ))
            })
        })
        .await?;
        assert_eq!(cleared, (true, false, true));
        let after = storage.standing(cid.id).await?.ok_or("cid stays removed")?;
        assert_eq!(
            (after.label.as_deref(), after.hold_name, after.hold_until),
            (Some("K7Q2"), None, None)
        );
        assert_eq!(storage.standing(bob.id).await?, None);
        Ok(())
    })
    .await
}
