//! Resetting a layer and the values of a person (CFG-06, CFG-04).

use super::*;

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

#[tokio::test]
async fn ntf_01_the_own_values_of_many_people_are_read_in_one_go() -> TestResult {
    each_database("ntf_01_person_settings", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let ada = create_person(storage, &mut source, "ada@example.test").await?;
        let ben = create_person(storage, &mut source, "ben@example.test").await?;
        let eve = create_person(storage, &mut source, "eve@example.test").await?;
        put(storage, &mut source, Scope::Person(ada), NOTE, text("mine")).await?;
        put(storage, &mut source, Scope::Person(ben), NOTE, text("his")).await?;
        let mut read = storage.person_settings(vec![ada, ben, eve]).await?;
        read.sort_by_key(|(person, _)| *person);
        let mut expected = vec![(ada, text("mine")), (ben, text("his"))];
        expected.sort_by_key(|(person, _)| *person);
        assert_eq!(
            read.len(),
            2,
            "a person with no value of their own has no row"
        );
        for ((person, values), (wanted, value)) in read.iter().zip(&expected) {
            assert_eq!(person, wanted);
            assert_eq!(values.get(NOTE), Some(value));
        }
        assert!(storage.person_settings(Vec::new()).await?.is_empty());
        Ok(())
    })
    .await
}
