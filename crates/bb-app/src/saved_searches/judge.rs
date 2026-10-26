//! Whether a saved search checks cleanly for a person (ADR-0034, decisions 3 and 4): the text is
//! read and checked with the fields the person may read, the groups and projects it names are ones
//! they may see, and the columns and the grouping name fields they may read. A field, a person, a
//! group or a project the person may not see is as unknown as one that does not exist, so nothing
//! of a search that does not check cleanly reaches them.

use bb_core::saved_search::Presentation;
use bb_searchlang::{ReadableFields, check, parse_with, print};

use super::reach::Reaching;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::search::{Looked, SearchScope, readable_of};

/// Judges saved searches for one person, reading what it needs once for the request: what the
/// person may search by and what the names of a text can stand for (`looked_at`, the same read as
/// a search that is typed has).
pub(crate) struct Judge<'a> {
    actor: &'a Actor,
    looked: Looked,
    fields: ReadableFields,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// A judge for the actor, with the fields they may read everywhere, from the view of the tree
    /// that `reaching` holds.
    pub(crate) async fn judge<'a>(
        &self,
        actor: &'a Actor,
        reaching: &Reaching,
    ) -> Result<Judge<'a>, AppError> {
        let looked = self
            .looked_in(reaching.shown.clone(), SearchScope::Everywhere)
            .await?;
        let fields = readable_of(&looked.fields);
        Ok(Judge {
            actor,
            looked,
            fields,
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
        // The names of the text (groups, projects, issues, people, statuses, ...) are resolved by
        // the one rule of the search; a name the person cannot know is as unknown as a missing one.
        let names = self
            .search_names(judge.actor, &judge.looked, &checked)
            .await?;
        if names.names_unknown() {
            return Ok(None);
        }
        Ok(Some(print(checked.search())))
    }
}
