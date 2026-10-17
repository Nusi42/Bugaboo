//! A change and its event are written together, and what is read back (CFG-06).

use super::*;

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
