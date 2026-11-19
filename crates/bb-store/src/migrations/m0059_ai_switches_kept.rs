//! Migration 59: AI apps and agents start off, and an upgrade keeps them as they were (COMP-04,
//! ADR-0047 decision 6).
//!
//! The settings `ai-apps.enabled` and `agents.enabled` used to start on, and a stored row of a
//! setting is only a change of its default. When the default became *off*, every installation that
//! never touched the two switches would have lost its AI apps and agents at the upgrade. So, where
//! people already work (a person exists), the migration writes the old default down as the
//! organization's own value: *on*, not locked. Where the organization set a value of its own, that
//! value stays. A new installation runs its migrations before setup makes the first person, so it
//! gets no row and starts with both off.
//!
//! The keys and the level are written here as the program named them when the migration was made,
//! so a later rename cannot change what this migration did.

use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, InsertStatement, Query};

use super::Step;
use crate::db::to_micros;
use crate::schema::{PersonRow, SettingRow};

/// The switches whose default became *off*.
const SWITCHES: [&str; 2] = ["ai-apps.enabled", "agents.enabled"];

/// The level of the organization's own value, and its scope (none).
const ORGANIZATION: &str = "organization";

/// The old default, as a stored value (JSON).
const ON: &str = "true";

pub(super) fn steps() -> Vec<Step> {
    SWITCHES
        .into_iter()
        .map(|key| Step::SeedAt(Box::new(move |now| keep_on(key, now))))
        .collect()
}

/// The organization's row `key = on`, written at `now`, when a person exists and the organization
/// has no value of its own for `key`.
fn keep_on(key: &str, now: Timestamp) -> Result<InsertStatement, sea_query::error::Error> {
    let mut people = Query::select();
    people.expr(Expr::val(1_i64)).from(PersonRow::Table);
    let mut own = Query::select();
    own.expr(Expr::val(1_i64))
        .from(SettingRow::Table)
        .and_where(Expr::col(SettingRow::Key).eq(key))
        .and_where(Expr::col(SettingRow::Level).eq(ORGANIZATION));
    let mut row = Query::select();
    row.expr(Expr::val(key))
        .expr(Expr::val(ORGANIZATION))
        .expr(Expr::val(Vec::<u8>::new()))
        .expr(Expr::val(ON))
        .expr(Expr::val(false))
        .expr(Expr::val(to_micros(now)))
        .and_where(Expr::exists(people))
        .and_where(Expr::exists(own).not());
    let mut insert = Query::insert();
    insert
        .into_table(SettingRow::Table)
        .columns([
            SettingRow::Key,
            SettingRow::Level,
            SettingRow::Scope,
            SettingRow::Value,
            SettingRow::Locked,
            SettingRow::UpdatedAt,
        ])
        .select_from(row)?;
    Ok(insert)
}

#[cfg(test)]
mod tests {
    use bb_core::id::UserId;
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, ExprTrait as _, Query};

    use super::super::Database;
    use super::{ORGANIZATION, SWITCHES};
    use crate::db::{StoredId as _, to_micros};
    use crate::schema::SettingRow;
    use crate::test_support::{at, ids, insert_old_person};

    /// The organization's rows of the two switches: the key, the value and when it was written.
    async fn organization_rows(
        database: &Database,
    ) -> Result<Vec<(String, String, i64)>, Box<dyn std::error::Error + Send + Sync>> {
        let mut select = Query::select();
        select
            .columns([SettingRow::Key, SettingRow::Value, SettingRow::UpdatedAt])
            .from(SettingRow::Table)
            .and_where(Expr::col(SettingRow::Key).is_in(SWITCHES))
            .and_where(Expr::col(SettingRow::Level).eq(ORGANIZATION))
            .order_by(SettingRow::Key, sea_query::Order::Asc);
        let mut rows = Vec::new();
        for row in database.fetch_all(&select).await? {
            rows.push((
                row.get::<String>(SettingRow::Key)?,
                row.get::<String>(SettingRow::Value)?,
                row.get::<i64>(SettingRow::UpdatedAt)?,
            ));
        }
        Ok(rows)
    }

    #[tokio::test]
    async fn comp_04_migration_59_keeps_ai_apps_and_agents_as_an_installation_had_them()
    -> TestResult {
        each_database("comp_04_migration_59_keeps", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(27, at(0)).await?;
            insert_old_person(&database, UserId::generate(&mut ids()).stored()).await?;
            database.migrate_until(58, at(1)).await?;
            // The organization switched agents off itself, and never touched AI apps.
            let mut off = Query::insert();
            off.into_table(SettingRow::Table)
                .columns([
                    SettingRow::Key,
                    SettingRow::Level,
                    SettingRow::Scope,
                    SettingRow::Value,
                    SettingRow::Locked,
                    SettingRow::UpdatedAt,
                ])
                .values([
                    Expr::val("agents.enabled"),
                    Expr::val(ORGANIZATION),
                    Expr::val(Vec::<u8>::new()),
                    Expr::val("false"),
                    Expr::val(false),
                    Expr::val(to_micros(at(2))),
                ])?;
            database.execute(&off).await?;
            assert_eq!(database.migrate_until(59, at(3)).await?, [59]);
            assert_eq!(
                organization_rows(&database).await?,
                [
                    (
                        "agents.enabled".to_owned(),
                        "false".to_owned(),
                        to_micros(at(2))
                    ),
                    (
                        "ai-apps.enabled".to_owned(),
                        "true".to_owned(),
                        to_micros(at(3))
                    ),
                ]
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn comp_04_migration_59_leaves_a_new_installation_with_both_off() -> TestResult {
        each_database("comp_04_migration_59_new", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate(at(0)).await?;
            // No row: the defaults hold, and both are off.
            assert_eq!(organization_rows(&database).await?, []);
            for key in SWITCHES {
                let definition = bb_core::settings::find(key).ok_or(key)?;
                assert_eq!(
                    definition.default(),
                    bb_core::settings::DefaultValue::Bool(false),
                    "{key}"
                );
            }
            Ok(())
        })
        .await
    }
}
