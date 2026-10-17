//! The handles of the in-memory store.

use bb_core::handle::PersonHandle;
use bb_core::id::UserId;
use bb_core::person::Person;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx};
use crate::notifications::HandleStore;

impl HandleStore for FakeStore {
    async fn handle_taken_in(tx: &mut Tx, handle: &str) -> Result<bool, FakeError> {
        Ok(tx.state.handles.iter().any(|known| known.handle == handle))
    }

    async fn handle_of_person(&self, person: UserId) -> Result<Option<String>, FakeError> {
        Ok(self.read(|state| {
            state
                .handles
                .iter()
                .find(|known| known.person == person && known.current)
                .map(|known| known.handle.clone())
        }))
    }

    async fn person_with_handle(&self, handle: &str) -> Result<Option<UserId>, FakeError> {
        Ok(self.read(|state| {
            state
                .handles
                .iter()
                .find(|known| known.handle == handle)
                .map(|known| known.person)
        }))
    }

    async fn people_without_handle(&self, limit: u64) -> Result<Vec<Person>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<Person> = state
                .people
                .iter()
                .map(|(person, _)| person)
                .filter(|person| !state.handles.iter().any(|known| known.person == person.id))
                .cloned()
                .collect();
            found.sort_by_key(|person| (person.created_at, person.id));
            found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            found
        }))
    }

    async fn handle_choices(
        &self,
        start: &str,
        limit: u64,
    ) -> Result<Vec<(Person, String)>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<(Person, String)> = state
                .handles
                .iter()
                .filter(|known| known.current && known.handle.starts_with(start))
                .filter_map(|known| {
                    state
                        .people
                        .iter()
                        .find(|(person, _)| person.id == known.person)
                        .map(|(person, _)| (person.clone(), known.handle.clone()))
                })
                .collect();
            found.sort_by(|first, second| first.1.cmp(&second.1));
            found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            found
        }))
    }

    async fn put_handle(
        tx: &mut Tx,
        person: UserId,
        handle: &str,
        at: Timestamp,
    ) -> Result<(), FakeError> {
        for known in &mut tx.state.handles {
            if known.person == person {
                known.current = false;
            }
        }
        match tx
            .state
            .handles
            .iter_mut()
            .find(|known| known.handle == handle)
        {
            Some(known) if known.person == person => {
                known.current = true;
                known.changed_at = at;
                Ok(())
            }
            Some(_) => Err(FakeError::Duplicate),
            None => {
                tx.state.handles.push(PersonHandle {
                    person,
                    handle: handle.to_owned(),
                    changed_at: at,
                    current: true,
                });
                Ok(())
            }
        }
    }
}
