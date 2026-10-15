//! Migration 13: the name of a kind of job is written like the others, in words joined by hyphens
//! (`area.what-it-does`, architecture §7): the daily clearing of drafts waits in the queue as
//! `editing.purge_drafts` on a database that ran before, and now has the name the program looks for.

use sea_query::{Expr, ExprTrait as _, Query};

use super::Step;
use crate::schema::Job;

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Update(
        Query::update()
            .table(Job::Table)
            .value(Job::Kind, "editing.purge-drafts")
            .and_where(Expr::col(Job::Kind).eq("editing.purge_drafts"))
            .to_owned(),
    )]
}

#[cfg(test)]
mod tests {
    use bb_core::id::JobId;
    use bb_core::technical::TechnicalName;
    use bb_testing::{TestResult, each_database};
    use serde_json::Value;

    use super::*;
    use crate::jobs::NewJob;
    use crate::test_support::{at, ids, open};

    #[tokio::test]
    async fn adr_0017_a_job_that_waits_under_the_old_name_gets_the_new_one() -> TestResult {
        each_database("adr_0017_job_kind_rename", async |db| {
            let database = open(db).await?;
            let old = TechnicalName::new("editing.purge_drafts")?;
            let id = JobId::generate(&mut ids())?;
            let job = NewJob {
                kind: old,
                payload: Value::Null,
                run_at: at(0),
                max_attempts: 5,
            };
            database.enqueue_job(id, &job, at(0)).await?;
            for step in steps() {
                let Step::Update(update) = step else {
                    return Err("the migration only rewrites a name".into());
                };
                database.execute(&update).await?;
            }
            let worker = TechnicalName::new("worker.one")?;
            let taken = database
                .lease_jobs(&worker, at(1), jiff::SignedDuration::from_secs(60), 5)
                .await?;
            assert_eq!(taken[0].kind.as_str(), "editing.purge-drafts");
            Ok(())
        })
        .await
    }
}
