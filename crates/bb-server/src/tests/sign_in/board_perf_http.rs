//! No N+1 queries in the board (PERF-N1): a board of six columns and four rows runs as many
//! statements with every cell filled as with a few, and a board with rows by *Assigned to* as many
//! for twelve people as for four (and as many past the most rows it shows), on SQLite and
//! PostgreSQL.

use bb_app::access::StartingRole;
use bb_store::Database;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, ok};
use super::perf_queries::{Counter, assert_as_many_statements};
use super::plan_site::change;
use super::views_site::{board, cards, issue_in, own_workflow};

/// The most statements a board may run, whatever its cards: the session and the rights, the search
/// prepared once with the fields and names of the reader, the issue types found, the totals of the
/// columns, the totals of the cells (with the people of the rows, read once), the first cards of
/// every cell, and what the cards show (their people, labels, time and card layouts) once for all
/// of them: measured on both databases, 59 with rows by priority, 61 with rows by person, and 66
/// for a reader who sees the people by their role, whose rows are added up again as sets of people
/// and shown with the labels of their roles.
const BOARD_AT_MOST: usize = 66;

/// The six statuses of ACME's workflow, where any status follows *Open*.
const STATUSES: [&str; 6] = ["open", "doing", "review", "testing", "done", "dropped"];

/// The four priorities, the rows of the board.
const PRIORITIES: [&str; 4] = ["low", "normal", "high", "critical"];

/// The statements of one read of `body`, after a read that warms the caches, and the board.
async fn statements_of(
    browser: &mut Browser,
    database: &Database,
    body: &Value,
) -> Result<(usize, Value), Failure> {
    let mut counter = Counter::on(database);
    board(browser, body.clone()).await?;
    counter.take();
    let read = board(browser, body.clone()).await?;
    Ok((counter.take(), read))
}

/// ACME's own workflow of six statuses.
async fn six_statuses(ada: &mut Browser, site: &Site<'_>) -> Result<(), Failure> {
    let statuses: Vec<Value> = STATUSES
        .iter()
        .enumerate()
        .map(|(at, key)| {
            let group = match at {
                0 => "not-started",
                4 | 5 => "done",
                _ => "in-progress",
            };
            json!({"key": key, "name": key, "status_group": group})
        })
        .collect();
    let changes: Vec<Value> = STATUSES
        .iter()
        .skip(1)
        .map(|to| json!({"from": "open", "to": to, "button": format!("To {to}")}))
        .collect();
    own_workflow(ada, &site.acme, (json!(statuses), json!(changes))).await
}

#[tokio::test]
async fn perf_n1_a_board_of_six_columns_and_four_rows_runs_as_many_statements_filled_as_sparse()
-> TestResult {
    each_database("perf_n1_board_cells", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        six_statuses(&mut ada, &site).await?;
        let body = json!({"search_text": "", "project": site.acme, "board": {"rows": "priority"}});
        // Sparse: one card in each row, all *Open*.
        for priority in PRIORITIES {
            issue_in(
                &mut ada,
                (&site.acme, &site.kind),
                priority,
                json!({"priority": priority}),
            )
            .await?;
        }
        let (on_sparse, sparse) = statements_of(&mut ada, &rig.database, &body).await?;
        assert_eq!(sparse["columns"].as_array().map(Vec::len), Some(6));
        assert_eq!(sparse["cells"].as_array().map(Vec::len), Some(4));
        // Filled: two cards in every one of the 24 cells.
        for status in STATUSES {
            for priority in PRIORITIES {
                for copy in 0..2 {
                    let key = issue_in(
                        &mut ada,
                        (&site.acme, &site.kind),
                        &format!("{status} {priority} {copy}"),
                        json!({"priority": priority}),
                    )
                    .await?;
                    if status != "open" {
                        ok(&change(&mut ada, &key, json!({"status": status})).await?)?;
                    }
                }
            }
        }
        let (on_filled, filled) = statements_of(&mut ada, &rig.database, &body).await?;
        assert_eq!(
            filled["cells"].as_array().map(Vec::len),
            Some(24),
            "{filled}"
        );
        assert_eq!(cards(&filled).len(), 52);
        assert_as_many_statements(
            on_sparse,
            on_filled,
            BOARD_AT_MOST,
            "a board with every cell filled",
        );
        Ok(())
    })
    .await
}

/// `count` more people, each with a task of ACME assigned to them.
async fn assigned(
    site: &Site<'_>,
    ada: &mut Browser,
    (from, count): (usize, usize),
) -> Result<(), Failure> {
    for at in from..from + count {
        let (_, person) = site
            .person_called(
                StartingRole::TeamMember,
                &format!("person{at}"),
                &format!("Person {at}"),
            )
            .await?;
        issue_in(
            ada,
            (&site.acme, &site.kind),
            &format!("Work {at}"),
            json!({"assigned_to": [person.to_string()]}),
        )
        .await?;
    }
    Ok(())
}

#[tokio::test]
async fn perf_n1_a_board_with_rows_by_person_runs_as_many_statements_for_many_people_as_for_few()
-> TestResult {
    each_database("perf_n1_board_people", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let body =
            json!({"search_text": "", "project": site.acme, "board": {"rows": "assigned to"}});
        // A Contributor sees the people of *Assigned to* by their role: all of them are one row.
        site.show_people(
            &mut ada,
            StartingRole::Contributor,
            "assigned-to",
            "role-only",
        )
        .await?;
        let (mut alike, _) = site
            .person(StartingRole::Contributor, "contributor")
            .await?;
        assigned(&site, &mut ada, (0, 4)).await?;
        let (on_four, four) = statements_of(&mut ada, &rig.database, &body).await?;
        assert_eq!(four["rows"].as_array().map(Vec::len), Some(4), "{four}");
        assert_eq!(four["rows_truncated"], false);
        let (on_four_alike, four_alike) = statements_of(&mut alike, &rig.database, &body).await?;
        assert_eq!(
            four_alike["rows"].as_array().map(Vec::len),
            Some(1),
            "{four_alike}"
        );
        assigned(&site, &mut ada, (4, 8)).await?;
        let (on_twelve, twelve) = statements_of(&mut ada, &rig.database, &body).await?;
        assert_eq!(twelve["rows"].as_array().map(Vec::len), Some(12));
        assert_as_many_statements(on_four, on_twelve, BOARD_AT_MOST, "rows of more people");
        let (on_twelve_alike, twelve_alike) =
            statements_of(&mut alike, &rig.database, &body).await?;
        assert_eq!(cards(&twelve_alike).len(), 12);
        assert_as_many_statements(
            on_four_alike,
            on_twelve_alike,
            BOARD_AT_MOST,
            "rows of people shown alike",
        );
        // Past the most rows a board shows, the rest are left out and the answer says so.
        assigned(&site, &mut ada, (12, 10)).await?;
        let (on_many, many) = statements_of(&mut ada, &rig.database, &body).await?;
        assert_eq!(many["rows"].as_array().map(Vec::len), Some(20), "{many}");
        assert_eq!(many["rows_truncated"], true);
        assert_eq!(cards(&many).len(), 20);
        assert_as_many_statements(on_four, on_many, BOARD_AT_MOST, "rows past the most");
        Ok(())
    })
    .await
}
