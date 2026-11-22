//! How much one person may ask of the assistant (SEC-14): at most `QUESTIONS_A_MINUTE` questions in
//! a minute and `QUESTIONS_AT_ONCE` running at the same time. Every question calls the provider,
//! which charges for each call and may take long, so a person, or a script with their session,
//! cannot make the server call it without end. The rounds of one question have their ceiling in
//! `bb_core::ai::ROUNDS_MOST`.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use bb_core::ai::{QUESTIONS_A_MINUTE, QUESTIONS_AT_ONCE};
use bb_core::id::UserId;
use jiff::Timestamp;

use crate::auth::AppError;
use crate::rate_window::{Overflow, Window};

/// How long a person waits for a question when two of theirs run already: the time of a short
/// answer.
const BUSY_WAIT_SECONDS: u64 = 5;

/// The questions of the last minute and the questions running now, by person.
pub(crate) struct AssistantRate {
    questions: Window<UserId, Timestamp>,
    running: Mutex<HashMap<UserId, usize>>,
}

impl Default for AssistantRate {
    fn default() -> Self {
        // A full table refuses a person it does not know yet: a flood of sessions cannot use up the
        // memory.
        Self {
            questions: Window::new(QUESTIONS_A_MINUTE, Overflow::Refuse),
            running: Mutex::new(HashMap::new()),
        }
    }
}

/// A question that runs; it stops counting as running when it is dropped (answered, refused, or
/// given up by the client).
pub(crate) struct Running<'a> {
    rate: &'a AssistantRate,
    person: UserId,
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        let mut running = self.rate.lock();
        if let Some(count) = running.get_mut(&self.person) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                running.remove(&self.person);
            }
        }
    }
}

impl AssistantRate {
    fn lock(&self) -> MutexGuard<'_, HashMap<UserId, usize>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts a question of `person`: it counts against their minute and runs until the
    /// [`Running`] is dropped.
    ///
    /// # Errors
    /// [`AppError::Limited`] with the seconds to wait, when the person asked too much in this
    /// minute or two of their questions run already.
    pub(crate) fn start(&self, person: UserId, now: Timestamp) -> Result<Running<'_>, AppError> {
        let mut running = self.lock();
        if running
            .get(&person)
            .is_some_and(|count| *count >= QUESTIONS_AT_ONCE)
        {
            return Err(AppError::Limited {
                retry_after: BUSY_WAIT_SECONDS,
            });
        }
        self.questions
            .hit(person, now)
            .map_err(|retry_after| AppError::Limited { retry_after })?;
        // Only a question that was let through is written down: a refused one leaves no entry.
        *running.entry(person).or_insert(0) += 1;
        Ok(Running { rate: self, person })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_14_at_most_two_questions_of_a_person_run_at_once() {
        let rate = AssistantRate::default();
        let (ada, bob) = (UserId::from_random([1; 16]), UserId::from_random([2; 16]));
        let now = Timestamp::UNIX_EPOCH;
        let first = rate.start(ada, now).unwrap();
        let _second = rate.start(ada, now).unwrap();
        assert!(matches!(
            rate.start(ada, now),
            Err(AppError::Limited { .. })
        ));
        // Another person is not held up, and an answered question makes room.
        let _bobs = rate.start(bob, now).unwrap();
        drop(first);
        rate.start(ada, now).unwrap();
    }

    #[test]
    fn sec_14_a_question_refused_for_the_minute_leaves_nothing_behind() {
        let rate = AssistantRate::default();
        let carl = UserId::from_random([3; 16]);
        let now = Timestamp::UNIX_EPOCH;
        for _ in 0..QUESTIONS_A_MINUTE {
            drop(rate.start(carl, now).unwrap());
        }
        assert!(matches!(
            rate.start(carl, now),
            Err(AppError::Limited { .. })
        ));
        assert!(rate.lock().is_empty());
    }
}
