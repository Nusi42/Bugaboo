//! Whether a saved search checks cleanly for a person (ADR-0034, decisions 3 and 4): the text is
//! read and checked with the fields the person may read, every name it holds (a person, a group, a
//! project, an Issue ID, a status, a type, a choice, a tag, a label) stands for something they may
//! know (`Ctx::search_names`), and the columns and the grouping name fields they may read. The
//! options of a board and a calendar are held to the same rules: their fields must be ones the
//! person may read and of a kind the view takes, and the statuses and choices the card limits name
//! go through the same resolution of names (ADR-0034, amendment of 2026-11-24). A field or a name
//! the person may not know is as unknown as one that does not exist, so nothing of a search that
//! does not check cleanly reaches them. The judge keeps what the names of the texts it
//! has judged stood for, so a list reads each group, tag, label and name once for the request.

use bb_core::saved_search::{Presentation, ViewKind};
use bb_searchlang::{ReadableFields, canonical_text, check, parse_with};

use super::reach::Reaching;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::boards::{columns_by, named_by_limits};
use crate::plan::DateTargets;
use crate::plan::timeline_groups_by;
use crate::search::{Looked, NameCache, SearchScope, readable_of, target_of};

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
            .looked_in(reaching.shown.clone(), SearchScope::Everywhere, None)
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
        // The two fields a timeline takes its days from are fields a bar can take its day from,
        // by the one rule of the plan (ADR-0043, decision 2).
        let dates = presentation
            .timeline
            .as_ref()
            .and_then(|timeline| timeline.dates.as_ref());
        if dates.is_some_and(|(start, end)| {
            [start, end]
                .into_iter()
                .any(|word| DateTargets::of_word(&judge.looked.fields, word).is_none())
        }) {
            return Ok(None);
        }
        if !view_options_check(&judge.looked, presentation) {
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
        // The values the card limits name are names of the search like any other.
        if let Some(named) = presentation.board.as_ref().and_then(named_by_limits) {
            let Ok(limits) = check(named, &judge.fields) else {
                return Ok(None);
            };
            self.search_names_in(judge.actor, &judge.looked, &limits, &mut judge.names)
                .await?;
        }
        // Out of budget, a name was not resolved and the text has no answer: it does not check
        // (the walk of a list reads `budget_spent` and judges the search again with the bound
        // lifted, or ends its page before it), so a name that was not resolved is never let through.
        if judge.names.spent() || judge.names.names.names_unknown() {
            return Ok(None);
        }
        // Only a text that the language reads back as the same search is saved or run.
        Ok(canonical_text(checked.search()))
    }
}

/// Whether the fields the options of a board and a calendar name are of the kind the view takes for
/// the person: the columns a status group or a choice field of one value, the rows a field a view
/// groups by, the calendar's days a date field. Whether they may be read is asked with the other
/// fields of the presentation.
fn view_options_check(looked: &Looked, presentation: &Presentation) -> bool {
    // A search shown as a board needs its columns, *Status* included; another view keeps options
    // of a board only when a field they name is of the right kind.
    let shown_as_board = presentation.view == ViewKind::Board;
    let board_holds = presentation.board.as_ref().is_none_or(|board| {
        ((board.columns.is_none() && !shown_as_board) || columns_by(looked, board).is_some())
            && board
                .rows
                .as_deref()
                .is_none_or(|word| target_of(&looked.fields, word).is_some_and(timeline_groups_by))
    });
    let calendar_holds = presentation
        .calendar
        .as_ref()
        .and_then(|calendar| calendar.dates.as_deref())
        .is_none_or(|word| DateTargets::of_word(&looked.fields, word).is_some());
    board_holds && calendar_holds
}

#[cfg(test)]
mod tests {
    #[test]
    fn qry_01_the_server_makes_canonical_texts_with_the_round_trip_guard_on() {
        // The module of the browser is built without it; the server never is (ADR-0005, decision 8).
        const { assert!(bb_searchlang::ROUND_TRIP_GUARD) };
    }
}
