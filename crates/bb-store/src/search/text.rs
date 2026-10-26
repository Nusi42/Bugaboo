//! `contains`: the words of a text in the titles, the descriptions or one long-text field of the
//! issues, through the port of the text search (ADR-0016, amendment M5).
//!
//! The port matches and nothing else. The visibility is the statement's own: the condition is a
//! test on `issue.id` inside the same statement that starts from the sight of the person, so a
//! hit never comes from an issue they do not read. For a field of the issues the condition adds,
//! on the issue itself, that the field is readable where the issue is (its project, its type), and
//! the words and the scope reach the database only as bound values.

use bb_app::search::FieldTarget;
use bb_core::own_fields::FieldType;
use bb_core::text_index::TextSource;
use bb_core::text_words::search_words;
use bb_searchlang::Text;
use sea_query::{Alias, Cond, Expr, ExprTrait as _, Query};

use super::{CompileError, Compiler, nothing};
use crate::db::StoredId as _;
use crate::schema::IssueRow;
use crate::text_index::{TextMatch, TextScope, TextSearch, text_matches};

impl Compiler<'_> {
    /// The issues whose text, in the part the field stands for, has every word of `text`.
    pub(super) fn contains(&self, target: FieldTarget, text: &Text) -> Result<Cond, CompileError> {
        let scope = match target {
            FieldTarget::Title => TextScope::Source(TextSource::Title),
            FieldTarget::Description => TextScope::Source(TextSource::Description),
            FieldTarget::Own {
                field,
                kind: FieldType::LongText,
            } => TextScope::Field(field),
            _ => {
                return Err(CompileError::Unsupported(
                    "contains on a field whose text is not indexed",
                ));
            }
        };
        let words = search_words(&text.text);
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
        let mut cond =
            Cond::all().add(Expr::col((IssueRow::Table, IssueRow::Id)).in_subquery(found));
        if let TextScope::Field(field) = scope {
            cond = cond.add(self.reads_field_here(field));
        }
        Ok(cond)
    }

    /// The field is readable where the issue is: in its project, and for its type when the field is
    /// limited to some types. A field that reaches nowhere is read nowhere.
    fn reads_field_here(&self, field: bb_core::id::FieldId) -> Cond {
        let Some(reach) = self
            .request
            .own_reach
            .iter()
            .find(|each| each.field == field)
        else {
            return nothing();
        };
        let mut cond = Cond::all().add(
            Expr::col((IssueRow::Table, IssueRow::Project))
                .is_in(reach.projects.iter().map(|project| project.stored())),
        );
        if let Some(types) = &reach.issue_types {
            cond = cond.add(
                Expr::col((IssueRow::Table, IssueRow::IssueType))
                    .is_in(types.iter().map(|kind| kind.stored())),
            );
        }
        cond
    }
}
