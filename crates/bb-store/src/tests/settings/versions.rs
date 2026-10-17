//! Finding the newest version event and the field changes after it (ISS-17).

use super::*;

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
