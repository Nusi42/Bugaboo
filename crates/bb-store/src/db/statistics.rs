//! The statistics SQLite chooses its indexes by (`sqlite_stat1`), made again one table at a time
//! by `Database::refresh_statistics`: why they must be exact, and how long a round holds the write
//! lock, cold and warm, are in architecture §7 (*Performance*, the SQLite paragraph).

/// Reads the statistics into the connection again: a connection keeps those it read when it
/// opened, and does not see what another one wrote since (`ANALYZE sqlite_schema` only reloads).
pub(super) const RELOAD: &str = "ANALYZE sqlite_schema";

/// What `PRAGMA optimize` would do, only reported (0x01), for the tables that might need it
/// (0x02), looking at every table and not only those this connection read (0x10000): one
/// `ANALYZE "main"."<table>"` for each table that is out of date.
pub(super) const OUT_OF_DATE: &str = "PRAGMA optimize(0x10003)";

/// Writes a table that `ANALYZE` left without statistics, because it is empty, down as empty:
/// SQLite reports a table without an index and without statistics as out of date for ever, and
/// takes a row of `0` as up to date until the table grows. The name is bound twice: the row and
/// the test that it has none yet.
pub(super) const EMPTY_IS_UP_TO_DATE: &str = "INSERT INTO sqlite_stat1 (tbl, idx, stat) \
     SELECT ?, NULL, '0' WHERE NOT EXISTS (SELECT 1 FROM sqlite_stat1 WHERE tbl = ?)";

/// How SQLite's report starts and ends; the name of the table is between.
const STARTS: &str = "ANALYZE \"main\".\"";
const ENDS: &str = "\"";

/// The statements of SQLite's report that analyse one table of the database, each with the name
/// of its table. Anything else is left out: only SQLite's own text for one table is run.
pub(super) fn tables_to_analyse(reported: Vec<String>) -> Vec<(String, String)> {
    reported
        .into_iter()
        .filter_map(|statement| {
            let table = statement
                .strip_prefix(STARTS)?
                .strip_suffix(ENDS)
                .filter(|name| !name.is_empty() && !name.contains('"'))?
                .to_owned();
            Some((statement, table))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_01_only_the_analysis_of_one_table_of_the_database_is_taken_from_the_report() {
        let reported = [
            "ANALYZE \"main\".\"issue\"",
            "ANALYZE \"temp\".\"issue\"",
            "ANALYZE",
            "ANALYZE \"main\".\"a\"\"b\"",
            "ANALYZE \"main\".\"\"",
            "ANALYZE \"main\".\"event\"",
        ]
        .map(str::to_owned)
        .to_vec();
        let tables: Vec<String> = tables_to_analyse(reported)
            .into_iter()
            .map(|(_, table)| table)
            .collect();
        assert_eq!(tables, ["issue", "event"]);
    }
}
