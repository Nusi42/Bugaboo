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
use sea_query::{Cond, Expr, ExprTrait as _};

use super::source::reach_condition;
use super::{CompileError, Compiler};
use crate::schema::IssueRow;
use crate::text_index::{TextScope, TextSearch, text_issues, text_of_issue};

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

    /// The issues that have every word of `text` in the texts of the scope, a test in the
    /// statement that starts from the sight, so the comments and the file names of an issue the
    /// person does not read are never found. A page is cut after the first rows, so there the
    /// issues are asked about one at a time (`EXISTS`, which stops at the first one that has the
    /// words); a count or the groups ask about all of them (`IN`, the set made once).
    pub(super) fn words_in(&self, scope: TextScope, text: &str) -> Cond {
        let words = search_words(text);
        let search = TextSearch {
            words: &words,
            scope,
        };
        let issue = Expr::col((IssueRow::Table, IssueRow::Id));
        if self.paged {
            Cond::all().add(Expr::exists(text_of_issue(self.dialect, &search, issue)))
        } else {
            Cond::all().add(issue.in_subquery(text_issues(self.dialect, &search)))
        }
    }
}
