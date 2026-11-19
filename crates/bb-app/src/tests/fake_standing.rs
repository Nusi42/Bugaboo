//! The standing of a person in the in-memory store (PRIV-04).

use bb_core::id::UserId;
use bb_core::person::{PersonStanding, Standing};

use super::fake::{FakeError, FakeStore, Tx};
use crate::people::StandingStore;

impl StandingStore for FakeStore {
    async fn standing_in(tx: &mut Tx, person: UserId) -> Result<Option<PersonStanding>, FakeError> {
        Ok(tx
            .state
            .standings
            .iter()
            .find(|standing| standing.person == person)
            .cloned())
    }

    async fn former_labels(&self) -> Result<Vec<String>, FakeError> {
        Ok(self.read(|state| {
            state
                .standings
                .iter()
                .filter_map(|standing| standing.label.clone())
                .collect()
        }))
    }

    async fn set_standing(tx: &mut Tx, standing: &PersonStanding) -> Result<(), FakeError> {
        tx.state
            .standings
            .retain(|kept| kept.person != standing.person);
        tx.state.standings.push(standing.clone());
        Ok(())
    }

    async fn clear_standing(tx: &mut Tx, person: UserId) -> Result<bool, FakeError> {
        let before = tx.state.standings.len();
        tx.state.standings.retain(|standing| {
            standing.person != person || standing.standing != Standing::SwitchedOff
        });
        Ok(tx.state.standings.len() < before)
    }

    async fn clear_legal_hold(tx: &mut Tx, person: UserId) -> Result<bool, FakeError> {
        let Some(standing) = tx
            .state
            .standings
            .iter_mut()
            .find(|standing| standing.person == person && standing.hold_name.is_some())
        else {
            return Ok(false);
        };
        standing.hold_name = None;
        standing.hold_reason = None;
        standing.hold_until = None;
        Ok(true)
    }
}

impl crate::privacy::replay::ReplayStore for FakeStore {
    async fn wait_to_replay(
        tx: &mut Tx,
        lines: &[bb_core::backup::replay::ReplayLine],
    ) -> Result<(), FakeError> {
        for line in lines {
            let line = line.to_string();
            if !tx.state.replay_waiting.contains(&line) {
                tx.state.replay_waiting.push(line);
            }
        }
        Ok(())
    }

    async fn waiting_replay_lines(&self) -> Result<Vec<String>, FakeError> {
        Ok(self.read(|state| {
            let mut lines = state.replay_waiting.clone();
            lines.sort();
            lines
        }))
    }

    async fn replay_lines_written(&self, lines: &[String]) -> Result<(), FakeError> {
        self.lock()
            .state
            .replay_waiting
            .retain(|line| !lines.contains(line));
        Ok(())
    }
}

impl crate::privacy::removal::RemovalStore for FakeStore {
    async fn person_words(
        tx: &mut Tx,
        person: UserId,
    ) -> Result<Option<crate::privacy::removal::PersonWords>, FakeError> {
        let handles = tx
            .state
            .handles
            .iter()
            .filter(|known| known.person == person)
            .map(|known| known.handle.clone())
            .collect();
        Ok(tx
            .state
            .people
            .iter()
            .find(|(known, _)| known.id == person)
            .map(|(known, _)| crate::privacy::removal::PersonWords {
                name: known.display_name.clone(),
                address: known.email.clone(),
                handles,
            }))
    }

    // The in-memory store keeps the rows of the walk apart from its tables (`state.walk`), so this
    // deletes from those rows only and the tables keep theirs: what the removal deletes from the
    // stored tables is tested on both databases (`bb-store`, `person_data::removal`) and over HTTP
    // (`bb-server`, `removal_http`). The comments go from the real state (`delete_comments_of`).
    async fn delete_linked(
        tx: &mut Tx,
        person: UserId,
        kind: &bb_core::personal::EntityMarks,
        links: &[bb_core::personal::Link],
    ) -> Result<u64, FakeError> {
        let before = tx.state.walk.len();
        tx.state.walk.retain(|walked| {
            walked.person != person
                || walked.marks.name() != kind.name()
                || !walked.row.links.iter().any(|link| links.contains(link))
        });
        Ok(u64::try_from(before - tx.state.walk.len()).unwrap_or(u64::MAX))
    }

    async fn delete_jobs_with_address(tx: &mut Tx, address: &str) -> Result<u64, FakeError> {
        let before = tx.state.jobs.len();
        let address = address.to_lowercase();
        tx.state
            .jobs
            .retain(|job| !job.payload.to_string().to_lowercase().contains(&address));
        Ok(u64::try_from(before - tx.state.jobs.len()).unwrap_or(u64::MAX))
    }

    async fn scrub_changes(
        tx: &mut Tx,
        person: UserId,
        scrub: &crate::privacy::removal::Scrub,
    ) -> Result<u64, FakeError> {
        tx.state.scrubs.push((person, scrub.clone()));
        Ok(0)
    }
}
