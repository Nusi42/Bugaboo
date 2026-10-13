//! Use cases per feature, depending only on ports (traits) for storage, files, mail and AI (ADR-0001).

#![forbid(unsafe_code)]

pub mod access;
pub mod auth;
pub mod bulk;
pub mod comments;
pub mod context;
pub mod editing;
pub mod fields;
pub mod follow;
pub mod history;
mod input;
pub mod issues;
pub mod jobs;
mod member;
pub mod people;
pub mod ports;
pub mod preview;
pub mod projects;
pub mod relations;
pub mod roles;
pub mod settings;
pub mod status;
mod store_macros;
pub mod tags;

#[cfg(test)]
mod tests;
