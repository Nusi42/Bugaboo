//! Use cases per feature, depending only on ports (traits) for storage, files, mail and AI (ADR-0001).

#![forbid(unsafe_code)]

pub mod access;
pub mod auth;
pub mod context;
pub mod editing;
mod input;
pub mod people;
pub mod ports;
pub mod projects;
pub mod roles;
pub mod settings;
pub mod status;
mod store_macros;

#[cfg(test)]
mod tests;
