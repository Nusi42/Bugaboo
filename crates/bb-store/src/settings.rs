//! Setting values per level (ADR-0018). The registry in `bb-core` says which settings exist, on
//! which levels they can be set and what type they have; this file stores the values and is the
//! only place that reads or writes the `setting` table (ARCH-SETTINGS).
//!
//! A value is stored for the organization, for one project or for one person. A lower level
//! overrides a higher one (person over project over organization) unless the organization locked
//! the setting. A key that is not in the registry is refused, so no value can exist that the
//! registry does not know about. A change is written through [`Database::record_change`], so it
//! comes with its event.

use bb_core::event::DomainEvent;
use bb_core::id::{EntityId, ProjectId, UserId};
use bb_core::settings::{DefaultValue, Level, Setting, SettingKind, SettingValue, StoredSetting};
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, OnConflict, Query};

use crate::db::{Database, Row, Transaction, to_micros};
use crate::error::{SettingProblem, StoreError};
use crate::schema::SettingRow as Columns;

/// A value to store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingChange {
    /// The key, as declared in the registry.
    pub key: String,
    /// The level the value is set on.
    pub level: Level,
    /// The project or person the value is for; `None` for the organization.
    pub scope: Option<EntityId>,
    /// The value.
    pub value: SettingValue,
    /// Whether lower levels may not override it (only organization values of lockable settings).
    pub locked: bool,
}

/// Whose settings are being read: lower levels are only looked at when given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scopes {
    /// The project, if the value is read in the context of one.
    pub project: Option<ProjectId>,
    /// The person, if the value is read for one.
    pub person: Option<UserId>,
}

/// Checks a change against the registry.
fn validate(change: &SettingChange) -> Result<TechnicalName, SettingProblem> {
    let definition = bb_core::settings::find(&change.key).ok_or(SettingProblem::UnknownKey)?;
    if !definition.levels().contains(&change.level) {
        return Err(SettingProblem::LevelNotAllowed);
    }
    if (change.level == Level::Organization) == change.scope.is_some() {
        return Err(SettingProblem::ScopeMismatch);
    }
    let fits = matches!(
        (definition.default(), &change.value),
        (DefaultValue::Bool(_), SettingValue::Bool(_))
            | (DefaultValue::Integer(_), SettingValue::Integer(_))
            | (DefaultValue::Text(_), SettingValue::Text(_))
    );
    if !fits {
        return Err(SettingProblem::WrongType);
    }
    if let (Some((least, most)), SettingValue::Integer(value)) =
        (definition.bounds(), &change.value)
        && !(least..=most).contains(value)
    {
        return Err(SettingProblem::OutOfBounds);
    }
    if let SettingValue::Text(value) = &change.value
        && !definition.list_fits(value)
    {
        return Err(SettingProblem::OutOfBounds);
    }
    if change.locked && !(definition.lockable() && change.level == Level::Organization) {
        return Err(SettingProblem::NotLockable);
    }
    TechnicalName::new(definition.key()).map_err(|_| SettingProblem::UnknownKey)
}

/// The key of a setting looked up for reading: it must be in the registry.
fn known(key: &str) -> Result<TechnicalName, SettingProblem> {
    let definition = bb_core::settings::find(key).ok_or(SettingProblem::UnknownKey)?;
    TechnicalName::new(definition.key()).map_err(|_| SettingProblem::UnknownKey)
}

/// Writes the value of a change inside the transaction of a change.
async fn upsert(
    transaction: &mut Transaction,
    key: &TechnicalName,
    change: &SettingChange,
    at: Timestamp,
) -> Result<(), StoreError> {
    let value = serde_json::to_string(&change.value)?;
    let mut insert = Query::insert();
    insert
        .into_table(Columns::Table)
        .columns([
            Columns::Key,
            Columns::Level,
            Columns::Scope,
            Columns::Value,
            Columns::Locked,
            Columns::UpdatedAt,
        ])
        .values([
            Expr::val(key.as_str()),
            Expr::val(change.level.code()),
            Expr::val(scope_bytes(change.scope)),
            Expr::val(value),
            Expr::val(change.locked),
            Expr::val(to_micros(at)),
        ])
        .map_err(|_| StoreError::Corrupt("a setting"))?
        .on_conflict(
            OnConflict::columns([Columns::Key, Columns::Level, Columns::Scope])
                .update_columns([Columns::Value, Columns::Locked, Columns::UpdatedAt])
                .to_owned(),
        );
    transaction.execute(&insert).await?;
    Ok(())
}

/// Stores the value of a setting of the whole organization inside the transaction of a change
/// that has its own event (the sign-in use cases change who may sign up this way).
pub(crate) async fn put_organization_setting(
    transaction: &mut Transaction,
    key: &str,
    value: &SettingValue,
    at: Timestamp,
) -> Result<(), StoreError> {
    let change = SettingChange {
        key: key.to_owned(),
        level: Level::Organization,
        scope: None,
        value: value.clone(),
        locked: false,
    };
    let key = validate(&change).map_err(StoreError::SettingRejected)?;
    upsert(transaction, &key, &change, at).await
}

fn scope_bytes(scope: Option<EntityId>) -> Vec<u8> {
    scope.map_or_else(Vec::new, |id| id.to_bytes().to_vec())
}

impl Database {
    /// Stores a value, together with its event (`event` is appended in the same transaction).
    ///
    /// # Errors
    /// Fails with [`StoreError::SettingRejected`] when the key is not in the registry, the level
    /// or the type does not fit, or the setting cannot be locked; and when the database fails.
    pub async fn set_setting(
        &self,
        event: &DomainEvent,
        change: &SettingChange,
    ) -> Result<(), StoreError> {
        let key = validate(change).map_err(StoreError::SettingRejected)?;
        self.record_change(event, async |transaction| {
            upsert(transaction, &key, change, event.time).await
        })
        .await
    }

    /// Removes a stored value, so the level below the removed one (or the default) applies
    /// again. The event is appended in the same transaction.
    ///
    /// # Errors
    /// Fails with [`StoreError::SettingRejected`] for a key that is not in the registry, and when
    /// the database fails.
    pub async fn clear_setting(
        &self,
        event: &DomainEvent,
        key: &str,
        level: Level,
        scope: Option<EntityId>,
    ) -> Result<bool, StoreError> {
        let key = known(key).map_err(StoreError::SettingRejected)?;
        self.record_change(event, async |transaction| {
            let mut delete = Query::delete();
            delete
                .from_table(Columns::Table)
                .and_where(Expr::col(Columns::Key).eq(key.as_str()))
                .and_where(Expr::col(Columns::Level).eq(level.code()))
                .and_where(Expr::col(Columns::Scope).eq(scope_bytes(scope)));
            Ok(transaction.execute(&delete).await? > 0)
        })
        .await
    }

    /// The value stored for one setting on one level, if there is one.
    ///
    /// # Errors
    /// Fails with [`StoreError::SettingRejected`] for a key that is not in the registry, and when
    /// the database fails or the stored value is unreadable.
    pub async fn stored_setting(
        &self,
        key: &str,
        level: Level,
        scope: Option<EntityId>,
    ) -> Result<Option<StoredSetting>, StoreError> {
        let key = known(key).map_err(StoreError::SettingRejected)?;
        let mut select = Query::select();
        select
            .columns([
                Columns::Key,
                Columns::Level,
                Columns::Scope,
                Columns::Value,
                Columns::Locked,
                Columns::UpdatedAt,
            ])
            .from(Columns::Table)
            .and_where(Expr::col(Columns::Key).eq(key.as_str()))
            .and_where(Expr::col(Columns::Level).eq(level.code()))
            .and_where(Expr::col(Columns::Scope).eq(scope_bytes(scope)));
        self.fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_setting)
            .transpose()
    }

    /// The value to use for a setting: the organization's if it is locked, else the person's,
    /// else the project's, else the organization's, else the default. A stored value of the
    /// wrong type is logged (by the key of the setting, never the value) and the default is
    /// used instead.
    ///
    /// # Errors
    /// Fails when the database fails or a stored value is unreadable.
    pub async fn resolve_setting<T: SettingKind>(
        &self,
        setting: &Setting<T>,
        scopes: &Scopes,
    ) -> Result<T::Value, StoreError> {
        let definition = setting.definition();
        let key = definition.key();
        let organization = self.stored_setting(key, Level::Organization, None).await?;
        let mut chosen = organization.clone();
        if !organization.as_ref().is_some_and(|stored| stored.locked) {
            // The project is looked at first and the person last, so the person wins.
            if let Some(project) = scopes.project
                && definition.levels().contains(&Level::Project)
                && let Some(stored) = self
                    .stored_setting(key, Level::Project, Some(project.into()))
                    .await?
            {
                chosen = Some(stored);
            }
            if let Some(person) = scopes.person
                && definition.levels().contains(&Level::Person)
                && let Some(stored) = self
                    .stored_setting(key, Level::Person, Some(person.into()))
                    .await?
            {
                chosen = Some(stored);
            }
        }
        match setting.resolve(chosen.as_ref().map(|stored| &stored.value)) {
            Ok(value) => Ok(value),
            Err(wrong) => {
                tracing::warn!(
                    setting = wrong.key,
                    "a stored setting has the wrong type; using the default"
                );
                setting
                    .default_value()
                    .map_err(|_| StoreError::Corrupt("a setting default"))
            }
        }
    }
}

fn read_setting(row: &Row) -> Result<StoredSetting, StoreError> {
    let scope = row.get::<Vec<u8>>(Columns::Scope)?;
    Ok(StoredSetting {
        key: TechnicalName::new(&row.get::<String>(Columns::Key)?)
            .map_err(|_| StoreError::Corrupt("a setting key"))?,
        level: Level::from_code(&row.get::<String>(Columns::Level)?)
            .ok_or(StoreError::Corrupt("a level"))?,
        scope: if scope.is_empty() {
            None
        } else {
            let bytes = <[u8; 16]>::try_from(scope).map_err(|_| StoreError::Corrupt("a scope"))?;
            Some(EntityId::from_bytes(bytes).map_err(|_| StoreError::Corrupt("a scope"))?)
        },
        value: serde_json::from_str(&row.get::<String>(Columns::Value)?)?,
        locked: row.get(Columns::Locked)?,
        updated_at: row.time(Columns::UpdatedAt)?,
    })
}

#[cfg(test)]
mod tests {
    use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef};
    use bb_core::id::EventId;
    use bb_core::settings::{
        APPEARANCE_THEME, SECURITY_SESSION_IDLE_MINUTES, SECURITY_TWO_STEP_REQUIRED,
    };
    use bb_testing::{TestResult, each_database};

    use super::*;
    use crate::test_support::{at, ids, open};

    /// The event of a settings change: about the setting `key`, made by the system.
    fn event(source: &mut impl bb_core::id::IdSource, key: &str) -> DomainEvent {
        let _ = source.now();
        DomainEvent {
            id: EventId::generate(source).unwrap(),
            time: at(5),
            actor: Actor::System,
            channel: Channel::System,
            entity: EntityRef {
                kind: EntityKind::new("setting").unwrap(),
                id: EntityId::from(EventId::generate(source).unwrap()),
            },
            changes: vec![bb_core::event::FieldChange {
                field: key.replace(['.', '-'], "_"),
                before: None,
                after: None,
            }],
            reason: None,
        }
    }

    fn change(
        key: &str,
        level: Level,
        scope: Option<EntityId>,
        value: SettingValue,
    ) -> SettingChange {
        SettingChange {
            key: key.to_owned(),
            level,
            scope,
            value,
            locked: false,
        }
    }

    fn text(value: &str) -> SettingValue {
        SettingValue::Text(value.to_owned())
    }

    #[tokio::test]
    async fn adr_0018_values_are_stored_per_level_and_the_lowest_level_wins() -> TestResult {
        each_database("adr_0018_levels", async |db| {
            let database = open(db).await?;
            let mut source = ids();
            let person = UserId::generate(&mut source);
            let someone_else = UserId::generate(&mut source);
            let project = ProjectId::generate(&mut source)?;
            let scopes = Scopes {
                project: Some(project),
                person: Some(person),
            };

            // Nothing stored: the default.
            let theme = &APPEARANCE_THEME;
            assert_eq!(database.resolve_setting(theme, &scopes).await?, "device");

            database
                .set_setting(
                    &event(&mut source, "appearance.theme"),
                    &change("appearance.theme", Level::Organization, None, text("paper")),
                )
                .await?;
            assert_eq!(database.resolve_setting(theme, &scopes).await?, "paper");

            database
                .set_setting(
                    &event(&mut source, "appearance.theme"),
                    &change(
                        "appearance.theme",
                        Level::Person,
                        Some(person.into()),
                        text("dusk"),
                    ),
                )
                .await?;
            assert_eq!(database.resolve_setting(theme, &scopes).await?, "dusk");
            let other = Scopes {
                person: Some(someone_else),
                ..scopes
            };
            assert_eq!(database.resolve_setting(theme, &other).await?, "paper");

            // A new value replaces the old one on the same level.
            database
                .set_setting(
                    &event(&mut source, "appearance.theme"),
                    &change(
                        "appearance.theme",
                        Level::Person,
                        Some(person.into()),
                        text("midnight"),
                    ),
                )
                .await?;
            assert_eq!(database.resolve_setting(theme, &scopes).await?, "midnight");

            // Taking the person's value away shows the organization's again.
            assert!(
                database
                    .clear_setting(
                        &event(&mut source, "appearance.theme"),
                        "appearance.theme",
                        Level::Person,
                        Some(person.into()),
                    )
                    .await?
            );
            assert_eq!(database.resolve_setting(theme, &scopes).await?, "paper");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn a_locked_organization_value_cannot_be_overridden() -> TestResult {
        each_database("adr_0018_locked", async |db| {
            let database = open(db).await?;
            let mut source = ids();
            let person = UserId::generate(&mut source);
            let scopes = Scopes {
                project: None,
                person: Some(person),
            };
            database
                .set_setting(
                    &event(&mut source, "appearance.theme"),
                    &change(
                        "appearance.theme",
                        Level::Person,
                        Some(person.into()),
                        text("dusk"),
                    ),
                )
                .await?;
            let mut organization =
                change("appearance.theme", Level::Organization, None, text("paper"));
            organization.locked = true;
            database
                .set_setting(&event(&mut source, "appearance.theme"), &organization)
                .await?;
            assert_eq!(
                database.resolve_setting(&APPEARANCE_THEME, &scopes).await?,
                "paper"
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn bad_example_the_registry_decides_what_may_be_stored() -> TestResult {
        each_database("adr_0018_rejected", async |db| {
            let database = open(db).await?;
            let mut source = ids();
            let person = UserId::generate(&mut source);
            let rejected = |problem: SettingProblem| {
                move |result: Result<(), StoreError>| {
                    assert!(
                        matches!(result, Err(StoreError::SettingRejected(found)) if found == problem),
                        "{result:?} should be {problem:?}"
                    );
                }
            };

            let unknown = change("appearance.colour", Level::Organization, None, text("x"));
            rejected(SettingProblem::UnknownKey)(
                database.set_setting(&event(&mut source, "a.b"), &unknown).await,
            );
            // A session that lasts a second, or a year, is a mistake: the bounds of the setting hold.
            for value in [1, 0, -5, 100_000] {
                let out_of_bounds = change(
                    "security.session-idle-minutes",
                    Level::Organization,
                    None,
                    SettingValue::Integer(value),
                );
                rejected(SettingProblem::OutOfBounds)(
                    database
                        .set_setting(&event(&mut source, "a.b"), &out_of_bounds)
                        .await,
                );
            }
            // A list that is longer than the setting allows, or has an item that is too long.
            for list in [
                (0..101)
                    .map(|n| format!("d{n}.test"))
                    .collect::<Vec<_>>()
                    .join(","),
                format!("{}.test", "a".repeat(250)),
            ] {
                let too_much = change(
                    "auth.sign-up-domains",
                    Level::Organization,
                    None,
                    SettingValue::Text(list),
                );
                rejected(SettingProblem::OutOfBounds)(
                    database
                        .set_setting(&event(&mut source, "a.b"), &too_much)
                        .await,
                );
            }
            let wrong_level = change(
                "security.session-idle-minutes",
                Level::Person,
                Some(person.into()),
                SettingValue::Integer(10),
            );
            rejected(SettingProblem::LevelNotAllowed)(
                database.set_setting(&event(&mut source, "a.b"), &wrong_level).await,
            );
            let wrong_type = change(
                "security.session-idle-minutes",
                Level::Organization,
                None,
                text("ten"),
            );
            rejected(SettingProblem::WrongType)(
                database.set_setting(&event(&mut source, "a.b"), &wrong_type).await,
            );
            let no_scope = change("appearance.theme", Level::Person, None, text("dusk"));
            rejected(SettingProblem::ScopeMismatch)(
                database.set_setting(&event(&mut source, "a.b"), &no_scope).await,
            );
            let mut locked = change(
                "security.two-step-required",
                Level::Organization,
                None,
                SettingValue::Bool(true),
            );
            locked.locked = true;
            rejected(SettingProblem::NotLockable)(
                database.set_setting(&event(&mut source, "a.b"), &locked).await,
            );
            assert!(matches!(
                database.stored_setting("appearance.colour", Level::Organization, None).await,
                Err(StoreError::SettingRejected(SettingProblem::UnknownKey))
            ));

            // Nothing was stored and no event was written for a refused change.
            assert_eq!(database.queued_job_count().await?, 0);
            assert!(
                database
                    .stored_setting("security.session-idle-minutes", Level::Organization, None)
                    .await?
                    .is_none()
            );
            assert!(
                database
                    .outbox_after(&TechnicalName::new("history").unwrap(), 10)
                    .await?
                    .is_empty()
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn a_setting_change_and_its_event_are_written_together() -> TestResult {
        each_database("adr_0018_event", async |db| {
            let database = open(db).await?;
            let mut source = ids();
            let event = event(&mut source, "security.session-idle-minutes");
            database
                .set_setting(
                    &event,
                    &change(
                        "security.session-idle-minutes",
                        Level::Organization,
                        None,
                        SettingValue::Integer(15),
                    ),
                )
                .await?;
            let stored = database
                .stored_setting("security.session-idle-minutes", Level::Organization, None)
                .await?
                .unwrap();
            assert_eq!(stored.value, SettingValue::Integer(15));
            assert_eq!(stored.updated_at, event.time);
            assert_eq!(database.events_about(&event.entity, 10).await?, [event]);
            assert_eq!(
                database
                    .resolve_setting(&SECURITY_SESSION_IDLE_MINUTES, &Scopes::default())
                    .await?,
                15
            );
            assert!(
                !database
                    .resolve_setting(&SECURITY_TWO_STEP_REQUIRED, &Scopes::default())
                    .await?
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn a_value_of_the_wrong_type_in_the_database_falls_back_to_the_default() -> TestResult {
        each_database("adr_0018_wrong_stored_type", async |db| {
            let database = open(db).await?;
            // Written around the registry, as a damaged or hand-edited database might be.
            let mut insert = Query::insert();
            insert
                .into_table(Columns::Table)
                .columns([
                    Columns::Key,
                    Columns::Level,
                    Columns::Scope,
                    Columns::Value,
                    Columns::Locked,
                    Columns::UpdatedAt,
                ])
                .values([
                    Expr::val("security.session-idle-minutes"),
                    Expr::val("organization"),
                    Expr::val(Vec::<u8>::new()),
                    Expr::val("\"soon\""),
                    Expr::val(false),
                    Expr::val(0_i64),
                ])?;
            database.execute(&insert).await?;
            assert_eq!(
                database
                    .resolve_setting(&SECURITY_SESSION_IDLE_MINUTES, &Scopes::default())
                    .await?,
                30
            );
            Ok(())
        })
        .await
    }
}
