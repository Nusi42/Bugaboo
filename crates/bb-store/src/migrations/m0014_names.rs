//! Migration 14: the names of things are the words of the glossary (M4 WP-02, ADR-0008 amendment).
//!
//! Before the API stops changing, the stored values that carried an old word are written with the
//! new one, the way migration 13 did for a kind of job:
//!
//! - the permissions to create (`make-issues`, `make-projects`, `make-sub-projects`) in the rows of
//!   roles and of access keys;
//! - the kind of record of a run of a change to many issues in the events (`bulk_run` is
//!   `change_run`), which the outbox points to, so it needs no rewrite of its own;
//! - the kind of job that goes on with such a run (`bulk.step` is `many-changes.step`);
//! - the reason that an event of a run carries (`bulk-run:` is `change-run:`), which tells a worker
//!   that goes on with an interrupted run that the change to an issue was made already;
//! - the table of the runs (`bulk_run` is `change_run`), which keeps its rows;
//! - the table of the people an issue is assigned to (`issue_assignee` is `assigned_person`), which
//!   keeps its rows and its reference to the issue. PostgreSQL keeps the old names of the table's
//!   key and constraint (`issue_assignee_pkey`, …) and of the indexes of the renamed tables; the
//!   program never writes them, and they are unique inside their own table.
//!
//! Migrations run once (the recorded number says so). Every update of a value changes only the rows
//! that still have the old word, so running the updates again changes nothing.

use sea_query::{Alias, Expr, ExprTrait as _, Func, Query, Table};

use super::Step;
use crate::schema::{
    AccessKeyPermissionRow, AssignedPersonRow, Event, Job, ManyChangeRunRow, RolePermissionRow,
};

/// The permissions whose word changed, old and new.
const PERMISSIONS: [(&str, &str); 3] = [
    ("make-issues", "create-issues"),
    ("make-projects", "create-projects"),
    ("make-sub-projects", "create-sub-projects"),
];

pub(super) fn steps() -> Vec<Step> {
    let mut steps = permission_words();
    steps.extend(permission_words_in_role_events());
    steps.push(rename_value(
        Event::Table,
        Event::EntityKind,
        "bulk_run",
        "change_run",
    ));
    steps.push(rename_value(
        Job::Table,
        Job::Kind,
        "bulk.step",
        "many-changes.step",
    ));
    steps.push(Step::Update(
        Query::update()
            .table(Event::Table)
            .value(
                Event::Reason,
                Func::cust("REPLACE").args([
                    Expr::col(Event::Reason),
                    Expr::val("bulk-run:"),
                    Expr::val("change-run:"),
                ]),
            )
            .and_where(Expr::col(Event::Reason).like("bulk-run:%"))
            .to_owned(),
    ));
    steps.push(Step::Rename(
        Table::rename()
            .table(Alias::new("bulk_run"), ManyChangeRunRow::Table)
            .to_owned(),
    ));
    steps.push(Step::Rename(
        Table::rename()
            .table(Alias::new("issue_assignee"), AssignedPersonRow::Table)
            .to_owned(),
    ));
    steps
}

/// Writes `new` where `column` of `table` holds `old`.
fn rename_value(
    table: impl sea_query::Iden + 'static,
    column: impl sea_query::Iden + Copy + 'static,
    old: &str,
    new: &str,
) -> Step {
    Step::Update(
        Query::update()
            .table(table)
            .value(column, new)
            .and_where(Expr::col(column).eq(old))
            .to_owned(),
    )
}

/// The permission words in the changes that the events of roles hold (the history of a role shows
/// them, and the conflict page of a role reads them). They stand in the JSON text between quotes, so
/// the quotes keep a longer word from being touched. The events of access keys, settings and layouts
/// hold no permission word (they hold the choice of a key, values and IDs of roles).
fn permission_words_in_role_events() -> Vec<Step> {
    PERMISSIONS
        .into_iter()
        .map(|(old, new)| {
            let quoted = |word: &str| format!("\"{word}\"");
            Step::Update(
                Query::update()
                    .table(Event::Table)
                    .value(
                        Event::Changes,
                        Func::cust("REPLACE").args([
                            Expr::col(Event::Changes),
                            Expr::val(quoted(old)),
                            Expr::val(quoted(new)),
                        ]),
                    )
                    .and_where(Expr::col(Event::EntityKind).eq("role"))
                    .and_where(Expr::col(Event::Changes).like(format!("%{}%", quoted(old))))
                    .to_owned(),
            )
        })
        .collect()
}

/// The permission words in the rows of roles and of access keys.
fn permission_words() -> Vec<Step> {
    let mut steps = Vec::new();
    for (old, new) in PERMISSIONS {
        steps.push(rename_value(
            RolePermissionRow::Table,
            RolePermissionRow::Permission,
            old,
            new,
        ));
        steps.push(rename_value(
            AccessKeyPermissionRow::Table,
            AccessKeyPermissionRow::Permission,
            old,
            new,
        ));
    }
    steps
}

#[cfg(test)]
mod tests {
    use bb_core::event::{EntityKind, FieldChange};
    use bb_core::id::JobId;
    use bb_core::technical::TechnicalName;
    use bb_testing::{Failure, TestDatabase, TestResult, each_database};
    use sea_query::{Alias, Expr, Order, Query};
    use serde_json::{Value, json};

    use super::*;
    use crate::db::Database;
    use crate::jobs::NewJob;
    use crate::schema::{AccessKeyRow, IssueRow, IssueTypeRow, PersonRow, ProjectRow, RoleRow};
    use crate::test_support::{at, event, ids};

    /// The database as M3 left it: migrations 1 to 13 applied, nothing newer.
    async fn m3_database(db: &TestDatabase) -> Result<Database, Failure> {
        let database = Database::connect(db.url()).await?;
        database.migrate_until(13, at(0)).await?;
        Ok(database)
    }

    /// Runs the update steps of the migration again.
    async fn run_updates_again(database: &Database) -> Result<(), Failure> {
        for step in steps() {
            if let Step::Update(update) = step {
                database.execute(&update).await?;
            }
        }
        Ok(())
    }

    async fn permissions_of(
        database: &Database,
        table: impl sea_query::Iden + Copy + 'static,
        column: impl sea_query::Iden + Copy + 'static,
    ) -> Result<Vec<String>, Failure> {
        let mut select = Query::select();
        select
            .column(column)
            .from(table)
            .order_by(column, Order::Asc);
        let rows = database.fetch_all(&select).await?;
        rows.iter()
            .map(|row| row.get::<String>(column).map_err(Into::into))
            .collect()
    }

    /// An issue of a project, as M3 stored it (the people it is assigned to come after).
    async fn an_issue(database: &Database) -> Result<(), Failure> {
        let mut project = Query::insert();
        project
            .into_table(ProjectRow::Table)
            .columns([
                ProjectRow::Id,
                ProjectRow::ShortName,
                ProjectRow::Name,
                ProjectRow::Version,
            ])
            .values([
                Expr::val(vec![1_u8; 16]),
                Expr::val("WEB"),
                Expr::val("Web"),
                Expr::val(1_i64),
            ])?;
        database.execute(&project).await?;
        let mut issue_type = Query::insert();
        issue_type
            .into_table(IssueTypeRow::Table)
            .columns([
                IssueTypeRow::Id,
                IssueTypeRow::Name,
                IssueTypeRow::Icon,
                IssueTypeRow::Position,
                IssueTypeRow::Version,
            ])
            .values([
                Expr::val(vec![2_u8; 16]),
                Expr::val("Bug"),
                Expr::val("bug"),
                Expr::val(0_i64),
                Expr::val(1_i64),
            ])?;
        database.execute(&issue_type).await?;
        let mut issue = Query::insert();
        issue
            .into_table(IssueRow::Table)
            .columns([
                IssueRow::Id,
                IssueRow::Project,
                IssueRow::IssueType,
                IssueRow::Code,
                IssueRow::Title,
                IssueRow::Description,
                IssueRow::Status,
                IssueRow::Priority,
                IssueRow::ReportedBy,
                IssueRow::CreatedAt,
                IssueRow::UpdatedAt,
                IssueRow::Version,
            ])
            .values([
                Expr::val(vec![3_u8; 16]),
                Expr::val(vec![1_u8; 16]),
                Expr::val(vec![2_u8; 16]),
                Expr::val("1"),
                Expr::val("Printer jams"),
                Expr::val(""),
                Expr::val("open"),
                Expr::val("medium"),
                Expr::val(vec![4_u8; 16]),
                Expr::val(0_i64),
                Expr::val(0_i64),
                Expr::val(1_i64),
            ])?;
        database.execute(&issue).await?;
        Ok(())
    }

    /// A role and an access key with a person to own it.
    async fn a_role_and_a_key(database: &Database) -> Result<(), Failure> {
        let mut role = Query::insert();
        role.into_table(RoleRow::Table)
            .columns([
                RoleRow::Id,
                RoleRow::Name,
                RoleRow::Position,
                RoleRow::TwoStep,
                RoleRow::Version,
            ])
            .values([
                Expr::val(vec![1_u8; 16]),
                Expr::val("Old role"),
                Expr::val(0_i64),
                Expr::val(false),
                Expr::val(1_i64),
            ])?;
        database.execute(&role).await?;
        let mut person = Query::insert();
        person
            .into_table(PersonRow::Table)
            .columns([
                PersonRow::Id,
                PersonRow::DisplayName,
                PersonRow::Email,
                PersonRow::CreatedAt,
            ])
            .values([
                Expr::val(vec![2_u8; 16]),
                Expr::val("Ada"),
                Expr::val("ada@example.test"),
                Expr::val(0_i64),
            ])?;
        database.execute(&person).await?;
        let mut key = Query::insert();
        key.into_table(AccessKeyRow::Table)
            .columns([
                AccessKeyRow::Id,
                AccessKeyRow::Owner,
                AccessKeyRow::Name,
                AccessKeyRow::Prefix,
                AccessKeyRow::SecretHash,
                AccessKeyRow::Choice,
                AccessKeyRow::CreatedAt,
                AccessKeyRow::ExpiresAt,
            ])
            .values([
                Expr::val(vec![3_u8; 16]),
                Expr::val(vec![2_u8; 16]),
                Expr::val("Key"),
                Expr::val("00000001"),
                Expr::val(vec![4_u8; 32]),
                Expr::val("permissions"),
                Expr::val(0_i64),
                Expr::val(1000_i64),
            ])?;
        database.execute(&key).await?;
        Ok(())
    }

    #[tokio::test]
    async fn adr_0008_an_event_of_a_run_of_a_change_to_many_issues_has_the_new_kind() -> TestResult
    {
        each_database("m0014_change_run_kind", async |db| {
            let database = m3_database(db).await?;
            let mut source = ids();
            for (seconds, kind) in [(1, "bulk_run"), (2, "issue")] {
                let mut event = event(&mut source, seconds);
                event.entity.kind = EntityKind::new(kind)?;
                database
                    .record_change(&event, async |_transaction| Ok(()))
                    .await?;
            }
            assert_eq!(database.migrate_until(14, at(1)).await?, [14]);
            for _ in 0..2 {
                let mut select = Query::select();
                select
                    .column(Event::EntityKind)
                    .from(Event::Table)
                    .order_by(Event::EntityKind, Order::Asc);
                let kinds: Vec<String> = database
                    .fetch_all(&select)
                    .await?
                    .iter()
                    .map(|row| row.get::<String>(Event::EntityKind))
                    .collect::<Result<_, _>>()?;
                assert_eq!(kinds, ["change_run", "issue"]);
                run_updates_again(&database).await?;
            }
            assert!(database.migrate_until(14, at(2)).await?.is_empty());
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn adr_0008_the_history_of_a_role_shows_the_new_permission_words() -> TestResult {
        each_database("m0014_permission_words_in_events", async |db| {
            let database = m3_database(db).await?;
            let mut source = ids();
            for (seconds, kind) in [(1, "role"), (2, "issue")] {
                let mut event = event(&mut source, seconds);
                event.entity.kind = EntityKind::new(kind)?;
                event.changes = vec![FieldChange {
                    field: "permissions".to_owned(),
                    before: Some(json!(["make-issues", "see-issues"])),
                    after: Some(json!(["make-projects", "make-sub-projects", "make-issues"])),
                }];
                database
                    .record_change(&event, async |_transaction| Ok(()))
                    .await?;
            }
            assert_eq!(database.migrate_until(14, at(1)).await?, [14]);
            for _ in 0..2 {
                let mut select = Query::select();
                select
                    .columns([Event::EntityKind, Event::Changes])
                    .from(Event::Table)
                    .order_by(Event::EntityKind, Order::Asc);
                let rows = database.fetch_all(&select).await?;
                let changes = |at: usize| -> Result<String, Failure> {
                    Ok(rows[at].get::<String>(Event::Changes)?)
                };
                // The event of a role has the new words; another record keeps what it holds.
                assert_eq!(rows[0].get::<String>(Event::EntityKind)?, "issue");
                assert!(changes(0)?.contains("\"make-issues\""));
                let role = changes(1)?;
                assert!(role.contains("\"create-issues\""), "{role}");
                assert!(role.contains("\"create-projects\""), "{role}");
                assert!(role.contains("\"create-sub-projects\""), "{role}");
                assert!(
                    role.contains("\"see-issues\"") && !role.contains("make-"),
                    "{role}"
                );
                run_updates_again(&database).await?;
            }
            assert!(database.migrate_until(14, at(2)).await?.is_empty());
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn adr_0017_the_reason_of_an_event_of_a_run_names_the_run_by_its_new_word() -> TestResult
    {
        each_database("m0014_change_run_reason", async |db| {
            let database = m3_database(db).await?;
            let mut source = ids();
            for (seconds, reason) in [(1, "bulk-run:one:7"), (2, "Reported")] {
                let mut event = event(&mut source, seconds);
                event.reason = Some(reason.to_owned());
                database
                    .record_change(&event, async |_transaction| Ok(()))
                    .await?;
            }
            assert_eq!(database.migrate_until(14, at(1)).await?, [14]);
            for _ in 0..2 {
                let mut select = Query::select();
                select
                    .column(Event::Reason)
                    .from(Event::Table)
                    .order_by(Event::Reason, Order::Asc);
                let reasons: Vec<String> = database
                    .fetch_all(&select)
                    .await?
                    .iter()
                    .map(|row| row.get::<String>(Event::Reason))
                    .collect::<Result<_, _>>()?;
                assert_eq!(reasons, ["Reported", "change-run:one:7"]);
                run_updates_again(&database).await?;
            }
            assert!(database.migrate_until(14, at(2)).await?.is_empty());
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn adr_0017_the_runs_keep_their_rows_under_the_new_table() -> TestResult {
        each_database("m0014_change_run_table", async |db| {
            let database = m3_database(db).await?;
            let mut run = Query::insert();
            run.into_table(Alias::new("bulk_run"))
                .columns(
                    [
                        "id",
                        "person",
                        "action",
                        "items",
                        "status",
                        "done",
                        "changed",
                        "skipped",
                        "created_at",
                    ]
                    .map(Alias::new),
                )
                .values([
                    Expr::val(vec![1_u8; 16]),
                    Expr::val(vec![2_u8; 16]),
                    Expr::val("{}"),
                    Expr::val("[]"),
                    Expr::val("done"),
                    Expr::val(3_i64),
                    Expr::val(2_i64),
                    Expr::val("[]"),
                    Expr::val(5_i64),
                ])?;
            database.execute(&run).await?;
            assert_eq!(database.migrate_until(14, at(1)).await?, [14]);
            let mut select = Query::select();
            select
                .columns([ManyChangeRunRow::Done, ManyChangeRunRow::Changed])
                .from(ManyChangeRunRow::Table);
            let rows = database.fetch_all(&select).await?;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].get::<i64>(ManyChangeRunRow::Done)?, 3);
            assert_eq!(rows[0].get::<i64>(ManyChangeRunRow::Changed)?, 2);
            let mut old = Query::select();
            old.column(Alias::new("done")).from(Alias::new("bulk_run"));
            assert!(database.fetch_all(&old).await.is_err());
            assert!(database.migrate_until(14, at(2)).await?.is_empty());
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn adr_0017_a_job_that_goes_on_with_a_run_waits_under_the_new_kind() -> TestResult {
        each_database("m0014_many_changes_job_kind", async |db| {
            let database = m3_database(db).await?;
            let mut source = ids();
            let job = NewJob {
                kind: TechnicalName::new("bulk.step")?,
                payload: Value::Null,
                run_at: at(0),
                max_attempts: 5,
            };
            database
                .enqueue_job(JobId::generate(&mut source)?, &job, at(0))
                .await?;
            assert_eq!(database.migrate_until(14, at(1)).await?, [14]);
            run_updates_again(&database).await?;
            let worker = TechnicalName::new("worker.one")?;
            let taken = database
                .lease_jobs(&worker, at(2), jiff::SignedDuration::from_secs(60), 5)
                .await?;
            assert_eq!(taken.len(), 1);
            assert_eq!(taken[0].kind.as_str(), "many-changes.step");
            assert!(database.migrate_until(14, at(3)).await?.is_empty());
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn adr_0021_the_people_an_issue_is_assigned_to_keep_their_rows_under_the_new_table()
    -> TestResult {
        each_database("m0014_assigned_person_table", async |db| {
            let database = m3_database(db).await?;
            an_issue(&database).await?;
            // The row as M3 wrote it: in the table with the old name.
            let mut assigned = Query::insert();
            assigned
                .into_table(Alias::new("issue_assignee"))
                .columns([
                    Alias::new("issue"),
                    Alias::new("person"),
                    Alias::new("added_at"),
                ])
                .values([
                    Expr::val(vec![3_u8; 16]),
                    Expr::val(vec![4_u8; 16]),
                    Expr::val(7_i64),
                ])?;
            database.execute(&assigned).await?;
            assert_eq!(database.migrate_until(14, at(1)).await?, [14]);
            let mut select = Query::select();
            select
                .columns([AssignedPersonRow::Person, AssignedPersonRow::AddedAt])
                .from(AssignedPersonRow::Table);
            let rows = database.fetch_all(&select).await?;
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].get::<Vec<u8>>(AssignedPersonRow::Person)?,
                vec![4_u8; 16]
            );
            assert_eq!(rows[0].get::<i64>(AssignedPersonRow::AddedAt)?, 7);
            // The reference to the issue is still there: an unknown issue is refused.
            let mut unknown = Query::insert();
            unknown
                .into_table(AssignedPersonRow::Table)
                .columns([
                    AssignedPersonRow::Issue,
                    AssignedPersonRow::Person,
                    AssignedPersonRow::AddedAt,
                ])
                .values([
                    Expr::val(vec![9_u8; 16]),
                    Expr::val(vec![4_u8; 16]),
                    Expr::val(8_i64),
                ])?;
            assert!(database.execute(&unknown).await.is_err());
            // Nothing is left under the old name, and migrating again changes nothing.
            let mut old = Query::select();
            old.column(Alias::new("person"))
                .from(Alias::new("issue_assignee"));
            assert!(database.fetch_all(&old).await.is_err());
            assert!(database.migrate_until(14, at(2)).await?.is_empty());
            assert_eq!(database.fetch_all(&select).await?.len(), 1);
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn adr_0008_the_permissions_to_create_are_written_with_the_new_words() -> TestResult {
        each_database("m0014_permission_words", async |db| {
            let database = m3_database(db).await?;
            // A role and an access key with the words M3 wrote.
            a_role_and_a_key(&database).await?;
            for old in [
                "make-issues",
                "make-projects",
                "make-sub-projects",
                "see-issues",
            ] {
                let mut role_row = Query::insert();
                role_row
                    .into_table(RolePermissionRow::Table)
                    .columns([RolePermissionRow::Role, RolePermissionRow::Permission])
                    .values([Expr::val(vec![1_u8; 16]), Expr::val(old)])?;
                database.execute(&role_row).await?;
                let mut key_row = Query::insert();
                key_row
                    .into_table(AccessKeyPermissionRow::Table)
                    .columns([
                        AccessKeyPermissionRow::Key,
                        AccessKeyPermissionRow::Permission,
                    ])
                    .values([Expr::val(vec![3_u8; 16]), Expr::val(old)])?;
                database.execute(&key_row).await?;
            }
            assert_eq!(database.migrate_until(14, at(1)).await?, [14]);
            let wanted = [
                "create-issues",
                "create-projects",
                "create-sub-projects",
                "see-issues",
            ];
            for _ in 0..2 {
                assert_eq!(
                    permissions_of(
                        &database,
                        RolePermissionRow::Table,
                        RolePermissionRow::Permission
                    )
                    .await?,
                    wanted
                );
                assert_eq!(
                    permissions_of(
                        &database,
                        AccessKeyPermissionRow::Table,
                        AccessKeyPermissionRow::Permission
                    )
                    .await?,
                    wanted
                );
                run_updates_again(&database).await?;
            }
            // Migrating again applies nothing.
            assert!(database.migrate_until(14, at(2)).await?.is_empty());
            Ok(())
        })
        .await
    }
}
