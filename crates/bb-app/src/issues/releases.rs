//! The two release fields of an issue (ISS-08, ADR-0049, decision 3): what a change or a new issue
//! sets of *Found in release* and *Planned for release*, judged by the one rule of a choice
//! (`releases::choose`), how the event writes them, and how a view shows them.

use std::collections::{HashMap, HashSet};

use bb_core::event::FieldChange;
use bb_core::id::{IssueId, ProjectId, ReleaseId};
use bb_core::issue::Issue;
use bb_core::project_release::{IssueFoundIn, Release, found_in_rows};
use serde_json::{Value, json};

use super::fields::member;
use super::types::{AskedBy, ChangeIssue, CreateIssue};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::{Shown, Why};
use crate::releases::{Naming, ReleaseChoice, ReleaseRef, ensure_still_choosable_in};

/// A release as an event of an issue keeps it: its ID, its name then and its project, so the
/// history shows the name it had and only to a reader who sees the project (ADR-0049, decision 7).
pub(crate) fn release_json(release: &Release) -> Value {
    json!({
        "id": release.id.to_string(),
        "name": release.name,
        "project": release.project.to_string(),
    })
}

/// The releases a value of either release member of an event holds (an array, or one release)
/// whose project `seen` allows: the one reading of such a value, for the history and for what a
/// tool shows of a change.
pub(crate) fn releases_seen_in(value: &Value, seen: impl Fn(ProjectId) -> bool) -> Vec<Value> {
    let items: Vec<&Value> = match value {
        Value::Array(items) => items.iter().collect(),
        Value::Object(_) => vec![value],
        _ => Vec::new(),
    };
    items
        .into_iter()
        .filter(|item| {
            item.get("project")
                .and_then(Value::as_str)
                .and_then(|text| text.parse::<ProjectId>().ok())
                .is_some_and(&seen)
        })
        .cloned()
        .collect()
}

/// A change of either release member as a reader whose sight is `seen` may read it: the releases
/// they may not know left out of both sides, and no change at all when both sides read the same
/// then. Any other change as it is.
pub(crate) fn release_change_seen(
    change: FieldChange,
    seen: impl Fn(ProjectId) -> bool,
) -> Option<FieldChange> {
    let several = change.field == member::FOUND_IN_RELEASE;
    if !several && change.field != member::PLANNED_FOR_RELEASE {
        return Some(change);
    }
    let side = |value: Option<&Value>| {
        let found = value
            .map(|value| releases_seen_in(value, &seen))
            .unwrap_or_default();
        if several {
            Some(Value::Array(found))
        } else {
            found.into_iter().next()
        }
    };
    let (before, after) = (side(change.before.as_ref()), side(change.after.as_ref()));
    (before != after).then_some(FieldChange {
        field: change.field,
        before,
        after,
    })
}

/// The releases an issue names before and after a change, with the records of all of them.
#[derive(Clone, Default)]
pub(crate) struct IssueReleases {
    /// The projects from the top down to the issue's project.
    chain: Vec<ProjectId>,
    found_now: Vec<ReleaseId>,
    found_next: Vec<ReleaseId>,
    /// The records of every release named before or after.
    records: Vec<Release>,
    /// How the releases named newly are named.
    naming: Naming,
}

impl IssueReleases {
    fn json_of(&self, releases: &[ReleaseId]) -> Value {
        Value::Array(
            releases
                .iter()
                .filter_map(|id| self.records.iter().find(|each| each.id == *id))
                .map(release_json)
                .collect(),
        )
    }

    fn one_json(&self, release: Option<ReleaseId>) -> Value {
        release
            .and_then(|id| self.records.iter().find(|each| each.id == id))
            .map_or(Value::Null, release_json)
    }

    /// The changes of the two fields for the event, with the old and the new values.
    pub(crate) fn changes(&self, before: &Issue, after: &Issue) -> Vec<FieldChange> {
        let mut changes = Vec::new();
        if self.found_now != self.found_next {
            changes.push(FieldChange {
                field: member::FOUND_IN_RELEASE.to_owned(),
                before: Some(self.json_of(&self.found_now)),
                after: Some(self.json_of(&self.found_next)),
            });
        }
        if before.planned_for_release != after.planned_for_release {
            changes.push(FieldChange {
                field: member::PLANNED_FOR_RELEASE.to_owned(),
                before: Some(self.one_json(before.planned_for_release)).filter(|v| !v.is_null()),
                after: Some(self.one_json(after.planned_for_release)).filter(|v| !v.is_null()),
            });
        }
        changes
    }

    /// The changes of a new issue: the fields it starts with.
    pub(crate) fn created(&self, issue: &Issue) -> Vec<FieldChange> {
        let blank = Issue {
            planned_for_release: None,
            ..issue.clone()
        };
        self.changes(&blank, issue)
    }

    /// What the change writes: the rows of *Found in release* when they changed, and the releases
    /// the issue names newly, which are judged again inside the change.
    pub(crate) fn into_write(
        self,
        issue: IssueId,
        planned: (Option<ReleaseId>, Option<ReleaseId>),
    ) -> IssueReleaseWrite {
        let (planned_now, planned_next) = planned;
        let mut new: Vec<ReleaseId> = self
            .found_next
            .iter()
            .copied()
            .filter(|release| !self.found_now.contains(release))
            .collect();
        if let Some(release) = planned_next.filter(|release| Some(*release) != planned_now)
            && !new.contains(&release)
        {
            new.push(release);
        }
        IssueReleaseWrite {
            found_rows: (self.found_now != self.found_next)
                .then(|| found_in_rows(issue, &self.found_next)),
            chain: self.chain,
            naming: self.naming,
            new,
        }
    }
}

/// What a change of an issue writes of its releases.
#[derive(Clone, Default)]
pub(crate) struct IssueReleaseWrite {
    chain: Vec<ProjectId>,
    /// How the releases named newly are named.
    naming: Naming,
    /// The releases the issue names newly.
    new: Vec<ReleaseId>,
    /// The rows of *Found in release*, when they changed.
    found_rows: Option<Vec<IssueFoundIn>>,
}

impl IssueReleaseWrite {
    /// Inside the change: the new releases are still offered and not retired or deleted, and the
    /// rows of *Found in release* are written.
    pub(crate) async fn write_in<S: AuthStore>(
        &self,
        transaction: &mut S::Transaction,
        issue: IssueId,
        stopped: &Why,
    ) -> Result<(), S::Error> {
        ensure_still_choosable_in::<S>(transaction, (&self.chain, self.naming), &self.new, stopped)
            .await?;
        match &self.found_rows {
            Some(rows) => S::replace_found_in(transaction, issue, rows).await,
            None => Ok(()),
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The releases the issue names after a change that sets `asked` (*Found in release*, *Planned
    /// for release*), judged by the one rule of a choice; `next` gets the release it is planned
    /// for.
    pub(crate) async fn releases_after(
        &self,
        actor: &Actor,
        chain: &[ProjectId],
        (current, next): (&Issue, &mut Issue),
        input: &ChangeIssue,
    ) -> Result<IssueReleases, AppError> {
        let (found_asked, planned_asked) =
            (input.found_in_release.as_deref(), input.planned_for_release);
        // A change that names neither field reads nothing of them, so the change of many issues
        // costs no statement more for each issue.
        if found_asked.is_none() && planned_asked.is_none() {
            return Ok(IssueReleases::default());
        }
        let found_now: Vec<ReleaseId> = self
            .store()
            .found_in_of_issues(vec![current.id])
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|row| row.release)
            .collect();
        let mut found_next = found_now.clone();
        let naming = input.asked_by.naming();
        {
            // For the AI, a release of a project kept from it is one the person may not know.
            let mut shown = self.shown_to(actor).await?;
            if let AskedBy::Ai(withheld) = &input.asked_by {
                shown = shown.kept_from_ai(withheld);
            }
            let choice = ReleaseChoice {
                chain,
                found_now: &found_now,
                planned_now: current.planned_for_release,
                naming,
            };
            if let Some(wanted) = found_asked {
                found_next = self.found_in_after(&shown, &choice, wanted).await?;
            }
            if let Some(wanted) = planned_asked {
                next.planned_for_release = self
                    .planned_for_after(&shown, &choice, wanted.into_option())
                    .await?;
            }
        }
        self.issue_releases((chain, naming), found_now, found_next, [current, next])
            .await
    }

    /// The releases a new issue names, judged by the one rule of a choice; `issue` gets the release
    /// it is planned for.
    pub(crate) async fn releases_of_new(
        &self,
        actor: &Actor,
        chain: &[ProjectId],
        issue: &mut Issue,
        input: &CreateIssue,
    ) -> Result<IssueReleases, AppError> {
        let (found_asked, planned_asked) =
            (input.found_in_release.as_deref(), input.planned_for_release);
        let mut found = Vec::new();
        if found_asked.is_some() || planned_asked.is_some() {
            let shown = self.shown_to(actor).await?;
            let choice = ReleaseChoice {
                chain,
                found_now: &[],
                planned_now: None,
                naming: Naming::Anew,
            };
            if let Some(wanted) = found_asked {
                found = self.found_in_after(&shown, &choice, wanted).await?;
            }
            issue.planned_for_release = self
                .planned_for_after(&shown, &choice, planned_asked)
                .await?;
        }
        let blank = Issue {
            planned_for_release: None,
            ..issue.clone()
        };
        self.issue_releases((chain, Naming::Anew), Vec::new(), found, [&blank, issue])
            .await
    }

    /// The releases before and after, with their records read once.
    async fn issue_releases(
        &self,
        (chain, naming): (&[ProjectId], Naming),
        found_now: Vec<ReleaseId>,
        found_next: Vec<ReleaseId>,
        issues: [&Issue; 2],
    ) -> Result<IssueReleases, AppError> {
        let mut ids: Vec<ReleaseId> = found_now
            .iter()
            .chain(&found_next)
            .copied()
            .chain(issues.iter().filter_map(|issue| issue.planned_for_release))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        let records = if ids.is_empty() {
            Vec::new()
        } else {
            self.store()
                .releases_by_ids(ids)
                .await
                .map_err(|error| internal::<S>(&error))?
        };
        Ok(IssueReleases {
            chain: chain.to_vec(),
            found_now,
            found_next,
            records,
            naming,
        })
    }

    /// The releases each of these issues names, as a viewer who sees the projects of `shown` sees
    /// them: the rows of all of them in one statement and the releases in another (PERF-N1).
    pub(crate) async fn releases_of_page(
        &self,
        issues: &[&Issue],
    ) -> Result<PageReleases, AppError> {
        let internal = |error| internal::<S>(&error);
        let ids: Vec<IssueId> = issues.iter().map(|issue| issue.id).collect();
        let rows = self
            .store()
            .found_in_of_issues(ids)
            .await
            .map_err(internal)?;
        let mut wanted: Vec<ReleaseId> = rows
            .iter()
            .map(|row| row.release)
            .chain(issues.iter().filter_map(|issue| issue.planned_for_release))
            .collect();
        wanted.sort_unstable();
        wanted.dedup();
        let records = if wanted.is_empty() {
            Vec::new()
        } else {
            self.store()
                .releases_by_ids(wanted)
                .await
                .map_err(internal)?
        };
        let mut found: HashMap<IssueId, Vec<ReleaseId>> = HashMap::new();
        for row in rows {
            found.entry(row.issue).or_default().push(row.release);
        }
        Ok(PageReleases {
            read: issues.iter().map(|issue| issue.id).collect(),
            found,
            records: records.into_iter().map(|each| (each.id, each)).collect(),
        })
    }
}

/// The releases of a page of issues.
#[derive(Clone, Debug, Default)]
pub(crate) struct PageReleases {
    /// The issues whose releases were read.
    read: HashSet<IssueId>,
    found: HashMap<IssueId, Vec<ReleaseId>>,
    records: HashMap<ReleaseId, Release>,
}

impl PageReleases {
    /// Whether the releases of the issue were read.
    pub(crate) fn has(&self, issue: &Issue) -> bool {
        self.read.contains(&issue.id)
            && issue
                .planned_for_release
                .is_none_or(|release| self.records.contains_key(&release))
    }

    /// Takes in what was read for other issues.
    pub(crate) fn merge(&mut self, other: Self) {
        self.read.extend(other.read);
        self.found.extend(other.found);
        self.records.extend(other.records);
    }

    /// The two fields of the issue in the project of `chain`, as a viewer who sees the projects of
    /// `shown` sees them: a release of a project they do not see is left out.
    pub(crate) fn of(
        &self,
        issue: &Issue,
        chain: &[ProjectId],
        shown: &Shown,
    ) -> (Vec<ReleaseRef>, Option<ReleaseRef>) {
        let seen = |id: &ReleaseId| {
            self.records
                .get(id)
                .filter(|release| shown.sees(release.project))
                .map(|release| ReleaseRef::of(release, chain))
        };
        let found = self
            .found
            .get(&issue.id)
            .map(|ids| ids.iter().filter_map(seen).collect())
            .unwrap_or_default();
        (found, issue.planned_for_release.as_ref().and_then(seen))
    }
}
