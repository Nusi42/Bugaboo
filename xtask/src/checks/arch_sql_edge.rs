//! ARCH-SQL-EDGE: only `bb-store` touches the database, and no SQL text is built with `format!`
//! (ADR-0003, SEC-17).
//!
//! Three rules:
//!
//! 1. A database library (`sqlx`, `sea-query`, `rusqlite`, …) is a dependency of `bb-store` only.
//!    The test harness `bb-testing` may use `sqlx` to create and drop the throwaway databases of
//!    the tests ([`HARNESS`]).
//! 2. Query text reaches sqlx in one place, [`QUERY_FILE`]: the query functions of sqlx and
//!    `AssertSqlSafe` (which promises that a string is safe) are named nowhere else.
//! 3. A `format!`, `write!`, `concat!` or `push_str` call that builds a statement (it holds
//!    `SELECT`, `INSERT INTO`, `CREATE TABLE`, …) is found in every product crate. Statements are
//!    built with the query builder, which binds all values.

use super::{Check, Context, Finding};
use crate::metadata::{self, Package};
use crate::rust_source::{Source, read_product_sources};

/// Libraries that talk to a database or build queries for one.
const DATABASE_LIBRARIES: &[&str] = &[
    "sqlx",
    "sea-query",
    "sea-query-sqlx",
    "sea-orm",
    "rusqlite",
    "libsql",
    "postgres",
    "tokio-postgres",
    "diesel",
    "mysql",
    "mysql_async",
];

/// The crate that owns the database.
const STORE: &str = "bb-store";

/// The test harness: it creates and drops test databases, which a query builder cannot do
/// (`CREATE DATABASE` takes a name, not a value). It builds the name from digits and letters
/// only, in one place each, and may use only `sqlx` (ADR-0003, architecture §11).
const HARNESS: &str = "bb-testing";
const HARNESS_FILE: &str = "crates/bb-testing/src/lib.rs";
const HARNESS_LIBRARY: &str = "sqlx";
/// The statements the harness may build with `format!`.
const HARNESS_STATEMENTS: &[&str] = &["CREATE DATABASE", "DROP DATABASE"];

/// The one file where the output of the query builder meets sqlx.
const QUERY_FILE: &str = "crates/bb-store/src/db.rs";

/// The names that run query text through sqlx.
const QUERY_FUNCTIONS: &[&str] = &[
    "sqlx::query",
    "AssertSqlSafe",
    "query_with",
    "query_scalar",
    "query_as",
];

/// Calls that put text together.
const TEXT_BUILDERS: &[&str] = &[
    "format!(",
    "format_args!(",
    "write!(",
    "writeln!(",
    "concat!(",
    ".push_str(",
];

/// Words that start an SQL statement, in capitals as statements are written.
const SQL_WORDS: &[&str] = &[
    "SELECT ",
    "INSERT INTO",
    "UPDATE ",
    "DELETE FROM",
    "CREATE TABLE",
    "CREATE INDEX",
    "CREATE DATABASE",
    "DROP TABLE",
    "DROP DATABASE",
    "DROP INDEX",
    "ALTER TABLE",
    "PRAGMA ",
];

pub(crate) struct ArchSqlEdge;

impl Check for ArchSqlEdge {
    fn id(&self) -> &'static str {
        "ARCH-SQL-EDGE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let packages = metadata::load(context.repo.root())?;
        let sources = read_product_sources(context.repo)?;
        let mut findings = evaluate_dependencies(&packages);
        findings.extend(evaluate_sources(&sources));
        Ok(findings)
    }
}

fn evaluate_dependencies(packages: &[Package]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for package in packages {
        if package.name == STORE {
            continue;
        }
        for dependency in &package.dependencies {
            if !DATABASE_LIBRARIES.contains(&dependency.name.as_str()) {
                continue;
            }
            if package.name == HARNESS && dependency.name == HARNESS_LIBRARY {
                continue;
            }
            findings.push(Finding::at(
                &package.name,
                format!(
                    "depends on the database library `{}`. Only `{STORE}` touches the database; \
                     add a port (trait) in `bb-app` and implement it in `{STORE}` (ADR-0003).",
                    dependency.name
                ),
            ));
        }
    }
    findings
}

/// Whether the line is only a comment.
fn is_comment(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

fn evaluate_sources(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources {
        let allowed_here = path == QUERY_FILE || path == HARNESS_FILE;
        if path != QUERY_FILE {
            for (index, line) in text.lines().enumerate() {
                if is_comment(line) {
                    continue;
                }
                for name in QUERY_FUNCTIONS {
                    if line.contains(name) && !(path == HARNESS_FILE && *name == "AssertSqlSafe") {
                        findings.push(Finding::at(
                            format!("{path}:{}", index + 1),
                            format!(
                                "`{name}` runs query text through sqlx. That happens in one place, \
                                 {QUERY_FILE}; build the query with sea-query and run it through \
                                 `Database` or `Transaction`."
                            ),
                        ));
                    }
                }
            }
        }
        findings.extend(sql_text_findings(path, text, allowed_here));
    }
    findings
}

/// SQL statements put together as text.
fn sql_text_findings(path: &str, text: &str, harness_may: bool) -> Vec<Finding> {
    let mut findings = Vec::new();
    for builder in TEXT_BUILDERS {
        for (at, _) in text.match_indices(builder) {
            let line_start = text[..at].rfind('\n').map_or(0, |newline| newline + 1);
            if is_comment(&text[line_start..]) {
                continue;
            }
            let rest = &text[at..];
            let end = rest.find(';').unwrap_or(rest.len()).min(800);
            let statement = &rest[..end];
            let words: Vec<&str> = SQL_WORDS
                .iter()
                .copied()
                .filter(|word| statement.contains(word))
                .collect();
            let only_harness_words = !words.is_empty()
                && words
                    .iter()
                    .all(|word| HARNESS_STATEMENTS.iter().any(|ok| word.starts_with(ok)));
            if words.is_empty() || (path == HARNESS_FILE && harness_may && only_harness_words) {
                continue;
            }
            let line = text[..at].matches('\n').count() + 1;
            findings.push(Finding::at(
                format!("{path}:{line}"),
                format!(
                    "SQL text (`{}`) is built with `{}`. Build statements with sea-query, which \
                     binds every value (SEC-17).",
                    words[0].trim(),
                    builder.trim_end_matches('(')
                ),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::fixtures::package;

    fn source(path: &str, text: &str) -> Source {
        (path.to_owned(), text.to_owned())
    }

    fn report(sources: &[Source]) -> String {
        evaluate_sources(sources)
            .into_iter()
            .map(|finding| finding.message)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn good_example_the_store_alone_uses_database_libraries() {
        let packages = vec![
            package("bb-store", &["sqlx", "sea-query", "bb-core"]),
            package("bb-testing", &["sqlx"]),
            package("bb-api", &["axum", "bb-core"]),
        ];
        assert!(evaluate_dependencies(&packages).is_empty());
    }

    #[test]
    fn bad_example_another_crate_depends_on_a_database_library() {
        let packages = vec![
            package("bb-api", &["sqlx"]),
            package("bb-app", &["sea-query"]),
            package("bb-testing", &["rusqlite"]),
        ];
        let findings = evaluate_dependencies(&packages);
        assert_eq!(findings.len(), 3);
        assert!(findings[0].message.contains("Only `bb-store`"));
    }

    #[test]
    fn good_example_queries_are_built_with_the_builder_and_run_in_one_place() {
        let store = source(
            QUERY_FILE,
            "let (sql, values) = q.build_sqlx(b);\nsqlx::query_with(AssertSqlSafe(sql), values);\n",
        );
        let other = source(
            "crates/bb-store/src/jobs.rs",
            "// sqlx::query is only in db.rs\nlet select = Query::select();\nlet message = format!(\"{count} jobs\");\n",
        );
        assert_eq!(report(&[store, other]), "");
    }

    #[test]
    fn bad_example_sql_built_with_format() {
        let bad = source(
            "crates/bb-app/src/x.rs",
            "let sql = format!(\"SELECT * FROM issue WHERE id = {id}\");\nlet mut text = String::new();\ntext.push_str(\"DELETE FROM issue\");\n",
        );
        let found = report(&[bad]);
        assert!(
            found.contains("(`SELECT`) is built with `format!`"),
            "{found}"
        );
        assert!(found.contains("`DELETE FROM`"), "{found}");
    }

    #[test]
    fn bad_example_sqlx_called_outside_the_one_file() {
        let bad = source(
            "crates/bb-store/src/jobs.rs",
            "sqlx::query(AssertSqlSafe(sql)).execute(&pool);\n",
        );
        let found = report(&[bad]);
        assert!(found.contains("`sqlx::query`"), "{found}");
        assert!(found.contains("`AssertSqlSafe`"), "{found}");
    }

    #[test]
    fn the_harness_may_name_a_database_but_nothing_else() {
        let harness = source(
            HARNESS_FILE,
            "admin.execute(AssertSqlSafe(format!(\"CREATE DATABASE \\\"{name}\\\"\"))).await;\n",
        );
        assert_eq!(report(&[harness]), "");
        let sneaky = source(
            HARNESS_FILE,
            "admin.execute(AssertSqlSafe(format!(\"DROP TABLE {name}\"))).await;\n",
        );
        assert!(report(&[sneaky]).contains("`DROP TABLE`"));
        let elsewhere = source(
            "crates/bb-api/src/x.rs",
            "let sql = format!(\"CREATE DATABASE {name}\");\n",
        );
        assert!(report(&[elsewhere]).contains("`CREATE DATABASE`"));
    }

    #[test]
    fn the_product_code_of_this_repository_passes() {
        let repo = crate::repo::Repo::discover().unwrap();
        let sources = read_product_sources(&repo).unwrap();
        assert_eq!(report(&sources), "");
        let packages = metadata::load(repo.root()).unwrap();
        let messages: Vec<String> = evaluate_dependencies(&packages)
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert!(messages.is_empty(), "{messages:?}");
    }
}
