//! ARCH-SQL-EDGE: only `bb-store` touches the database, and no SQL text is built with `format!`
//! (ADR-0003, SEC-17).
//!
//! Three rules:
//!
//! 1. A database library (`sqlx`, `sea-query`, `rusqlite`, the connection pools, …) is a
//!    dependency of `bb-store` only. The test harness `bb-testing` may use `sqlx` to create and
//!    drop the throwaway databases of the tests ([`HARNESS`]).
//! 2. Query text reaches sqlx in one place, [`QUERY_FILE`]: the query functions of sqlx
//!    (`query`, `query_with`, `query_scalar`, `query_as`, `raw_sql`) and `AssertSqlSafe` (which
//!    promises that a string is safe) are named nowhere else.
//! 3. A `format!`, `write!`, `concat!` or `push_str` call that builds a statement is found in
//!    every product crate. The call is read up to its closing parenthesis; a statement is
//!    recognised by its shape, in any letter case (`select … from`, `insert into`,
//!    `update … set`, `create table`, `grant … to`, `truncate`, `vacuum`, …), so ordinary
//!    sentences with one of the words do not count. Statements are built with the query builder,
//!    which binds all values.

use super::{Check, Context, Finding};
use crate::metadata::{self, Package};
use crate::rust_source::strip_comments;
use crate::source::{Kind, Source};

/// Libraries that talk to a database, build queries for one, or pool connections to one.
const DATABASE_LIBRARIES: &[&str] = &[
    "sqlx",
    "sqlx-core",
    "sqlx-sqlite",
    "sqlx-postgres",
    "sqlx-mysql",
    "sea-query",
    "sea-query-sqlx",
    "sea-orm",
    "rusqlite",
    "libsqlite3-sys",
    "libsql",
    "postgres",
    "tokio-postgres",
    "diesel",
    "mysql",
    "mysql_async",
    "r2d2",
    "deadpool",
    "deadpool-postgres",
    "bb8",
    "bb8-postgres",
];

/// The crate that owns the database.
const STORE: &str = "bb-store";

/// The test harness: it creates and drops test databases, which a query builder cannot do
/// (`CREATE DATABASE` takes a name, not a value). It builds the name from digits and letters
/// only, in one place each, may use only `sqlx`, and may run only those two statements (ADR-0003,
/// architecture §11).
const HARNESS: &str = "bb-testing";
const HARNESS_FILE: &str = "crates/bb-testing/src/lib.rs";
const HARNESS_LIBRARY: &str = "sqlx";
/// The only statements the harness may build as text.
const HARNESS_STATEMENTS: &[&str] = &["CREATE DATABASE", "DROP DATABASE"];

/// The one file where the output of the query builder meets sqlx.
const QUERY_FILE: &str = "crates/bb-store/src/db.rs";

/// The names that run query text through sqlx.
const QUERY_FUNCTIONS: &[&str] = &[
    "sqlx::query",
    "sqlx::raw_sql",
    "AssertSqlSafe",
    "query_with",
    "query_scalar",
    "query_as",
    "raw_sql",
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

/// What follows `create`, `drop` or `alter` in a statement.
const DEFINITION_OBJECTS: &[&str] = &[
    "table",
    "index",
    "database",
    "view",
    "trigger",
    "schema",
    "extension",
    "role",
    "user",
    "sequence",
    "type",
    "unique",
];

pub(crate) struct ArchSqlEdge;

impl Check for ArchSqlEdge {
    fn id(&self) -> &'static str {
        "ARCH-SQL-EDGE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let packages = metadata::load(context.repo.root())?;
        let sources = context.sources(self.id(), Kind::Rust)?;
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

/// The text of a call from its opening parenthesis to the closing one (quotes are respected, so
/// a parenthesis inside a string does not end it); a call that never closes is cut at 1000 bytes.
fn call_text(text: &str) -> &str {
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    for (at, character) in text.char_indices() {
        if in_string {
            match (escaped, character) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth <= 0 {
                    return &text[..=at];
                }
            }
            _ => {}
        }
        if at > 1000 {
            return &text[..at];
        }
    }
    text
}

/// The kinds of SQL statement in `text`, by their shape and in any letter case.
fn sql_statements(text: &str) -> Vec<&'static str> {
    let lower = text.to_ascii_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|token| !token.is_empty())
        .collect();
    let has_after = |first: usize, wanted: &[&str], within: usize| {
        tokens
            .iter()
            .skip(first + 1)
            .take(within)
            .any(|token| wanted.contains(token))
    };
    let mut found = Vec::new();
    for (at, token) in tokens.iter().enumerate() {
        let next = tokens.get(at + 1).copied();
        let kind = match *token {
            "select" if has_after(at, &["from"], 12) => "SELECT",
            "insert" if next == Some("into") => "INSERT INTO",
            "replace" if next == Some("into") => "REPLACE INTO",
            "update" if has_after(at, &["set"], 4) => "UPDATE",
            "delete" if next == Some("from") => "DELETE FROM",
            "create" if next.is_some_and(|n| DEFINITION_OBJECTS.contains(&n)) => {
                if next == Some("database") {
                    "CREATE DATABASE"
                } else {
                    "CREATE"
                }
            }
            "drop" if next.is_some_and(|n| DEFINITION_OBJECTS.contains(&n)) => {
                if next == Some("database") {
                    "DROP DATABASE"
                } else {
                    "DROP"
                }
            }
            "alter" if next.is_some_and(|n| DEFINITION_OBJECTS.contains(&n)) => "ALTER",
            "truncate" if !matches!(next, Some("to" | "the" | "at" | "it" | "after" | "long")) => {
                "TRUNCATE"
            }
            "grant" | "revoke"
                if has_after(at, &["on", "all"], 3) && has_after(at, &["to", "from"], 8) =>
            {
                "GRANT"
            }
            "copy" if has_after(at, &["stdin", "stdout", "program"], 12) => "COPY",
            "vacuum" => "VACUUM",
            "attach" if next == Some("database") => "ATTACH",
            "pragma" if next.is_some() => "PRAGMA",
            _ => continue,
        };
        found.push(kind);
    }
    found
}

/// Whether only the statements the harness may build are in `statements`.
fn only_harness_statements(statements: &[&str]) -> bool {
    !statements.is_empty()
        && statements
            .iter()
            .all(|statement| HARNESS_STATEMENTS.contains(statement))
}

fn evaluate_sources(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, original) in sources {
        // A comment about a call is not a call; the texts stay (the statements are in them).
        let text = &strip_comments(original);
        if path != QUERY_FILE {
            findings.extend(query_function_findings(path, text));
        }
        findings.extend(sql_text_findings(path, text));
    }
    findings
}

fn line_of(text: &str, at: usize) -> usize {
    text[..at].matches('\n').count() + 1
}

/// Names that run query text through sqlx, outside the one file.
fn query_function_findings(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut offset = 0;
    for line in text.lines() {
        // Importing a name is not calling it; the harness imports `AssertSqlSafe` for its two
        // statements.
        let imports = line.trim_start().starts_with("use ");
        if !imports {
            for name in QUERY_FUNCTIONS {
                for (column, _) in line.match_indices(name) {
                    let at = offset + column;
                    // The harness may wrap its two statements, nothing else.
                    if path == HARNESS_FILE && *name == "AssertSqlSafe" {
                        let call = call_text(&text[at + name.len()..]);
                        if only_harness_statements(&sql_statements(call)) {
                            continue;
                        }
                    }
                    findings.push(Finding::at(
                        format!("{path}:{}", line_of(text, at)),
                        format!(
                            "`{name}` runs query text through sqlx. That happens in one place, \
                             {QUERY_FILE}; build the query with sea-query and run it through \
                             `Database` or `Transaction`."
                        ),
                    ));
                }
            }
        }
        offset += line.len() + 1;
    }
    findings
}

/// SQL statements put together as text.
fn sql_text_findings(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for builder in TEXT_BUILDERS {
        for (at, _) in text.match_indices(builder) {
            let call = call_text(&text[at + builder.len() - 1..]);
            let statements = sql_statements(call);
            if statements.is_empty()
                || (path == HARNESS_FILE && only_harness_statements(&statements))
            {
                continue;
            }
            findings.push(Finding::at(
                format!("{path}:{}", line_of(text, at)),
                format!(
                    "SQL text (`{}`) is built with `{}`. Build statements with sea-query, which \
                     binds every value (SEC-17).",
                    statements[0],
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
            package("bb-app", &["sea-query", "r2d2"]),
            package(
                "bb-testing",
                &["rusqlite", "sqlx-core", "libsqlite3-sys", "deadpool"],
            ),
        ];
        let findings = evaluate_dependencies(&packages);
        assert_eq!(findings.len(), 7);
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
            "// sqlx::query is only in db.rs\nlet select = Query::select();\nlet message = format!(\"{count} jobs\");\nlet hint = format!(\"Select a project to update the list; copy it to your notes\");\n",
        );
        assert_eq!(report(&[store, other]), "");
    }

    #[test]
    fn bad_example_sql_built_with_format_in_any_case_and_across_lines() {
        let bad = source(
            "crates/bb-app/src/x.rs",
            "let sql = format!(\"SELECT * FROM issue WHERE id = {id}\");\nlet mut text = String::new();\ntext.push_str(\"delete from issue\");\nlet x = format!(\n    \"update issue\\n set title = {t}\"\n);\nlet y = write!(out, \"Truncate issue\"); let z = format!(\"VACUUM\"); let w = format!(\"grant all on issue to {u}\"); let q = format!(\"replace into t values ({v});\");\n",
        );
        let found = report(&[bad]);
        for kind in [
            "SELECT",
            "DELETE FROM",
            "UPDATE",
            "TRUNCATE",
            "VACUUM",
            "GRANT",
            "REPLACE INTO",
        ] {
            assert!(found.contains(&format!("(`{kind}`)")), "{kind} in {found}");
        }
    }

    #[test]
    fn a_semicolon_inside_the_call_does_not_hide_the_statement() {
        let bad = source(
            "crates/bb-app/src/x.rs",
            "let sql = format!(\"SELECT 1; \", a = 1).replace(\";\", \"\") + \" FROM issue\";\nlet s2 = format!(\"x; SELECT a;  FROM b\");\n",
        );
        assert!(report(&[bad]).contains("(`SELECT`)"));
    }

    #[test]
    fn bad_example_sqlx_called_outside_the_one_file() {
        let bad = source(
            "crates/bb-store/src/jobs.rs",
            "sqlx::query(AssertSqlSafe(sql)).execute(&pool);\nsqlx::raw_sql(text);\n",
        );
        let found = report(&[bad]);
        assert!(found.contains("`sqlx::query`"), "{found}");
        assert!(found.contains("`AssertSqlSafe`"), "{found}");
        assert!(found.contains("`sqlx::raw_sql`"), "{found}");
    }

    #[test]
    fn the_harness_may_name_a_database_but_nothing_else() {
        let harness = source(
            HARNESS_FILE,
            "admin.execute(AssertSqlSafe(format!(\"CREATE DATABASE \\\"{name}\\\"\"))).await;\nadmin.execute(AssertSqlSafe(format!(\"DROP DATABASE IF EXISTS \\\"{name}\\\" WITH (FORCE)\"))).await;\n",
        );
        assert_eq!(report(&[harness]), "");
        let sneaky = source(
            HARNESS_FILE,
            "admin.execute(AssertSqlSafe(format!(\"DROP TABLE {name}\"))).await;\nadmin.execute(AssertSqlSafe(other)).await;\nadmin.execute(AssertSqlSafe(format!(\"CREATE DATABASE x; DROP TABLE y\"))).await;\n",
        );
        let found = report(&[sneaky]);
        assert!(found.contains("(`DROP`)"), "{found}");
        assert_eq!(found.matches("`AssertSqlSafe`").count(), 3, "{found}");
        let elsewhere = source(
            "crates/bb-api/src/x.rs",
            "let sql = format!(\"CREATE DATABASE {name}\");\n",
        );
        assert!(report(&[elsewhere]).contains("(`CREATE DATABASE`)"));
    }

    #[test]
    fn the_product_code_of_this_repository_passes() {
        let repo = crate::repo::Repo::discover().unwrap();
        let sources = crate::source::read_sources(&repo, None, Kind::Rust).unwrap();
        assert_eq!(report(&sources), "");
        let packages = metadata::load(repo.root()).unwrap();
        let messages: Vec<String> = evaluate_dependencies(&packages)
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert!(messages.is_empty(), "{messages:?}");
    }
}
