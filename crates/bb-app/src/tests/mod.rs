//! Tests of the use cases against an in-memory store (M1-F18, M1-F52): no database, no HTTP. The
//! store (`fake`, `fake_policy`) has the same ports as the database adapter; `world` makes a small
//! server on it; the files below are the tests.

mod events;
mod failures;
mod fake;
mod fake_policy;
mod scenarios;
mod world;
