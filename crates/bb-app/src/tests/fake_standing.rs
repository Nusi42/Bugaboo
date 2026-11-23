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

    async fn person_exists(tx: &mut Tx, person: UserId) -> Result<bool, FakeError> {
        Ok(tx.state.people.iter().any(|(known, _)| known.id == person))
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
    async fn reindex_removed_texts(
        &self,
        issues: &[bb_core::id::IssueId],
        comments: &[bb_core::id::CommentId],
    ) -> Result<(), FakeError> {
        self.lock()
            .state
            .reindexed
            .push((issues.to_vec(), comments.to_vec()));
        Ok(())
    }

    async fn compact_removed_texts(&self) -> Result<(), FakeError> {
        self.lock().state.compactions += 1;
        Ok(())
    }

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

    async fn scrub_records(
        tx: &mut Tx,
        person: UserId,
        scrub: &crate::privacy::removal::Scrub,
    ) -> Result<u64, FakeError> {
        tx.state.scrubs.push((person, scrub.clone()));
        Ok(0)
    }

    async fn seal_removal_words(
        &self,
        person: UserId,
        words: &crate::privacy::removal::PersonWords,
        at: jiff::Timestamp,
    ) -> Result<bb_core::secret::StoredSecret, FakeError> {
        let name = |text: &str| {
            bb_core::technical::TechnicalName::new(text).map_err(|_| FakeError::Failed)
        };
        Ok(bb_core::secret::StoredSecret {
            name: name(&format!("privacy.removal.{person}"))?,
            key_id: name("fake")?,
            wrapped_key: Vec::new(),
            wrapped_nonce: Vec::new(),
            nonce: Vec::new(),
            ciphertext: serde_json::to_vec(words).map_err(|_| FakeError::Failed)?,
            updated_at: at,
        })
    }

    async fn removal_words(
        &self,
        person: UserId,
    ) -> Result<Option<crate::privacy::removal::PersonWords>, FakeError> {
        self.read(|state| {
            state
                .pending_removals
                .iter()
                .find(|(pending, _)| pending.person == person)
                .map(|(_, secret)| {
                    serde_json::from_slice(&secret.ciphertext).map_err(|_| FakeError::Failed)
                })
                .transpose()
        })
    }

    async fn stopped_removals(&self) -> Result<Vec<bb_core::removal::PendingRemoval>, FakeError> {
        Ok(self.read(|state| {
            state
                .pending_removals
                .iter()
                .filter(|(pending, _)| {
                    !state.jobs.iter().any(|job| {
                        job.id == pending.job && job.state != bb_core::job::JobState::Dead
                    })
                })
                .map(|(pending, _)| pending.clone())
                .collect()
        }))
    }

    async fn pending_removal_in(
        tx: &mut Tx,
        person: UserId,
    ) -> Result<Option<bb_core::removal::PendingRemoval>, FakeError> {
        Ok(tx
            .state
            .pending_removals
            .iter()
            .find(|(pending, _)| pending.person == person)
            .map(|(pending, _)| pending.clone()))
    }

    async fn start_removal(
        tx: &mut Tx,
        pending: &bb_core::removal::PendingRemoval,
        words: &bb_core::secret::StoredSecret,
    ) -> Result<(), FakeError> {
        if tx
            .state
            .pending_removals
            .iter()
            .any(|(known, _)| known.person == pending.person)
        {
            return Err(FakeError::Duplicate);
        }
        tx.state
            .pending_removals
            .push((pending.clone(), words.clone()));
        Ok(())
    }

    async fn move_removal(
        tx: &mut Tx,
        person: UserId,
        from: bb_core::id::JobId,
        to: bb_core::id::JobId,
        step: &str,
        at: jiff::Timestamp,
    ) -> Result<bool, FakeError> {
        let Some((pending, _)) = tx
            .state
            .pending_removals
            .iter_mut()
            .find(|(pending, _)| pending.person == person && pending.job == from)
        else {
            return Ok(false);
        };
        pending.job = to;
        step.clone_into(&mut pending.step);
        pending.updated_at = at;
        Ok(true)
    }

    async fn end_removal(
        tx: &mut Tx,
        person: UserId,
        job: bb_core::id::JobId,
    ) -> Result<bool, FakeError> {
        let before = tx.state.pending_removals.len();
        tx.state
            .pending_removals
            .retain(|(pending, _)| !(pending.person == person && pending.job == job));
        Ok(tx.state.pending_removals.len() < before)
    }

    async fn hide_comments_of(
        tx: &mut Tx,
        author: UserId,
        at: jiff::Timestamp,
    ) -> Result<u64, FakeError> {
        let mut hidden = 0;
        for comment in &mut tx.state.comments {
            if comment.author == author && comment.deleted_at.is_none() {
                comment.deleted_at = Some(at);
                hidden += 1;
            }
        }
        Ok(hidden)
    }

    // One step a table: the in-memory store keeps no changes to read; what a step reads and writes
    // is tested on both databases (`bb-store`, `person_data::removal`).
    async fn scrub_step(
        &self,
        scrub: &crate::privacy::removal::Scrub,
        from: &crate::privacy::removal::ScrubPlace,
    ) -> Result<Option<crate::privacy::removal::ScrubPlace>, FakeError> {
        self.lock()
            .state
            .word_scrubs
            .push((scrub.clone(), from.clone()));
        Ok(from
            .table
            .next()
            .map(|table| crate::privacy::removal::ScrubPlace {
                table,
                after: Vec::new(),
            }))
    }
}
