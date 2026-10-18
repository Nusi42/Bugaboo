//! Tags and labels on one issue (TAG-01, TAG-02): reading them, putting them on and taking them off.
//!
//! Anybody who may change the issue, and whose role lets them change the field *Tags* or *Labels*
//! in this issue type, puts a tag on it: a name nobody used makes the tag (a name is one tag
//! whatever its capitals), and a tag that is limited to other projects is refused. A label comes
//! from the list the people who look after labels keep; in a *Pick one* group the label takes the
//! place of the one the issue has from that group. Each change is one event of the issue, with
//! the names it had and the names it has (History reads them). Each also counts the version of
//! the issue up by one, whatever the version is, because it takes no version: two of them at the
//! same moment, or one beside a save of the title, both succeed, and the event and the row agree on
//! the version they count up to (ADR-0017). Only an issue that went to the trash is not found.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::Field;
use bb_core::id::{IssueId, LabelId, TagId};
use bb_core::issue::Issue;
use bb_core::own_fields::ChoiceColor;
use bb_core::tags::{IssueLabel, IssueTag, Label, Tag, tag_allowed_in};
use bb_core::version::counted_up;
use bb_policy::{Authorized, Evaluation};
use jiff::Timestamp;
use serde_json::json;

use super::judge::{LabelAddition, TagAddition, judge_label, judge_tag, label_names, tag_names};
use super::types::{
    AddLabel, AddTag, IssueLabelsAndTags, IssueLabelsAndTagsQuery, LabelRef, RemoveLabel,
    RemoveTag, TagRef,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, EventMaker, by, entity, internal};
use crate::input::clean_tag_name;
use crate::issues::{InTrash, IssueTypeRef, member};
use crate::offers::group_offers_in;
use crate::projects::Why;

/// What a change of the tags or the labels says in the event of the issue.
fn names_changed(field: &str, before: &[String], after: &[String]) -> FieldChange {
    FieldChange {
        field: field.to_owned(),
        before: Some(json!(before)),
        after: Some(json!(after)),
    }
}

/// Everything a request to change the tags or the labels of an issue is judged on.
struct Kept {
    issue: Issue,
    held: Arc<Evaluation>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The issue a person asks to put tags on or take them off: they see it, may change it, and
    /// their role lets them change `field` in this issue type.
    async fn issue_to_sort(
        &self,
        actor: &Actor,
        address: &str,
        field: Field,
    ) -> Result<Kept, AppError> {
        let (held, issue) = self.issue_to_change(actor, address).await?;
        crate::issues::ensure_may_change(&held.field_mask_for(Some(issue.issue_type)), &[field])?;
        Ok(Kept { issue, held })
    }

    /// The tags and labels of an issue and what the viewer may do with them.
    async fn labels_and_tags_of(
        &self,
        held: &Evaluation,
        issue: &Issue,
    ) -> Result<IssueLabelsAndTags, AppError> {
        let store = self.store();
        let internal = |error| internal::<S>(&error);
        let tags = store.tags().await.map_err(internal)?;
        let held_tags = store.issue_tags(issue.id).await.map_err(internal)?;
        let labels = store.labels().await.map_err(internal)?;
        let held_labels = store.issue_labels(issue.id).await.map_err(internal)?;
        // The issue is one the viewer reads, so its type and its labels are known to them by use
        // (`offers/known.rs`).
        let issue_type = store
            .issue_type(issue.issue_type)
            .await
            .map_err(internal)?
            .ok_or_else(|| AppError::Internal("an issue has an issue type that is gone".into()))?;
        let mask = held.field_mask_for(Some(issue.issue_type));
        let may_change = held.may_change_issue(issue.reported_by);
        Ok(IssueLabelsAndTags {
            issue: issue.id,
            version: issue.version,
            issue_type: IssueTypeRef::from(&issue_type),
            tags: held_tags
                .iter()
                .filter_map(|link| tags.iter().find(|tag| tag.id == link.tag))
                .map(TagRef::from)
                .collect(),
            labels: held_labels
                .iter()
                .filter_map(|link| labels.iter().find(|label| label.id == link.label))
                .map(LabelRef::from)
                .collect(),
            may_change_tags: may_change && mask.can_change(Field::Tags),
            may_change_labels: may_change && mask.can_change(Field::Labels),
        })
    }

    /// What is stored about tags that putting a tag on an issue is judged on.
    async fn tag_state(&self, issue: IssueId) -> Result<TagState, AppError> {
        let store = self.store();
        let internal = |error| internal::<S>(&error);
        Ok(TagState {
            tags: store.tags().await.map_err(internal)?,
            limits: store.tag_projects().await.map_err(internal)?,
            held: store.issue_tags(issue).await.map_err(internal)?,
        })
    }

    /// What is stored about labels that putting a label on an issue is judged on.
    async fn label_state(&self, issue: IssueId) -> Result<LabelState, AppError> {
        let store = self.store();
        let internal = |error| internal::<S>(&error);
        Ok(LabelState {
            labels: store.labels().await.map_err(internal)?,
            groups: store.label_groups().await.map_err(internal)?,
            held: store.issue_labels(issue).await.map_err(internal)?,
        })
    }

    /// Whether every tag an issue has is allowed in a project whose projects from the top down are
    /// `chain`: whether the issue can move there without a tag that is not for that project.
    pub(crate) async fn tags_follow(
        &self,
        issue: IssueId,
        chain: &[bb_core::id::ProjectId],
    ) -> Result<bool, AppError> {
        let state = self.tag_state(issue).await?;
        Ok(state
            .held
            .iter()
            .all(|link| tag_allowed_in(link.tag, &state.limits, chain)))
    }

    /// Judges putting a label on an issue: what the preview of a change to many issues shows, and
    /// what putting it on judges first.
    ///
    /// # Errors
    /// The errors of [`add_label`].
    pub(crate) async fn plan_label(
        &self,
        actor: &Actor,
        issue: &str,
        label: LabelId,
    ) -> Result<LabelPlan, AppError> {
        let Kept { issue, .. } = self.issue_to_sort(actor, issue, Field::Labels).await?;
        let state = self.label_state(issue.id).await?;
        // A label of a group the person may not know is not there for them.
        let group = state
            .labels
            .iter()
            .find(|found| found.id == label)
            .and_then(|found| found.group);
        if let Some(group) = group {
            self.known_group(actor, group).await?;
        }
        let offers = self.group_offers(issue.project).await?;
        let addition = judge_label(label, &state.labels, &state.groups, &state.held, &offers)?;
        let before = label_names(&state.labels, &state.held);
        let mut after = label_names(
            &state.labels,
            &state
                .held
                .iter()
                .filter(|link| !addition.replaces.contains(&link.label))
                .cloned()
                .collect::<Vec<_>>(),
        );
        if !addition.already
            && let Some(added) = state.labels.iter().find(|found| found.id == label)
        {
            after.push(added.name.clone());
        }
        Ok(LabelPlan {
            issue,
            addition,
            before,
            after,
        })
    }
}

/// The rows a tag is judged on.
struct TagState {
    tags: Vec<Tag>,
    limits: Vec<bb_core::tags::TagProject>,
    held: Vec<IssueTag>,
}

/// The rows a label is judged on.
struct LabelState {
    labels: Vec<Label>,
    groups: Vec<bb_core::tags::LabelGroup>,
    held: Vec<IssueLabel>,
}

/// A label that may be put on an issue, judged but not written yet.
pub(crate) struct LabelPlan {
    /// The issue.
    pub(crate) issue: Issue,
    addition: LabelAddition,
    /// The names of its labels now.
    pub(crate) before: Vec<String>,
    /// The names of its labels after.
    pub(crate) after: Vec<String>,
}

impl LabelPlan {
    /// The issue has the label already: there is nothing to do.
    pub(crate) const fn is_done_already(&self) -> bool {
        self.addition.already
    }
}

/// Counts the version of an issue up inside the change that puts a tag or a label on it or takes
/// one off, and writes the event of the change with the version the row has then, so the event
/// and the row always agree. These changes take no version: whatever the version is, it counts up
/// by one, so two of them at the same moment both succeed (ADR-0017). Only an issue that went to
/// the trash in between is not found. Gives the new version of the issue.
async fn count_up_and_record<S: AuthStore>(
    transaction: &mut S::Transaction,
    maker: &EventMaker,
    issue: IssueId,
    names: FieldChange,
    now: Timestamp,
    stopped: &Why,
) -> Result<i64, S::Error> {
    let Some(version) = S::count_issue_up(transaction, issue, None, InTrash::NotFound, now).await?
    else {
        return Err(stopped.stop::<S>(ErrorCode::NotFound));
    };
    let event = maker
        .create(
            entity(RecordKind::Issue, issue),
            vec![names, counted_up(version - 1)],
        )
        .map_err(|_| stopped.stop::<S>(ErrorCode::Internal))?;
    S::append_follow_up_event(transaction, &event).await?;
    Ok(version)
}

/// The end of the change that takes a tag or a label off an issue: when it was on the issue, the
/// version of the issue counts up; when it was not (somebody took it off in between), nothing is
/// found.
async fn counted_after_take_off<S: AuthStore>(
    transaction: &mut S::Transaction,
    maker: &EventMaker,
    taken_off: bool,
    issue: IssueId,
    names: FieldChange,
    now: Timestamp,
    stopped: &Why,
) -> Result<i64, S::Error> {
    if taken_off {
        count_up_and_record::<S>(transaction, maker, issue, names, now, stopped).await
    } else {
        Err(stopped.stop::<S>(ErrorCode::NotFound))
    }
}

/// Shows the tags and labels of an issue and what the person may do with them.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist, is in the trash or that the person may
/// not see.
pub async fn issue_labels_and_tags<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: IssueLabelsAndTagsQuery,
) -> Result<Authorized<IssueLabelsAndTags>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let view = ctx.labels_and_tags_of(&held, &issue).await?;
    Ok(held.authorize(view))
}

/// Puts a tag on an issue, by its name. A name nobody used makes the tag, with the next color in
/// turn.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist or that the person may not see,
/// `access.forbidden` without the right to change the issue or its field *Tags*, `request.invalid`
/// for a name that cannot be one, `tag.not_here` for a tag limited to other projects,
/// `limit.reached` for an issue that has as many tags as it may.
pub async fn add_tag<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AddTag,
) -> Result<Authorized<IssueLabelsAndTags>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let Kept { mut issue, held } = ctx.issue_to_sort(&actor, &input.issue, Field::Tags).await?;
    let name = clean_tag_name(&input.name)?;
    let chain = ctx.chain_of(issue.project).await?;
    let state = ctx.tag_state(issue.id).await?;
    let TagAddition { tag, already } =
        judge_tag(&name, &state.tags, &state.limits, &state.held, &chain)?;
    if !already {
        let shown = tag
            .and_then(|id| state.tags.iter().find(|found| found.id == id))
            .map_or_else(|| name.clone(), |found| found.name.clone());
        let before = tag_names(&state.tags, &state.held);
        let mut after = before.clone();
        after.push(shown);
        let names = names_changed(member::TAGS, &before, &after);
        let maker = ctx.event_maker(by(actor.person.id));
        let (id, person, now) = (issue.id, actor.person.id, ctx.now());
        let (why, mut ids) = (Why::default(), ctx.owned_ids());
        let stopped = why.clone();
        let color = ChoiceColor::nth(state.tags.len());
        let counted = ctx
            .store()
            .record_changes(&[], move |transaction| {
                Box::pin(async move {
                    S::lock_tags_and_labels(transaction).await?;
                    // Judged again on what is stored now: the tag may have been made, limited or
                    // combined away since the request began.
                    let tags = S::tags_in(transaction).await?;
                    let limits = S::tag_projects_in(transaction).await?;
                    let held = S::issue_tags_in(transaction, id).await?;
                    let judged = match judge_tag(&name, &tags, &limits, &held, &chain) {
                        Ok(judged) => judged,
                        Err(code) => return Err(stopped.stop::<S>(code)),
                    };
                    if judged.already {
                        return Ok(None);
                    }
                    let tag = if let Some(tag) = judged.tag {
                        tag
                    } else {
                        let made = Tag {
                            id: TagId::generate(&mut ids)
                                .map_err(|_| stopped.stop::<S>(ErrorCode::Internal))?,
                            name: name.clone(),
                            color,
                            created_at: now,
                            version: bb_core::version::FIRST_VERSION,
                        };
                        S::insert_tag(transaction, &made).await?;
                        made.id
                    };
                    let version =
                        count_up_and_record::<S>(transaction, &maker, id, names, now, &stopped)
                            .await?;
                    S::insert_issue_tag(
                        transaction,
                        &IssueTag {
                            issue: id,
                            tag,
                            added_by: person,
                            added_at: now,
                        },
                    )
                    .await?;
                    Ok(Some(version))
                })
            })
            .await
            .map_err(|error| why.answer::<S>(&error, ErrorCode::TagNotHere))?;
        if let Some(version) = counted {
            issue.version = version;
        }
    }
    let view = ctx.labels_and_tags_of(&held, &issue).await?;
    Ok(held.authorize(view))
}

/// Takes a tag off an issue. A tag that is on no issue any more is not offered again.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist or that the person may not see, or a tag
/// the issue does not have, `access.forbidden` without the right to change the issue or its field
/// *Tags*.
pub async fn remove_tag<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RemoveTag,
) -> Result<Authorized<IssueLabelsAndTags>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let Kept { mut issue, held } = ctx.issue_to_sort(&actor, &input.issue, Field::Tags).await?;
    let state = ctx.tag_state(issue.id).await?;
    if !state.held.iter().any(|link| link.tag == input.tag) {
        return Err(AppError::Refused(ErrorCode::NotFound));
    }
    let before = tag_names(&state.tags, &state.held);
    let rest: Vec<IssueTag> = state
        .held
        .iter()
        .filter(|link| link.tag != input.tag)
        .cloned()
        .collect();
    let after = tag_names(&state.tags, &rest);
    let names = names_changed(member::TAGS, &before, &after);
    let maker = ctx.event_maker(by(actor.person.id));
    let (id, tag, why, now) = (issue.id, input.tag, Why::default(), ctx.now());
    let stopped = why.clone();
    issue.version = ctx
        .store()
        .record_changes(&[], move |transaction| {
            Box::pin(async move {
                let taken_off = S::delete_issue_tag(transaction, id, tag).await?;
                counted_after_take_off::<S>(
                    transaction,
                    &maker,
                    taken_off,
                    id,
                    names,
                    now,
                    &stopped,
                )
                .await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    let view = ctx.labels_and_tags_of(&held, &issue).await?;
    Ok(held.authorize(view))
}

/// Puts a label on an issue. In a *Pick one* group the label takes the place of the one the issue
/// has from that group. An issue that has the label already is left as it is.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist or that the person may not see, or a
/// label that is not there, `access.forbidden` without the right to change the issue or its field
/// *Labels*, `request.invalid` for a retired label, `limit.reached` for an issue that has as many
/// labels as it may.
pub async fn add_label<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AddLabel,
) -> Result<Authorized<IssueLabelsAndTags>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let mut plan = ctx.plan_label(&actor, &input.issue, input.label).await?;
    let (_, held) = ctx.visible_issue(&actor, &input.issue).await?;
    if !plan.is_done_already() {
        let names = names_changed(member::LABELS, &plan.before, &plan.after);
        let maker = ctx.event_maker(by(actor.person.id));
        let (id, label, person, now) = (plan.issue.id, input.label, actor.person.id, ctx.now());
        let why = Why::default();
        let stopped = why.clone();
        let counted = ctx
            .store()
            .record_changes(&[], move |transaction| {
                Box::pin(async move {
                    S::lock_tags_and_labels(transaction).await?;
                    // Judged again on what is stored now.
                    let labels = S::labels_in(transaction).await?;
                    let groups = S::label_groups_in(transaction).await?;
                    let held = S::issue_labels_in(transaction, id).await?;
                    let Some(stored) = S::issue_in(transaction, id).await? else {
                        return Err(stopped.stop::<S>(ErrorCode::NotFound));
                    };
                    let offers = group_offers_in::<S>(transaction, stored.project).await?;
                    let judged = match judge_label(label, &labels, &groups, &held, &offers) {
                        Ok(judged) => judged,
                        Err(code) => return Err(stopped.stop::<S>(code)),
                    };
                    if judged.already {
                        return Ok(None);
                    }
                    let version =
                        count_up_and_record::<S>(transaction, &maker, id, names, now, &stopped)
                            .await?;
                    for replaced in &judged.replaces {
                        S::delete_issue_label(transaction, id, *replaced).await?;
                    }
                    S::insert_issue_label(
                        transaction,
                        &IssueLabel {
                            issue: id,
                            label,
                            added_by: person,
                            added_at: now,
                        },
                    )
                    .await?;
                    Ok(Some(version))
                })
            })
            .await
            .map_err(|error| why.answer::<S>(&error, ErrorCode::InvalidRequest))?;
        if let Some(version) = counted {
            plan.issue.version = version;
        }
    }
    let view = ctx.labels_and_tags_of(&held, &plan.issue).await?;
    Ok(held.authorize(view))
}

/// Takes a label off an issue.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist or that the person may not see, or a
/// label the issue does not have, `access.forbidden` without the right to change the issue or its
/// field *Labels*.
pub async fn remove_label<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RemoveLabel,
) -> Result<Authorized<IssueLabelsAndTags>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let Kept { mut issue, held } = ctx
        .issue_to_sort(&actor, &input.issue, Field::Labels)
        .await?;
    let state = ctx.label_state(issue.id).await?;
    if !state.held.iter().any(|link| link.label == input.label) {
        return Err(AppError::Refused(ErrorCode::NotFound));
    }
    let before = label_names(&state.labels, &state.held);
    let rest: Vec<IssueLabel> = state
        .held
        .iter()
        .filter(|link| link.label != input.label)
        .cloned()
        .collect();
    let after = label_names(&state.labels, &rest);
    let names = names_changed(member::LABELS, &before, &after);
    let maker = ctx.event_maker(by(actor.person.id));
    let (id, label, why, now) = (issue.id, input.label, Why::default(), ctx.now());
    let stopped = why.clone();
    issue.version = ctx
        .store()
        .record_changes(&[], move |transaction| {
            Box::pin(async move {
                let taken_off = S::delete_issue_label(transaction, id, label).await?;
                counted_after_take_off::<S>(
                    transaction,
                    &maker,
                    taken_off,
                    id,
                    names,
                    now,
                    &stopped,
                )
                .await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    let view = ctx.labels_and_tags_of(&held, &issue).await?;
    Ok(held.authorize(view))
}
