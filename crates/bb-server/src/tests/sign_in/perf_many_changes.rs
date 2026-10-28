//! What the preview of a change to many issues costs (PERF-05, M3-F133): it judges each issue of
//! the selection with the use cases of a single change, so its cost grows with the issues; the
//! statements for each issue are counted and held to a ceiling, so a change that makes an issue
//! cost more is seen. The definitions of the own fields are read once for the request, not once
//! for each issue (M5 WP-10). Every flow runs on SQLite and PostgreSQL.

use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::rig_and_app;
use super::issues_site::{Site, key_of, ok};
use super::perf_queries::Counter;

const PREVIEW: &str = "/api/v1/many-changes/preview";

/// The most statements each issue of a selection costs the preview, however many there are (the
/// issue and the project it is in, who is assigned and the chain of the project; 6 measured, 10
/// before the definitions of the fields were read once).
const PER_ISSUE_AT_MOST: usize = 6;

/// The most statements a preview of one issue runs: the session and the rights, which every
/// request pays (M4-F137: 31 measured, 42 before the rights of the person were read once), and the
/// one issue.
const ONE_AT_MOST: usize = 31;

#[tokio::test]
async fn perf_05_each_issue_of_a_preview_costs_the_same_few_statements() -> TestResult {
    each_database("perf_05_preview_per_issue", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut issues = Vec::new();
        for number in 0..12 {
            issues.push(key_of(
                &site.create(&mut ada, &format!("Issue {number}")).await?,
            )?);
        }
        let action = json!({"kind": "change", "change": {"status": "done"}});
        let mut counter = Counter::on(&rig.database);
        let mut statements = Vec::new();
        // The first read warms what is kept across requests and is not counted.
        for count in [1_usize, 1, 4, 12] {
            counter.take();
            let body = json!({"issues": &issues[..count], "action": action});
            ok(&ada.send("POST", PREVIEW, Some(body)).await?)?;
            statements.push(counter.take());
        }
        let (one, four, twelve) = (statements[1], statements[2], statements[3]);
        assert!(one <= ONE_AT_MOST, "{one} statements for one issue");
        let (extra, per_issue) = (twelve - four, (twelve - four) / (12 - 4));
        assert_eq!(
            extra % (12 - 4),
            0,
            "{four} and {twelve} statements are not a line"
        );
        assert!(
            per_issue <= PER_ISSUE_AT_MOST,
            "{per_issue} statements for each issue"
        );
        assert_eq!(four - one, 3 * per_issue, "the issues cost the same each");
        Ok(())
    })
    .await
}
