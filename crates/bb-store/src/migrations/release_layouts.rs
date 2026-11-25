//! A part of migration 65: *Found in release* and *Planned for release* become fields the issue
//! page starts with (ADR-0049, decision 3), so a layout of the issue page that somebody saved
//! before gets them where the starting layout has them: right after *Labels*, before the own
//! fields, shown to the roles *Labels* is shown to. The fields after *Labels* in its section move
//! two places on. A layout that holds either field already (on the form or taken off it), a layout
//! without *Labels* on its form and the new-issue form are left as they are: there the two fields
//! wait under the fields that are not on the form, as the two dates did. The words of the fields
//! are written out here, not read from the program, so the migration stays what it was however
//! the catalogue changes later.
//!
//! The version of each changed layout counts up first, so an editor that was open across the
//! upgrade cannot save over the change. The fields come after the roles: which layouts lack the
//! fields is told by the fields.

use sea_query::{
    Alias, Condition, Expr, ExprTrait as _, Iden, JoinType, Query, SelectStatement,
    SubQueryStatement, UnionType,
};

use super::Step;
use crate::schema::{LayoutFieldRoleRow, LayoutFieldRow, LayoutOffFieldRow, LayoutRow};

/// The two fields, with the place each takes after *Labels*.
pub(super) const RELEASES: [(&str, i64); 2] = [("found-in-release", 1), ("planned-for-release", 2)];

/// How many places the fields after *Labels* move on: the place of the last of the two.
const ROOM: i64 = RELEASES[1].1;

/// The word of *Labels*, which the two fields follow.
const LABELS: &str = "labels";

/// The kind of layout the migration changes.
const ISSUE_PAGE: &str = "issue-page";

/// The name of the rows of [`releases`] and of their two columns.
const RELEASE_ROWS: &str = "release_field";
const WORD: &str = "word";
const PLACE: &str = "place";

/// The name of the row of *Labels* in a statement that reads the fields of a layout twice.
const AT_LABELS: &str = "at_labels";

/// One row for each of the two fields: `word` and `place`.
fn releases() -> SelectStatement {
    let row = |(word, place): (&str, i64)| {
        let mut select = Query::select();
        select
            .expr_as(Expr::val(word), Alias::new(WORD))
            .expr_as(Expr::val(place), Alias::new(PLACE));
        select
    };
    let mut rows = RELEASES.into_iter().map(row);
    let mut all = rows.next().unwrap_or_else(Query::select);
    for next in rows {
        all.union(UnionType::All, next);
    }
    all
}

/// A column of the rows of [`releases`].
fn release(column: &str) -> Expr {
    Expr::col((Alias::new(RELEASE_ROWS), Alias::new(column)))
}

/// The ID of the layout a statement is about.
fn layout_id() -> (LayoutRow, LayoutRow) {
    (LayoutRow::Table, LayoutRow::Id)
}

/// Whether the layout holds a row in a table of its fields whose field is one of `words`.
fn holds(
    table: impl Iden + Copy + 'static,
    (layout, field): (impl Iden + Copy + 'static, impl Iden + Copy + 'static),
    words: Vec<&'static str>,
) -> Expr {
    let mut rows = Query::select();
    rows.expr(Expr::val(1))
        .from(table)
        .and_where(Expr::col((table, layout)).equals(layout_id()))
        .and_where(Expr::col((table, field)).is_in(words));
    Expr::exists(rows)
}

/// The layouts the migration changes: of the issue page, with *Labels* on the form, holding
/// neither of the two fields.
fn lacking() -> Condition {
    let field = (LayoutFieldRow::Layout, LayoutFieldRow::Field);
    let off = (LayoutOffFieldRow::Layout, LayoutOffFieldRow::Field);
    let words = || RELEASES.map(|(word, _)| word).to_vec();
    Condition::all()
        .add(Expr::col((LayoutRow::Table, LayoutRow::Kind)).eq(ISSUE_PAGE))
        .add(holds(LayoutFieldRow::Table, field, vec![LABELS]))
        .add(holds(LayoutFieldRow::Table, field, words()).not())
        .add(holds(LayoutOffFieldRow::Table, off, words()).not())
}

/// The IDs of the layouts the migration changes.
fn lacking_layouts() -> SelectStatement {
    let mut layouts = Query::select();
    layouts
        .column(layout_id())
        .from(LayoutRow::Table)
        .cond_where(lacking());
    layouts
}

/// The section or the place of *Labels* in the layout of the field the statement changes.
fn of_labels(column: LayoutFieldRow) -> Expr {
    let at = Alias::new(AT_LABELS);
    let mut select = Query::select();
    select
        .column((at.clone(), column))
        .from_as(LayoutFieldRow::Table, at.clone())
        .and_where(
            Expr::col((at.clone(), LayoutFieldRow::Layout))
                .equals((LayoutFieldRow::Table, LayoutFieldRow::Layout)),
        )
        .and_where(Expr::col((at, LayoutFieldRow::Field)).eq(LABELS));
    Expr::SubQuery(None, Box::new(SubQueryStatement::SelectStatement(select)))
}

/// The rows of *Labels* of the layouts to change, each joined with one row for each field, and
/// with its layout.
fn labels_with_releases(
    table: impl Iden + Copy + 'static,
    layout: impl Iden + Copy + 'static,
) -> SelectStatement {
    let mut select = Query::select();
    select
        .from(table)
        .join_subquery(
            JoinType::InnerJoin,
            releases(),
            Alias::new(RELEASE_ROWS),
            Condition::all(),
        )
        .join(
            JoinType::InnerJoin,
            LayoutRow::Table,
            Expr::col(layout_id()).equals((table, layout)),
        )
        .cond_where(lacking());
    select
}

pub(super) fn steps() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut count_up = Query::update();
    count_up
        .table(LayoutRow::Table)
        .value(LayoutRow::Version, Expr::col(LayoutRow::Version).add(1))
        .cond_where(lacking());

    // The fields after *Labels* in its section move two places on.
    let mut make_room = Query::update();
    make_room
        .table(LayoutFieldRow::Table)
        .value(
            LayoutFieldRow::Position,
            Expr::col(LayoutFieldRow::Position).add(ROOM),
        )
        .and_where(
            Expr::col((LayoutFieldRow::Table, LayoutFieldRow::Layout))
                .in_subquery(lacking_layouts()),
        )
        .and_where(
            Expr::col((LayoutFieldRow::Table, LayoutFieldRow::Section))
                .eq(of_labels(LayoutFieldRow::Section)),
        )
        .and_where(
            Expr::col((LayoutFieldRow::Table, LayoutFieldRow::Position))
                .gt(of_labels(LayoutFieldRow::Position)),
        );

    let mut roles = labels_with_releases(LayoutFieldRoleRow::Table, LayoutFieldRoleRow::Layout);
    roles
        .expr(Expr::col((
            LayoutFieldRoleRow::Table,
            LayoutFieldRoleRow::Layout,
        )))
        .expr(release(WORD))
        .expr(Expr::col((
            LayoutFieldRoleRow::Table,
            LayoutFieldRoleRow::Role,
        )))
        .and_where(Expr::col((LayoutFieldRoleRow::Table, LayoutFieldRoleRow::Field)).eq(LABELS));
    let mut add_roles = Query::insert();
    add_roles
        .into_table(LayoutFieldRoleRow::Table)
        .columns([
            LayoutFieldRoleRow::Layout,
            LayoutFieldRoleRow::Field,
            LayoutFieldRoleRow::Role,
        ])
        .select_from(roles)?;

    let mut fields = labels_with_releases(LayoutFieldRow::Table, LayoutFieldRow::Layout);
    fields
        .expr(Expr::col((LayoutFieldRow::Table, LayoutFieldRow::Layout)))
        .expr(release(WORD))
        .expr(Expr::col((LayoutFieldRow::Table, LayoutFieldRow::Section)))
        .expr(Expr::col((LayoutFieldRow::Table, LayoutFieldRow::Position)).add(release(PLACE)))
        .and_where(Expr::col((LayoutFieldRow::Table, LayoutFieldRow::Field)).eq(LABELS));
    let mut add_fields = Query::insert();
    add_fields
        .into_table(LayoutFieldRow::Table)
        .columns([
            LayoutFieldRow::Layout,
            LayoutFieldRow::Field,
            LayoutFieldRow::Section,
            LayoutFieldRow::Position,
        ])
        .select_from(fields)?;

    Ok(vec![
        Step::Update(count_up),
        Step::Update(make_room),
        Step::Seed(add_roles),
        Step::Seed(add_fields),
    ])
}

#[cfg(test)]
mod tests {
    use bb_core::fields::{Field, FieldKey};
    use bb_core::id::{FieldId, LayoutId, RoleId, SequentialIds};
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, IntoIden as _};

    use super::super::Database;
    use super::super::old_rows::{insert, old_layout, old_role, read_layout, shown};
    use crate::db::StoredId as _;
    use crate::schema::LayoutFieldRoleRow;
    use crate::test_support::at;

    fn field(word: &str) -> FieldKey {
        FieldKey::from_code(word).unwrap()
    }

    #[tokio::test]
    async fn iss_08_migration_65_puts_both_release_fields_into_the_layouts_saved_before()
    -> TestResult {
        each_database("migration_65_release_layouts", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(64, at(0)).await?;
            let mut ids = SequentialIds::default();
            let (team, guest) = (RoleId::generate(&mut ids)?, RoleId::generate(&mut ids)?);
            old_role(&database, team, 0).await?;
            old_role(&database, guest, 1).await?;
            let own = FieldKey::Own(FieldId::generate(&mut ids)?);
            let own_word = own.to_string();
            let [plain, holds, off, unlabelled, form] =
                [(); 5].map(|()| LayoutId::generate(&mut ids).unwrap());
            let page = "issue-page";
            let details: &[&[&str]] = &[
                &["description"],
                &["reported-by", "labels", own_word.as_str(), "time-left"],
            ];
            old_layout(&database, plain, page, details, &[], &[team]).await?;
            // *Labels* is shown to the guest too: the two fields follow its roles.
            insert(
                &database,
                LayoutFieldRoleRow::Table,
                [
                    LayoutFieldRoleRow::Layout.into_iden(),
                    LayoutFieldRoleRow::Field.into_iden(),
                    LayoutFieldRoleRow::Role.into_iden(),
                ],
                [
                    Expr::val(plain.stored()),
                    Expr::val("labels"),
                    Expr::val(guest.stored()),
                ],
            )
            .await?;
            let planned: &[&[&str]] = &[&["labels", "planned-for-release"]];
            old_layout(&database, holds, page, planned, &[], &[team]).await?;
            let labels: &[&[&str]] = &[&["labels"]];
            old_layout(&database, off, page, labels, &["found-in-release"], &[team]).await?;
            let without: &[&[&str]] = &[&["description", "tags"]];
            old_layout(&database, unlabelled, page, without, &[], &[team]).await?;
            old_layout(&database, form, "new-issue", labels, &[], &[team]).await?;

            assert_eq!(database.migrate_until(65, at(1)).await?.last(), Some(&65));

            // Right after *Labels*, shown to its roles; the fields after it move on; the version
            // counts up.
            let (version, found) = read_layout(&database, plain).await?;
            assert_eq!(version, 2);
            let mut details = shown(&[field("reported-by")], &[team]);
            details.extend(shown(&[field("labels")], &[team, guest]));
            let releases = [Field::FoundInRelease, Field::PlannedForRelease].map(FieldKey::from);
            details.extend(shown(&releases, &[team, guest]));
            details.extend(shown(&[own, field("time-left")], &[team]));
            assert_eq!(found, [shown(&[field("description")], &[team]), details]);
            // A layout that holds one of them, one without *Labels* and the new-issue form are as
            // they were: the fields, the roles and the version.
            for (layout, fields) in [
                (holds, vec![field("labels"), field("planned-for-release")]),
                (off, vec![field("labels")]),
                (unlabelled, vec![field("description"), field("tags")]),
                (form, vec![field("labels")]),
            ] {
                let (version, found) = read_layout(&database, layout).await?;
                assert_eq!((version, found), (1, vec![shown(&fields, &[team])]));
            }
            Ok(())
        })
        .await
    }

    #[test]
    fn the_words_of_the_release_fields_are_the_fields_of_the_catalogue_today() {
        // The migration writes the words out; this keeps them the words of the two fields now.
        let words = [Field::FoundInRelease, Field::PlannedForRelease].map(Field::code);
        assert_eq!(super::RELEASES.map(|(word, _)| word), words);
    }
}
