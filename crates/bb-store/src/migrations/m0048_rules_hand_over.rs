//! Migration 48: how a standing rule works, and a rule with no end (AI-32, ADR-0029, amendment
//! "Rules that hand over").
//!
//! `standing_rule.way` holds the word of the way (`proposals` or `hand-over`), `powers` the powers
//! of each hand-over the rule makes (a list of words), `accept_by_itself` whether those hand-overs
//! may be accepted by themselves. `standing_rule.fields` held the fields of the dropped option
//! "change only these fields"; it now holds the fields a hand-over of the rule may change, as
//! their keys.
//!
//! Every rule that is there proposed, or changed only the fields it named; the second never
//! changed anything without an attempt (M7-F120), so every rule becomes a rule that proposes, with
//! no powers, no fields and no accepting by itself.

use sea_query::{ColumnDef, Query, Table};

use super::Step;
use crate::schema::StandingRuleRow;

pub(super) fn steps() -> Vec<Step> {
    let mut way = ColumnDef::new(StandingRuleRow::Way);
    way.text().not_null().default("proposals");
    let mut powers = ColumnDef::new(StandingRuleRow::Powers);
    powers.text().not_null().default("[]");
    let mut accept = ColumnDef::new(StandingRuleRow::AcceptByItself);
    accept.boolean().not_null().default(false);
    let add = |column: ColumnDef| {
        Step::Alter(
            Table::alter()
                .table(StandingRuleRow::Table)
                .add_column(column)
                .to_owned(),
        )
    };
    vec![
        add(way),
        add(powers),
        add(accept),
        Step::Update(
            Query::update()
                .table(StandingRuleRow::Table)
                .value(StandingRuleRow::Fields, "[]")
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, Query};

    use super::super::Database;
    use super::super::old_rows::insert;
    use crate::schema::StandingRuleRow;
    use crate::test_support::at;

    #[tokio::test]
    async fn m7_wp14_a_migrated_rule_proposes() -> TestResult {
        each_database("migration_48_rules", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(47, at(0)).await?;
            // A rule as the database held it: one that changed only the fields it named.
            insert(
                &database,
                StandingRuleRow::Table,
                [
                    StandingRuleRow::Id,
                    StandingRuleRow::Agent,
                    StandingRuleRow::Person,
                    StandingRuleRow::Search,
                    StandingRuleRow::Project,
                    StandingRuleRow::Fields,
                    StandingRuleRow::CreatesIn,
                    StandingRuleRow::State,
                    StandingRuleRow::EndsAt,
                    StandingRuleRow::MadeAt,
                    StandingRuleRow::Version,
                ]
                .map(sea_query::IntoIden::into_iden),
                [
                    Expr::val(vec![1_u8; 16]),
                    Expr::val(vec![2_u8; 16]),
                    Expr::val(vec![3_u8; 16]),
                    Expr::val(vec![4_u8; 16]),
                    Expr::val(vec![5_u8; 16]),
                    Expr::val(r#"["labels","type"]"#),
                    Expr::val(Option::<Vec<u8>>::None),
                    Expr::val("open"),
                    Expr::val(5_000_000_i64),
                    Expr::val(1_000_000_i64),
                    Expr::val(1_i64),
                ],
            )
            .await?;
            database.migrate(at(1)).await?;
            let mut select = Query::select();
            select
                .columns([
                    StandingRuleRow::Way,
                    StandingRuleRow::Powers,
                    StandingRuleRow::AcceptByItself,
                    StandingRuleRow::Fields,
                    StandingRuleRow::EndsAt,
                ])
                .from(StandingRuleRow::Table);
            let rows = database.fetch_all(&select).await?;
            assert_eq!(rows.len(), 1);
            // It proposes, with nothing of a hand-over, and keeps its end.
            assert_eq!(rows[0].get::<String>(StandingRuleRow::Way)?, "proposals");
            assert_eq!(rows[0].get::<String>(StandingRuleRow::Powers)?, "[]");
            assert!(!rows[0].get::<bool>(StandingRuleRow::AcceptByItself)?);
            assert_eq!(rows[0].get::<String>(StandingRuleRow::Fields)?, "[]");
            assert_eq!(rows[0].get::<i64>(StandingRuleRow::EndsAt)?, 5_000_000);
            Ok(())
        })
        .await
    }
}
