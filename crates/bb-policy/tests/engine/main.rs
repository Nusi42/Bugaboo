//! The acceptance tests of the policy engine (M2 WP-05): every rule of ADR-0025 on plain data.
//! The tests that run the engine through the store, on both databases, are in `bb-store`.

// Dependencies of the library that these tests do not use themselves (`unused_crate_dependencies`).
#[cfg(feature = "openapi")]
use utoipa as _;

#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod common;
#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod compile_fail;
#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod evaluation;
#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod guards;
#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod masks;
#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod people;
#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod properties;
