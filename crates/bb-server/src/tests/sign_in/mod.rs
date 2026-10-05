//! Signing in, tested through the real wiring: the use cases of `bb-app`, the store of `bb-store`
//! and, for the HTTP tests, the router of `bb-api`. Every flow runs on SQLite and on PostgreSQL
//! (`each_database`, ARCH-DB-PARITY).
//!
//! `rig` builds one complete server without a socket; the other files are the flows:
//! `setup_and_password` (AUTH-09, AUTH-01), `sessions` (SEC-12), `second_step` (AUTH-03),
//! `caching` (entity tags, `304` and what is never stored, PERF-03), `passkeys` (AUTH-02), `invitations` (AUTH-06), `secrets` (SEC-15, SEC-16), `status` (the administrators' status page, OPS-08), `http` (CSRF,
//! cookies, limits), `timing` (equal time for unknown accounts) and `tls` (a real browser-like
//! client over TLS and HTTP/2).

mod caching;
mod http;
mod invitations;
mod lists;
mod passkeys;
mod rig;
mod second_step;
mod secrets;
mod sessions;
mod setup_and_password;
mod status;
mod timing;
mod tls;
