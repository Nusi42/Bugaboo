//! The record of what the matrix covered: one line for every route with what each actor was
//! answered, so the file says what was proved and a change of any answer shows in review.
//!
//! The test compares what it ran with `tools/checks/sec-matrix-record.txt`; the check SEC-MATRIX
//! (xtask) holds that file against the API description, so a route that the file does not cover
//! fails there even before the tests run. To write the file again after adding a route or an
//! actor: `BB_SEC_MATRIX_RECORD=write cargo xtask db run -- cargo test -p bb-server sec_01_every_route`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::actors::Actor;
use super::routes::{Expect, Route};
use crate::sign_in_tests::http::Failure;

/// The file, from the folder of this crate.
const FILE: &str = "../../tools/checks/sec-matrix-record.txt";
/// Set to `write` to write the file instead of comparing with it.
const WRITE_VARIABLE: &str = "BB_SEC_MATRIX_RECORD";

/// What ran.
pub(super) struct Record {
    actors: Vec<String>,
    routes: BTreeMap<String, (Route, Vec<(String, Expect)>)>,
}

impl Record {
    pub(super) fn new(actors: &[Actor]) -> Self {
        Self {
            actors: actors.iter().map(|actor| actor.name()).collect(),
            routes: BTreeMap::new(),
        }
    }

    /// An actor ran a route and the server answered as `expected`.
    pub(super) fn ran(&mut self, route: &Route, actor: Actor, expected: Expect) {
        self.routes
            .entry(route.operation.clone())
            .or_insert_with(|| (route.clone(), Vec::new()))
            .1
            .push((actor.name(), expected));
    }

    /// The text of the file.
    pub(super) fn text(&self) -> String {
        let mut text = String::from(
            "# What the permission matrix covered (SEC-MATRIX): every route of the API description\n\
             # for every actor, with what the server answered. Written by the test\n\
             # `sec_01_every_route_answers_every_actor_as_the_permission_table_says`, which fails when\n\
             # this file differs from what ran. After adding a route or an actor, write it again:\n\
             # BB_SEC_MATRIX_RECORD=write cargo xtask db run -- cargo test -p bb-server sec_01_every_route\n",
        );
        let _ = writeln!(text, "actors {}", self.actors.join(" "));
        for (route, ran) in self.routes.values() {
            let answers: Vec<String> = ran
                .iter()
                .map(|(actor, expected)| format!("{actor}={}", word(*expected)))
                .collect();
            let _ = writeln!(
                text,
                "route {} {} {} {} : {}",
                route.operation,
                route.method,
                route.path,
                route.declared,
                answers.join(" ")
            );
        }
        text
    }

    /// Compares the text with the file of the repository, or writes the file when asked to.
    ///
    /// # Errors
    /// The file is missing, or differs from what ran.
    pub(super) fn matches_the_repository(&self) -> Result<(), Failure> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FILE);
        let text = self.text();
        if std::env::var(WRITE_VARIABLE).as_deref() == Ok("write") {
            std::fs::write(&path, &text)?;
            return Ok(());
        }
        let stored = std::fs::read_to_string(&path)
            .map_err(|error| format!("could not read the record {FILE}: {error}"))?
            .replace("\r\n", "\n");
        if stored == text {
            return Ok(());
        }
        let first = stored
            .lines()
            .zip(text.lines())
            .enumerate()
            .find(|(_, (before, now))| before != now);
        let place = first.map_or_else(
            || "one has more lines than the other".to_owned(),
            |(line, (before, now))| {
                format!(
                    "line {}: the file has `{before}`, the matrix ran `{now}`",
                    line + 1
                )
            },
        );
        Err(format!(
            "the record {FILE} differs from what the matrix ran ({place}). Write it again with \
             {WRITE_VARIABLE}=write."
        )
        .into())
    }
}

/// How an answer is written in the record.
const fn word(expected: Expect) -> &'static str {
    match expected {
        Expect::Allowed => "allowed",
        Expect::NotSignedIn => "not-signed-in",
        Expect::Forbidden => "forbidden",
        Expect::NotFound => "not-found",
    }
}

#[cfg(test)]
mod tests {
    use bb_app::access::StartingRole;

    use super::*;

    fn route(operation: &str) -> Route {
        Route {
            operation: operation.to_owned(),
            method: "GET".to_owned(),
            path: "/api/v1/x".to_owned(),
            declared: "see-projects".to_owned(),
        }
    }

    #[test]
    fn the_text_lists_the_actors_and_one_line_for_every_route_in_the_order_of_its_name() {
        let actors = [Actor::Nobody, Actor::Role(StartingRole::Guest)];
        let mut record = Record::new(&actors);
        for operation in ["zeta", "alpha"] {
            for actor in actors {
                record.ran(&route(operation), actor, Expect::Forbidden);
            }
        }
        let text = record.text();
        assert!(text.contains("actors nobody role-guest\n"));
        let alpha = text.find("route alpha").unwrap_or(usize::MAX);
        let zeta = text.find("route zeta").unwrap_or(usize::MAX);
        assert!(alpha < zeta);
        assert!(text.contains(
            "route alpha GET /api/v1/x see-projects : nobody=forbidden role-guest=forbidden\n"
        ));
    }
}
