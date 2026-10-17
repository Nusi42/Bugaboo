//! The registry decides what may be stored (bad examples).

use super::*;

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
