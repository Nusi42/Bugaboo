//! The days without work of the people of the timeline (ADR-0030, decision 3 and amendment M8):
//! what a day of a bar holds, the one rule for the hatched days of a bar and of a row, and the away
//! days and holidays of each person's row under *Group by person* (M8-F90).

use std::collections::HashSet;

use bb_core::id::{ProjectId, UserId};
use bb_core::plan::Days;
use bb_core::work_times::DayMinutes;
use bb_policy::PersonRef;

use super::types::{Hatched, HatchedDay, PersonDaysView, TimelinePart, TimelineRow};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::search::{FieldTarget, ShownKey};
use crate::work_times::{PLAN_PEOPLE_MAX, WorkMinutesQuery};

/// What keeps a person from working on a day, as the reader may know it: away (only ever *Away*), a
/// public holiday, and, when `days_off` is asked for, a day that is not a usual work day. `named`
/// says whether the reader sees who the person is: without it a day carries no word, so no holiday
/// name, no half of a day and no day off is sent.
pub(crate) fn hatched_of(day: &DayMinutes, days_off: bool, named: bool) -> Option<HatchedDay> {
    let what = if day.away.count() > 0 {
        if named {
            Hatched::away(day.away)
        } else {
            Hatched::Away {
                morning: false,
                afternoon: false,
            }
        }
    } else if let Some(holiday) = day.holiday {
        Hatched::Holiday {
            holiday: named.then_some(holiday),
        }
    } else if days_off && named && day.minutes == 0 {
        Hatched::DayOff
    } else {
        return None;
    };
    Some(HatchedDay {
        date: day.date,
        what,
    })
}

/// The people of the rows of *Group by person* whom the reader sees by name, each once.
fn named_people(rows: &[TimelineRow]) -> Vec<(UserId, PersonRef)> {
    let mut seen = HashSet::new();
    rows.iter()
        .filter_map(|row| match &row.group {
            Some(ShownKey::Person(person @ PersonRef::Name { id, .. })) => {
                seen.insert(*id).then(|| (*id, person.clone()))
            }
            _ => None,
        })
        .collect()
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Under *Group by person* only (M8-F90), on the part of the rows: the away days and public
    /// holidays over `span` of each person of a row
    /// whose work times the reader may know (the one rule of `whose.rs`), through the one port of
    /// work times: a fixed number of reads for the page. Days off are not hatched in a row: the
    /// weekends of the chart already stand behind it.
    pub(crate) async fn people_days(
        &self,
        actor: &Actor,
        rows: &[TimelineRow],
        (group_by, part): (Option<FieldTarget>, TimelinePart),
        project: Option<ProjectId>,
        span: Days,
    ) -> Result<Vec<PersonDaysView>, AppError> {
        if group_by != Some(FieldTarget::AssignedTo) || part != TimelinePart::Rows {
            return Ok(Vec::new());
        }
        let people = named_people(rows);
        let (Some(project), false) = (project, people.is_empty()) else {
            return Ok(Vec::new());
        };
        let ids: Vec<UserId> = people.iter().map(|(id, _)| *id).collect();
        let known = self.work_times_known(actor, &ids).await?;
        let mut found = Vec::new();
        for chunk in ids.chunks(PLAN_PEOPLE_MAX) {
            let minutes = self
                .work_minutes(
                    &known,
                    WorkMinutesQuery {
                        people: chunk.to_vec(),
                        projects: vec![project],
                        first: span.first,
                        last: span.last,
                    },
                )
                .await?;
            for each in minutes.people.into_iter().filter(|each| each.away_visible) {
                let Some((_, person)) = people.iter().find(|(id, _)| *id == each.person) else {
                    continue;
                };
                found.push(PersonDaysView {
                    person: person.clone(),
                    hatched: each
                        .days
                        .iter()
                        .filter_map(|day| hatched_of(day, false, true))
                        .collect(),
                });
            }
        }
        Ok(found)
    }
}
