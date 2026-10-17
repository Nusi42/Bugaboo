//! The preview of a change to many issues (ISS-09, UX principle 6): what will happen to each issue
//! of the selection, before anything does.
//!
//! Every issue is judged on its own (`judge.rs`); the answer says, first, how many will change and
//! how many are left alone, then one row for each issue with what changes or why it is skipped,
//! and the version of the issue it read, which the change takes back.

use bb_core::ErrorCode;
use bb_core::many_changes::{MANY_CHANGES_ISSUES_MAX, MANY_CHANGES_JOB_ABOVE};
use bb_policy::{Authorized, Place};

use super::judge::{Judged, Planned, Probe, Selection};
use super::types::{
    ManyChangeAction, ManyChangeDetail, ManyChangePreview, ManyChangePreviewQuery, ManyChangeRow,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::history::ChangeReader;
use crate::preview::PreviewSummary;
use crate::relations::LineMaker;

impl<S: AuthStore> Ctx<'_, S> {
    /// What the whole selection shares, checked: the action asks for something, and what it names
    /// (the project, the label, the issue at the other end of a relation) is there for the person.
    ///
    /// # Errors
    /// `request.invalid` for nothing asked or a retired kind of relation, `resource.not_found` for
    /// a project, a label, a kind or an issue that is not there for the person.
    pub(super) async fn selection<'a>(
        &self,
        actor: &'a Actor,
        token: &'a str,
        action: &'a ManyChangeAction,
    ) -> Result<Selection<'a>, AppError> {
        let mut other = None;
        match action {
            ManyChangeAction::Change { change } => {
                if change.status.is_none()
                    && change.priority.is_none()
                    && change.assigned_to.is_none()
                    && change.fields.is_empty()
                {
                    return Err(ErrorCode::InvalidRequest.into());
                }
            }
            ManyChangeAction::Move { project } => {
                self.move_target(actor, project).await?;
            }
            ManyChangeAction::AddLabel { label } => {
                self.label_exists(*label).await?;
            }
            ManyChangeAction::AddRelation {
                relation_kind,
                other: address,
                ..
            } => {
                let kinds = self
                    .store()
                    .relation_kinds()
                    .await
                    .map_err(|error| crate::auth::internal::<S>(&error))?;
                let kind = kinds
                    .iter()
                    .find(|found| found.id == *relation_kind)
                    .ok_or(AppError::Refused(ErrorCode::NotFound))?;
                if kind.retired_at.is_some() {
                    return Err(ErrorCode::InvalidRequest.into());
                }
                other = Some(self.visible_issue(actor, address).await?.0.id);
            }
            ManyChangeAction::Delete => {}
        }
        Ok(Selection {
            actor,
            token,
            action,
            other,
        })
    }

    /// The details of a row of the preview, in the words of the history.
    async fn detail_of(
        &self,
        actor: &Actor,
        reader: &mut ChangeReader,
        planned: Planned,
        maker: &mut LineMaker,
    ) -> Result<Option<ManyChangeDetail>, AppError> {
        Ok(match planned {
            Planned::Fields(plan) => {
                let fields = self
                    .changes_as_read(reader, actor, &plan.held, &plan.issue, &plan.changes)
                    .await?;
                Some(ManyChangeDetail::Changes { fields })
            }
            Planned::Relation { name, other } => {
                self.line_of(actor, maker, &other)
                    .await?
                    .map(|issue| ManyChangeDetail::Relation {
                        name,
                        issue: Box::new(issue),
                    })
            }
            Planned::Trash => None,
        })
    }
}

/// The issues of a selection, each once and in the order asked, and refused when there are none or
/// too many.
///
/// # Errors
/// `request.invalid` for an empty selection or one with more issues than
/// [`MANY_CHANGES_ISSUES_MAX`].
pub(super) fn distinct<'a>(
    addresses: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<&'a str>, AppError> {
    let mut seen = std::collections::HashSet::new();
    let each: Vec<&str> = addresses
        .into_iter()
        .filter(|address| seen.insert(*address))
        .collect();
    if each.is_empty() || each.len() > MANY_CHANGES_ISSUES_MAX {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Ok(each)
}

/// Says what a change to many issues would do to each issue of the selection: what will change,
/// what is left alone and why, and the version of each issue, which the change takes back.
///
/// # Errors
/// `request.invalid` for no issue, too many issues, or an action that asks for nothing,
/// `resource.not_found` for a project, a label, a kind of relation or an issue named in the
/// action that is not there for the person.
pub async fn preview_many_changes<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ManyChangePreviewQuery,
) -> Result<Authorized<ManyChangePreview>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let addresses = distinct(input.issues.iter().map(String::as_str))?;
    let selection = ctx
        .selection(&actor, &input.session_token, &input.action)
        .await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let mut maker = ctx.line_maker().await?;
    let mut reader = ctx.change_reader().await?;
    let (mut will_change, mut skipped) = (0_usize, 0_usize);
    let mut rows = Vec::with_capacity(addresses.len());
    for address in &addresses {
        let probe = Probe {
            address,
            version: None,
        };
        let judgment = ctx.judge_item(&selection, &probe, &mut maker).await?;
        let (reason, detail) = match judgment.judged {
            Judged::Skip(reason) => {
                skipped += 1;
                (Some(reason), None)
            }
            Judged::Change(planned) => {
                will_change += 1;
                (
                    None,
                    ctx.detail_of(&actor, &mut reader, planned, &mut maker)
                        .await?,
                )
            }
        };
        rows.push(ManyChangeRow {
            address: (*address).to_owned(),
            issue: judgment.issue,
            version: judgment.version,
            skipped: reason,
            detail,
        });
    }
    Ok(held.authorize(ManyChangePreview {
        summary: PreviewSummary::of(will_change, skipped),
        rows,
        runs_in_background: addresses.len() > MANY_CHANGES_JOB_ABOVE,
    }))
}
