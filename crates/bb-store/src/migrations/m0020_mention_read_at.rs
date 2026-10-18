//! Migration 20: `mention` loses `read_at` (M4-F83).
//!
//! Nothing sets or reads it since the notifications replaced the *Mentions* block (M4 WP-08); a
//! notification keeps its own `read_at`. The column is taken away the way migration 3 took away
//! `person.administrator`: one `ALTER TABLE … DROP COLUMN`, which both SQLite (3.35 and later) and
//! PostgreSQL run in place. No index or key of `mention` names the column, so SQLite accepts it.

use sea_query::Table;

use super::Step;
use crate::schema::{MentionRow, Retired};

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Alter(
        Table::alter()
            .table(MentionRow::Table)
            .drop_column(Retired::ReadAt)
            .to_owned(),
    )]
}

#[cfg(test)]
mod tests {
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, ExprTrait as _, Query};

    use super::super::Database;
    use crate::test_support::at;

    #[tokio::test]
    async fn m4_f83_migration_20_takes_the_read_at_of_a_mention_away_and_keeps_the_rest()
    -> TestResult {
        each_database("migration_20_mention", async |db| {
            use bb_core::id::IssueTypeId;
            use jiff::Timestamp;

            use sea_query::IntoIden as _;

            use crate::db::StoredId as _;
            use crate::schema::{CommentRow, IssueRow, MentionRow, ProjectRow, Retired};
            use crate::test_support::insert_old_issue_type;

            let database = Database::connect(db.url()).await?;
            database.migrate_until(19, at(0)).await?;
            // A comment that mentions a person, and the mention marked as read: what a database of
            // M4 before migration 20 holds.
            let issue_type = IssueTypeId::from_parts(Timestamp::UNIX_EPOCH, [2; 10])?;
            insert_old_issue_type(&database, issue_type, "Task").await?;
            let (project, issue, comment) = (vec![3_u8; 16], vec![4_u8; 16], vec![5_u8; 16]);
            let (author, recipient) = (vec![6_u8; 16], vec![7_u8; 16]);
            let mut made = Query::insert();
            made.into_table(ProjectRow::Table)
                .columns([
                    ProjectRow::Id,
                    ProjectRow::ShortName,
                    ProjectRow::Name,
                    ProjectRow::Version,
                ])
                .values([
                    Expr::val(project.clone()),
                    Expr::val("ACME"),
                    Expr::val("Acme"),
                    Expr::val(1_i64),
                ])?;
            database.execute(&made).await?;
            let mut made = Query::insert();
            made.into_table(IssueRow::Table)
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
                    Expr::val(issue.clone()),
                    Expr::val(project),
                    Expr::val(issue_type.stored()),
                    Expr::val("ACME-1"),
                    Expr::val("An issue"),
                    Expr::val(""),
                    Expr::val("open"),
                    Expr::val("normal"),
                    Expr::val(author.clone()),
                    Expr::val(1_i64),
                    Expr::val(1_i64),
                    Expr::val(1_i64),
                ])?;
            database.execute(&made).await?;
            let mut made = Query::insert();
            made.into_table(CommentRow::Table)
                .columns([
                    CommentRow::Id,
                    CommentRow::Issue,
                    CommentRow::Author,
                    CommentRow::Text,
                    CommentRow::CreatedAt,
                    CommentRow::Version,
                ])
                .values([
                    Expr::val(comment.clone()),
                    Expr::val(issue),
                    Expr::val(author),
                    Expr::val("Over to you, @ana"),
                    Expr::val(2_i64),
                    Expr::val(1_i64),
                ])?;
            database.execute(&made).await?;
            let mut made = Query::insert();
            made.into_table(MentionRow::Table)
                .columns([
                    MentionRow::Comment.into_iden(),
                    MentionRow::Recipient.into_iden(),
                    MentionRow::Via.into_iden(),
                    Retired::ReadAt.into_iden(),
                ])
                .values([
                    Expr::val(comment.clone()),
                    Expr::val(recipient.clone()),
                    Expr::val("name"),
                    Expr::val(3_i64),
                ])?;
            database.execute(&made).await?;

            database.migrate(at(1)).await?;

            // The column is gone: an UPDATE that names it fails (a SELECT would read the missing
            // column as a string on SQLite).
            let mut with_read = Query::update();
            with_read
                .table(MentionRow::Table)
                .value(Retired::ReadAt, 1_i64)
                .and_where(Expr::val(1_i64).eq(0_i64));
            assert!(database.execute(&with_read).await.is_err());
            // The comment, who is mentioned and how are kept.
            let mut rows = Query::select();
            rows.columns([MentionRow::Comment, MentionRow::Recipient, MentionRow::Via])
                .from(MentionRow::Table);
            let kept = database.fetch_all(&rows).await?;
            assert_eq!(kept.len(), 1);
            assert_eq!(kept[0].get::<Vec<u8>>(MentionRow::Comment)?, comment);
            assert_eq!(kept[0].get::<Vec<u8>>(MentionRow::Recipient)?, recipient);
            assert_eq!(kept[0].get::<String>(MentionRow::Via)?, "name");
            let mut comments = Query::select();
            comments.column(CommentRow::Text).from(CommentRow::Table);
            let found = database.fetch_all(&comments).await?;
            assert_eq!(found.len(), 1);
            assert_eq!(
                found[0].get::<String>(CommentRow::Text)?,
                "Over to you, @ana"
            );
            Ok(())
        })
        .await
    }
}
