//! The search of a view (PRJ-06, ADR-0034, amendment of 2026-11-24): every view (the timeline, the
//! board, the calendar) reads a saved search or a search text, and a project's view covers the
//! project and everything below it. The text is the one source of what a view shows: *Only this
//! project* is written into it as `project is KEY` in place of `project within KEY`, never kept as
//! an option beside it (M10-Q18).

use bb_core::ErrorCode;
use bb_core::id::SavedSearchId;
use bb_core::names::same_name;
use bb_core::saved_search::SavedSearch;
use bb_policy::Place as Where;
use bb_searchlang::{
    Expr, FieldRef, ProjectRef, Search, Span, Test, canonical_text, check, parse_with,
};

use super::readable::{SearchScope, readable_of};
use super::run::{Prepared, Shape};
use super::target::{FieldTarget, target_of};
use crate::access::{Actor, place_of_address};
use crate::auth::{AppError, AuthStore, Ctx};

/// What a view shows: a saved search, or a search text that need not be saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewSearch {
    /// A saved search, by its ID.
    Saved(SavedSearchId),
    /// A search text, which need not be saved.
    Text(String),
}

/// The search of a view, prepared to run.
pub(crate) struct ViewPrepared {
    /// The text that is run: for a project's view with its project written in (`project within
    /// KEY` or `project is KEY`).
    pub(crate) text: String,
    /// The search, checked for the person.
    pub(crate) prepared: Prepared,
    /// The saved search the view shows, when it shows one.
    pub(crate) saved: Option<SavedSearch>,
}

/// Whether `expr` is a test of `word`, the project field, on the project named `key`, in either
/// form a project's view writes.
fn is_project_test(fields: &[super::Searchable], expr: &Expr, key: &str) -> bool {
    let Expr::Field { field, test, .. } = expr else {
        return false;
    };
    if target_of(fields, &field.word) != Some(FieldTarget::Project) {
        return false;
    }
    match test {
        Test::Within { project, .. } => same_name(&project.text, key),
        Test::Is {
            value: bb_searchlang::Operand::Text { text, .. },
            ..
        } => same_name(text, key),
        _ => false,
    }
}

/// `search` with the project of a view written into its filter: `project within KEY`, or `project
/// is KEY` for *Only this project*, in place of either one that names the project at the top of
/// the filter. The rest of the search stays as it is.
#[must_use]
pub(crate) fn scoped_to(
    mut search: Search,
    fields: &[super::Searchable],
    (word, key): (&str, &str),
    only_this_project: bool,
) -> Search {
    let span = Span::default();
    let field = FieldRef {
        word: word.to_owned(),
        span,
    };
    let test = if only_this_project {
        Test::Is {
            value: bb_searchlang::Operand::Text {
                text: key.to_owned(),
                span,
            },
            span,
        }
    } else {
        Test::Within {
            project: ProjectRef {
                text: key.to_owned(),
                span,
            },
            span,
        }
    };
    let mut items = match search.filter.take() {
        Some(Expr::And { items, .. }) => items,
        Some(other) => vec![other],
        None => Vec::new(),
    };
    items.retain(|item| !is_project_test(fields, item, key));
    items.insert(0, Expr::Field { field, test, span });
    search.filter = Some(if items.len() == 1 {
        items.remove(0)
    } else {
        Expr::And { items, span }
    });
    search
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The text of a view's search: the text given, or the text of the saved search.
    ///
    /// # Errors
    /// `resource.not-found` for a saved search the person may not open.
    pub(crate) async fn view_search_text(
        &self,
        actor: &Actor,
        search: &ViewSearch,
    ) -> Result<String, AppError> {
        Ok(self.view_search(actor, search).await?.0)
    }

    /// The text of a view's search, and the saved search it is, if it is one.
    async fn view_search(
        &self,
        actor: &Actor,
        search: &ViewSearch,
    ) -> Result<(String, Option<SavedSearch>), AppError> {
        Ok(match search {
            ViewSearch::Text(text) => (text.clone(), None),
            ViewSearch::Saved(id) => {
                let saved = self.open_search(actor, *id).await?.stored.search;
                (saved.text.clone(), Some(saved))
            }
        })
    }

    /// Where a view's search looks: everywhere the person sees, or a project (by ID or short name)
    /// and everything below it, with the project's short name.
    ///
    /// # Errors
    /// `resource.not-found` for a project the person may not see.
    pub(crate) async fn view_scope(
        &self,
        project: Option<&str>,
    ) -> Result<(SearchScope, Option<String>), AppError> {
        let Some(address) = project else {
            return Ok((SearchScope::Everywhere, None));
        };
        let found = self.find_project(address).await?;
        let key = found
            .as_ref()
            .map(|found| found.project.short_name.to_string());
        match place_of_address(found.as_ref().map(|found| found.project.id))? {
            Where::Project(project) => Ok((SearchScope::Within(project), key)),
            Where::Organization => Err(AppError::Refused(ErrorCode::NotFound)),
        }
    }

    /// The search of a board or a calendar, prepared: what the person may search by is read once,
    /// and for a project's view the project is written into the text by [`scoped_to`], so the
    /// answer can say which text it ran. A saved search comes with what it keeps of its views.
    ///
    /// # Errors
    /// As [`Ctx::view_search_text`] and [`Ctx::view_scope`]; the mistakes of the text.
    pub(crate) async fn prepare_view(
        &self,
        actor: &Actor,
        search: &ViewSearch,
        (project, only_this_project): (Option<&str>, bool),
    ) -> Result<ViewPrepared, AppError> {
        let (text, saved) = self.view_search(actor, search).await?;
        let (scope, key) = self.view_scope(project).await?;
        let looked = self.looked_at(actor, scope, None).await?;
        let text = match key {
            None => text,
            Some(key) => {
                let readable = readable_of(&looked.fields);
                let parsed = parse_with(&text, &readable)
                    .map_err(|mistake| AppError::Mistakes(vec![mistake]))?;
                let word = looked
                    .fields
                    .iter()
                    .find(|each| each.target == FieldTarget::Project)
                    .map(|each| each.field.word.clone())
                    .ok_or(AppError::Refused(ErrorCode::NotFound))?;
                let scoped = scoped_to(parsed, &looked.fields, (&word, &key), only_this_project);
                let checked = check(scoped, &readable).map_err(AppError::Mistakes)?;
                canonical_text(checked.search()).ok_or_else(|| {
                    AppError::Internal("a view's search has no canonical text".into())
                })?
            }
        };
        let prepared = self
            .prepare_in(actor, looked, &text, Shape::default(), |_| None)
            .await?;
        Ok(ViewPrepared {
            text,
            prepared,
            saved,
        })
    }
}

#[cfg(test)]
mod tests {
    use bb_searchlang::{canonical_text, parse};

    use super::{FieldTarget, scoped_to};
    use crate::search::Searchable;

    fn fields() -> Vec<Searchable> {
        vec![Searchable {
            field: bb_searchlang::Field::new("project", bb_searchlang::FieldKind::Project),
            target: FieldTarget::Project,
        }]
    }

    fn written(text: &str, only: bool) -> Option<String> {
        let search = parse(text).ok()?;
        canonical_text(&scoped_to(search, &fields(), ("project", "WEB"), only))
    }

    #[test]
    fn prj_06_a_project_view_writes_its_project_into_the_text_and_only_this_project_narrows_it() {
        assert_eq!(written("", false).as_deref(), Some("project within WEB"));
        assert_eq!(written("", true).as_deref(), Some("project is WEB"));
        assert_eq!(
            written("status is Open", false).as_deref(),
            Some("project within WEB and status is Open")
        );
        // The choice changes the one test of the project, and keeps the rest.
        assert_eq!(
            written("project within web and priority is High", true).as_deref(),
            Some("project is WEB and priority is High")
        );
        assert_eq!(
            written("project is WEB and priority is High", false).as_deref(),
            Some("project within WEB and priority is High")
        );
        // Another project in the text is left as it is: the text stays what its writer asked.
        assert_eq!(
            written("project is APP or status is Open", true).as_deref(),
            Some("project is WEB and (project is APP or status is Open)")
        );
    }
}
