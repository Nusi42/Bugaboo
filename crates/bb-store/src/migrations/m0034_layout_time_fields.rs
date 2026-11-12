//! Migration 34: the lengths of time become fields a layout places (TIME-03, CFG-01, M6-F161).
//!
//! Until now the issue page showed *Estimated time*, *Time left* and *Time spent* in its details
//! whatever the layout said. From now on the page shows them only where the layout places them, so
//! a layout of the issue page that somebody saved before would lose them. Every stored layout of the
//! issue page that holds none of the three (neither on the form nor taken off it) and has a section
//! gets them at the end of its last section, in the order the starting layout has them, for every
//! role. A layout that holds one of them is left as it is, and so is a layout without a section.
//! The words of the three fields are written out here, not read from the program, so the migration
//! stays what it was however the catalogue changes later.
//!
//! The version of each changed layout counts up first, so an editor that was open across the
//! upgrade cannot save over the change. The fields come after the roles: which layouts lack the
//! fields is told by the fields.

use sea_query::{
    Alias, Condition, Expr, ExprTrait as _, Func, Iden, JoinType, Query, SelectStatement,
    SubQueryStatement, UnionType,
};

use super::Step;
use crate::schema::{
    LayoutFieldRoleRow, LayoutFieldRow, LayoutOffFieldRow, LayoutRow, LayoutSectionRow, RoleRow,
};

/// The three fields, with the place each takes after the last field of the section.
const LENGTHS: [(&str, i64); 3] = [("estimated-time", 0), ("time-left", 1), ("time-spent", 2)];

/// The name of the rows of [`lengths`] and of their two columns.
const LENGTH_ROWS: &str = "length";
const WORD: &str = "word";
const PLACE: &str = "place";

/// The kind of layout the migration changes.
const ISSUE_PAGE: &str = "issue-page";

/// One row for each of the three fields: `word` and `place`.
fn lengths() -> SelectStatement {
    let row = |(word, place): (&str, i64)| {
        let mut select = Query::select();
        select
            .expr_as(Expr::val(word), Alias::new(WORD))
            .expr_as(Expr::val(place), Alias::new(PLACE));
        select
    };
    let mut rows = LENGTHS.into_iter().map(row);
    let mut all = rows.next().unwrap_or_else(Query::select);
    for next in rows {
        all.union(UnionType::All, next);
    }
    all
}

/// The ID of the layout a statement is about.
fn layout_id() -> (LayoutRow, LayoutRow) {
    (LayoutRow::Table, LayoutRow::Id)
}

/// The sections of the layout a statement is about.
fn sections_of_layout() -> SelectStatement {
    let mut sections = Query::select();
    sections.from(LayoutSectionRow::Table).and_where(
        Expr::col((LayoutSectionRow::Table, LayoutSectionRow::Layout)).equals(layout_id()),
    );
    sections
}

/// The position of the last section of the layout.
fn last_section() -> SelectStatement {
    let mut last = sections_of_layout();
    last.expr(Expr::col((LayoutSectionRow::Table, LayoutSectionRow::Position)).max());
    last
}

/// Whether the layout holds one of the three words in a table of its fields, on the form or taken
/// off it.
fn holds(
    table: impl Iden + Copy + 'static,
    layout: impl Iden + Copy + 'static,
    field: impl Iden + Copy + 'static,
) -> Expr {
    let mut rows = Query::select();
    rows.expr(Expr::val(1))
        .from(table)
        .and_where(Expr::col((table, layout)).equals(layout_id()))
        .and_where(Expr::col((table, field)).is_in(LENGTHS.map(|(word, _)| word)));
    Expr::exists(rows)
}

/// The layouts the migration changes: of the issue page, holding none of the three fields, and
/// with a section.
fn lacking() -> Condition {
    let mut some_section = sections_of_layout();
    some_section.expr(Expr::val(1));
    Condition::all()
        .add(Expr::col((LayoutRow::Table, LayoutRow::Kind)).eq(ISSUE_PAGE))
        .add(
            holds(
                LayoutFieldRow::Table,
                LayoutFieldRow::Layout,
                LayoutFieldRow::Field,
            )
            .not(),
        )
        .add(
            holds(
                LayoutOffFieldRow::Table,
                LayoutOffFieldRow::Layout,
                LayoutOffFieldRow::Field,
            )
            .not(),
        )
        .add(Expr::exists(some_section))
}

/// A column of the rows of [`lengths`].
fn length(column: &str) -> Expr {
    Expr::col((Alias::new(LENGTH_ROWS), Alias::new(column)))
}

/// The layouts to change, each joined with one row for each of the three fields.
fn layouts_with_lengths() -> SelectStatement {
    let mut select = Query::select();
    select
        .from(LayoutRow::Table)
        .join_subquery(
            JoinType::InnerJoin,
            lengths(),
            Alias::new(LENGTH_ROWS),
            Condition::all(),
        )
        .cond_where(lacking());
    select
}

fn sub(select: SelectStatement) -> Expr {
    Expr::SubQuery(None, Box::new(SubQueryStatement::SelectStatement(select)))
}

pub(super) fn steps() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut count_up = Query::update();
    count_up
        .table(LayoutRow::Table)
        .value(LayoutRow::Version, Expr::col(LayoutRow::Version).add(1))
        .cond_where(lacking());

    let mut roles = layouts_with_lengths();
    roles
        .expr(Expr::col(layout_id()))
        .expr(length(WORD))
        .expr(Expr::col((RoleRow::Table, RoleRow::Id)))
        .join(JoinType::InnerJoin, RoleRow::Table, Condition::all());
    let mut add_roles = Query::insert();
    add_roles
        .into_table(LayoutFieldRoleRow::Table)
        .columns([
            LayoutFieldRoleRow::Layout,
            LayoutFieldRoleRow::Field,
            LayoutFieldRoleRow::Role,
        ])
        .select_from(roles)?;

    // After the last field of the last section: one place further for each field.
    let mut after_last = Query::select();
    after_last
        .expr(Expr::col((LayoutFieldRow::Table, LayoutFieldRow::Position)).max())
        .from(LayoutFieldRow::Table)
        .and_where(Expr::col((LayoutFieldRow::Table, LayoutFieldRow::Layout)).equals(layout_id()))
        .and_where(
            Expr::col((LayoutFieldRow::Table, LayoutFieldRow::Section)).eq(sub(last_section())),
        );
    let mut fields = layouts_with_lengths();
    fields
        .expr(Expr::col(layout_id()))
        .expr(length(WORD))
        .expr(sub(last_section()))
        .expr(
            Expr::expr(Func::coalesce([sub(after_last), Expr::val(-1_i64)]))
                .add(1)
                .add(length(PLACE)),
        );
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
        Step::Seed(add_roles),
        Step::Seed(add_fields),
    ])
}

#[cfg(test)]
mod tests {
    use bb_app::layouts::LayoutStore as _;
    use bb_core::fields::{Field, FieldKey};
    use bb_core::id::{FieldId, LayoutId, RoleId, SequentialIds};
    use bb_core::layout::{LayoutContent, LayoutKind};
    use bb_testing::{Failure, TestResult, each_database};

    use super::super::Database;
    use super::super::old_rows::{old_layout, old_role};
    use crate::auth::AuthStorage;
    use crate::test_support::at;

    /// A field of a stored layout with the roles it is shown to, in order of role.
    type Placed = (FieldKey, Vec<RoleId>);

    /// What a stored layout holds: its version and the fields of each section.
    async fn read(
        database: &Database,
        layout: LayoutId,
    ) -> Result<(i64, Vec<Vec<Placed>>), Failure> {
        let mut read = database.begin_read().await?;
        let stored = AuthStorage::stored_layout_in(&mut read, layout)
            .await?
            .ok_or("the layout is gone")?;
        let sections = stored
            .content
            .sections
            .iter()
            .map(|section| {
                section
                    .fields
                    .iter()
                    .map(|placed| {
                        let mut roles = placed.roles.clone();
                        roles.sort();
                        (placed.field, roles)
                    })
                    .collect()
            })
            .collect();
        Ok((stored.layout.version, sections))
    }

    /// These fields, each shown to these roles.
    fn shown(fields: &[FieldKey], roles: &[RoleId]) -> Vec<Placed> {
        let mut roles = roles.to_vec();
        roles.sort();
        fields.iter().map(|field| (*field, roles.clone())).collect()
    }

    fn field(word: &str) -> FieldKey {
        FieldKey::from_code(word).unwrap()
    }

    #[tokio::test]
    async fn time_03_migration_34_puts_the_lengths_of_time_into_the_layouts_saved_before()
    -> TestResult {
        each_database("migration_34_layout_time_fields", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(33, at(0)).await?;
            let mut ids = SequentialIds::default();
            let (team, guest) = (RoleId::generate(&mut ids)?, RoleId::generate(&mut ids)?);
            old_role(&database, team, 0).await?;
            old_role(&database, guest, 1).await?;
            let own = FieldKey::Own(FieldId::generate(&mut ids)?);
            let own_word = own.to_string();
            let [plain, holds, off, form, empty, hollow, with_own] =
                [(); 7].map(|()| LayoutId::generate(&mut ids).unwrap());
            let page = "issue-page";
            let sections: &[&[&str]] = &[&["description", "status"], &["reported-by"]];
            old_layout(&database, plain, page, sections, &[], &[team]).await?;
            let with_left: &[&[&str]] = &[&["description", "time-left"]];
            old_layout(&database, holds, page, with_left, &[], &[team]).await?;
            let only: &[&[&str]] = &[&["description"]];
            old_layout(&database, off, page, only, &["estimated-time"], &[team]).await?;
            old_layout(&database, form, "new-issue", only, &[], &[team]).await?;
            old_layout(&database, empty, page, &[], &[], &[team]).await?;
            // An empty last section: the fields start at its first place.
            let empty_last: &[&[&str]] = &[&["description"], &[]];
            old_layout(&database, hollow, page, empty_last, &[], &[team]).await?;
            // An own field in the last section: the lengths come after it.
            let after_own: &[&[&str]] = &[&["description"], &["reported-by", own_word.as_str()]];
            old_layout(&database, with_own, page, after_own, &[], &[team]).await?;

            assert_eq!(database.migrate_until(34, at(1)).await?, [34]);

            let lengths = ["estimated-time", "time-left", "time-spent"].map(field);
            // A layout with none of them has them last in its last section, for every role, and
            // its version counts up.
            let (version, found) = read(&database, plain).await?;
            assert_eq!(version, 2);
            let mut last = shown(&[field("reported-by")], &[team]);
            last.extend(shown(&lengths, &[team, guest]));
            assert_eq!(
                found,
                [
                    shown(&[field("description"), field("status")], &[team]),
                    last
                ]
            );
            // An empty last section is filled from its first place.
            let (_, found) = read(&database, hollow).await?;
            assert_eq!(found[1], shown(&lengths, &[team, guest]));
            // The lengths come after an own field.
            let (_, found) = read(&database, with_own).await?;
            let order: Vec<FieldKey> = found[1].iter().map(|(each, _)| *each).collect();
            assert_eq!(
                order,
                [
                    field("reported-by"),
                    own,
                    lengths[0],
                    lengths[1],
                    lengths[2]
                ]
            );
            // The others are as they were: the fields, the roles and the version.
            for (layout, fields) in [
                (holds, vec![field("description"), field("time-left")]),
                (off, vec![field("description")]),
                (form, vec![field("description")]),
            ] {
                let (version, found) = read(&database, layout).await?;
                assert_eq!((version, found), (1, vec![shown(&fields, &[team])]));
            }
            let (version, found) = read(&database, empty).await?;
            assert_eq!((version, found.is_empty()), (1, true));
            Ok(())
        })
        .await
    }

    // A layout nobody changed, saved as the starting layout of the issue page was before, is the
    // starting layout of now once the migration has run.
    #[tokio::test]
    async fn time_03_migration_34_makes_the_old_starting_layout_the_starting_layout_of_now()
    -> TestResult {
        each_database("migration_34_starting_layout", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(33, at(0)).await?;
            let mut ids = SequentialIds::default();
            let roles = [RoleId::generate(&mut ids)?, RoleId::generate(&mut ids)?];
            for (position, role) in (0_i64..).zip(roles) {
                old_role(&database, role, position).await?;
            }
            let own = [FieldKey::Own(FieldId::generate(&mut ids)?)];
            let layout = LayoutId::generate(&mut ids)?;
            let own_word = own[0].to_string();
            let old: &[&[&str]] = &[
                &["description", "status", "assigned-to", "priority"],
                &["reported-by", "tags", "labels", own_word.as_str()],
            ];
            old_layout(&database, layout, "issue-page", old, &[], &roles).await?;

            database.migrate_until(34, at(1)).await?;

            let starting = LayoutContent::starting(LayoutKind::IssuePage, &own, &roles);
            let expected: Vec<Vec<Placed>> = starting
                .sections
                .iter()
                .map(|section| {
                    // The two dates came with M8: a layout that was saved before does not hold
                    // them, and they wait under the fields that are not on the form.
                    let fields: Vec<FieldKey> = section
                        .fields
                        .iter()
                        .map(|placed| placed.field)
                        .filter(|field| {
                            ![Field::StartDate, Field::DueDate]
                                .map(FieldKey::from)
                                .contains(field)
                        })
                        .collect();
                    shown(&fields, &roles)
                })
                .collect();
            assert_eq!(read(&database, layout).await?.1, expected);
            Ok(())
        })
        .await
    }

    #[test]
    fn the_words_of_the_lengths_are_the_fields_of_the_catalogue_today() {
        // The migration writes the words out; this keeps them the words of the three fields now.
        // A word that is renamed gets a new migration and a change to this test, never an edit of
        // this migration.
        let words = [Field::EstimatedTime, Field::TimeLeft, Field::TimeSpent].map(Field::code);
        assert_eq!(super::LENGTHS.map(|(word, _)| word), words);
    }
}
