//! Whether a saved search checks cleanly for a person (ADR-0034, decisions 3 and 4): the text is
//! read and checked with the fields the person may read, the groups and projects it names are ones
//! they may see, and the columns and the grouping name fields they may read. A field, a person, a
//! group or a project the person may not see is as unknown as one that does not exist, so nothing
//! of a search that does not check cleanly reaches them.

use std::collections::HashMap;

use bb_core::access::Group;
use bb_core::names::same_name;
use bb_core::saved_search::Presentation;
use bb_policy::Permission;
use bb_searchlang::{Expr, FieldKind, Operand, ReadableFields, Test, check, parse_with, print};

use super::reach::Reaching;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::search::SearchScope;

/// What a text names that stands outside the fields: groups by name, projects by short name.
#[derive(Default)]
struct Named {
    groups: Vec<String>,
    projects: Vec<String>,
}

/// The project a word names when it is written like an Issue ID (`WEB-142`): its short name.
fn project_of_issue(id: &str) -> &str {
    id.rsplit_once('-').map_or(id, |(project, _)| project)
}

/// Collects the groups and projects a filter names.
fn named_in(filter: &Expr, fields: &ReadableFields) -> Named {
    let mut named = Named::default();
    filter.each_filter(&mut |each| match each {
        Expr::Field { field, test, .. } => {
            let is_project = fields
                .field(&field.word)
                .is_some_and(|found| found.kind == FieldKind::Project);
            let operands: Vec<&Operand> = match test {
                Test::Is { value, .. }
                | Test::IsNot { value, .. }
                | Test::Compare { value, .. } => vec![value],
                Test::In { values, .. } => values.iter().collect(),
                _ => Vec::new(),
            };
            for operand in operands {
                match operand {
                    Operand::Text { text, .. } if is_project => named.projects.push(text.clone()),
                    Operand::Issue { id, .. } => {
                        named.projects.push(project_of_issue(&id.text).to_owned());
                    }
                    _ => {}
                }
            }
            match test {
                Test::MemberOf { group, .. } => named.groups.push(group.text.clone()),
                Test::Within { project, .. } => named.projects.push(project.text.clone()),
                _ => {}
            }
        }
        Expr::Under { id, .. }
        | Expr::Blocks { id, .. }
        | Expr::BlockedBy { id, .. }
        | Expr::ConnectedTo { id, .. } => {
            named.projects.push(project_of_issue(&id.text).to_owned());
        }
        Expr::Or { .. } | Expr::And { .. } | Expr::Not { .. } => {}
    });
    named
}

/// Judges saved searches for one person, reading what it needs once for the request.
pub(crate) struct Judge<'a> {
    reaching: &'a Reaching,
    fields: ReadableFields,
    manages_groups: bool,
    groups: Vec<Group>,
    projects: HashMap<String, bool>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// A judge for the actor, with the fields they may read everywhere.
    pub(crate) async fn judge<'a>(
        &self,
        actor: &Actor,
        reaching: &'a Reaching,
    ) -> Result<Judge<'a>, AppError> {
        let fields = self
            .readable_fields_in(&reaching.shown, SearchScope::Everywhere)
            .await?;
        // Read whatever the answer, so that a search that is there costs what one that is not does.
        let groups = self
            .store()
            .groups()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let manages_groups = self
            .evaluation_of(actor, bb_policy::Place::Organization)
            .await?
            .allows(Permission::ManageGroups);
        Ok(Judge {
            reaching,
            fields,
            manages_groups,
            groups,
            projects: HashMap::new(),
        })
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The text in its canonical spelling when the text and the presentation check cleanly for the
    /// person `judge` is for; `None` when they do not.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn canonical_text(
        &self,
        judge: &mut Judge<'_>,
        text: &str,
        presentation: &Presentation,
    ) -> Result<Option<String>, AppError> {
        if presentation
            .fields()
            .any(|word| judge.fields.field(word).is_none())
        {
            return Ok(None);
        }
        let Ok(tree) = parse_with(text, &judge.fields) else {
            return Ok(None);
        };
        let Ok(checked) = check(tree, &judge.fields) else {
            return Ok(None);
        };
        let named = checked
            .search()
            .filter
            .as_ref()
            .map(|filter| named_in(filter, &judge.fields))
            .unwrap_or_default();
        for group in &named.groups {
            if !self.sees_group(judge, group).await? {
                return Ok(None);
            }
        }
        for project in &named.projects {
            if !self.sees_project(judge, project).await? {
                return Ok(None);
            }
        }
        Ok(Some(print(checked.search())))
    }

    /// Whether the person sees a group of this name: they are in it, or may manage groups.
    async fn sees_group(&self, judge: &mut Judge<'_>, name: &str) -> Result<bool, AppError> {
        Ok(judge.groups.iter().any(|group| {
            same_name(&group.name, name) && (judge.manages_groups || judge.reaching.is_in(group.id))
        }))
    }

    /// Whether the person sees a project with this short name (or an old one).
    async fn sees_project(
        &self,
        judge: &mut Judge<'_>,
        short_name: &str,
    ) -> Result<bool, AppError> {
        if let Some(known) = judge.projects.get(short_name) {
            return Ok(*known);
        }
        let sees = self
            .find_project(short_name)
            .await?
            .is_some_and(|found| judge.reaching.shown.sees(found.project.id));
        judge.projects.insert(short_name.to_owned(), sees);
        Ok(sees)
    }
}
