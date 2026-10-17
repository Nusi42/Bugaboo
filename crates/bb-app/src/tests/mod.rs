//! Tests of the use cases against an in-memory store (M1-F18, M1-F52): no database, no HTTP. The
//! store (`fake`, `fake_policy`) has the same ports as the database adapter; `world` makes a small
//! server on it; the files below are the tests.

mod editing;
mod events;
mod failures;
mod fake;
mod fake_auth;
mod fake_editing;
mod fake_fields;
mod fake_handles;
mod fake_issues;
mod fake_layouts;
mod fake_many_changes;
mod fake_notifications;
mod fake_policy;
mod fake_projects;
mod fake_relations;
mod fake_settings;
mod fake_tags;
mod handles;
mod hidden;
mod key_scope;
mod keys;
mod layout_roles;
mod limits;
mod mail;
mod many_changes;
mod many_changes_property;
mod moving;
mod notifications;
mod recurring_jobs;
mod scenarios;
mod scenarios_fields;
mod scenarios_issues;
mod scenarios_layouts;
mod scenarios_notifications;
mod scenarios_relations;
mod scenarios_tags;
mod settings;
mod tags;
mod tags_hidden;
mod world;
