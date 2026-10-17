//! The permission matrix (SEC-MATRIX, WP-10): every route of the API description for every kind of
//! actor, written once and generated from the description, so a route that is added without its
//! permission or its example fails on every build.
//!
//! `actors` (who asks, and what each may do, written by hand from the documents), `examples` (the
//! table of example requests, one row per route; later packages add theirs), `routes` (the run
//! over the real server on both databases) and `record` (what ran, compared with the file that the
//! check SEC-MATRIX holds against the API description). The rows of fields and of kinds of people
//! information are tests of the engine, in `bb-policy` (`tests/engine/matrix.rs`).
//!
//! These files are part of the tests of the sign-in flows (`super`), whose server and browser
//! they use.

mod actors;
mod delivery;
mod examples;
mod examples_editing;
mod examples_fields;
mod examples_files;
mod examples_issues;
mod examples_layouts;
mod examples_people;
mod examples_relations;
mod examples_tags;
mod record;
mod routes;
