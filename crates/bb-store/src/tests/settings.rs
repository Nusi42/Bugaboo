//! Setting values on both databases: the layers (CFG-06), the person's own values (CFG-04), the
//! lock, *Reset to default*, and what the registry refuses (ADR-0018).

use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, FieldChange};
use bb_core::id::{EventId, IdSource, ProjectId, UserId};
use bb_core::project::{Project, ShortName};
use bb_core::settings::{
    APPEARANCE_MODE, SECURITY_SESSION_IDLE_MINUTES, SECURITY_TWO_STEP_REQUIRED, Source,
    for_tests::{TEST_PROJECT_LIMIT, TEST_PROJECT_NOTE, TEST_SWITCH},
};
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, fixture, ids, person};

/// The event of a settings change: about the setting `key`, made by the system.
fn event(source: &mut impl IdSource, key: &str) -> DomainEvent {
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
        changes: vec![FieldChange {
            field: key.replace(['.', '-'], "_"),
            before: None,
            after: None,
        }],
        reason: None,
    }
}

fn change(key: &str, scope: Scope, value: SettingValue) -> SettingChange {
    SettingChange {
        key: key.to_owned(),
        scope,
        value,
        locked: false,
    }
}

fn text(value: &str) -> SettingValue {
    SettingValue::Text(value.to_owned())
}

/// Runs `write` as a change with its event, the way a use case does.
async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    let event = event(source, "x.y");
    ChangeStore::record_change(storage, &event, write).await
}

async fn create_person(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    email: &str,
) -> Result<UserId, StoreError> {
    let person = person(source, email);
    let id = person.id;
    changed(storage, source, move |tx| {
        Box::pin(async move {
            <AuthStorage as bb_app::auth::AccountStore>::insert_person(tx, &person, "hash", at(1))
                .await
        })
    })
    .await?;
    Ok(id)
}

async fn create_project(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    parent: Option<ProjectId>,
    short_name: &str,
) -> Result<ProjectId, StoreError> {
    let project = Project {
        id: ProjectId::generate(source).map_err(|_| StoreError::Corrupt("an ID"))?,
        parent,
        short_name: ShortName::new(short_name).map_err(|_| StoreError::Corrupt("a name"))?,
        name: short_name.to_owned(),
        version: 1,
    };
    let id = project.id;
    changed(storage, source, move |tx| {
        Box::pin(async move {
            <AuthStorage as bb_app::access::PolicyStore>::insert_project(tx, &project).await
        })
    })
    .await?;
    Ok(id)
}

async fn put(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    scope: Scope,
    key: &'static str,
    value: SettingValue,
) -> Result<(), StoreError> {
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::put_setting_value(tx, scope, key, &value, at(6)).await })
    })
    .await
}

async fn clear(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    scope: Scope,
    key: &'static str,
) -> Result<bool, StoreError> {
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::clear_setting_value(tx, scope, key).await })
    })
    .await
}

async fn lock(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    key: &'static str,
    locked: bool,
) -> Result<(), StoreError> {
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::put_setting_lock(tx, key, locked, at(7)).await })
    })
    .await
}

const NOTE: &str = "test.project-note";

fn resolved_note(layers: &Layers) -> (SettingValue, Source, bool) {
    let found = layers.resolve(TEST_PROJECT_NOTE.definition());
    (found.value, found.source, found.locked)
}

#[tokio::test]
async fn adr_0018_values_are_stored_per_level_and_the_lowest_level_wins() -> TestResult {
    each_database("adr_0018_levels", async |db| {
        let fixture = fixture(db).await?;
        let (storage, database) = (&fixture.storage, fixture.storage.database());
        let mut source = ids();
        let person = create_person(storage, &mut source, "ada@example.test").await?;
        let someone_else = create_person(storage, &mut source, "bea@example.test").await?;
        let project = create_project(storage, &mut source, None, "ACME").await?;
        let scopes = Scopes {
            project: Some(project),
            person: Some(person),
        };

        // Nothing stored: the default.
        let theme = &APPEARANCE_MODE;
        assert_eq!(database.resolve_setting(theme, &scopes).await?, "device");

        database
            .set_setting(
                &event(&mut source, "appearance.mode"),
                &change("appearance.mode", Scope::Organization, text("light")),
            )
            .await?;
        assert_eq!(database.resolve_setting(theme, &scopes).await?, "light");

        database
            .set_setting(
                &event(&mut source, "appearance.mode"),
                &change("appearance.mode", Scope::Person(person), text("dark")),
            )
            .await?;
        assert_eq!(database.resolve_setting(theme, &scopes).await?, "dark");
        let other = Scopes {
            person: Some(someone_else),
            ..scopes
        };
        assert_eq!(database.resolve_setting(theme, &other).await?, "light");

        // A new value replaces the old one on the same level.
        database
            .set_setting(
                &event(&mut source, "appearance.mode"),
                &change("appearance.mode", Scope::Person(person), text("device")),
            )
            .await?;
        assert_eq!(database.resolve_setting(theme, &scopes).await?, "device");

        // Taking the person's value away shows the organization's again.
        assert!(
            database
                .clear_setting(
                    &event(&mut source, "appearance.mode"),
                    "appearance.mode",
                    Scope::Person(person),
                )
                .await?
        );
        assert_eq!(database.resolve_setting(theme, &scopes).await?, "light");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_a_project_value_is_inherited_by_sub_projects_and_a_person_value_wins_for_the_person()
-> TestResult {
    each_database("cfg_06_inherited", async |db| {
        let fixture = fixture(db).await?;
        let (storage, database) = (&fixture.storage, fixture.storage.database());
        let mut source = ids();
        let ada = create_person(storage, &mut source, "ada@example.test").await?;
        let bea = create_person(storage, &mut source, "bea@example.test").await?;
        let acme = create_project(storage, &mut source, None, "ACME").await?;
        let web = create_project(storage, &mut source, Some(acme), "WEB").await?;
        let shop = create_project(storage, &mut source, Some(web), "SHOP").await?;
        let other = create_project(storage, &mut source, None, "OTHER").await?;
        let layers =
            async |project, person| database.setting_layers(&Scopes { project, person }).await;

        put(storage, &mut source, Scope::Organization, NOTE, text("org")).await?;
        put(
            storage,
            &mut source,
            Scope::Project(acme),
            NOTE,
            text("acme"),
        )
        .await?;

        // Sub-projects at any depth take the value of the project above; others do not.
        for project in [acme, web, shop] {
            let found = resolved_note(&layers(Some(project), None).await?);
            assert_eq!(found, (text("acme"), Source::Project(acme), false));
        }
        assert_eq!(
            resolved_note(&layers(Some(other), None).await?).1,
            Source::Organization
        );

        // The nearest project that sets it counts: WEB's own value for WEB and SHOP.
        put(storage, &mut source, Scope::Project(web), NOTE, text("web")).await?;
        assert_eq!(
            resolved_note(&layers(Some(shop), None).await?),
            (text("web"), Source::Project(web), false)
        );
        assert_eq!(
            resolved_note(&layers(Some(acme), None).await?).0,
            text("acme"),
            "the project above is not changed by one below"
        );

        // A person's own value wins for that person, in every project, and for nobody else.
        put(storage, &mut source, Scope::Person(ada), NOTE, text("mine")).await?;
        assert_eq!(
            resolved_note(&layers(Some(shop), Some(ada)).await?),
            (text("mine"), Source::Person, false)
        );
        assert_eq!(
            resolved_note(&layers(Some(shop), Some(bea)).await?).0,
            text("web")
        );
        assert_eq!(
            resolved_note(&layers(None, Some(ada)).await?).0,
            text("mine")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_a_locked_organization_value_wins_and_lower_values_are_refused() -> TestResult {
    each_database("cfg_06_locked", async |db| {
        let fixture = fixture(db).await?;
        let (storage, database) = (&fixture.storage, fixture.storage.database());
        let mut source = ids();
        let ada = create_person(storage, &mut source, "ada@example.test").await?;
        let acme = create_project(storage, &mut source, None, "ACME").await?;
        let scopes = Scopes {
            project: Some(acme),
            person: Some(ada),
        };
        // Values below the organization exist before the lock.
        put(storage, &mut source, Scope::Organization, NOTE, text("org")).await?;
        put(
            storage,
            &mut source,
            Scope::Project(acme),
            NOTE,
            text("acme"),
        )
        .await?;
        put(storage, &mut source, Scope::Person(ada), NOTE, text("mine")).await?;
        assert_eq!(
            resolved_note(&database.setting_layers(&scopes).await?).0,
            text("mine")
        );

        lock(storage, &mut source, NOTE, true).await?;
        // The organization's value wins over every lower layer, which stay stored but count for
        // nothing.
        assert_eq!(
            resolved_note(&database.setting_layers(&scopes).await?),
            (text("org"), Source::Organization, true)
        );
        // New values below are refused, inside the change: the change stops itself and is undone.
        for scope in [Scope::Project(acme), Scope::Person(ada)] {
            let refused = put(storage, &mut source, scope, NOTE, text("again")).await;
            assert!(
                matches!(refused, Err(StoreError::Aborted)),
                "{scope:?}: {refused:?}"
            );
        }
        assert_eq!(
            database
                .stored_setting(NOTE, Scope::Person(ada))
                .await?
                .map(|stored| stored.value),
            Some(text("mine")),
            "a refused change leaves what was there"
        );
        // Reset to default of a lower layer is allowed: it only takes away a value that counted
        // for nothing.
        assert!(clear(storage, &mut source, Scope::Person(ada), NOTE).await?);
        // The organization can still change its own locked value, and the lock stays.
        put(
            storage,
            &mut source,
            Scope::Organization,
            NOTE,
            text("org2"),
        )
        .await?;
        assert_eq!(
            resolved_note(&database.setting_layers(&scopes).await?),
            (text("org2"), Source::Organization, true)
        );
        // Unlocked, the layers below count again.
        lock(storage, &mut source, NOTE, false).await?;
        assert_eq!(
            resolved_note(&database.setting_layers(&scopes).await?),
            (text("acme"), Source::Project(acme), false)
        );
        // A setting that cannot be locked refuses the lock.
        let refused = lock(storage, &mut source, "test.project-limit", true).await;
        assert!(
            matches!(
                refused,
                Err(StoreError::SettingRejected(SettingProblem::NotLockable))
            ),
            "{refused:?}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_locking_a_setting_without_a_value_locks_its_default() -> TestResult {
    each_database("cfg_06_lock_default", async |db| {
        let fixture = fixture(db).await?;
        let (storage, database) = (&fixture.storage, fixture.storage.database());
        let mut source = ids();
        lock(storage, &mut source, NOTE, true).await?;
        let layers = database.setting_layers(&Scopes::default()).await?;
        assert_eq!(
            resolved_note(&layers),
            (text("none"), Source::Organization, true)
        );
        // Taking the organization's value away takes the lock with it.
        assert!(clear(storage, &mut source, Scope::Organization, NOTE).await?);
        let layers = database.setting_layers(&Scopes::default()).await?;
        assert_eq!(
            resolved_note(&layers),
            (text("none"), Source::Default, false)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_unlocking_drops_a_row_that_only_held_the_lock_and_keeps_a_real_value() -> TestResult
{
    each_database("cfg_06_unlock", async |db| {
        let fixture = fixture(db).await?;
        let (storage, database) = (&fixture.storage, fixture.storage.database());
        let mut source = ids();
        // Unlocking a setting without a row changes nothing, not even a row.
        lock(storage, &mut source, NOTE, false).await?;
        let layers = database.setting_layers(&Scopes::default()).await?;
        assert!(layers.organization.is_empty());
        // A lock that held the standard value leaves no row behind when it goes.
        lock(storage, &mut source, NOTE, true).await?;
        lock(storage, &mut source, NOTE, false).await?;
        let layers = database.setting_layers(&Scopes::default()).await?;
        assert!(layers.organization.is_empty());
        // A value of the organization stays when only the lock goes.
        put(
            storage,
            &mut source,
            Scope::Organization,
            NOTE,
            text("kept"),
        )
        .await?;
        lock(storage, &mut source, NOTE, true).await?;
        lock(storage, &mut source, NOTE, false).await?;
        let layers = database.setting_layers(&Scopes::default()).await?;
        assert_eq!(
            resolved_note(&layers),
            (text("kept"), Source::Organization, false)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_reset_to_default_removes_only_that_layer() -> TestResult {
    each_database("cfg_06_reset", async |db| {
        let fixture = fixture(db).await?;
        let (storage, database) = (&fixture.storage, fixture.storage.database());
        let mut source = ids();
        let ada = create_person(storage, &mut source, "ada@example.test").await?;
        let acme = create_project(storage, &mut source, None, "ACME").await?;
        let web = create_project(storage, &mut source, Some(acme), "WEB").await?;
        let scopes = Scopes {
            project: Some(web),
            person: Some(ada),
        };
        put(storage, &mut source, Scope::Organization, NOTE, text("org")).await?;
        put(
            storage,
            &mut source,
            Scope::Project(acme),
            NOTE,
            text("acme"),
        )
        .await?;
        put(storage, &mut source, Scope::Project(web), NOTE, text("web")).await?;
        put(storage, &mut source, Scope::Person(ada), NOTE, text("mine")).await?;

        let counts = async || -> Result<SettingValue, StoreError> {
            Ok(resolved_note(&database.setting_layers(&scopes).await?).0)
        };
        assert_eq!(counts().await?, text("mine"));
        // The person's reset shows the nearest project's value, and leaves every other layer.
        assert!(clear(storage, &mut source, Scope::Person(ada), NOTE).await?);
        assert_eq!(counts().await?, text("web"));
        assert!(clear(storage, &mut source, Scope::Project(web), NOTE).await?);
        assert_eq!(counts().await?, text("acme"));
        assert!(
            database
                .stored_setting(NOTE, Scope::Project(acme))
                .await?
                .is_some()
        );
        assert!(clear(storage, &mut source, Scope::Project(acme), NOTE).await?);
        assert_eq!(counts().await?, text("org"));
        assert!(clear(storage, &mut source, Scope::Organization, NOTE).await?);
        assert_eq!(counts().await?, text("none"));
        // Resetting what is not there is no change.
        assert!(!clear(storage, &mut source, Scope::Person(ada), NOTE).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_04_a_persons_values_are_their_own_and_belong_to_a_person_who_exists() -> TestResult {
    each_database("cfg_04_person_values", async |db| {
        let fixture = fixture(db).await?;
        let (storage, database) = (&fixture.storage, fixture.storage.database());
        let mut source = ids();
        let ada = create_person(storage, &mut source, "ada@example.test").await?;
        let bea = create_person(storage, &mut source, "bea@example.test").await?;
        let mode = "appearance.mode";
        put(storage, &mut source, Scope::Person(ada), mode, text("dark")).await?;
        put(
            storage,
            &mut source,
            Scope::Person(ada),
            "region.time-zone",
            text("Europe/Berlin"),
        )
        .await?;
        put(
            storage,
            &mut source,
            Scope::Person(bea),
            mode,
            text("light"),
        )
        .await?;
        let layers = database
            .setting_layers(&Scopes {
                project: None,
                person: Some(ada),
            })
            .await?;
        assert_eq!(layers.person.len(), 2);
        assert_eq!(layers.person["region.time-zone"], text("Europe/Berlin"));
        assert_eq!(layers.person[mode], text("dark"));
        // The value is stored for a person who exists: a person that does not exist has no row.
        let nobody = UserId::generate(&mut source);
        let refused = put(
            storage,
            &mut source,
            Scope::Person(nobody),
            mode,
            text("dark"),
        )
        .await;
        assert!(refused.is_err(), "{refused:?}");
        // A value that is a person's own is not an organization value of the same key.
        assert!(
            database
                .stored_setting(mode, Scope::Organization)
                .await?
                .is_none()
        );
        Ok(())
    })
    .await
}

fn rejected(problem: SettingProblem, result: Result<(), StoreError>) {
    let error = result.err();
    assert!(
        matches!(error, Some(StoreError::SettingRejected(found)) if found == problem),
        "{error:?} should be {problem:?}"
    );
}

#[tokio::test]
async fn bad_example_the_registry_decides_what_may_be_stored() -> TestResult {
    each_database("adr_0018_rejected", async |db| {
        let fixture = fixture(db).await?;
        let database = fixture.storage.database();
        let mut source = ids();

        let unknown = change("appearance.colour", Scope::Organization, text("x"));
        rejected(
            SettingProblem::UnknownKey,
            database
                .set_setting(&event(&mut source, "a.b"), &unknown)
                .await,
        );
        // A session that lasts a second, or a year, is a mistake: the bounds of the setting hold.
        for value in [1, 0, -5, 100_000] {
            let out_of_bounds = change(
                "security.session-idle-minutes",
                Scope::Organization,
                SettingValue::Integer(value),
            );
            rejected(
                SettingProblem::OutOfBounds,
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
            let too_much = change("auth.sign-up-domains", Scope::Organization, text(&list));
            rejected(
                SettingProblem::OutOfBounds,
                database
                    .set_setting(&event(&mut source, "a.b"), &too_much)
                    .await,
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn bad_example_a_text_must_be_in_the_format_of_its_setting_and_on_a_level_it_has()
-> TestResult {
    each_database("adr_0018_rejected_formats", async |db| {
        let fixture = fixture(db).await?;
        let (storage, database) = (&fixture.storage, fixture.storage.database());
        let mut source = ids();
        let person = create_person(storage, &mut source, "ada@example.test").await?;
        let acme = create_project(storage, &mut source, None, "ACME").await?;
        let history = TechnicalName::new("history").unwrap();
        let events_before = database.outbox_after(&history, 100).await?.len();
        // A choice that is not on the list, a time zone that is not one, a page that is another
        // website and keys that are not the keys of shortcuts.
        for (key, value) in [
            ("appearance.mode", "sometimes"),
            ("region.time-zone", "Mars/Olympus Mons"),
            ("start.page", "https://evil.test/"),
            ("shortcuts.keys", "[1, 2]"),
            ("window.sizes", "[800, 600]"),
        ] {
            let scope = Scope::Person(person);
            rejected(
                SettingProblem::OutOfBounds,
                database
                    .set_setting(&event(&mut source, "a.b"), &change(key, scope, text(value)))
                    .await,
            );
        }
        let wrong_level = change(
            "security.session-idle-minutes",
            Scope::Person(person),
            SettingValue::Integer(10),
        );
        rejected(
            SettingProblem::LevelNotAllowed,
            database
                .set_setting(&event(&mut source, "a.b"), &wrong_level)
                .await,
        );
        let project_level = change("appearance.mode", Scope::Project(acme), text("dark"));
        rejected(
            SettingProblem::LevelNotAllowed,
            database
                .set_setting(&event(&mut source, "a.b"), &project_level)
                .await,
        );
        let wrong_type = change(
            "security.session-idle-minutes",
            Scope::Organization,
            text("ten"),
        );
        rejected(
            SettingProblem::WrongType,
            database
                .set_setting(&event(&mut source, "a.b"), &wrong_type)
                .await,
        );
        let mut locked = change(
            "security.two-step-required",
            Scope::Organization,
            SettingValue::Bool(true),
        );
        locked.locked = true;
        rejected(
            SettingProblem::NotLockable,
            database
                .set_setting(&event(&mut source, "a.b"), &locked)
                .await,
        );
        assert!(matches!(
            database
                .stored_setting("appearance.colour", Scope::Organization)
                .await,
            Err(StoreError::SettingRejected(SettingProblem::UnknownKey))
        ));

        // Nothing was stored and no event was written for a refused change.
        assert!(
            database
                .stored_setting("security.session-idle-minutes", Scope::Organization)
                .await?
                .is_none()
        );
        assert_eq!(
            database.outbox_after(&history, 100).await?.len(),
            events_before
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn a_setting_change_and_its_event_are_written_together() -> TestResult {
    each_database("adr_0018_event", async |db| {
        let fixture = fixture(db).await?;
        let database = fixture.storage.database();
        let mut source = ids();
        let event = event(&mut source, "security.session-idle-minutes");
        database
            .set_setting(
                &event,
                &change(
                    "security.session-idle-minutes",
                    Scope::Organization,
                    SettingValue::Integer(15),
                ),
            )
            .await?;
        let stored = database
            .stored_setting("security.session-idle-minutes", Scope::Organization)
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
        let fixture = fixture(db).await?;
        let database = fixture.storage.database();
        // Written around the registry, as a damaged or hand-edited database might be: a text for
        // a number, and a setting that no longer exists.
        for (key, value) in [
            ("security.session-idle-minutes", "\"soon\""),
            ("security.gone-setting", "1"),
        ] {
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
                    Expr::val(key),
                    Expr::val("organization"),
                    Expr::val(Vec::<u8>::new()),
                    Expr::val(value),
                    Expr::val(false),
                    Expr::val(0_i64),
                ])?;
            database.execute(&insert).await?;
        }
        assert_eq!(
            database
                .resolve_setting(&SECURITY_SESSION_IDLE_MINUTES, &Scopes::default())
                .await?,
            30
        );
        assert!(
            database
                .setting_layers(&Scopes::default())
                .await?
                .organization
                .is_empty()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cfg_06_a_bounded_number_and_a_switch_are_stored_on_a_project() -> TestResult {
    each_database("cfg_06_project_types", async |db| {
        let fixture = fixture(db).await?;
        let (storage, database) = (&fixture.storage, fixture.storage.database());
        let mut source = ids();
        let acme = create_project(storage, &mut source, None, "ACME").await?;
        let limit = "test.project-limit";
        put(
            storage,
            &mut source,
            Scope::Project(acme),
            limit,
            SettingValue::Integer(7),
        )
        .await?;
        put(
            storage,
            &mut source,
            Scope::Project(acme),
            "test.switch",
            SettingValue::Bool(true),
        )
        .await?;
        let layers = database
            .setting_layers(&Scopes {
                project: Some(acme),
                person: None,
            })
            .await?;
        assert_eq!(
            layers.resolve(TEST_PROJECT_LIMIT.definition()).value,
            SettingValue::Integer(7)
        );
        assert_eq!(
            layers.resolve(TEST_SWITCH.definition()).value,
            SettingValue::Bool(true)
        );
        let too_big = put(
            storage,
            &mut source,
            Scope::Project(acme),
            limit,
            SettingValue::Integer(10),
        )
        .await;
        assert!(
            matches!(
                too_big,
                Err(StoreError::SettingRejected(SettingProblem::OutOfBounds))
            ),
            "{too_big:?}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_the_newest_version_event_is_found_however_many_events_came_after() -> TestResult {
    each_database("iss_17_version_event_search", async |db| {
        let fixture = fixture(db).await?;
        let database = fixture.storage.database();
        let mut source = ids();
        let first = event(&mut source, "security.session-idle-minutes");
        let mut versioned = first.clone();
        versioned.changes.push(bb_core::version::counted_up(1));
        let write = change(
            "security.session-idle-minutes",
            Scope::Organization,
            SettingValue::Integer(15),
        );
        database.set_setting(&versioned, &write).await?;
        // More events than one page of the search, none of which counted the version up.
        for _ in 0..120 {
            let mut later = event(&mut source, "security.session-idle-minutes");
            later.entity = versioned.entity.clone();
            database.set_setting(&later, &write).await?;
        }
        let found = database
            .latest_event_about_where(&versioned.entity, DomainEvent::counted_version)
            .await?;
        assert_eq!(found, Some(versioned.clone()));
        let none = database
            .latest_event_about_where(&versioned.entity, |_| false)
            .await?;
        assert_eq!(none, None);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_the_search_for_a_field_change_reads_only_the_events_after_the_version() -> TestResult
{
    each_database("iss_17_field_change_search", async |db| {
        let fixture = fixture(db).await?;
        let database = fixture.storage.database();
        let mut source = ids();
        let write = change(
            "security.session-idle-minutes",
            Scope::Organization,
            SettingValue::Integer(15),
        );
        let mut first = event(&mut source, "security.session-idle-minutes");
        first.changes = vec![field_change("old"), bb_core::version::counted_up(0)];
        database.set_setting(&first, &write).await?;
        // Far more events than one page, each counting the version up by one.
        for version in 1..120 {
            let mut later = event(&mut source, "security.session-idle-minutes");
            later.entity = first.entity.clone();
            later.changes = vec![field_change("new"), bb_core::version::counted_up(version)];
            database.set_setting(&later, &write).await?;
        }
        let entity = &first.entity;
        // Version 119 was sent: the newest event (version 120) is after it, version 119 ends it.
        assert_eq!(
            database.search_field_change(entity, "new", 119).await?,
            (true, 1)
        );
        assert_eq!(
            database.search_field_change(entity, "old", 119).await?,
            (false, 2)
        );
        // Asked from the start, the old change is found, though only after all the others.
        assert_eq!(
            database.search_field_change(entity, "old", 0).await?,
            (true, 120)
        );
        Ok(())
    })
    .await
}

fn field_change(field: &str) -> FieldChange {
    FieldChange {
        field: field.to_owned(),
        before: None,
        after: None,
    }
}
