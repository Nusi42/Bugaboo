//! Database adapter: schema, migrations, repositories and the search compiler (ADR-0003).
//!
//! `bb-store` is the only crate that talks to a database. Schema and queries are written once with
//! the query builder and run on SQLite and PostgreSQL. Domain data is written only through
//! [`Database::record_change`], which appends the event and the outbox entry in the same
//! transaction (ADR-0007).

#![forbid(unsafe_code)]

mod anchor;
mod auth;
mod chain;
mod db;
mod error;
mod events;
mod jobs;
mod keys;
mod layout;
mod migrations;
mod schema;
mod secrets;
mod security_log;
mod settings;

pub use anchor::LogSeal;
pub use auth::AuthStorage;
pub use chain::{BreakReason, Verification};
pub use db::{Database, Dialect, Transaction};
pub use error::{SecretProblem, SettingProblem, StoreError};
pub use jobs::{FailureOutcome, NewJob, backoff};
pub use keys::{KeyRing, MASTER_KEY_FILE, MasterKey, SigningKey};
pub use layout::{DATABASE_FILE, KEYS_FOLDER, LOGS_FOLDER, SECURITY_LOG_ANCHOR_FILE};
pub use migrations::latest_version;
pub use schema::{StoredTable, tables};
pub use security_log::NewSecurityLogEntry;
pub use settings::{Scopes, SettingChange};

/// Helpers for the tests of this crate: they run on both databases through the harness.
#[cfg(test)]
mod test_support {
    use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, FieldChange};
    use bb_core::id::{EntityId, EventId, IdSource, SequentialIds, UserId};
    use bb_testing::TestDatabase;
    use jiff::Timestamp;

    use crate::Database;

    /// A moment `seconds` after 2026-01-01.
    pub(crate) fn at(seconds: i64) -> Timestamp {
        Timestamp::from_second(1_767_225_600 + seconds).unwrap()
    }

    /// A predictable ID source for the test.
    pub(crate) fn ids() -> SequentialIds {
        SequentialIds::default()
    }

    /// An event about a new record of the kind `issue`, made by a person, at `seconds`.
    pub(crate) fn event(ids: &mut impl IdSource, seconds: i64) -> DomainEvent {
        let person = UserId::generate(ids);
        DomainEvent {
            id: EventId::generate(ids).unwrap(),
            time: at(seconds),
            actor: Actor::Person(person),
            channel: Channel::Api,
            entity: EntityRef {
                kind: EntityKind::new("issue").unwrap(),
                id: EntityId::from(EventId::generate(ids).unwrap()),
            },
            changes: vec![FieldChange {
                field: "title".to_owned(),
                before: None,
                after: Some(serde_json::json!("Printer jams")),
            }],
            reason: Some("Reported".to_owned()),
        }
    }

    /// Opens the database of the test through the real `connect` and migrates it.
    pub(crate) async fn open(
        db: &TestDatabase,
    ) -> Result<Database, Box<dyn std::error::Error + Send + Sync>> {
        let database = Database::connect(db.url()).await?;
        database.migrate(at(0)).await?;
        Ok(database)
    }
}
