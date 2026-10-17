//! What *Publish* would drop (WF-02, M4-F42, M4-F43): before the editor asks where the issues of a
//! dropped status go, it learns which statuses are dropped and how many issues are in each.
//!
//! It judges the workflow with the code of *Publish* itself (`publishable`, `dropping`,
//! `mover_of`), so the preview and the publish cannot disagree. The issues the person sees are
//! named as a list shows them; the others are only counted.

use bb_core::many_changes::MANY_CHANGES_ISSUES_MAX;
use bb_policy::Authorized;

use super::publish::{Drafts, Dropping, Mover};
use super::types::{
    DroppedStatus, PREVIEW_NAMED_MAX, PublishPreview, PublishPreviewQuery, StatusView,
};
use crate::auth::{AppError, AuthStore, Ctx};

/// Says which statuses publishing a workflow would drop, and which issues are in them. Needs
/// *Manage workflows* wherever the workflow is used. It changes nothing and takes no lock.
///
/// # Errors
/// `resource.not_found` for a workflow that is not there, `access.forbidden` without *Manage
/// workflows* where it is used, `workflow.invalid` for a workflow that breaks a rule or a rule
/// that names a field or a group the person may not know, `field.not_here` for a rule that names
/// an own field that issues do not have.
pub async fn publish_preview<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PublishPreviewQuery,
) -> Result<Authorized<PublishPreview>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (stored, held, _) = ctx.workflow_to_manage(&actor, input.workflow).await?;
    let drafts = Drafts {
        name: &stored.workflow.name,
        statuses: &input.statuses,
        changes: &input.changes,
    };
    let content = ctx.publishable(&actor, &held, &stored, drafts).await?;
    let Dropping { statuses, issues } = ctx.dropping(&stored, &content).await?;
    let mut maker = ctx.line_maker().await?;
    let mut dropped = Vec::with_capacity(statuses.len());
    for key in statuses {
        let Some(status) = stored.content.status(&key) else {
            continue;
        };
        let mut shown = DroppedStatus {
            status: StatusView::from(status),
            issues: 0,
            named: Vec::new(),
            unseen: 0,
            blocked: 0,
        };
        for issue in issues.iter().filter(|each| each.status == key) {
            shown.issues += 1;
            match ctx.mover_of(&actor, issue).await? {
                Mover::Unseen => {
                    shown.unseen += 1;
                    shown.blocked += 1;
                }
                Mover::Blocked => shown.blocked += 1,
                Mover::Can => {}
            }
            if shown.named.len() < PREVIEW_NAMED_MAX
                && let Some(line) = ctx.line_of(&actor, &mut maker, issue).await?
            {
                shown.named.push(line);
            }
        }
        dropped.push(shown);
    }
    Ok(held.authorize(PublishPreview {
        dropped,
        limit: MANY_CHANGES_ISSUES_MAX,
    }))
}
