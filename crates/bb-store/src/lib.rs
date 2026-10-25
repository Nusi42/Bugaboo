//! Database adapter: schema, migrations, repositories and the search compiler (ADR-0003).
//!
//! `bb-store` is the only crate that talks to a database. Schema and queries are written once with
//! the query builder and run on SQLite and PostgreSQL. Domain data is written only through
//! [`Database::record_change`], which appends the event and the outbox entry in the same
//! transaction (ADR-0007).

#![forbid(unsafe_code)]

mod access_keys;
mod anchor;
mod auth;
mod chain;
mod comments;
#[cfg(feature = "dataset")]
mod dataset;
mod db;
mod editing;
mod error;
mod events;
mod fields;
mod files;
mod follow;
mod handles;
mod history;
mod issues;
mod jobs;
mod keys;
mod layout;
mod layouts;
mod locks;
mod many_changes;
mod migrations;
mod notifications;
mod offers;
mod paging;
mod policy;
mod projects;
mod relations;
mod saved_searches;
mod schema;
mod secrets;
mod security_log;
mod settings;
mod tags;
#[cfg(feature = "testing")]
mod testing;
mod workflows;

pub use anchor::LogSeal;
pub use auth::AuthStorage;
pub use chain::{BreakReason, Verification};
pub use db::{Database, Dialect, Transaction};
pub use error::{SecretProblem, SettingProblem, StoreError};
pub use jobs::{NewJob, backoff};
pub use keys::{KeyRing, MASTER_KEY_FILE, MasterKey, OpenMode, SigningKey};
pub use layout::{DATABASE_FILE, KEYS_FOLDER, LOGS_FOLDER, SECURITY_LOG_ANCHOR_FILE};
pub use migrations::latest_version;
pub use schema::{StoredTable, tables};
pub use security_log::NewSecurityLogEntry;
#[cfg(feature = "testing")]
pub use settings::M3Setting;
pub use settings::{Scopes, SettingChange, StoredValue};
#[cfg(feature = "testing")]
pub use testing::M1Person;

/// Helpers for the tests of this crate: they run on both databases through the harness.
#[cfg(test)]
mod test_support {
    use std::sync::Arc;

    use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, FieldChange};
    use bb_core::id::{EntityId, EventId, IdSource, SequentialIds, UserId};
    use bb_core::person::Person;
    use bb_testing::TestDatabase;
    use jiff::Timestamp;

    use bb_app::access::PolicyStore as _;
    use bb_app::issues::IssueStore as _;
    use bb_app::ports::ChangeStore;
    use bb_core::id::{IssueId, IssueTypeId, ProjectId};
    use bb_core::issue::{Issue, IssueIcon, IssueType, Priority};
    use bb_core::project::{Project, ShortName};
    use bb_core::readable_id::IssueCode;
    use bb_core::version::FIRST_VERSION;
    use bb_core::workflow::starter;

    use crate::{AuthStorage, Database, LogSeal};

    /// A moment `seconds` after 2026-01-01.
    pub(crate) fn at(seconds: i64) -> Timestamp {
        Timestamp::from_second(1_767_225_600 + seconds).unwrap()
    }

    /// A row of an issue type, written the way the program wrote it before issue types had a project
    /// (the columns of migration 7): what a test of a migration starts from.
    #[cfg(test)]
    pub(crate) async fn insert_old_issue_type(
        database: &Database,
        id: IssueTypeId,
        name: &str,
    ) -> Result<(), crate::StoreError> {
        use crate::db::StoredId as _;
        use crate::schema::IssueTypeRow;
        use sea_query::{Expr, Query};

        let mut insert = Query::insert();
        insert
            .into_table(IssueTypeRow::Table)
            .columns([
                IssueTypeRow::Id,
                IssueTypeRow::Name,
                IssueTypeRow::Icon,
                IssueTypeRow::Position,
                IssueTypeRow::Version,
            ])
            .values([
                Expr::val(id.stored()),
                Expr::val(name),
                Expr::val("square-check"),
                Expr::val(0_i64),
                Expr::val(1_i64),
            ])
            .map_err(|_| crate::StoreError::Corrupt("an issue type"))?;
        database.execute(&insert).await.map(|_| ())
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

    /// A person for the tests, with the e-mail address.
    pub(crate) fn person(ids: &mut impl IdSource, email: &str) -> Person {
        Person {
            id: UserId::generate(ids),
            display_name: "Test Person".to_owned(),
            email: email.to_owned(),
            created_at: at(0),
        }
    }

    pub(crate) fn issue(
        source: &mut impl IdSource,
        project: ProjectId,
        issue_type: IssueTypeId,
        number: i64,
    ) -> Issue {
        Issue {
            id: IssueId::generate(source).unwrap(),
            project,
            issue_type,
            code: IssueCode::counting(number),
            title: format!("Issue {number}"),
            description: String::new(),
            status: starter::open(),
            priority: Priority::Normal,
            reported_by: UserId::generate(source),
            created_at: at(1),
            updated_at: at(1),
            closed_at: None,
            deleted_at: None,
            version: FIRST_VERSION,
        }
    }

    /// A project, a type and the issues the tests put tags and labels on.
    pub(crate) struct World {
        pub(crate) project: ProjectId,
        pub(crate) issues: Vec<Issue>,
        pub(crate) person: UserId,
    }

    pub(crate) async fn world(
        storage: &AuthStorage,
        source: &mut impl IdSource,
        count: i64,
    ) -> World {
        let project = ProjectId::generate(source).unwrap();
        let issue_type = IssueTypeId::generate(source).unwrap();
        let issues: Vec<Issue> = (1..=count)
            .map(|number| issue(source, project, issue_type, number))
            .collect();
        let (row, kind, rows) = (
            Project {
                id: project,
                parent: None,
                short_name: ShortName::new("ACME").unwrap(),
                name: "Acme".to_owned(),
                version: FIRST_VERSION,
            },
            IssueType {
                project: None,
                id: issue_type,
                name: "Bug".to_owned(),
                icon: IssueIcon::Bug,
                position: 0,
                retired_at: None,
                version: FIRST_VERSION,
            },
            issues.clone(),
        );
        ChangeStore::record_change(storage, &event(source, 5), move |tx| {
            Box::pin(async move {
                AuthStorage::insert_project(tx, &row).await?;
                AuthStorage::insert_issue_type(tx, &kind).await?;
                for issue in &rows {
                    AuthStorage::insert_issue(tx, issue, &[]).await?;
                }
                Ok(())
            })
        })
        .await
        .unwrap();
        World {
            project,
            issues,
            person: UserId::generate(source),
        }
    }

    /// The storage of the sign-in data on a database of the test, with its keys and its log seal
    /// in a folder that goes away with it.
    pub(crate) struct Fixture {
        pub(crate) storage: AuthStorage,
        _folder: tempfile::TempDir,
    }

    pub(crate) async fn fixture(
        db: &TestDatabase,
    ) -> Result<Fixture, Box<dyn std::error::Error + Send + Sync>> {
        let database = open(db).await?;
        let folder = tempfile::tempdir()?;
        std::fs::create_dir_all(folder.path().join("logs"))?;
        let keys = Arc::new(database.open_keys(&folder.path().join("keys")).await?);
        let seal = LogSeal::new(&keys, folder.path().join("logs").join("head.log"));
        Ok(Fixture {
            storage: AuthStorage::new(database, keys, seal),
            _folder: folder,
        })
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
