//! The visibility predicate of ADR-0004 as a condition on the table `issue`: the one place that says
//! which issues a person reads (SEC-06). Tags, offers and the search all build their statements on
//! it, so no statement has its own idea of what a person may see.

use bb_core::tags::IssueSight;
use sea_query::{Alias, Cond, DynIden, Expr, ExprTrait as _, IntoIden as _};

use crate::db::StoredId as _;
use crate::schema::IssueRow;

/// The condition that an issue (table `issue`) is one the person reads (SEC-06): in a project where
/// they see every issue, or one they reported where they see only their own; not in the trash unless
/// asked; and not of a type the sight hides in its project.
pub(crate) fn sight_condition(sight: &IssueSight) -> Cond {
    sight_condition_on(sight, &IssueRow::Table.into_iden())
}

/// The same for the table `issue` under another name in the statement: the issue at the other end of
/// a row that points to an issue (a parent, an issue in an own field), which counts only when the
/// person reads it too.
pub(crate) fn sight_condition_as(sight: &IssueSight, name: &str) -> Cond {
    sight_condition_on(sight, &Alias::new(name).into_iden())
}

fn sight_condition_on(sight: &IssueSight, issue: &DynIden) -> Cond {
    let column = |column: IssueRow| Expr::col((issue.clone(), column));
    // A branch is there only for projects there are: an `OR` with a branch that can never hold
    // keeps a database from using the index of the project.
    let mut readable = Cond::any();
    if !sight.all.is_empty() {
        readable = readable
            .add(column(IssueRow::Project).is_in(sight.all.iter().map(|project| project.stored())));
    }
    if !sight.own.is_empty() {
        readable = readable.add(
            column(IssueRow::Project)
                .is_in(sight.own.iter().map(|project| project.stored()))
                .and(column(IssueRow::ReportedBy).eq(sight.person.stored())),
        );
    }
    if sight.all.is_empty() && sight.own.is_empty() {
        readable = readable.add(Expr::val(1).eq(2));
    }
    let visible = sight
        .hidden
        .iter()
        .fold(Cond::all(), |cond, (project, issue_type)| {
            cond.add(
                column(IssueRow::Project)
                    .ne(project.stored())
                    .or(column(IssueRow::IssueType).ne(issue_type.stored())),
            )
        });
    let kept = if sight.with_trashed {
        Cond::all()
    } else {
        Cond::all().add(column(IssueRow::DeletedAt).is_null())
    };
    Cond::all().add(kept).add(readable).add(visible)
}
