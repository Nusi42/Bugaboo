//! The words that the developer tools, the tests and `xtask` share, each written once: which
//! databases Bugaboo runs on and the variables that tell the test harness what to do
//! ([`database`]), and the schema of `tools/dev/sample.json`, which says where the sample
//! organization is made and what its files, groups, projects and roles are called ([`sample`]).
//!
//! The crate does no work and no input or output, so `bb-testing`, `bb-sample` and `xtask` can
//! import it without pulling in the store or the use cases (coding-standards §1, one definition).

#![forbid(unsafe_code)]

pub mod database;
pub mod sample;
