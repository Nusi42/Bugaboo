//! The relations, comments, followers and history parts of the in-memory store.

use bb_core::comment::{Comment, CommentVersion, Mention};
use bb_core::event::DomainEvent;
use bb_core::follow::Follower;
use bb_core::history::{HistoryEntry, HistoryKind};
use bb_core::id::{CommentId, IssueId, RelationId, RelationKindId, UserId};
use bb_core::issue::Issue;
use bb_core::relation::{IssueParent, Relation, RelationKind};
use bb_core::version::Version;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, State, Tx};
use crate::comments::CommentStore;
use crate::follow::FollowStore;
use crate::history::HistoryStore;
use crate::relations::RelationStore;
use crate::text_index::{TextIndexStore, Touched};

fn kinds_of(state: &State) -> Vec<RelationKind> {
    let mut kinds = state.relation_kinds.clone();
    kinds.sort_by_key(|kind| (kind.position, kind.id));
    kinds
}

fn relations_of(state: &State, issue: IssueId) -> Vec<Relation> {
    let mut found: Vec<Relation> = state
        .relations
        .iter()
        .filter(|relation| relation.from == issue || relation.to == issue)
        .cloned()
        .collect();
    found.sort_by_key(|relation| (relation.added_at, relation.id));
    found
}

impl RelationStore for FakeStore {
    async fn relation_kinds_in(tx: &mut Tx) -> Result<Vec<RelationKind>, FakeError> {
        Ok(kinds_of(&tx.state))
    }

    async fn relation_in(tx: &mut Tx, id: RelationId) -> Result<Option<Relation>, FakeError> {
        Ok(tx
            .state
            .relations
            .iter()
            .find(|relation| relation.id == id)
            .cloned())
    }

    async fn relations_of_in(tx: &mut Tx, issue: IssueId) -> Result<Vec<Relation>, FakeError> {
        Ok(relations_of(&tx.state, issue))
    }

    async fn relation_between_in(
        tx: &mut Tx,
        kind: RelationKindId,
        from: IssueId,
        to: IssueId,
    ) -> Result<Option<Relation>, FakeError> {
        Ok(tx
            .state
            .relations
            .iter()
            .find(|relation| relation.kind == kind && relation.from == from && relation.to == to)
            .cloned())
    }

    async fn parent_of_in(tx: &mut Tx, child: IssueId) -> Result<Option<IssueParent>, FakeError> {
        Ok(tx
            .state
            .parents
            .iter()
            .find(|parent| parent.child == child)
            .cloned())
    }

    async fn sub_issues_of_in(tx: &mut Tx, parent: IssueId) -> Result<Vec<IssueParent>, FakeError> {
        let mut found: Vec<IssueParent> = tx
            .state
            .parents
            .iter()
            .filter(|row| row.parent == parent)
            .cloned()
            .collect();
        found.sort_by_key(|row| (row.set_at, row.child));
        Ok(found)
    }

    async fn issues_by_ids(&self, ids: &[IssueId]) -> Result<Vec<Issue>, FakeError> {
        Ok(self.read(|state| {
            state
                .issues
                .iter()
                .filter(|issue| ids.contains(&issue.id))
                .cloned()
                .collect()
        }))
    }

    async fn insert_relation_kind(tx: &mut Tx, kind: &RelationKind) -> Result<(), FakeError> {
        tx.state.relation_kinds.push(kind.clone());
        Ok(())
    }

    async fn update_relation_kind(
        tx: &mut Tx,
        id: RelationKindId,
        forward: &str,
        backward: &str,
        retired_at: Option<Timestamp>,
        version: Version,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .relation_kinds
            .iter_mut()
            .find(|found| found.id == id && found.version == version.get())
        else {
            return Ok(false);
        };
        forward.clone_into(&mut found.forward);
        backward.clone_into(&mut found.backward);
        found.retired_at = retired_at;
        found.version += 1;
        Ok(true)
    }

    async fn insert_relation(tx: &mut Tx, relation: &Relation) -> Result<(), FakeError> {
        // The unique key of a relation.
        if tx.state.relations.iter().any(|known| {
            known.kind == relation.kind && known.from == relation.from && known.to == relation.to
        }) {
            return Err(FakeError::Duplicate);
        }
        tx.state.relations.push(relation.clone());
        Ok(())
    }

    async fn delete_relation(tx: &mut Tx, id: RelationId) -> Result<bool, FakeError> {
        let before = tx.state.relations.len();
        tx.state.relations.retain(|relation| relation.id != id);
        Ok(tx.state.relations.len() < before)
    }

    async fn set_issue_parent(tx: &mut Tx, parent: &IssueParent) -> Result<(), FakeError> {
        tx.state.parents.retain(|row| row.child != parent.child);
        tx.state.parents.push(parent.clone());
        Ok(())
    }

    async fn clear_issue_parent(tx: &mut Tx, child: IssueId) -> Result<bool, FakeError> {
        let before = tx.state.parents.len();
        tx.state.parents.retain(|row| row.child != child);
        Ok(tx.state.parents.len() < before)
    }

    async fn lock_issue_parents(_tx: &mut Tx) -> Result<(), FakeError> {
        Ok(())
    }

    async fn lock_issue_relations(_tx: &mut Tx) -> Result<(), FakeError> {
        Ok(())
    }
}

impl CommentStore for FakeStore {
    async fn comment_in(tx: &mut Tx, id: CommentId) -> Result<Option<Comment>, FakeError> {
        Ok(tx
            .state
            .comments
            .iter()
            .find(|comment| comment.id == id)
            .cloned())
    }

    async fn comment_versions_in(
        tx: &mut Tx,
        id: CommentId,
    ) -> Result<Vec<CommentVersion>, FakeError> {
        let mut found: Vec<CommentVersion> = tx
            .state
            .comment_versions
            .iter()
            .filter(|version| version.comment == id)
            .cloned()
            .collect();
        found.sort_by_key(|version| version.version);
        Ok(found)
    }

    async fn comments_by_ids(&self, ids: &[CommentId]) -> Result<Vec<Comment>, FakeError> {
        Ok(self.read(|state| {
            state
                .comments
                .iter()
                .filter(|comment| ids.contains(&comment.id))
                .cloned()
                .collect()
        }))
    }

    async fn mentions_in_comment(&self, comment: CommentId) -> Result<Vec<Mention>, FakeError> {
        Ok(self.read(|state| {
            state
                .mentions
                .iter()
                .filter(|mention| mention.comment == comment)
                .cloned()
                .collect()
        }))
    }

    async fn insert_comment(tx: &mut Tx, comment: &Comment) -> Result<(), FakeError> {
        tx.state.comments.push(comment.clone());
        Ok(())
    }

    async fn update_comment_text(
        tx: &mut Tx,
        id: CommentId,
        text: &str,
        edited_at: Timestamp,
        version: Version,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .comments
            .iter_mut()
            .find(|found| found.id == id && found.version == version.get())
        else {
            return Ok(false);
        };
        text.clone_into(&mut found.text);
        found.edited_at = Some(edited_at);
        found.version += 1;
        Ok(true)
    }

    async fn insert_comment_version(
        tx: &mut Tx,
        version: &CommentVersion,
    ) -> Result<(), FakeError> {
        tx.state.comment_versions.push(version.clone());
        Ok(())
    }

    async fn set_comment_deleted(
        tx: &mut Tx,
        id: CommentId,
        deleted_at: Option<Timestamp>,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .comments
            .iter_mut()
            .find(|found| found.id == id && found.deleted_at.is_some() != deleted_at.is_some())
        else {
            return Ok(false);
        };
        found.deleted_at = deleted_at;
        found.version += 1;
        Ok(true)
    }

    async fn replace_mentions(
        tx: &mut Tx,
        comment: CommentId,
        mentions: &[Mention],
    ) -> Result<(), FakeError> {
        let known: Vec<Mention> = tx
            .state
            .mentions
            .iter()
            .filter(|mention| mention.comment == comment)
            .cloned()
            .collect();
        tx.state.mentions.retain(|mention| {
            mention.comment != comment
                || mentions
                    .iter()
                    .any(|wanted| wanted.recipient == mention.recipient)
        });
        for mention in mentions {
            if !known.iter().any(|old| old.recipient == mention.recipient) {
                tx.state.mentions.push(mention.clone());
            }
        }
        Ok(())
    }
}

impl FollowStore for FakeStore {
    async fn followers_of_in(tx: &mut Tx, issue: IssueId) -> Result<Vec<Follower>, FakeError> {
        let mut found: Vec<Follower> = tx
            .state
            .followers
            .iter()
            .filter(|follower| follower.issue == issue)
            .cloned()
            .collect();
        found.sort_by_key(|follower| (follower.since, follower.person));
        Ok(found)
    }

    async fn add_followers(tx: &mut Tx, followers: &[Follower]) -> Result<(), FakeError> {
        for follower in followers {
            let there = tx
                .state
                .followers
                .iter()
                .any(|known| known.issue == follower.issue && known.person == follower.person);
            if !there {
                tx.state.followers.push(follower.clone());
            }
        }
        Ok(())
    }

    async fn remove_follower(
        tx: &mut Tx,
        issue: IssueId,
        person: UserId,
    ) -> Result<bool, FakeError> {
        let before = tx.state.followers.len();
        tx.state
            .followers
            .retain(|follower| !(follower.issue == issue && follower.person == person));
        Ok(tx.state.followers.len() < before)
    }
}

impl HistoryStore for FakeStore {
    async fn history_events(&self, limit: u64) -> Result<Vec<(i64, DomainEvent)>, FakeError> {
        let shared = self.lock();
        let done = usize::try_from(shared.state.history_position).unwrap_or(usize::MAX);
        Ok(shared
            .events
            .iter()
            .enumerate()
            .skip(done)
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .map(|(at, event)| (i64::try_from(at + 1).unwrap_or(i64::MAX), event.clone()))
            .collect())
    }

    async fn store_history(
        &self,
        entries: &[HistoryEntry],
        position: i64,
        _now: Timestamp,
    ) -> Result<(), FakeError> {
        let mut shared = self.lock();
        for entry in entries {
            let there = shared
                .state
                .history
                .iter()
                .any(|known| known.event == entry.event && known.issue == entry.issue);
            if !there {
                shared.state.history.push(entry.clone());
            }
        }
        shared.state.history_position = shared.state.history_position.max(position);
        Ok(())
    }

    async fn history_page(
        &self,
        issue: IssueId,
        kind: Option<HistoryKind>,
        before: Option<i64>,
        limit: u64,
    ) -> Result<Vec<HistoryEntry>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<HistoryEntry> = state
                .history
                .iter()
                .filter(|entry| {
                    entry.issue == issue
                        && kind.is_none_or(|kind| entry.kind == kind)
                        && before.is_none_or(|before| entry.position < before)
                })
                .cloned()
                .collect();
            found.sort_by_key(|entry| std::cmp::Reverse((entry.position, entry.event)));
            found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            found
        }))
    }
}

impl TextIndexStore for FakeStore {
    async fn text_index_events(&self, limit: u64) -> Result<Vec<(i64, DomainEvent)>, FakeError> {
        let shared = self.lock();
        let done = usize::try_from(shared.state.text_index_position).unwrap_or(usize::MAX);
        Ok(shared
            .events
            .iter()
            .enumerate()
            .skip(done)
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .map(|(at, event)| (i64::try_from(at + 1).unwrap_or(i64::MAX), event.clone()))
            .collect())
    }

    async fn store_text_index(
        &self,
        _touched: &[Touched],
        position: i64,
        _now: Timestamp,
    ) -> Result<(), FakeError> {
        let mut shared = self.lock();
        shared.state.text_index_position = shared.state.text_index_position.max(position);
        Ok(())
    }

    async fn rebuild_text_index(&self) -> Result<u64, FakeError> {
        Ok(0)
    }
}
