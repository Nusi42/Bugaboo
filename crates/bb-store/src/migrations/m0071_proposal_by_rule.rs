//! Migration 71: an index of the proposals of standing rules by their rule (ADR-0044, amendment
//! 2026-11-24). A proposal is a part of its rule, so the walk of a person's data finds it by the
//! IDs of the person's rules, and a removal deletes the open ones with their rule; both look the
//! proposals of a rule up by this index instead of reading the table. A new index, no table written
//! again.

use sea_query::Index;

use super::Step;
use crate::schema::ProposalRow;

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Index(
        Index::create()
            .name("agent_proposal_by_rule")
            .table(ProposalRow::Table)
            .col(ProposalRow::Rule)
            .to_owned(),
    )]
}

#[cfg(test)]
mod tests {
    use bb_testing::{TestResult, each_database};

    use super::super::Database;
    use crate::test_support::at;

    /// The walk's test of its arms (`priv_03_every_arm_of_the_walk_is_answered_by_an_index`) shows
    /// that the index answers the proposals of a person's rules; this one, that an organization made
    /// before it gets it.
    #[tokio::test]
    async fn priv_03_migration_71_runs_on_a_database_made_before_it() -> TestResult {
        each_database("migration_71_proposal_by_rule", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(70, at(0)).await?;
            assert!(database.migrate(at(1)).await?.contains(&71));
            Ok(())
        })
        .await
    }
}
