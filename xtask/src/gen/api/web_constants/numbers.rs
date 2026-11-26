//! The single numbers of the server that the browser client takes as constants: each is a plain
//! `const` of one Rust file, read as source and written once for the browser.

use super::super::TS_HEADER;
use super::notifications::parsed;
use super::number_const;
use crate::source::Source;

const MANY_CHANGES_SOURCE: &str = "crates/bb-core/src/many_changes.rs";
pub(super) const RELATION_MAP_SOURCE: &str = "crates/bb-app/src/relations/map.rs";

/// One number of the server as a constant for the browser client: `name` of the Rust file
/// `source`, written as `export const <ts>` with what it is, so the browser does not write it a
/// second time.
fn one_number(
    sources: &[Source],
    source: &str,
    name: &str,
    ts: &str,
    what: &str,
) -> anyhow::Result<String> {
    let number = number_const(&parsed(sources, source)?, source, name)?;
    Ok(format!(
        "{TS_HEADER}/** {what} (`{name}` of the server). */
export const {ts} = {number};
"
    ))
}

/// The most issues one change to many issues names: the rule of *Tick all results* is this number.
pub(in super::super) fn many_changes(sources: &[Source]) -> anyhow::Result<String> {
    one_number(
        sources,
        MANY_CHANGES_SOURCE,
        "MANY_CHANGES_ISSUES_MAX",
        "manyChangesIssuesMax",
        "The most issues one change to many issues names",
    )
}

/// The most issues one relation map holds: the map says it when it was cut there.
pub(in super::super) fn relation_map(sources: &[Source]) -> anyhow::Result<String> {
    one_number(
        sources,
        RELATION_MAP_SOURCE,
        "MAP_ISSUES_MOST",
        "mapIssuesMost",
        "The most issues one relation map holds",
    )
}
