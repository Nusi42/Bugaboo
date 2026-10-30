//! Values are stored per level and the lowest level wins; locks (ADR-0018, CFG-06).

use super::*;

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
        // A lock that held the default value leaves no row behind when it goes.
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
