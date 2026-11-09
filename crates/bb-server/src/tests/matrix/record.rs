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
use bb_core::ErrorCode;
use bb_core::message::Message;

use super::actors::Actor;
use super::delivery::Delivered;
use super::routes::Expect;

/// The root of the repository, from the folder of this crate.
const REPOSITORY_ROOT: &str = "../..";

/// What ran.
pub(super) struct Record {
    actors: Vec<String>,
    routes: BTreeMap<String, (Operation, Vec<(String, Expect)>)>,
    deliveries: Vec<Delivered>,
}

impl Record {
    pub(super) fn new(actors: &[Actor]) -> Self {
        Self {
            actors: actors.iter().map(|actor| actor.name()).collect(),
            routes: BTreeMap::new(),
            deliveries: Vec::new(),
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

    /// What the server sent on its own to the people who could receive it.
    pub(super) fn delivered(&mut self, deliveries: Vec<Delivered>) {
        self.deliveries = deliveries;
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
        for delivered in &self.deliveries {
            let _ = writeln!(text, "{}", delivered.line());
        }
        for (route, ran) in self.routes.values() {
            let answers: Vec<String> = ran
                .iter()
                .map(|(actor, expected)| format!("{actor}={}", word(*expected)))
                .collect();
            let _ = writeln!(
                text,
                "route {} {} {} {} {} : {}",
                route.id,
                words::ENTRY_API,
                route.method,
                route.path,
                route.needs.key(),
                answers.join(" ")
            );
        }
        text
    }

    /// Compares the text with the file of the repository, or writes the file when asked to. The
    /// lines of the tools and of the assistant are written and compared by their own tests; they
    /// stay as they are here.
    ///
    /// # Errors
    /// The file is missing, or differs from what ran.
    pub(super) fn matches_the_repository(&self) -> Result<(), Failure> {
        let (path, stored) = stored()?;
        let text = self.text();
        let others = |keep: bool| {
            stored
                .lines()
                .filter(|line| is_of_another_test(line) == keep)
                .fold(String::new(), |mut text, line| {
                    let _ = writeln!(text, "{line}");
                    text
                })
        };
        if std::env::var(words::WRITE_VARIABLE).as_deref() == Ok(words::WRITE_VALUE) {
            std::fs::write(&path, format!("{text}{}", others(true)))?;
            return Ok(());
        }
        let stored = others(false);
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
            "the record {} differs from what the matrix ran ({place}). Write it again with              {}={}.",
            words::RECORD_PATH,
            words::WRITE_VARIABLE,
            words::WRITE_VALUE
        )
        .into())
    }
}

/// The lines of the record that other tests than the route matrix write and compare: the lines of
/// the tools and of the assistant.
const OTHER_TESTS: [&str; 2] = [words::TOOL_LINE, words::ASK_LINE];

/// Whether a line of the record is written by another test than the route matrix.
fn is_of_another_test(line: &str) -> bool {
    OTHER_TESTS.iter().any(|prefix| line.starts_with(prefix))
}

/// The record of the repository, with the line ends of Unix and the path it was read from.
fn stored() -> Result<(std::path::PathBuf, String), Failure> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(REPOSITORY_ROOT)
        .join(words::RECORD_PATH);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read the record {}: {error}", words::RECORD_PATH))?;
    Ok((
        path,
        text.replace(
            "

", "
",
        ),
    ))
}

/// Compares the lines that start with `prefix` with the file of the repository, or writes them
/// when asked to, and keeps every other line as it is: the lines of the tools and of the assistant
/// each have a test of their own.
///
/// # Errors
/// The file is missing, or its lines differ from what ran.
pub(super) fn matches_the_repository_for(prefix: &str, found: &[String]) -> Result<(), Failure> {
    let (path, stored) = stored()?;
    if std::env::var(words::WRITE_VARIABLE).as_deref() == Ok(words::WRITE_VALUE) {
        let mut text: String = stored
            .lines()
            .filter(|line| !line.starts_with(prefix))
            .fold(String::new(), |mut text, line| {
                let _ = writeln!(text, "{line}");
                text
            });
        for line in found {
            let _ = writeln!(text, "{line}");
        }
        std::fs::write(&path, text)?;
        return Ok(());
    }
    let wanted: Vec<&str> = found.iter().map(String::as_str).collect();
    let kept: Vec<&str> = stored
        .lines()
        .filter(|line| line.starts_with(prefix))
        .collect();
    if kept == wanted {
        return Ok(());
    }
    let place = kept
        .iter()
        .zip(&wanted)
        .enumerate()
        .find(|(_, (before, now))| before != now)
        .map_or_else(
            || "one has more lines than the other".to_owned(),
            |(at, (before, now))| {
                format!(
                    "line {} of `{}`: the file has `{before}`, the matrix ran `{now}`",
                    at + 1,
                    prefix.trim()
                )
            },
        );
    Err(format!(
        "the `{}` lines of the record differ from what the matrix ran ({place}). Write them again          with {}={}.",
        prefix.trim(),
        words::WRITE_VARIABLE,
        words::WRITE_VALUE
    )
    .into())
}

/// What a refusal of a tool or of *Apply* says, in the words of the record: the person may not
/// know of it (`not-found`) or may not do it (`forbidden`). `None` is no answer of the matrix.
pub(super) fn refusal_word(text: &str) -> Option<&'static str> {
    let said = |code: ErrorCode| Message::from(code.message()).render();
    if text == said(ErrorCode::NotFound) {
        Some(words::NOT_FOUND)
    } else if text == said(ErrorCode::Forbidden) {
        Some(words::FORBIDDEN)
    } else {
        None
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
            reads_by_post: false,
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
            "route alpha api GET /api/v1/x see-projects : nobody=forbidden role-guest=forbidden\n"
        ));
    }
}
