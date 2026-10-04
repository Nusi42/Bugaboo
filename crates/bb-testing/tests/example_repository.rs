//! An example repository test (WP-03): the pattern every `bb-store` repository test follows. One
//! body, run on SQLite and PostgreSQL, each time on a brand-new database with a typed pool.
//!
//! The repository holds the typed pool of its database and runs one query definition on it.
//! `bb-store` builds the query with sea-query (ADR-0003); this example writes the one SQL text
//! that both databases understand (`$1` placeholders, ids chosen by the caller).

use bb_testing::{Pool, TestResult, each_database};
use sqlx::Row as _;
// Used by the harness, not by this test program; named here so the unused-dependency lint stays on.
use tempfile as _;

/// Runs the same expression on whichever typed pool the database has.
macro_rules! on_pool {
    ($pool:expr, $name:ident => $body:expr) => {
        match $pool {
            Pool::Sqlite($name) => $body,
            Pool::Postgres($name) => $body,
        }
    };
}

/// A tiny repository: issues with a number and a title.
struct Issues<'a> {
    pool: &'a Pool,
}

impl Issues<'_> {
    async fn create_table(&self) -> Result<(), sqlx::Error> {
        on_pool!(self.pool, pool => {
            sqlx::query("CREATE TABLE issues (id BIGINT PRIMARY KEY, title TEXT NOT NULL)")
                .execute(pool)
                .await?;
        });
        Ok(())
    }

    async fn add(&self, id: i64, title: &str) -> Result<(), sqlx::Error> {
        on_pool!(self.pool, pool => {
            sqlx::query("INSERT INTO issues (id, title) VALUES ($1, $2)")
                .bind(id)
                .bind(title)
                .execute(pool)
                .await?;
        });
        Ok(())
    }

    async fn title(&self, id: i64) -> Result<Option<String>, sqlx::Error> {
        on_pool!(self.pool, pool => {
            let row = sqlx::query("SELECT title FROM issues WHERE id = $1")
                .bind(id)
                .fetch_optional(pool)
                .await?;
            row.map(|row| row.try_get::<String, _>("title")).transpose()
        })
    }

    async fn count(&self) -> Result<i64, sqlx::Error> {
        on_pool!(self.pool, pool => {
            let row = sqlx::query("SELECT COUNT(*) AS total FROM issues")
                .fetch_one(pool)
                .await?;
            row.try_get::<i64, _>("total")
        })
    }
}

#[tokio::test]
async fn example_repository_stores_and_finds_an_issue() -> TestResult {
    each_database("example_repository_stores_and_finds_an_issue", async |db| {
        let issues = Issues { pool: db.pool() };
        issues.create_table().await?;
        issues.add(1, "Login fails").await?;
        assert_eq!(issues.title(1).await?.as_deref(), Some("Login fails"));
        assert_eq!(issues.title(2).await?, None);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn example_repository_starts_every_test_with_an_empty_database() -> TestResult {
    each_database(
        "example_repository_starts_every_test_with_an_empty_database",
        async |db| {
            let issues = Issues { pool: db.pool() };
            // The table does not exist yet: nothing is left over from other tests.
            assert!(issues.count().await.is_err());
            issues.create_table().await?;
            assert_eq!(issues.count().await?, 0);
            issues.add(7, "Slow search").await?;
            assert_eq!(issues.count().await?, 1);
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn example_repository_rejects_a_duplicate_id_the_same_way_on_both() -> TestResult {
    each_database(
        "example_repository_rejects_a_duplicate_id_the_same_way_on_both",
        async |db| {
            let issues = Issues { pool: db.pool() };
            issues.create_table().await?;
            issues.add(1, "First").await?;
            assert!(issues.add(1, "Second").await.is_err());
            assert_eq!(issues.title(1).await?.as_deref(), Some("First"));
            Ok(())
        },
    )
    .await
}
