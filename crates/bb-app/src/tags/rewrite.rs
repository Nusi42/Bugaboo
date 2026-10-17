//! Renaming and combining tags and labels across all issues (TAG-03): the one place that works out,
//! for each issue the change touches, the names it has and the names it will have.
//!
//! The preview reads this and shows it to the people who look after tags and labels (only for the
//! issues they see); the change reads the same and writes one event of the issue for each, inside
//! the change itself, so the preview and the change cannot differ and every issue records what
//! happened to it.

use std::collections::BTreeMap;

use bb_core::event::{DomainEvent, FieldChange, RecordKind};
use bb_core::id::{EntityId, IssueId, LabelId, TagId};
use jiff::Timestamp;
use serde_json::json;

use super::types::{CHANGE_PREVIEW_ISSUES_MAX, ChangedIssue, TagChangePreview};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, EventMaker, by, entity, internal, set_to};
use crate::issues::{InTrash, Where, member};
use crate::member::{label as label_member, tag as tag_member};
use crate::preview::PreviewSummary;
use crate::projects::Why;

/// How many issues a walk through the issues of a tag, a label or a group of labels reads at a
/// time. It is about the size of an answer the walk holds, not about the database: the store cuts
/// a long list into statements itself.
pub(super) const ISSUES_PER_READ: u64 = 200;

/// Which kind of thing is renamed or combined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Sorted {
    /// A tag.
    Tag,
    /// A label.
    Label,
}

impl Sorted {
    /// The member of the event of an issue that holds the names.
    const fn member(self) -> &'static str {
        match self {
            Self::Tag => member::TAGS,
            Self::Label => member::LABELS,
        }
    }
}

/// What a combination of tags or labels starts with.
pub(super) struct Combining {
    /// The event of the thing that goes away.
    pub(super) event: DomainEvent,
    /// Makes the events of the issues that change with it.
    pub(super) events: EventMaker,
    /// Carries the reason a refusal inside the change gives.
    pub(super) why: Why,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The event of combining `from` into `into`, and the maker of the events of the issues that
    /// change with it: the start of every combination of tags or labels.
    ///
    /// # Errors
    /// An internal error when an event cannot be made.
    pub(super) fn combine_events(
        &self,
        actor: &Actor,
        sorted: Sorted,
        from: EntityId,
        into: EntityId,
    ) -> Result<Combining, AppError> {
        let (kind, member) = match sorted {
            Sorted::Tag => (RecordKind::Tag, tag_member::COMBINED_INTO),
            Sorted::Label => (RecordKind::Label, label_member::COMBINED_INTO),
        };
        let event = self.event(
            by(actor.person.id),
            entity(kind, from),
            vec![set_to(member, json!(into.to_string()))],
        )?;
        Ok(Combining {
            event,
            events: self.event_maker(by(actor.person.id)),
            why: Why::default(),
        })
    }
}

/// A tag or a label that takes the place of another by a combine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Replacement {
    /// Its ID.
    pub(super) id: EntityId,
    /// What it is called.
    pub(super) name: String,
}

/// What happens to the things called `from`: they are renamed, or replaced by another one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Rewrite {
    pub(super) sorted: Sorted,
    /// The tag or label that is renamed or goes away.
    pub(super) from: EntityId,
    /// The new name of `from`, when it is renamed.
    pub(super) rename: Option<String>,
    /// The tag or label that takes its place, when it is combined.
    pub(super) into: Option<Replacement>,
}

/// An issue the change touches, with the names of its tags or labels before and after.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Touched {
    pub(super) issue: IssueId,
    pub(super) before: Vec<String>,
    pub(super) after: Vec<String>,
}

/// One tag or label on an issue, as the change reads it.
struct Link {
    issue: IssueId,
    item: EntityId,
    added_at: Timestamp,
}

impl Rewrite {
    /// The issues that have `from`, after `after`, at most [`ISSUES_PER_READ`].
    async fn issues_with<S: AuthStore>(
        &self,
        transaction: &mut S::Transaction,
        after: Option<IssueId>,
    ) -> Result<Vec<IssueId>, S::Error> {
        match self.sorted {
            Sorted::Tag => {
                let from = TagId::from_bytes(self.from.to_bytes()).map_err(|_| S::aborted())?;
                S::issues_with_tag_in(transaction, from, after, ISSUES_PER_READ).await
            }
            Sorted::Label => {
                let from = LabelId::from_bytes(self.from.to_bytes()).map_err(|_| S::aborted())?;
                S::issues_with_label_in(transaction, from, after, ISSUES_PER_READ).await
            }
        }
    }

    /// Everything the issues of a chunk have of this kind, and what everything of the kind is
    /// called.
    async fn links_of<S: AuthStore>(
        &self,
        transaction: &mut S::Transaction,
        issues: &[IssueId],
    ) -> Result<(Vec<Link>, BTreeMap<EntityId, String>), S::Error> {
        match self.sorted {
            Sorted::Tag => {
                let names = S::tags_in(transaction)
                    .await?
                    .into_iter()
                    .map(|tag| (EntityId::from(tag.id), tag.name))
                    .collect();
                let links = S::tags_of_issues_in(transaction, issues.to_vec())
                    .await?
                    .into_iter()
                    .map(|link| Link {
                        issue: link.issue,
                        item: link.tag.into(),
                        added_at: link.added_at,
                    })
                    .collect();
                Ok((links, names))
            }
            Sorted::Label => {
                let names = S::labels_in(transaction)
                    .await?
                    .into_iter()
                    .map(|label| (EntityId::from(label.id), label.name))
                    .collect();
                let links = S::labels_of_issues_in(transaction, issues.to_vec())
                    .await?
                    .into_iter()
                    .map(|link| Link {
                        issue: link.issue,
                        item: link.label.into(),
                        added_at: link.added_at,
                    })
                    .collect();
                Ok((links, names))
            }
        }
    }

    /// The names an issue has after the change, from the links it has now, in the order the store
    /// reads them (the day they were put there, then the ID).
    fn after(&self, links: &[&Link], names: &BTreeMap<EntityId, String>) -> Vec<String> {
        let mut kept: Vec<(Timestamp, EntityId, String)> = Vec::with_capacity(links.len());
        let has_into = self
            .into
            .as_ref()
            .is_some_and(|into| links.iter().any(|link| link.item == into.id));
        for link in links {
            let name = names.get(&link.item).cloned().unwrap_or_default();
            if link.item != self.from {
                kept.push((link.added_at, link.item, name));
            } else if let Some(into) = &self.into {
                if !has_into {
                    kept.push((link.added_at, into.id, into.name.clone()));
                }
            } else {
                kept.push((
                    link.added_at,
                    link.item,
                    self.rename.clone().unwrap_or(name),
                ));
            }
        }
        kept.sort_by_key(|entry| (entry.0, entry.1));
        kept.into_iter().map(|(_, _, name)| name).collect()
    }

    /// Every issue the change touches, with its names before and after, in the order of the issues.
    /// Read inside the change (after the lock) or in a snapshot for the preview.
    pub(super) async fn touched_in<S: AuthStore>(
        &self,
        transaction: &mut S::Transaction,
    ) -> Result<Vec<Touched>, S::Error> {
        let mut touched = Vec::new();
        let mut after = None;
        loop {
            let issues = self.issues_with::<S>(transaction, after).await?;
            let Some(last) = issues.last().copied() else {
                break;
            };
            let (links, names) = self.links_of::<S>(transaction, &issues).await?;
            for issue in &issues {
                let mine: Vec<&Link> = links.iter().filter(|link| link.issue == *issue).collect();
                let mut ordered: Vec<&Link> = mine.clone();
                ordered.sort_by_key(|link| (link.added_at, link.item));
                let before: Vec<String> = ordered
                    .iter()
                    .map(|link| names.get(&link.item).cloned().unwrap_or_default())
                    .collect();
                touched.push(Touched {
                    issue: *issue,
                    before,
                    after: self.after(&mine, &names),
                });
            }
            if issues.len() < usize::try_from(ISSUES_PER_READ).unwrap_or(usize::MAX) {
                break;
            }
            after = Some(last);
        }
        Ok(touched)
    }

    /// Appends the event of every issue the change touches, inside the change, and counts the
    /// version of each of them up.
    pub(super) async fn append_events<S: AuthStore>(
        &self,
        transaction: &mut S::Transaction,
        events: &EventMaker,
    ) -> Result<(), S::Error> {
        for touched in self.touched_in::<S>(transaction).await? {
            let each = self.event_of(events, &touched).map_err(|_| S::aborted())?;
            // The version of the issue counts up too (M3-F126): a change to many issues that was
            // previewed before the rename is told that the issue changed, and a person who looks at
            // the issue sees the new name as a change after what they started from.
            S::count_issue_up(
                transaction,
                touched.issue,
                None,
                InTrash::Counted,
                each.time,
            )
            .await?;
            S::append_follow_up_event(transaction, &each).await?;
        }
        Ok(())
    }

    /// The event of an issue the change touched.
    fn event_of(
        &self,
        events: &EventMaker,
        touched: &Touched,
    ) -> Result<DomainEvent, bb_core::id::IdError> {
        events.create(
            entity(RecordKind::Issue, touched.issue),
            vec![FieldChange {
                field: self.sorted.member().to_owned(),
                before: Some(json!(touched.before)),
                after: Some(json!(touched.after)),
            }],
        )
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What a rename or a combine would do, as the person sees the issues it touches: how many,
    /// and the first of them. Issues the person cannot see are not counted (PRIV-13).
    pub(super) async fn preview_rewrite(
        &self,
        actor: &Actor,
        rewrite: Rewrite,
    ) -> Result<TagChangePreview, AppError> {
        let touched = self
            .store()
            .read_snapshot(move |transaction| {
                Box::pin(async move { rewrite.touched_in::<S>(transaction).await })
            })
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut maker = self.line_maker().await?;
        let mut issues = Vec::new();
        let mut seen = 0_usize;
        for page in touched.chunks(usize::try_from(ISSUES_PER_READ).unwrap_or(usize::MAX)) {
            let ids: Vec<IssueId> = page.iter().map(|each| each.issue).collect();
            let found = self.issues_named(&ids).await?;
            for each in page {
                let Some(issue) = found.iter().find(|issue| issue.id == each.issue) else {
                    continue;
                };
                if self.seen_in(actor, issue, Where::Live).await?.is_none() {
                    continue;
                }
                seen += 1;
                if issues.len() < CHANGE_PREVIEW_ISSUES_MAX
                    && let Some(line) = self.line_of(actor, &mut maker, issue).await?
                {
                    issues.push(ChangedIssue {
                        issue: line,
                        before: each.before.clone(),
                        after: each.after.clone(),
                    });
                }
            }
        }
        Ok(TagChangePreview {
            summary: PreviewSummary::of(seen, 0),
            truncated: seen > issues.len(),
            issues,
        })
    }
}
