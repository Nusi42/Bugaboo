//! The tags, labels and groups of labels part of the in-memory store.

use bb_core::id::{IssueId, LabelGroupId, LabelId, ProjectId, TagId};
use bb_core::own_fields::ChoiceColor;
use bb_core::tags::{
    IssueLabel, IssueSight, IssueTag, Label, LabelGroup, Tag, TagProject, name_key,
};
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, State, Tx};
use crate::tags::TagStore;

fn page_after(mut issues: Vec<IssueId>, after: Option<IssueId>, limit: u64) -> Vec<IssueId> {
    issues.sort();
    issues
        .into_iter()
        .filter(|issue| after.is_none_or(|after| *issue > after))
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .collect()
}

fn tags_of(state: &State, issues: &[IssueId]) -> Vec<IssueTag> {
    let mut found: Vec<IssueTag> = state
        .issue_tags
        .iter()
        .filter(|link| issues.contains(&link.issue))
        .cloned()
        .collect();
    found.sort_by_key(|link| (link.issue, link.added_at, link.tag));
    found
}

fn labels_of(state: &State, issues: &[IssueId]) -> Vec<IssueLabel> {
    let mut found: Vec<IssueLabel> = state
        .issue_labels
        .iter()
        .filter(|link| issues.contains(&link.issue))
        .cloned()
        .collect();
    found.sort_by_key(|link| (link.issue, link.added_at, link.label));
    found
}

impl TagStore for FakeStore {
    async fn tags_in(tx: &mut Tx) -> Result<Vec<Tag>, FakeError> {
        let mut tags = tx.state.tags.clone();
        tags.sort_by_key(|tag| (name_key(&tag.name), tag.id));
        Ok(tags)
    }

    async fn tag_projects_in(tx: &mut Tx) -> Result<Vec<TagProject>, FakeError> {
        let mut limits = tx.state.tag_projects.clone();
        limits.sort_by_key(|limit| (limit.tag, limit.project));
        Ok(limits)
    }

    async fn issue_tags_in(tx: &mut Tx, issue: IssueId) -> Result<Vec<IssueTag>, FakeError> {
        Ok(tags_of(&tx.state, &[issue]))
    }

    async fn label_groups_in(tx: &mut Tx) -> Result<Vec<LabelGroup>, FakeError> {
        let mut groups = tx.state.label_groups.clone();
        groups.sort_by_key(|group| (group.position, group.id));
        Ok(groups)
    }

    async fn labels_in(tx: &mut Tx) -> Result<Vec<Label>, FakeError> {
        let mut labels = tx.state.labels.clone();
        labels.sort_by_key(|label| (label.position, label.id));
        Ok(labels)
    }

    async fn issue_labels_in(tx: &mut Tx, issue: IssueId) -> Result<Vec<IssueLabel>, FakeError> {
        Ok(labels_of(&tx.state, &[issue]))
    }

    async fn issues_with_tag_in(
        tx: &mut Tx,
        tag: TagId,
        after: Option<IssueId>,
        limit: u64,
    ) -> Result<Vec<IssueId>, FakeError> {
        let issues = tx
            .state
            .issue_tags
            .iter()
            .filter(|link| link.tag == tag)
            .map(|link| link.issue)
            .collect();
        Ok(page_after(issues, after, limit))
    }

    async fn issues_with_label_in(
        tx: &mut Tx,
        label: LabelId,
        after: Option<IssueId>,
        limit: u64,
    ) -> Result<Vec<IssueId>, FakeError> {
        let issues = tx
            .state
            .issue_labels
            .iter()
            .filter(|link| link.label == label)
            .map(|link| link.issue)
            .collect();
        Ok(page_after(issues, after, limit))
    }

    async fn tags_of_issues_in(
        tx: &mut Tx,
        issues: Vec<IssueId>,
    ) -> Result<Vec<IssueTag>, FakeError> {
        Ok(tags_of(&tx.state, &issues))
    }

    async fn labels_of_issues_in(
        tx: &mut Tx,
        issues: Vec<IssueId>,
    ) -> Result<Vec<IssueLabel>, FakeError> {
        Ok(labels_of(&tx.state, &issues))
    }

    async fn used_tags(
        &self,
        sight: &IssueSight,
        part: &str,
        limit: u64,
    ) -> Result<Vec<Tag>, FakeError> {
        let part = name_key(part);
        Ok(self.read(|state| {
            let mut found: Vec<Tag> = state
                .tags
                .iter()
                .filter(|tag| name_key(&tag.name).contains(&part))
                .filter(|tag| {
                    state.issue_tags.iter().any(|link| {
                        link.tag == tag.id
                            && state.issues.iter().any(|issue| {
                                issue.id == link.issue
                                    && issue.deleted_at.is_none()
                                    && (sight.all.contains(&issue.project)
                                        || (sight.own.contains(&issue.project)
                                            && issue.reported_by == sight.person))
                            })
                    })
                })
                .cloned()
                .collect();
            found.sort_by_key(|tag| (name_key(&tag.name), tag.id));
            found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            found
        }))
    }

    async fn insert_tag(tx: &mut Tx, tag: &Tag) -> Result<(), FakeError> {
        if tx
            .state
            .tags
            .iter()
            .any(|other| name_key(&other.name) == name_key(&tag.name))
        {
            return Err(FakeError::Duplicate);
        }
        tx.state.tags.push(tag.clone());
        Ok(())
    }

    async fn update_tag(
        tx: &mut Tx,
        id: TagId,
        name: &str,
        color: ChoiceColor,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .tags
            .iter_mut()
            .find(|found| found.id == id && found.version == version)
        else {
            return Ok(false);
        };
        name.clone_into(&mut found.name);
        found.color = color;
        found.version += 1;
        Ok(true)
    }

    async fn replace_tag_projects(
        tx: &mut Tx,
        tag: TagId,
        projects: &[ProjectId],
    ) -> Result<(), FakeError> {
        tx.state.tag_projects.retain(|limit| limit.tag != tag);
        tx.state
            .tag_projects
            .extend(projects.iter().map(|project| TagProject {
                tag,
                project: *project,
            }));
        Ok(())
    }

    async fn delete_tag(tx: &mut Tx, id: TagId) -> Result<bool, FakeError> {
        let before = tx.state.tags.len();
        tx.state.tags.retain(|tag| tag.id != id);
        tx.state.tag_projects.retain(|limit| limit.tag != id);
        tx.state.issue_tags.retain(|link| link.tag != id);
        Ok(tx.state.tags.len() < before)
    }

    async fn insert_issue_tag(tx: &mut Tx, link: &IssueTag) -> Result<(), FakeError> {
        if tx
            .state
            .issue_tags
            .iter()
            .any(|other| other.issue == link.issue && other.tag == link.tag)
        {
            return Err(FakeError::Duplicate);
        }
        tx.state.issue_tags.push(link.clone());
        Ok(())
    }

    async fn delete_issue_tag(tx: &mut Tx, issue: IssueId, tag: TagId) -> Result<bool, FakeError> {
        let before = tx.state.issue_tags.len();
        tx.state
            .issue_tags
            .retain(|link| !(link.issue == issue && link.tag == tag));
        Ok(tx.state.issue_tags.len() < before)
    }

    async fn retarget_issue_tags(tx: &mut Tx, from: TagId, into: TagId) -> Result<(), FakeError> {
        let moved: Vec<IssueTag> = tx
            .state
            .issue_tags
            .iter()
            .filter(|link| link.tag == from)
            .cloned()
            .collect();
        for link in moved {
            let has_it = tx
                .state
                .issue_tags
                .iter()
                .any(|other| other.issue == link.issue && other.tag == into);
            if !has_it {
                tx.state.issue_tags.push(IssueTag { tag: into, ..link });
            }
        }
        tx.state.issue_tags.retain(|link| link.tag != from);
        Ok(())
    }

    async fn insert_label_group(tx: &mut Tx, group: &LabelGroup) -> Result<(), FakeError> {
        if tx
            .state
            .label_groups
            .iter()
            .any(|other| name_key(&other.name) == name_key(&group.name))
        {
            return Err(FakeError::Duplicate);
        }
        tx.state.label_groups.push(group.clone());
        Ok(())
    }

    async fn update_label_group(
        tx: &mut Tx,
        id: LabelGroupId,
        name: &str,
        pick_one: bool,
        retired_at: Option<Timestamp>,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .label_groups
            .iter_mut()
            .find(|found| found.id == id && found.version == version)
        else {
            return Ok(false);
        };
        name.clone_into(&mut found.name);
        found.pick_one = pick_one;
        found.retired_at = retired_at;
        found.version += 1;
        Ok(true)
    }

    async fn insert_label(tx: &mut Tx, label: &Label) -> Result<(), FakeError> {
        if tx.state.labels.iter().any(|other| {
            other.group == label.group && name_key(&other.name) == name_key(&label.name)
        }) {
            return Err(FakeError::Duplicate);
        }
        tx.state.labels.push(label.clone());
        Ok(())
    }

    async fn update_label(
        tx: &mut Tx,
        id: LabelId,
        name: &str,
        color: ChoiceColor,
        group: Option<LabelGroupId>,
        retired_at: Option<Timestamp>,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .labels
            .iter_mut()
            .find(|found| found.id == id && found.version == version)
        else {
            return Ok(false);
        };
        name.clone_into(&mut found.name);
        found.color = color;
        found.group = group;
        found.retired_at = retired_at;
        found.version += 1;
        Ok(true)
    }

    async fn insert_issue_label(tx: &mut Tx, link: &IssueLabel) -> Result<(), FakeError> {
        if tx
            .state
            .issue_labels
            .iter()
            .any(|other| other.issue == link.issue && other.label == link.label)
        {
            return Err(FakeError::Duplicate);
        }
        tx.state.issue_labels.push(link.clone());
        Ok(())
    }

    async fn delete_issue_label(
        tx: &mut Tx,
        issue: IssueId,
        label: LabelId,
    ) -> Result<bool, FakeError> {
        let before = tx.state.issue_labels.len();
        tx.state
            .issue_labels
            .retain(|link| !(link.issue == issue && link.label == label));
        Ok(tx.state.issue_labels.len() < before)
    }

    async fn retarget_issue_labels(
        tx: &mut Tx,
        from: LabelId,
        into: LabelId,
    ) -> Result<(), FakeError> {
        let moved: Vec<IssueLabel> = tx
            .state
            .issue_labels
            .iter()
            .filter(|link| link.label == from)
            .cloned()
            .collect();
        for link in moved {
            let has_it = tx
                .state
                .issue_labels
                .iter()
                .any(|other| other.issue == link.issue && other.label == into);
            if !has_it {
                tx.state.issue_labels.push(IssueLabel {
                    label: into,
                    ..link
                });
            }
        }
        tx.state.issue_labels.retain(|link| link.label != from);
        Ok(())
    }

    async fn delete_label(tx: &mut Tx, id: LabelId) -> Result<bool, FakeError> {
        let before = tx.state.labels.len();
        tx.state.labels.retain(|label| label.id != id);
        Ok(tx.state.labels.len() < before)
    }

    async fn lock_tags_and_labels(_tx: &mut Tx) -> Result<(), FakeError> {
        Ok(())
    }
}
