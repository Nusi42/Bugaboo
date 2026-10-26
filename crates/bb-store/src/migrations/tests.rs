use bb_testing::{TestResult, each_database};
use sea_query::{Alias, ExprTrait as _, Query};

use super::*;
use crate::test_support::at;

/// By the number of the migration, not by place in the list: the numbers of other migrations that
/// are added later fall between these.
const APPLIED: [(i64, &str); 23] = [
    (
        1,
        "955f17699da59d0dbebd0aa84f15d41babff5bf3493ba3e72e422dfaf8d53de5",
    ),
    (
        2,
        "2f79f1fcf6ccadf282c92d37e1803b440c1759d098887112f96b5cbc24d2f30e",
    ),
    (
        3,
        "559268007329af29e9b173a8e836d1e5e5d924a70eb5a509147de6d2f1a54d2a",
    ),
    (
        4,
        "da7d77fae7b54c478965753e08448db6e9ff6bb7cc1910aa5acefeff4d15c990",
    ),
    (
        5,
        "197f6e67ca2b33794b484a19c5b8577eb97023cd759f34689ddd8fe3131fb9cd",
    ),
    (
        6,
        "888c571ed03837927111de0bc757b71c8608e08dff06c11631d94c6a292e8ed4",
    ),
    (
        7,
        "8fbd8b15ec9aed2d418484ea4ee883801c5aacc7e2cbc20e4c967852e6db8ab0",
    ),
    (
        8,
        "a994400ac493c6282be22dca5f4d6dd33dcd9e9556a80c648ac1322df0ff590e",
    ),
    (
        9,
        "c5dd6be187a9c42674b2bdfd05cf9d28eb88486bea90f96ec4d12b2a4f2fff07",
    ),
    (
        10,
        "d1579888290157b29550deda66ad28230d214174efffc6c0345a7d94edfdc10c",
    ),
    (
        11,
        "dcaa647639437b3e8751fee41009ecde93b5872c8064d39f2600bd992f7b559c",
    ),
    (
        12,
        "fffa0b40e2a8b74a36236e5d54896ec8d4a5078c82dcde3215f9ff610a4b6d2e",
    ),
    (
        13,
        "672ecd2ebb383bd7483263729b11f4f2f3e2695825b42b86bbaf22330b12a4bf",
    ),
    (
        14,
        "045061588034b6040764c6ba14cfd34631f165361100372b98f44a2812c6b790",
    ),
    (
        15,
        "16ed6f2372e8700c9a3158a53edbeb5e976ecf9737498802ee081c241ed286b2",
    ),
    (
        16,
        "b479b723a711237578a12f2cedcbce795c386d54a1a11430ffbb6ebb6193c9b7",
    ),
    (
        17,
        "d21f77afec1f2020ece56ef48d087d4f90e572a090a160a88488dac81164822a",
    ),
    (
        18,
        "c9688e8ee165a9f05d678b5d580e2ba23b46d619392a8342f9265fc31dfdefa7",
    ),
    (
        19,
        "f4b6ea4cb7a8bcd2247daefcc92413b74a89cff1eb45e5de62a697851f5eaedd",
    ),
    (
        20,
        "e1510d877cd996f9285d03a87bd8024feac32701a89fb0a9a8340785ac674ee6",
    ),
    (
        21,
        "d80ffbf5d3a0b8999ffadcf0dbf1a9fe63bd1d1628f7973ce02aabc7368f816d",
    ),
    (
        22,
        "b17ab3436cb24d81dcf609ca5ece9ade808ab32f0b8cbd03d00c3e4a2f650562",
    ),
    (23, "0000"),
];

/// The checksum of every migration a database may have applied. If one fails after a rename in
/// the code, a migration now builds other statements than the ones databases applied before:
/// databases that applied it would refuse to start. Add a new migration instead of changing an
/// old one, and name the old tables and columns it uses in the migration itself.
#[test]
fn the_checksum_of_an_applied_migration_never_changes() {
    for (version, wanted) in APPLIED {
        let definition = ALL
            .iter()
            .find(|definition| definition.version == version)
            .unwrap();
        let checksum = definition.checksum().unwrap();
        assert_eq!(
            bb_core::hex::encode(checksum.as_bytes()),
            wanted,
            "the checksum of migration {version} changed",
        );
    }
}

#[tokio::test]
async fn migrations() -> TestResult {
    each_database("migrations", async |db| {
        let database = Database::connect(db.url()).await?;
        let versions: Vec<i64> = ALL.iter().map(|definition| definition.version).collect();
        assert_eq!(database.migrate(at(0)).await?, versions);
        assert_eq!(database.migrate(at(1)).await?, Vec::<i64>::new());
        let applied = database.applied_migrations().await?;
        assert_eq!(applied.len(), ALL.len());
        assert_eq!(applied[0].name.as_str(), "initial");
        assert_eq!(applied[1].name.as_str(), "sign_in");
        assert_eq!(applied[2].name.as_str(), "policy");
        assert_eq!(applied[3].name.as_str(), "settings");
        assert_eq!(applied[4].name.as_str(), "projects");
        assert_eq!(applied[5].name.as_str(), "access_keys");
        assert_eq!(applied[6].name.as_str(), "issues");
        for (record, definition) in applied.iter().zip(ALL) {
            assert_eq!(record.name.as_str(), definition.name);
            assert_eq!(record.applied_at, at(0));
            assert_eq!(record.checksum, definition.checksum()?);
        }
        // Every table of the schema exists and belongs to an entity.
        for table in crate::schema::tables() {
            let mut select = Query::select();
            select
                .expr(Expr::col(sea_query::Asterisk))
                .from(Alias::new(&table.name))
                .limit(1);
            database.fetch_all(&select).await?;
            assert!(!table.marks.fields().is_empty());
        }
        database.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0019_groups_and_roles_that_were_there_start_at_the_first_version() -> TestResult {
    each_database("migration_8_first_version", async |db| {
        use crate::schema::{GroupRow, RoleRow};
        use bb_core::version::FIRST_VERSION;
        let database = Database::connect(db.url()).await?;
        database.migrate_until(6, at(0)).await?;
        // A group and a role as the database held them before they had a version.
        let mut group = Query::insert();
        group
            .into_table(GroupRow::Table)
            .columns([GroupRow::Id, GroupRow::Name])
            .values([Expr::val(vec![1_u8; 16]), Expr::val("Old group")])?;
        database.execute(&group).await?;
        let mut role = Query::insert();
        role.into_table(RoleRow::Table)
            .columns([
                RoleRow::Id,
                RoleRow::Name,
                RoleRow::Position,
                RoleRow::TwoStep,
            ])
            .values([
                Expr::val(vec![2_u8; 16]),
                Expr::val("Old role"),
                Expr::val(0_i64),
                Expr::val(false),
            ])?;
        database.execute(&role).await?;
        database.migrate(at(1)).await?;
        let mut groups = Query::select();
        groups.column(GroupRow::Version).from(GroupRow::Table);
        let rows = database.fetch_all(&groups).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get::<i64>(GroupRow::Version)?, FIRST_VERSION);
        let mut roles = Query::select();
        roles.column(RoleRow::Version).from(RoleRow::Table);
        let rows = database.fetch_all(&roles).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get::<i64>(RoleRow::Version)?, FIRST_VERSION);
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_processes_migrating_at_once_apply_each_migration_once() -> TestResult {
    each_database("migrations_race", async |db| {
        // Each process has its own pool. They connect one after the other (switching a new
        // SQLite file to WAL mode is not something two processes can do at once) and then
        // migrate at the same moment.
        let mut databases = Vec::new();
        for _ in 0..4 {
            databases.push(Database::connect(db.url()).await?);
        }
        let mut tasks = Vec::new();
        for (number, database) in databases.into_iter().enumerate() {
            let number = i64::try_from(number).unwrap_or(0);
            tasks.push(tokio::spawn(async move {
                let done = database.migrate(at(number)).await?;
                database.close().await;
                Ok::<_, StoreError>(done)
            }));
        }
        let mut applied = 0;
        for task in tasks {
            applied += task.await??.len();
        }
        assert_eq!(
            applied,
            ALL.len(),
            "each migration was applied by exactly one process"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn bad_example_a_newer_schema_is_refused() -> TestResult {
    each_database("bad_example_a_newer_schema_is_refused", async |db| {
        let database = Database::connect(db.url()).await?;
        database.migrate(at(0)).await?;
        let mut insert = Query::insert();
        insert
            .into_table(Migration::Table)
            .columns([
                Migration::Version,
                Migration::Name,
                Migration::Checksum,
                Migration::AppliedAt,
            ])
            .values([
                Expr::val(99_i64),
                Expr::val("from-the-future"),
                Expr::val(vec![0_u8; 32]),
                Expr::val(0_i64),
            ])?;
        database.execute(&insert).await?;
        let error = database.migrate(at(1)).await.unwrap_err();
        assert!(
            matches!(
                error,
                StoreError::SchemaTooNew {
                    found: 99,
                    known
                } if known == latest_version()
            ),
            "{error}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn bad_example_a_renamed_or_changed_migration_is_refused() -> TestResult {
    each_database("bad_example_a_renamed_migration_is_refused", async |db| {
        let database = Database::connect(db.url()).await?;
        database.migrate(at(0)).await?;
        for (column, value) in [
            (Migration::Name, sea_query::Value::from("other")),
            (Migration::Checksum, sea_query::Value::from(vec![1_u8; 32])),
        ] {
            let mut update = Query::update();
            update
                .table(Migration::Table)
                .value(column, value)
                .and_where(Expr::col(Migration::Version).eq(1_i64));
            database.execute(&update).await?;
            let error = database.migrate(at(1)).await.unwrap_err();
            assert!(
                matches!(error, StoreError::MigrationMismatch { version: 1 }),
                "{error}"
            );
            // Put the record right for the next round.
            let mut restore = Query::update();
            restore
                .table(Migration::Table)
                .value(Migration::Name, "initial")
                .value(Migration::Checksum, ALL[0].checksum()?.as_bytes().to_vec())
                .and_where(Expr::col(Migration::Version).eq(1_i64));
            database.execute(&restore).await?;
        }
        assert!(database.migrate(at(2)).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn a_number_the_program_does_not_have_is_not_taken_for_granted() -> TestResult {
    each_database("migrations_unknown_number", async |db| {
        let database = Database::connect(db.url()).await?;
        database.migrate(at(0)).await?;
        let mut update = Query::update();
        update
            .table(Migration::Table)
            .value(Migration::Version, 0_i64)
            .and_where(Expr::col(Migration::Version).eq(4_i64));
        database.execute(&update).await?;
        let error = database.migrate(at(1)).await.unwrap_err();
        assert!(
            matches!(error, StoreError::MigrationMismatch { version: 0 }),
            "{error}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sqlite_runs_in_wal_mode_with_foreign_keys_and_full_sync() -> TestResult {
    each_database("sqlite_runs_in_wal_mode_with_foreign_keys", async |db| {
        if db.sqlite_pool().is_some() {
            let database = Database::connect(db.url()).await?;
            assert_eq!(database.dialect(), crate::Dialect::Sqlite);
            assert_eq!(database.sqlite_pragmas().await?, ("wal".to_owned(), 1, 2));
        }
        Ok(())
    })
    .await
}
