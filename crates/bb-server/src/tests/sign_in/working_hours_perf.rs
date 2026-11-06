//! No N+1 queries in the work days (PERF-05, PERF-N1): a stretch of forty days, with the
//! comparison of which of them changed, runs as many statements as one of five, on SQLite and
//! PostgreSQL.

use bb_testing::{Failure, TestResult, each_database};

use super::http::rig_and_app;
use super::issues_site::{Site, ok};
use super::perf_queries::{Counter, assert_as_many_statements, assert_same_statements};
use super::working_hours_http::{MINE, days, times};

/// The most statements a stretch of work days runs, however many days it has: the session, the
/// rights of the person, the settings, and the reads of the days (days, periods, breaks) and the
/// one grouped read of which days were changed.
const DAYS_AT_MOST: usize = 24;

#[tokio::test]
async fn perf_05_a_stretch_of_forty_work_days_runs_as_many_statements_as_one_of_five() -> TestResult
{
    each_database("perf_05_work_days", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (_site, mut ada) = Site::start(&rig, &app).await?;
        let first = jiff::civil::date(2026, 11, 1);
        for offset in 0..40 {
            let date = first
                .checked_add(jiff::Span::new().days(offset))?
                .to_string();
            rig.clock.advance(1);
            let made = ada
                .send(
                    "PATCH",
                    &format!("{MINE}/{date}"),
                    Some(times(
                        &date,
                        "08:00",
                        "16:00",
                        0,
                        Some("From the paper list"),
                    )),
                )
                .await?;
            ok(&made)?;
        }
        let range = |count: i64| -> Result<String, Failure> {
            let last = first.checked_add(jiff::Span::new().days(count - 1))?;
            Ok(format!("{MINE}?from={first}&to={last}"))
        };
        let mut counter = Counter::on(&rig.database);
        ok(&ada.send("GET", &range(5)?, None).await?)?;
        counter.take();
        let five = ada.send("GET", &range(5)?, None).await?;
        assert_eq!(days(&five).len(), 5);
        let (on_five, five_log) = counter.take_logged();
        let forty = ada.send("GET", &range(40)?, None).await?;
        assert_eq!(days(&forty).len(), 40);
        // Every day was typed afterwards with a reason, so the grouped read has them all to name.
        assert!(days(&forty).iter().all(|day| day["changed"] == true));
        let (on_forty, forty_log) = counter.take_logged();
        assert_same_statements(&five_log, &forty_log);
        assert_as_many_statements(on_five, on_forty, DAYS_AT_MOST, "a longer stretch of days");
        Ok(())
    })
    .await
}
