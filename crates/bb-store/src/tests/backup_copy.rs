//! The copy of the database inside a backup (OPS-07, ADR-0046 decision 4): a copy that cannot be
//! put back changes nothing, and a table whose rows point into itself comes back whole.

use bb_core::backup::member::MemberName;

use sea_query::{Alias, Table};

use super::*;

/// A copy of the backup at `archive` with its copy of the database changed by `change`, at
/// `into`: every member's size and checksum is counted again, and the copy is sealed with
/// `keys` again, so only the change is wrong.
fn changed_backup(
    keys: &KeyRing,
    archive: &Path,
    into: &Path,
    change: impl FnOnce(&Path) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let work = tempfile::tempdir()?;
    let mut manifest = archive::read(archive, work.path(), keys)?;
    let mut sources = Vec::new();
    for listed in &manifest.members {
        let name = MemberName::parse(&listed.name).map_err(|problem| format!("{problem}"))?;
        sources.push(archive::Source {
            path: archive::extracted(work.path(), &name),
            name,
        });
    }
    let database = archive::extracted(
        work.path(),
        &MemberName::parse(manifest.database.member()).map_err(|problem| format!("{problem}"))?,
    );
    change(&database)?;
    archive::write(into, &mut manifest, &sources, keys)?;
    Ok(())
}

/// The copy of a PostgreSQL database with one more line after the first row of the table of
/// events: a copy of that row (which the database refuses, its ID is taken), or `bad`.
fn with_line_after_the_first_event(copy: &Path, bad: Option<&str>) -> Result<(), Failure> {
    let text = std::fs::read_to_string(copy)?;
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let head = lines
        .iter()
        .position(|line| {
            line.starts_with(r#"{"table":"event""#) || line.contains(r#""table":"event""#)
        })
        .ok_or("no table of events in the copy")?;
    let row = lines.get(head + 1).ok_or("no event in the copy")?.clone();
    assert!(head + 2 < lines.len(), "the table of events is the last");
    lines.insert(head + 2, bad.map_or(row, str::to_owned));
    std::fs::write(
        copy,
        lines.join(
            "
",
        ) + "
",
    )?;
    Ok(())
}

#[tokio::test]
async fn ops_07_a_restore_that_fails_half_way_leaves_the_database_as_it_was() -> TestResult {
    each_database("ops_07_restore_half_way", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        save(&database, 1).await?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        save(&database, 2).await?;
        database.close().await;
        let changed = installation.folder.path().join("changed.backup");
        // The data folder's replay list stays as it was too; a copy given beside it is not merged
        // into it.
        let list = "2026-11-02T10:00:00Z switch-off 0195f7a0-0000-4000-8000-000000000001
";
        std::fs::write(&installation.places.replay_list, list)?;
        let given = installation.folder.path().join("given.list");
        std::fs::write(
            &given,
            "2026-11-03T10:00:00Z switch-off 0195f7a0-0000-4000-8000-000000000002
",
        )?;
        let cases: [Option<&str>; 2] = [None, Some(r#"["not", "a", "row"]"#)];
        for bad in cases {
            match db.kind() {
                // The copy of a SQLite database is one file, which a restore checks whole before it
                // replaces anything: a damaged one is refused.
                bb_testing::DatabaseKind::Sqlite => {
                    changed_backup(&installation.places.keys, &backup.path, &changed, |copy| {
                        let mut bytes = std::fs::read(copy)?;
                        bytes[..16].fill(0xff);
                        std::fs::write(copy, bytes)?;
                        Ok(())
                    })?;
                }
                // A row the database refuses, or one that is not a row at all, in the middle of
                // the copy of PostgreSQL.
                bb_testing::DatabaseKind::Postgres => {
                    changed_backup(&installation.places.keys, &backup.path, &changed, |copy| {
                        with_line_after_the_first_event(copy, bad)
                    })?;
                }
            }
            installation
                .restore(&changed, std::slice::from_ref(&given), at(20))
                .await
                .expect_err("a copy that cannot be put back");
            assert_eq!(
                std::fs::read_to_string(&installation.places.replay_list)?,
                list
            );
            let database = installation.open().await?;
            assert_eq!(saved(&database).await?, [1, 2], "{bad:?}");
            database.close().await;
            std::fs::remove_file(&changed)?;
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_07_a_table_whose_rows_point_into_itself_comes_back_whole() -> TestResult {
    use crate::schema::ProjectRow;
    each_database("ops_07_self_reference", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let mut source = ids();
        let count = 2_000_usize;
        let projects: Vec<bb_core::id::ProjectId> = (0..count)
            .map(|_| bb_core::id::ProjectId::generate(&mut source))
            .collect::<Result<_, _>>()?;
        let mut transaction = database.begin().await?;
        for (number, chunk) in projects.chunks(200).enumerate() {
            let mut insert = Query::insert();
            insert.into_table(ProjectRow::Table).columns([
                ProjectRow::Id,
                ProjectRow::ShortName,
                ProjectRow::Name,
                ProjectRow::Version,
            ]);
            for (place, project) in chunk.iter().enumerate() {
                let short = format!("P{number}X{place}");
                insert.values([
                    Expr::val(crate::db::StoredId::stored(*project)),
                    Expr::val(short),
                    Expr::val("Chain"),
                    Expr::val(1_i64),
                ])?;
            }
            transaction.execute(&insert).await?;
        }
        // Each project sits under the next one, set in order: on PostgreSQL every row the copy
        // reads comes before the row it points to.
        for pair in projects.windows(2) {
            let mut update = Query::update();
            update
                .table(ProjectRow::Table)
                .value(ProjectRow::Parent, crate::db::StoredId::stored(pair[1]))
                .and_where(Expr::col(ProjectRow::Id).eq(crate::db::StoredId::stored(pair[0])));
            transaction.execute(&update).await?;
        }
        transaction.commit().await?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        database.close().await;
        installation
            .restore(&backup.path, &[], at(20))
            .await
            .map_err(failed)?;
        let restored = installation.open().await?;
        let mut select = Query::select();
        select
            .column(ProjectRow::Id)
            .from(ProjectRow::Table)
            .and_where(Expr::col(ProjectRow::Parent).is_not_null());
        assert_eq!(restored.fetch_all(&select).await?.len(), count - 1);
        restored.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_07_a_restore_touches_only_bugaboos_own_tables() -> TestResult {
    each_database("ops_07_own_tables", async |db| {
        if db.kind() == bb_testing::DatabaseKind::Sqlite {
            // The copy of SQLite is the file itself; a table of the schema matters on PostgreSQL.
            return Ok(());
        }
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        // Every table the migrations make is one Bugaboo knows, so the copy takes all of them.
        let mut transaction = database.begin_read().await?;
        let all = catalog::tables_in_schema(&mut transaction).await?;
        let own = catalog::tables(&mut transaction).await?;
        transaction.commit().await?;
        assert_eq!(all, own);
        // A table somebody else keeps beside them stays as it is through a backup and a restore.
        let mut other = Table::create();
        other
            .table(Alias::new("someone_elses"))
            .col(sea_query::ColumnDef::new(Alias::new("note")).text());
        let mut transaction = database.begin().await?;
        transaction.execute_schema(&other).await?;
        let mut note = Query::insert();
        note.into_table(Alias::new("someone_elses"))
            .columns([Alias::new("note")])
            .values([Expr::val("kept")])?;
        transaction.execute(&note).await?;
        transaction.commit().await?;
        save(&database, 1).await?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        database.close().await;
        installation
            .restore(&backup.path, &[], at(20))
            .await
            .map_err(failed)?;
        let restored = installation.open().await?;
        let mut select = Query::select();
        select
            .column(Alias::new("note"))
            .from(Alias::new("someone_elses"));
        assert_eq!(restored.fetch_all(&select).await?.len(), 1);
        restored.close().await;
        Ok(())
    })
    .await
}
