//! Migration 60: the standing of a person may name nobody as who did it (PRIV-04, ADR-0044,
//! decision 8).
//!
//! A restore that runs a line of the replay list again names no administrator, because the line
//! holds none. Migration 57 made `person_standing.by` a column that names somebody always, so such a
//! row was written with the person themselves, whom no administrator can be, and read back as
//! nobody. The column may be empty now, and the rows that name the person themselves name nobody.
//!
//! PostgreSQL lets a column go of `NOT NULL`. SQLite cannot change a column, so the table is made
//! again under another name with the same columns, the rows are copied, the old table goes and the
//! new one takes its name and its index. Nothing refers to the table, so it can go while foreign
//! keys are checked.

use sea_query::{Expr, ExprTrait as _, Query};

use super::Step;
use crate::db::FixedSql;
use crate::schema::PersonStandingRow;

/// The table made again with `by` that may be empty, its rows copied, and its index.
const SQLITE_REBUILD: [&str; 5] = [
    r#"CREATE TABLE "person_standing_new" ( "person" blob NOT NULL PRIMARY KEY, "standing" text NOT NULL, "since" integer NOT NULL, "by" blob, "label" text, "hold_name" text, "hold_reason" text, "hold_until" text, FOREIGN KEY ("person") REFERENCES "person" ("id") )"#,
    r#"INSERT INTO "person_standing_new" ("person", "standing", "since", "by", "label", "hold_name", "hold_reason", "hold_until") SELECT "person", "standing", "since", "by", "label", "hold_name", "hold_reason", "hold_until" FROM "person_standing""#,
    r#"DROP TABLE "person_standing""#,
    r#"ALTER TABLE "person_standing_new" RENAME TO "person_standing""#,
    r#"CREATE UNIQUE INDEX "person_standing_label" ON "person_standing" ("label")"#,
];

/// `by` may be empty.
const POSTGRES_ALTER: &str = r#"ALTER TABLE "person_standing" ALTER COLUMN "by" DROP NOT NULL"#;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Sql(FixedSql {
            sqlite: SQLITE_REBUILD.to_vec(),
            postgres: vec![POSTGRES_ALTER],
        }),
        // A row that names the person themselves as who did it was written by a restore.
        Step::Update(
            Query::update()
                .table(PersonStandingRow::Table)
                .value(PersonStandingRow::By, Option::<Vec<u8>>::None)
                .and_where(Expr::col(PersonStandingRow::By).equals(PersonStandingRow::Person))
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use bb_core::id::UserId;
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, ExprTrait as _, Query};

    use crate::db::{Database, StoredId as _, to_micros};
    use crate::schema::{PersonRow, PersonStandingRow};
    use crate::test_support::{at, ids};

    /// The number of this migration.
    const VERSION: i64 = 60;

    async fn person(database: &Database, id: UserId, name: &str) -> TestResult {
        let mut insert = Query::insert();
        insert
            .into_table(PersonRow::Table)
            .columns([
                PersonRow::Id,
                PersonRow::DisplayName,
                PersonRow::NameKey,
                PersonRow::Email,
                PersonRow::CreatedAt,
            ])
            .values([
                Expr::val(id.stored()),
                Expr::val(name),
                Expr::val(name.to_lowercase()),
                Expr::val(format!("{}@example.test", name.to_lowercase())),
                Expr::val(to_micros(at(0))),
            ])?;
        database.execute(&insert).await?;
        Ok(())
    }

    async fn standing(database: &Database, of: UserId, by: UserId) -> TestResult {
        let mut insert = Query::insert();
        insert
            .into_table(PersonStandingRow::Table)
            .columns([
                PersonStandingRow::Person,
                PersonStandingRow::Standing,
                PersonStandingRow::Since,
                PersonStandingRow::By,
            ])
            .values([
                Expr::val(of.stored()),
                Expr::val("switched-off"),
                Expr::val(to_micros(at(1))),
                Expr::val(by.stored()),
            ])?;
        database.execute(&insert).await?;
        Ok(())
    }

    async fn by_of(
        database: &Database,
        of: UserId,
    ) -> Result<Option<UserId>, Box<dyn std::error::Error + Send + Sync>> {
        let mut select = Query::select();
        select
            .column(PersonStandingRow::By)
            .from(PersonStandingRow::Table)
            .and_where(Expr::col(PersonStandingRow::Person).eq(of.stored()));
        let row = database
            .fetch_optional(&select)
            .await?
            .ok_or("no standing")?;
        Ok(row.optional_id(PersonStandingRow::By)?)
    }

    #[tokio::test]
    async fn priv_04_a_standing_written_by_a_restore_names_nobody_after_the_migration() -> TestResult
    {
        each_database("m0060_standing_by_optional", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(VERSION - 1, at(0)).await?;
            let mut source = ids();
            let [ada, max, lea] = [
                UserId::generate(&mut source),
                UserId::generate(&mut source),
                UserId::generate(&mut source),
            ];
            for (id, name) in [(ada, "Ada"), (max, "Max"), (lea, "Lea")] {
                person(&database, id, name).await?;
            }
            // Max was switched off by Ada; Lea by a restore, written as herself.
            standing(&database, max, ada).await?;
            standing(&database, lea, lea).await?;
            database.migrate_until(VERSION, at(2)).await?;
            assert_eq!(by_of(&database, max).await?, Some(ada));
            assert_eq!(by_of(&database, lea).await?, None);
            // The label stays unique on the table made again: two rows cannot share one.
            let mut label = Query::update();
            label
                .table(PersonStandingRow::Table)
                .value(PersonStandingRow::Label, "K7Q2");
            assert!(database.execute(&label).await.is_err());
            Ok(())
        })
        .await
    }
}
