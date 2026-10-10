//! Signing in, tested through the real wiring: the use cases of `bb-app`, the store of `bb-store`
//! and, for the HTTP tests, the router of `bb-api`. Every flow runs on SQLite and on PostgreSQL
//! (`each_database`, ARCH-DB-PARITY).
//!
//! `rig` builds one complete server without a socket; the other files are the flows:
//! `setup_and_password` (AUTH-09, AUTH-01), `sessions` (SEC-12), `second_step` (AUTH-03),
//! `caching` (entity tags, `304` and what is never stored, PERF-03), `passkeys` (AUTH-02), `invitations` (AUTH-06), `secrets` (SEC-15, SEC-16), `status` (the administrators' status page, OPS-08), `http` (CSRF,
//! cookies, limits), `timing` (equal time for unknown accounts) and `tls` (a real browser-like
//! client over TLS and HTTP/2).

mod access_keys;
mod access_keys_cursor;
mod access_keys_gate;
mod access_keys_guessing;
mod access_keys_http;
mod access_keys_never;
mod access_screens;
mod access_screens_review;
mod assignments_http;
mod caching;
mod editing;
pub(crate) mod http;
mod invitations;
mod issues_assign;
mod issues_editing;
mod issues_fields;
mod issues_http;
mod issues_ids;
mod issues_site;
mod lists;
// The permission matrix (SEC-MATRIX, WP-10) lives in `tests/matrix` and uses the server and the
// browser of these tests.
#[path = "../matrix/mod.rs"]
mod matrix;
mod own_access_http;
mod passkeys;
mod people_http;
mod permissions_http;
mod policy;
mod policy_probes;
mod policy_rig;
mod policy_rules;
mod projects;
mod projects_agree;
mod projects_http;
mod projects_review;
pub(crate) mod rig;
mod roles_http;
mod second_step;
mod second_step_by_role;
mod secrets;
mod session_ended;
mod sessions;
mod settings;
mod settings_http;
mod settings_keys;
mod setup_and_password;
mod status;
mod timing;
mod tls;
mod upgrade;
