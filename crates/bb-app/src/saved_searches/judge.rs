//! Whether a saved search checks cleanly for a person (ADR-0034, decisions 3 and 4): the text is
//! read and checked with the fields the person may read, every name it holds (a person, a group, a
//! project, an Issue ID, a status, a type, a choice, a tag, a label) stands for something they may
//! know (`Ctx::search_names`), and the columns and the grouping name fields they may read. A field
//! or a name the person may not know is as unknown as one that does not exist, so nothing of a
//! search that does not check cleanly reaches them. The judge keeps what the names of the texts it
//! has judged stood for, so a list reads each group, tag, label and name once for the request.

use bb_core::saved_search::Presentation;
use bb_searchlang::{ReadableFields, check, parse_with, print};

use super::reach::Reaching;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::search::{Looked, NameCache, SearchScope, readable_of};

/// Judges saved searches for one person, reading what it needs once for the request: what the
/// person may search by and what the names of a text can stand for (`looked_at`, the same read as
/// a search that is typed has).
pub(crate) struct Judge<'a> {
    actor: &'a Actor,
    looked: Looked,
    fields: ReadableFields,
    names: NameCache,
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
            names: NameCache::default(),
        })
    }
}

impl Judge<'_> {
    /// Whether the last text needed a name past the bound of the request (see
    /// `NAME_RESOLUTIONS_MAX`): its answer cannot be used.
    pub(crate) const fn budget_spent(&self) -> bool {
        self.names.spent()
    }

    /// Lifts the bound for the rest of the request.
    pub(crate) const fn lift_budget(&mut self) {
        self.names.lift();
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
        self.search_names_in(judge.actor, &judge.looked, &checked, &mut judge.names)
            .await?;
        // Out of budget, a name was not resolved and the text has no answer: it does not check
        // (the walk of a list reads `budget_spent` and judges the search again with the bound
        // lifted, or ends its page before it), so a name that was not resolved is never let through.
        if judge.names.spent() || judge.names.names.names_unknown() {
            return Ok(None);
        }
        Ok(Some(print(checked.search())))
    }
}
