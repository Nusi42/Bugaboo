//! The check of the large dataset (architecture §11, *Large dataset*): it reads the database back
//! and holds it against the rules of the sample that apply to the large dataset, and makes a fingerprint
//! that two runs with one seed must share.
//!
//! - **Counters:** each project that counts its issue IDs has a counter equal to the number of
//!   its issues, and every counting code is a number from 1 up to it.
//! - **Versions:** the version of an issue is one more than the number of its events that count a
//!   version up.
//! - **History:** every event about an issue has its row in the history of the issue.
//! - **What people are told:** a Guest is told only about issues they reported; neither a Guest
//!   nor a Contributor is told the value of *Cost center*. The rule counts the notifications it
//!   read and fails when it read none for a kind of person, so it cannot pass without looking.
//!
//! Every issue is read when there are few; with many, every n-th, so the check stays a matter of
//! minutes.

use std::collections::BTreeMap;
use std::hash::{Hash as _, Hasher as _};

use bb_app::access::StartingRole;
use bb_app::comments::CommentStore as _;
use bb_app::history::HistoryStore as _;
use bb_app::notifications::NotificationStore as _;
use bb_app::tags::TagStore as _;
use bb_app::workflows::{StatusFilter, WorkflowStore as _};
use bb_core::event::{EntityRef, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::history::HistoryKind;
use bb_core::id::{CommentId, FieldId, ProjectId, UserId};
use bb_core::notification::NotificationFilter;
use bb_core::readable_id::IdMode;

use crate::error::SampleError;
use crate::generate::Sample;

/// How many issues one read of the issues takes.
const PAGE: usize = 1000;
/// The most issues whose events and history are held against each other.
const SAMPLED_AT_MOST: usize = 20_000;
/// The most notifications of one person that are read.
const NOTIFICATIONS_AT_MOST: u64 = 200;
/// The most events and history rows read for one issue.
const ROWS_AT_MOST: u64 = 500;

/// What the check needs to know of the dataset that the database cannot say.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    /// The Guests and the Contributors, who may not be told of some things.
    pub(crate) guests: Vec<UserId>,
    pub(crate) contributors: Vec<UserId>,
    /// The own field *Cost center*.
    pub(crate) cost_center: Option<FieldId>,
}

impl Facts {
    pub(crate) fn people(folk: &[crate::large_org::Folk]) -> (Vec<UserId>, Vec<UserId>) {
        let of = |role| {
            folk.iter()
                .filter(|each| each.role == role)
                .map(|each| each.id)
                .collect()
        };
        (of(StartingRole::Guest), of(StartingRole::Contributor))
    }
}

/// What the check found.
#[derive(Clone, Debug)]
pub struct Report {
    /// Every rule that does not hold, in a sentence.
    pub problems: Vec<String>,
    /// The same for two runs of one seed.
    pub fingerprint: u64,
    /// How many issues there are, and how many were held against their events.
    pub issues: usize,
    /// How many were held against their events.
    pub sampled: usize,
}

impl Sample {
    /// Holds the large dataset against the rules that apply to the large dataset.
    ///
    /// # Errors
    /// [`SampleError`] when the database cannot be read.
    pub async fn check_large(&self, facts: &Facts) -> Result<Report, SampleError> {
        let storage = &self.storage;
        let database = storage.database();
        let mut problems = Vec::new();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let mut issues = Vec::new();
        let mut after = None;
        loop {
            let page = storage
                .live_issues_after(None, None, StatusFilter::Any, after, PAGE)
                .await?;
            after = page.last().map(|issue| issue.id);
            let done = page.len() < PAGE;
            issues.extend(page);
            if done {
                break;
            }
        }
        let mut counted: BTreeMap<ProjectId, (i64, i64)> = BTreeMap::new();
        for issue in &issues {
            (&issue.title, &issue.description, issue.status.as_str()).hash(&mut hasher);
            (issue.priority.code(), issue.code.as_str(), issue.version).hash(&mut hasher);
            (
                issue.created_at.as_microsecond(),
                issue.closed_at.map(jiff::Timestamp::as_microsecond),
            )
                .hash(&mut hasher);
            if issue.code.mode() == IdMode::Counting {
                let number: i64 = issue.code.as_str().parse().unwrap_or(0);
                let entry = counted.entry(issue.project).or_insert((0, 0));
                entry.0 += 1;
                entry.1 = entry.1.max(number);
            }
        }
        for (project, (count, highest)) in &counted {
            let stored = database.issue_counter(*project).await?.unwrap_or(0);
            if stored != *count || *highest != *count {
                problems.push(format!(
                    "the project {project} has {count} issues with counting IDs, the highest is {highest} and the counter says {stored}"
                ));
            }
        }
        let step = issues.len().div_ceil(SAMPLED_AT_MOST).max(1);
        let mut sampled = 0;
        for issue in issues.iter().step_by(step) {
            sampled += 1;
            let about = EntityRef {
                kind: RecordKind::Issue.into(),
                id: issue.id.into(),
            };
            let events = database.events_about(&about, ROWS_AT_MOST).await?;
            let counted_up = events
                .iter()
                .filter(|event| event.counted_version())
                .count();
            if i64::try_from(counted_up).unwrap_or(-1) + 1 != issue.version {
                problems.push(format!(
                    "the issue {} has the version {} but {counted_up} events that count a version up",
                    issue.id, issue.version
                ));
            }
            let rows = storage
                .history_page(issue.id, None, None, ROWS_AT_MOST)
                .await?;
            for event in &events {
                if rows.iter().all(|row| row.event != event.id) {
                    problems.push(format!(
                        "the history of the issue {} has no row for the event {}",
                        issue.id, event.id
                    ));
                }
            }
            self.hash_around(issue.id, &mut hasher).await?;
        }
        self.check_told(facts, &issues, &mut problems).await?;
        Ok(Report {
            problems,
            fingerprint: hasher.finish(),
            issues: issues.len(),
            sampled,
        })
    }

    /// Puts what hangs on a sampled issue into the fingerprint: its comments (how many and their
    /// texts) and the tags and labels on it, which the columns of the issue do not show.
    async fn hash_around(
        &self,
        issue: bb_core::id::IssueId,
        hasher: &mut impl std::hash::Hasher,
    ) -> Result<(), SampleError> {
        let storage = &self.storage;
        let written = storage
            .history_page(issue, Some(HistoryKind::Comment), None, ROWS_AT_MOST)
            .await?;
        let ids: Vec<CommentId> = written
            .iter()
            .filter_map(|row| CommentId::from_bytes(row.record.to_bytes()).ok())
            .collect();
        let mut texts: Vec<String> = storage
            .comments_by_ids(&ids)
            .await?
            .into_iter()
            .map(|comment| comment.text)
            .collect();
        texts.sort();
        (written.len(), texts).hash(hasher);
        let tags: Vec<_> = storage
            .issue_tags(issue)
            .await?
            .iter()
            .map(|each| each.tag)
            .collect();
        let labels: Vec<_> = storage
            .issue_labels(issue)
            .await?
            .iter()
            .map(|each| each.label)
            .collect();
        (tags, labels).hash(hasher);
        Ok(())
    }

    async fn check_told(
        &self,
        facts: &Facts,
        issues: &[bb_core::issue::Issue],
        problems: &mut Vec<String>,
    ) -> Result<(), SampleError> {
        let reporters: BTreeMap<_, _> = issues
            .iter()
            .map(|issue| (issue.id, issue.reported_by))
            .collect();
        let hidden = facts
            .cost_center
            .map(|field| FieldKey::Own(field).code().into_owned());
        let storage = &self.storage;
        for (people, only_own) in [(&facts.guests, true), (&facts.contributors, false)] {
            let mut read = 0_usize;
            for person in people {
                let told = storage
                    .notifications_of(
                        *person,
                        NotificationFilter::All,
                        None,
                        NOTIFICATIONS_AT_MOST,
                    )
                    .await?;
                read += told.len();
                for each in told {
                    let own = each
                        .issue
                        .is_none_or(|issue| reporters.get(&issue) == Some(person));
                    if only_own && !own {
                        problems.push(format!(
                            "a Guest was told of the issue {:?}, which they did not report",
                            each.issue
                        ));
                    }
                    if hidden
                        .as_ref()
                        .is_some_and(|key| each.changes.iter().any(|change| &change.field == key))
                    {
                        problems.push("a notification tells the value of Cost center to somebody who may not see it".to_owned());
                    }
                }
            }
            if !people.is_empty() && read == 0 {
                problems.push(format!(
                    "no notification of the {} was read, so the rules of what they are told were not checked",
                    if only_own { "Guests" } else { "Contributors" }
                ));
            }
        }
        Ok(())
    }
}
