//! The read tool `relation_map` (ISS-16, AI-21, M10-Q20): the use case of the relation map, run as
//! the actor with what is kept from AI left out of the walk itself, so a chain never passes through
//! a project kept from AI either. Each issue is a part of its own, whose title is fenced as text
//! written by somebody the AI does not know (ADR-0029); the lines are one part more.

use serde_json::{Value, json};

use super::inputs::RelationMapInput;
use super::read::{masked, project_of};
use super::registry::{Boxed, Run};
use crate::ai::context::{AuthorKind, Part, drop_withheld};
use crate::auth::{AppError, AuthStore, Ctx};
use crate::issues::IssueView;
use crate::relations::{IssueMapQuery, KindsShown, SearchMapQuery};
use bb_core::ErrorCode;

/// The member of a map that holds its issues.
const ISSUES: &str = "issues";

impl<S: AuthStore> Ctx<'_, S> {
    /// `relation_map`: of an issue, or of a search, never both.
    async fn tool_relation_map(
        &self,
        run: &Run<'_>,
        input: RelationMapInput,
    ) -> Result<Vec<Part>, AppError> {
        let shown = KindsShown {
            kinds: None,
            part_of: input.part_of.unwrap_or(true),
        };
        let map = match (input.issue, input.query) {
            (Some(issue), None) => {
                let query = IssueMapQuery {
                    session_token: run.token.to_owned(),
                    issue,
                    steps: input.steps,
                    shown,
                };
                self.issue_map_for(run.actor, &query, Some(run.withheld))
                    .await?
            }
            (None, Some(text)) if input.steps.is_none() => {
                let query = SearchMapQuery {
                    session_token: run.token.to_owned(),
                    text,
                    project: None,
                    shown,
                };
                self.search_map_for(run.actor, &query, Some(run.withheld))
                    .await?
            }
            _ => return Err(AppError::Refused(ErrorCode::InvalidRequest)),
        };
        let mut whole = masked(&map)?;
        let mut parts = Vec::new();
        if let Some(Value::Array(issues)) = whole.get_mut(ISSUES).map(Value::take) {
            for mut each in issues {
                let project = each.get("issue").and_then(project_of);
                if let Some(issue) = each.get_mut("issue") {
                    drop_withheld::<IssueView>(issue, run.withheld);
                }
                parts.push(Part {
                    name: "issue",
                    value: each,
                    author: AuthorKind::Outsider,
                    project,
                });
            }
        }
        if let Some(object) = whole.as_object_mut() {
            object.remove(ISSUES);
        }
        parts.push(Part {
            name: "map",
            value: json!(whole),
            author: AuthorKind::Member,
            project: None,
        });
        Ok(parts)
    }
}

/// `relation_map`.
pub(super) fn relation_map<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: RelationMapInput = run.input()?;
        ctx.tool_relation_map(&run, input).await
    })
}
