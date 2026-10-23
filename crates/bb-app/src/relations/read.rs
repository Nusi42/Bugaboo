//! The sub-issues, the issue it sits under and the related issues of an issue, as one viewer sees
//! them (ISS-03, ISS-04, PRIV-13). An issue the viewer may not see is left out of every list and of
//! the progress, and nothing in the answer says that something was left out.

use bb_core::ErrorCode;
use bb_core::issue::{Issue, StatusGroup};
use bb_core::paging::window;
use bb_core::relation::Relation;
use bb_policy::{Authorized, Evaluation};

use super::types::{
    IssueLine, IssueRelations, IssueRelationsQuery, Progress, RelatedIssuePage, RelatedIssuesQuery,
    RelationView, SubIssuePage, SubIssuesQuery,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Everything the viewer may see of how an issue stands among the others.
pub(crate) struct Standing {
    pub(crate) header: IssueRelations,
    pub(crate) sub_issues: Vec<IssueLine>,
    pub(crate) related: Vec<RelationView>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// How `issue` stands among the others for the actor, whose evaluation at the project of the
    /// issue is `held`.
    pub(crate) async fn standing(
        &self,
        actor: &Actor,
        held: &Evaluation,
        issue: &Issue,
    ) -> Result<Standing, AppError> {
        let store = self.store();
        let failed = |error| internal::<S>(&error);
        let mut maker = self.line_maker().await?;
        // The issue it sits under.
        let part_of = match store.parent_of(issue.id).await.map_err(failed)? {
            Some(above) => match self.issues_named(&[above.parent]).await?.first() {
                Some(parent) => self.line_of(actor, &mut maker, parent).await?,
                None => None,
            },
            None => None,
        };
        // The issues under it, only those the viewer may see, in the order they were put there.
        let below = store.sub_issues_of(issue.id).await.map_err(failed)?;
        let ids: Vec<_> = below.iter().map(|child| child.child).collect();
        let found = self.issues_named(&ids).await?;
        let mut sub_issues = Vec::new();
        for id in &ids {
            let Some(child) = found.iter().find(|candidate| candidate.id == *id) else {
                continue;
            };
            if let Some(line) = self.line_of(actor, &mut maker, child).await? {
                sub_issues.push(line);
            }
        }
        // Progress is told only when the viewer may see the status of every sub-issue they see, so
        // it never tells what a hidden status holds.
        let progress =
            if sub_issues.is_empty() || sub_issues.iter().any(|line| line.status.is_none()) {
                None
            } else {
                let count = |lines: &[&IssueLine]| u32::try_from(lines.len()).unwrap_or(u32::MAX);
                let done: Vec<&IssueLine> = sub_issues
                    .iter()
                    .filter(|line| {
                        line.status
                            .as_ref()
                            .is_some_and(|status| status.group == StatusGroup::Done)
                    })
                    .collect();
                Some(Progress {
                    done: count(&done),
                    total: u32::try_from(sub_issues.len()).unwrap_or(u32::MAX),
                })
            };
        // The related issues, by kind in the order of the kinds, then oldest first.
        let kinds = store.relation_kinds().await.map_err(failed)?;
        let mut relations = store.relations_of(issue.id).await.map_err(failed)?;
        let position = |relation: &Relation| {
            kinds
                .iter()
                .position(|kind| kind.id == relation.kind)
                .unwrap_or(usize::MAX)
        };
        relations.sort_by_key(|relation| (position(relation), relation.added_at, relation.id));
        let others: Vec<_> = relations
            .iter()
            .filter_map(|relation| relation.other_end(issue.id))
            .collect();
        let found = self.issues_named(&others).await?;
        let mut related = Vec::new();
        for relation in &relations {
            let (Some(other), Some(kind), Some(direction)) = (
                relation.other_end(issue.id),
                kinds.iter().find(|kind| kind.id == relation.kind),
                relation.direction_of(issue.id),
            ) else {
                continue;
            };
            let Some(other) = found.iter().find(|candidate| candidate.id == other) else {
                continue;
            };
            if let Some(line) = self.line_of(actor, &mut maker, other).await? {
                related.push(RelationView {
                    id: relation.id,
                    kind: kind.id,
                    direction,
                    name: kind.name(direction).to_owned(),
                    issue: line,
                });
            }
        }
        Ok(Standing {
            header: IssueRelations {
                issue: issue.id,
                version: issue.version,
                part_of,
                progress,
                may_change: held.may_change_issue(issue.reported_by),
            },
            sub_issues,
            related,
        })
    }
}

/// How an issue stands among the others: the issue it sits under and the progress of its
/// sub-issues.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see.
pub async fn issue_relations<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: IssueRelationsQuery,
) -> Result<Authorized<IssueRelations>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let standing = ctx.standing(&actor, &held, &issue).await?;
    Ok(held.authorize(standing.header))
}

/// The sub-issues of an issue that the person may see, a page at a time.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see, `request.invalid` for a cursor that is not one.
pub async fn sub_issues<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SubIssuesQuery,
) -> Result<Authorized<SubIssuePage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let standing = ctx.standing(&actor, &held, &issue).await?;
    let page = window(
        standing.sub_issues.len(),
        input.cursor.as_deref(),
        input.limit,
    )
    .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(held.authorize(SubIssuePage {
        items: standing.sub_issues[page.start..page.end].to_vec(),
        next_cursor: page.next,
    }))
}

/// The issues related to an issue whose other end the person may see, a page at a time.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see, `request.invalid` for a cursor that is not one.
pub async fn related_issues<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RelatedIssuesQuery,
) -> Result<Authorized<RelatedIssuePage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let standing = ctx.standing(&actor, &held, &issue).await?;
    let page = window(standing.related.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(held.authorize(RelatedIssuePage {
        items: standing.related[page.start..page.end].to_vec(),
        next_cursor: page.next,
    }))
}
