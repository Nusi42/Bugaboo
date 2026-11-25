//! Releases of a project (ISS-08, ADR-0049): a planned version of the product, made in one
//! project and offered in it and its sub-projects. Not a release of Bugaboo itself, which
//! [`crate::release`] is about.
//!
//! The record, its three states, the one rule of where a release is offered and the limits are
//! defined here once; the use cases, the store, the search and the API read them from here.

use jiff::Timestamp;
use jiff::civil::Date;

use crate::id::{IssueId, ProjectId, ReleaseId, UserId};
use crate::personal::Entity;

/// The most releases one issue names in *Found in release* (ADR-0049, decision 3): a limit of its
/// own, so a row of them stays a row.
pub const FOUND_IN_RELEASES_MAX: usize = 20;

words! {
    /// The state of a release (ADR-0049, decision 2): one of three, defined once.
    pub enum ReleaseState {
        /// The state when it is made.
        Planned => "planned",
        /// It is out; the day it was marked is kept as *Released on*.
        Released => "released",
        /// No longer offered for a new choice, still shown on the issues that name it.
        Retired => "retired",
    }
    messages name = "release-state."
}

words_in_api!(ReleaseState, "The state of a release.");

impl ReleaseState {
    /// The day kept as *Released on* once a release is in this state, given the day it kept and
    /// the day of the change: every move into *Released* keeps a day it has (a release retired
    /// after it was out and marked *Released* again was out on its first day) and else takes
    /// today, *Planned* clears it, *Retired* keeps what there is.
    #[must_use]
    pub fn released_on(self, kept: Option<Date>, today: Date) -> Option<Date> {
        match self {
            Self::Released => kept.or(Some(today)),
            Self::Planned => None,
            Self::Retired => kept,
        }
    }

    /// Whether a release in this state may be chosen anew where it is offered: not when retired.
    #[must_use]
    pub fn takes_new_choices(self) -> bool {
        self != Self::Retired
    }
}

/// A release of a project (ADR-0049, decision 2).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Release {
    /// The ID of the release.
    #[not_personal]
    pub id: ReleaseId,
    /// The project it was made in: offered there and in its sub-projects.
    #[not_personal]
    pub project: ProjectId,
    /// What it is called (*2.4*), unique within its project by the one rule of names. Free text
    /// that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// *Release date*: the planned day, a calendar date with no time of day; none until somebody
    /// gives one.
    #[not_personal]
    pub release_date: Option<Date>,
    /// Its state.
    #[not_personal]
    pub state: ReleaseState,
    /// *Released on*: the day it was marked *Released*, kept while it is released or retired.
    #[not_personal]
    pub released_on: Option<Date>,
    /// Who made it.
    #[personal(category = "activity", basis = "contract")]
    #[person(author)]
    pub made_by: UserId,
    /// When it was made.
    #[not_personal]
    pub made_at: Timestamp,
    /// Who changed it last.
    #[personal(category = "activity", basis = "contract")]
    #[person(actor)]
    pub changed_by: UserId,
    /// When it was changed last.
    #[not_personal]
    pub changed_at: Timestamp,
    /// When it was deleted into the trash, while it is there.
    #[not_personal]
    pub deleted_at: Option<Timestamp>,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

impl Release {
    /// Whether a change of the release to `next` needs a reason, which the history shows: it goes
    /// back to *Planned* after it was out, from *Released* or from *Retired* (ADR-0049, decision 2).
    #[must_use]
    pub fn needs_reason(&self, next: ReleaseState) -> bool {
        next == ReleaseState::Planned && self.state != next && self.released_on.is_some()
    }

    /// Whether the release is offered where an issue is, given the projects from the top down to
    /// the issue's project: it was made in one of them. The one rule of where a release is
    /// offered (ADR-0049, decision 1), for forms, the API, many changes and the tools.
    #[must_use]
    pub fn offered_in(&self, chain: &[ProjectId]) -> bool {
        self.deleted_at.is_none() && chain.contains(&self.project)
    }

    /// Whether the release may be chosen anew where an issue is: offered there and not retired.
    #[must_use]
    pub fn choosable_in(&self, chain: &[ProjectId]) -> bool {
        self.offered_in(chain) && self.state.takes_new_choices()
    }
}

/// One release an issue names in *Found in release*, at its place in the order given.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct IssueFoundIn {
    /// The issue.
    #[not_personal]
    pub issue: IssueId,
    /// The release.
    #[not_personal]
    pub release: ReleaseId,
    /// The place among the releases of the issue, counted from 0.
    #[not_personal]
    pub position: i64,
}

/// The rows of *Found in release* of an issue, in the order given.
#[must_use]
pub fn found_in_rows(issue: IssueId, releases: &[ReleaseId]) -> Vec<IssueFoundIn> {
    releases
        .iter()
        .zip(0_i64..)
        .map(|(release, position)| IssueFoundIn {
            issue,
            release: *release,
            position,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;

    fn day(text: &str) -> Date {
        text.parse().unwrap()
    }

    fn a_release(state: ReleaseState, released_on: Option<Date>) -> Release {
        let mut ids = SequentialIds::default();
        let person = UserId::generate(&mut ids);
        Release {
            id: ReleaseId::generate(&mut ids).unwrap(),
            project: ProjectId::generate(&mut ids).unwrap(),
            name: "2.4".to_owned(),
            release_date: None,
            state,
            released_on,
            made_by: person,
            made_at: Timestamp::UNIX_EPOCH,
            changed_by: person,
            changed_at: Timestamp::UNIX_EPOCH,
            deleted_at: None,
            version: 1,
        }
    }

    #[test]
    fn iss_08_a_release_moves_planned_released_retired_and_keeps_the_day_released() {
        let today = day("2026-11-24");
        let earlier = day("2026-11-02");
        let (planned, released, retired) = (
            ReleaseState::Planned,
            ReleaseState::Released,
            ReleaseState::Retired,
        );
        assert_eq!(released.released_on(None, today), Some(today));
        // Every move into Released keeps the day it was first out: also from Retired.
        assert_eq!(released.released_on(Some(earlier), today), Some(earlier));
        assert_eq!(retired.released_on(Some(earlier), today), Some(earlier));
        assert_eq!(retired.released_on(None, today), None);
        assert_eq!(planned.released_on(Some(earlier), today), None);
        assert!(!retired.takes_new_choices());
        assert!(planned.takes_new_choices() && released.takes_new_choices());
    }

    #[test]
    fn iss_08_back_to_planned_after_it_was_out_needs_a_reason_also_from_retired() {
        let earlier = Some(day("2026-11-02"));
        let planned = ReleaseState::Planned;
        assert!(a_release(ReleaseState::Released, earlier).needs_reason(planned));
        assert!(a_release(ReleaseState::Retired, earlier).needs_reason(planned));
        // Retired before it was ever out, or staying planned: no reason.
        assert!(!a_release(ReleaseState::Retired, None).needs_reason(planned));
        assert!(!a_release(planned, None).needs_reason(planned));
        assert!(!a_release(planned, None).needs_reason(ReleaseState::Released));
    }

    #[test]
    fn iss_08_a_release_is_offered_in_its_project_and_sub_projects_only() {
        let mut ids = SequentialIds::default();
        let (top, below, beside) = (
            ProjectId::generate(&mut ids).unwrap(),
            ProjectId::generate(&mut ids).unwrap(),
            ProjectId::generate(&mut ids).unwrap(),
        );
        let person = UserId::generate(&mut ids);
        let at = Timestamp::UNIX_EPOCH;
        let mut release = Release {
            id: ReleaseId::generate(&mut ids).unwrap(),
            project: top,
            name: "2.4".to_owned(),
            release_date: None,
            state: ReleaseState::Planned,
            released_on: None,
            made_by: person,
            made_at: at,
            changed_by: person,
            changed_at: at,
            deleted_at: None,
            version: 1,
        };
        assert!(release.offered_in(&[top]));
        assert!(release.offered_in(&[top, below]));
        assert!(!release.offered_in(&[beside]));
        release.state = ReleaseState::Retired;
        assert!(release.offered_in(&[top, below]));
        assert!(!release.choosable_in(&[top, below]));
        release.state = ReleaseState::Planned;
        release.deleted_at = Some(at);
        assert!(!release.offered_in(&[top]));
    }

    #[test]
    fn iss_08_found_in_rows_keep_the_order_given() {
        let mut ids = SequentialIds::default();
        let issue = IssueId::generate(&mut ids).unwrap();
        let (one, two) = (
            ReleaseId::generate(&mut ids).unwrap(),
            ReleaseId::generate(&mut ids).unwrap(),
        );
        let rows = found_in_rows(issue, &[two, one]);
        assert_eq!(
            rows.iter()
                .map(|row| (row.release, row.position))
                .collect::<Vec<_>>(),
            [(two, 0), (one, 1)]
        );
    }
}
