//! Use cases per feature, depending only on ports (traits) for storage, files, mail and AI (ADR-0001).

#![forbid(unsafe_code)]

pub mod access;
pub mod auth;
pub mod comments;
pub mod context;
pub mod editing;
pub mod fields;
pub mod files;
pub mod first_start;
pub mod follow;
pub mod history;
mod input;
pub mod issues;
pub mod jobs;
mod keeping;
pub mod layouts;
pub mod mailer;
pub mod many_changes;
pub mod member;
pub mod notifications;
pub mod people;
pub mod ports;
pub mod preview;
pub mod projects;
pub mod relations;
pub mod roles;
pub mod server_log;
pub mod settings;
pub mod status;
mod store_macros;
pub mod tags;

#[cfg(test)]
mod tests;
