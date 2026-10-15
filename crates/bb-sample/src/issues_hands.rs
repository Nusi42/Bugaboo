//! The hands of the sample: the people who do the work of the issues, each signed in when they
//! need to be.

use std::collections::BTreeMap;

use bb_app::auth::{SignIn, sign_in};
use bb_app::context::Clock as _;

use crate::error::{SampleError, Step};
use crate::generate::World;
use crate::issues_time::Calendar;
use crate::organization::workers;

/// A session is renewed when it was idle this long (the server ends it after thirty minutes).
const IDLE_SECONDS: i64 = 25 * 60;
/// A session is renewed when it is this old (the server ends it after twenty-four hours).
const AGE_SECONDS: i64 = 20 * 3600;
/// The most people who work on a day, out of all the workers.
const PEOPLE_A_DAY: usize = 3;
/// A person works on a day only when there are at least this many changes for them to make, so
/// a sample with few changes does not sign in more people than it needs.
const CHANGES_EACH: usize = 8;

/// A session of a worker.
struct Session {
    token: String,
    signed_in: i64,
    used: i64,
}

/// The hands of the sample: the Team members who do the work, each signed in when they need to be.
pub(crate) struct Hands<'a, 'w> {
    world: &'a World<'w>,
    calendar: Calendar,
    /// The places in the plan of the people of the people who work.
    pub(crate) workers: Vec<usize>,
    /// How many people work on a day.
    a_day: usize,
    sessions: BTreeMap<usize, Session>,
}

impl<'a, 'w> Hands<'a, 'w> {
    pub(crate) fn new(
        world: &'a World<'w>,
        calendar: Calendar,
        changes: usize,
    ) -> Result<Self, SampleError> {
        let workers = workers(world)?;
        let calendar_days =
            usize::try_from(calendar.length() / crate::issues_time::WORK_DAY).unwrap_or(1);
        if workers.len() < PEOPLE_A_DAY {
            return Err(SampleError::PersonMissing);
        }
        Ok(Self {
            world,
            calendar,
            workers,
            a_day: (changes / calendar_days.max(1) / CHANGES_EACH).clamp(1, PEOPLE_A_DAY),
            sessions: BTreeMap::new(),
        })
    }

    /// The person (a place in the plan of the people) a pick stands for, on the day of a working second: one
    /// of the few who work that day.
    pub(crate) fn person(&self, working: i64, pick: u32) -> usize {
        let day = usize::try_from(self.calendar.day(working)).unwrap_or(0);
        let first = day * 5;
        let at = (first + pick as usize % self.a_day) % self.workers.len();
        self.workers[at]
    }

    /// Anyone of all the workers, by a pick (for who is assigned or mentioned).
    pub(crate) fn anyone(&self, pick: u32) -> usize {
        self.workers[pick as usize % self.workers.len()]
    }

    /// The secret of the session of a person, signed in again when the session ran out.
    pub(crate) async fn token(&mut self, person: usize) -> Result<String, SampleError> {
        let now = self.world.clock.now().as_second();
        let previous = match self.sessions.get(&person) {
            Some(session)
                if now - session.used < IDLE_SECONDS && now - session.signed_in < AGE_SECONDS =>
            {
                let token = session.token.clone();
                if let Some(session) = self.sessions.get_mut(&person) {
                    session.used = now;
                }
                return Ok(token);
            }
            Some(session) => Some(session.token.clone()),
            None => None,
        };
        let made = self
            .world
            .people
            .get(person)
            .ok_or(SampleError::PersonMissing)?;
        let started = sign_in(
            &self.world.services.ctx(self.world.request()),
            SignIn {
                email: made.person.email.clone(),
                password: made.password.clone(),
                previous_session: previous,
            },
        )
        .await
        .map_err(|error| SampleError::step(Step::SigningIn, &error))?;
        self.sessions.insert(
            person,
            Session {
                token: started.session_token.clone(),
                signed_in: now,
                used: now,
            },
        );
        Ok(started.session_token)
    }
}
