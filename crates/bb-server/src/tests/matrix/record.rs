//! The record of what the matrix covered: one line for every route with what each actor was
//! answered, so the file says what was proved and a change of any answer shows in review.
//!
//! The test compares what it ran with `tools/checks/sec-matrix-record.txt`; the check SEC-MATRIX
//! (xtask) holds that file against the API description, so a route that the file does not cover
//! fails there even before the tests run. To write the file again after adding a route or an
//! actor: the command of [`words::write_command`].

use bb_testing::Failure;
use std::collections::BTreeMap;
use std::fmt::Write as _;

use bb_api::openapi::Operation;
use bb_api_words::sec_matrix as words;

use super::actors::Actor;
use super::routes::Expect;

/// The root of the repository, from the folder of this crate.
const REPOSITORY_ROOT: &str = "../..";

/// What ran.
pub(super) struct Record {
    actors: Vec<String>,
    routes: BTreeMap<String, (Operation, Vec<(String, Expect)>)>,
}

impl Record {
    pub(super) fn new(actors: &[Actor]) -> Self {
        Self {
            actors: actors.iter().map(|actor| actor.name()).collect(),
            routes: BTreeMap::new(),
        }
    }

    /// An actor ran a route and the server answered as `expected`.
    pub(super) fn ran(&mut self, route: &Operation, actor: Actor, expected: Expect) {
        self.routes
            .entry(route.id.clone())
            .or_insert_with(|| (route.clone(), Vec::new()))
            .1
            .push((actor.name(), expected));
    }

    /// The text of the file.
    pub(super) fn text(&self) -> String {
        let mut text = format!(
            "# What the permission matrix covered (SEC-MATRIX): every route of the API description\n\
             # for every actor, with what the server answered. Written by the test\n\
             # `sec_01_every_route_answers_every_actor_as_the_permission_table_says`, which fails when\n\
             # this file differs from what ran. After adding a route or an actor, write it again:\n\
             # {}\n",
            words::write_command()
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
                route.id,
                route.method,
                route.path,
                route.needs.key(),
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
        let file = words::RECORD_PATH;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(REPOSITORY_ROOT)
            .join(file);
        let text = self.text();
        if std::env::var(words::WRITE_VARIABLE).as_deref() == Ok(words::WRITE_VALUE) {
            std::fs::write(&path, &text)?;
            return Ok(());
        }
        let stored = std::fs::read_to_string(&path)
            .map_err(|error| format!("could not read the record {file}: {error}"))?
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
            "the record {file} differs from what the matrix ran ({place}). Write it again with \
             {}={}.",
            words::WRITE_VARIABLE,
            words::WRITE_VALUE
        )
        .into())
    }
}

/// How an answer is written in the record.
const fn word(expected: Expect) -> &'static str {
    match expected {
        Expect::Allowed => words::ALLOWED,
        Expect::NotSignedIn => words::NOT_SIGNED_IN,
        Expect::Forbidden => words::FORBIDDEN,
        Expect::NotFound => words::NOT_FOUND,
        Expect::NotForKeys => words::NOT_FOR_KEYS,
    }
}

#[cfg(test)]
mod tests {
    use axum::http::Method;
    use bb_api::permissions::{Answer, Declared};
    use bb_app::access::{Permission, StartingRole};

    use super::*;

    fn route(operation: &str) -> Operation {
        Operation {
            id: operation.to_owned(),
            method: Method::GET,
            path: "/api/v1/x".to_owned(),
            needs: Declared::Permission(Permission::SeeProjects),
            kind: Answer::Ordinary,
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
