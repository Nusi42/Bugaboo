//! The port of the plan (PLAN-12, PLAN-15, ADR-0030, decision 3): for a set of people and a span of
//! days, the minutes each person can work on each day in a project, and the away days and public
//! holidays by name. The plan (WP-06) reads work times only through [`Ctx::work_minutes`], so the
//! sum of minutes is worked out once, in `bb_core::work_times::minutes_on`: usual work days times
//! work hours times the share, minus away times and public holidays.
//!
//! A share counts as nothing while the person has no role in the project. The answer holds no note
//! and nobody who entered an away time: only the days, which the reader sees as *Away*, and the
//! public holidays by their words. Whether the reader may name those days is the field
//! `away_visible`; a reader who may not still gets the minutes (they shape the bar), and no away
//! half and no holiday.

use std::collections::{BTreeMap, HashMap};

use bb_core::access::PeopleLevel;
use bb_core::holidays::Holiday;
use bb_core::id::{ProjectId, UserId};
use bb_core::work_times::record::AwayTime;
use bb_core::work_times::{DayMinutes, Halves, Share, UsualDays, minutes_on};
use bb_policy::{Evaluation, Place};
use jiff::civil::Date;

use super::PLAN_PEOPLE_MAX;
use super::read::checked_range;
use super::whose::work_times_level;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use bb_core::ErrorCode;

/// The people and the days to work the minutes out for.
pub struct WorkMinutesQuery {
    /// The people, at most [`PLAN_PEOPLE_MAX`].
    pub people: Vec<UserId>,
    /// The project the work is in: the share is the share there.
    pub project: ProjectId,
    /// The first day.
    pub first: Date,
    /// The last day: a stretch of at most `RANGE_DAYS_MAX` days.
    pub last: Date,
}

/// The days of one person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonMinutes {
    /// The person.
    pub person: UserId,
    /// The share that counts in the project: nothing while the person has no role there.
    pub share: Share,
    /// Whether the reader may name the person's away days (the kind *Work times* is not *Hidden*
    /// for them).
    pub away_visible: bool,
    /// Every day from the first to the last, in order.
    pub days: Vec<DayMinutes>,
}

/// The minutes of the people asked for, in the order they were asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkMinutes {
    /// One entry for each person asked for.
    pub people: Vec<PersonMinutes>,
}

/// The dates from `first` to `last`, in order.
fn dates(first: Date, last: Date) -> Vec<Date> {
    let mut found = Vec::new();
    let mut date = first;
    while date <= last {
        found.push(date);
        let Ok(next) = date.tomorrow() else {
            break;
        };
        date = next;
    }
    found
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The minutes each of these people can work on each day from `first` to `last` in a project,
    /// for a reader whose rights in the project are `reader`. Reads the work times of everybody in
    /// a fixed number of statements, whatever the number of people.
    ///
    /// # Errors
    /// `request.invalid` for a stretch that is backwards or longer than `RANGE_DAYS_MAX` days, or
    /// for more than [`PLAN_PEOPLE_MAX`] people.
    pub async fn work_minutes(
        &self,
        reader: &Evaluation,
        query: WorkMinutesQuery,
    ) -> Result<WorkMinutes, AppError> {
        checked_range(query.first, query.last)?;
        if query.people.len() > PLAN_PEOPLE_MAX {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let people = query.people;
        let usual: HashMap<UserId, UsualDays> = self
            .store()
            .usual_days_of(people.clone())
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .collect();
        let shares: HashMap<UserId, Share> = self
            .store()
            .shares_in(query.project, people.clone())
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .collect();
        let mut away: HashMap<UserId, Vec<AwayTime>> = HashMap::new();
        for time in self
            .store()
            .away_times_of(people.clone(), query.first, query.last, None)
            .await
            .map_err(|error| internal::<S>(&error))?
        {
            away.entry(time.person).or_default().push(time);
        }
        let (_, layers) = self.setting_layers_of_people(&people, None).await?;
        let pairs: Vec<(UserId, ProjectId)> = people
            .iter()
            .map(|person| (*person, query.project))
            .collect();
        self.evaluate_people_in(&pairs).await?;
        let away_visible = work_times_level(reader) != PeopleLevel::Hidden;
        let days = dates(query.first, query.last);
        let mut holidays_by_region: HashMap<_, BTreeMap<Date, Holiday>> = HashMap::new();
        let mut found = Vec::with_capacity(people.len());
        for person in people {
            let region = layers
                .get(&person)
                .map_or(bb_core::holidays::Region::Germany, |layers| {
                    bb_core::holidays::Region::chosen(layers)
                });
            let holidays = holidays_by_region.entry(region).or_insert_with(|| {
                region
                    .holidays_between(query.first, query.last)
                    .into_iter()
                    .map(|day| (day.date, day.holiday))
                    .collect()
            });
            let has_role = self
                .has_a_role_at(person, Place::Project(query.project))
                .await?;
            let share = if has_role {
                shares.get(&person).copied().unwrap_or(Share::START)
            } else {
                // Kept, and counting again when a role comes back (ADR-0030, amendment M8).
                Share::NOTHING
            };
            let usual = usual.get(&person).copied().unwrap_or_default();
            let times = away.get(&person).map_or(&[][..], Vec::as_slice);
            found.push(PersonMinutes {
                person,
                share,
                away_visible,
                days: days
                    .iter()
                    .map(|date| {
                        let holiday = holidays.get(date).copied();
                        let away = times
                            .iter()
                            .fold(Halves::NONE, |halves, time| halves.and(time.away_on(*date)));
                        DayMinutes {
                            date: *date,
                            minutes: minutes_on(&usual, share, *date, times, holiday.is_some()),
                            // The minutes shape the bar; the days are named only to a reader who
                            // may know the person's work times.
                            away: if away_visible { away } else { Halves::NONE },
                            holiday: holiday.filter(|_| away_visible),
                        }
                    })
                    .collect(),
            });
        }
        Ok(WorkMinutes { people: found })
    }
}
