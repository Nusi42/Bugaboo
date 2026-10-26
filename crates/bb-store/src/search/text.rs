//! `contains`: the words of a text in the titles, the descriptions or one long-text field of the
//! issues, through the port of the text search (ADR-0016, amendment M5).
//!
//! The port matches and nothing else. The visibility is the statement's own: the condition is a
//! test on `issue.id` inside the same statement that starts from the sight of the person, so a
//! hit never comes from an issue they do not read. For a field of the issues the condition adds,
//! on the issue itself, that the field is readable where the issue is (its project, its type), and
//! the words and the scope reach the database only as bound values.

use bb_app::search::{FieldTarget, TextPlace};
use bb_core::text_index::TextSource;
use bb_core::text_words::search_words;
use bb_searchlang::Text;
use sea_query::{Alias, Cond, Expr, ExprTrait as _, Query};

use super::source::reach_condition;
use super::{CompileError, Compiler};
use crate::schema::IssueRow;
use crate::text_index::{TextMatch, TextScope, TextSearch, text_matches};

impl Compiler<'_> {
    /// The issues whose text, in the part the field stands for, has every word of `text`.
    pub(super) fn contains(&self, target: FieldTarget, text: &Text) -> Result<Cond, CompileError> {
        // The language offers `contains` only where `text_place` says the text is indexed.
        let scope = match target.text_place() {
            Some(TextPlace::Title) => TextScope::Source(TextSource::Title),
            Some(TextPlace::Description) => TextScope::Source(TextSource::Description),
            Some(TextPlace::Field(field)) => TextScope::Field(field),
            None => {
                return Err(CompileError::Unsupported(
                    "contains on a field whose text is not indexed",
                ));
            }
        };
        let mut cond = self.words_in(scope, &text.text);
        if let TextScope::Field(field) = scope {
            cond = cond.add(reach_condition(field, &self.request.own_reach));
        }
        Ok(cond)
    }

    /// The issues that have every word of `text` in the texts of the scope: a test on `issue.id`
    /// inside the statement that starts from the sight, so the comments and the file names of an
    /// issue the person does not read are never found.
    pub(super) fn words_in(&self, scope: TextScope, text: &str) -> Cond {
        let words = search_words(text);
        let matched = text_matches(
            self.dialect,
            &TextSearch {
                words: &words,
                scope,
            },
        );
        let mut found = Query::select();
        found
            .column(Alias::new(TextMatch::ISSUE))
            .from_subquery(matched.select, Alias::new("text_hit"));
        Cond::all().add(Expr::col((IssueRow::Table, IssueRow::Id)).in_subquery(found))
    }
}
