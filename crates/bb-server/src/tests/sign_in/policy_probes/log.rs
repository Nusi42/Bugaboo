//! The security log and the events of roles, levels and removed rows (SEC-08, ADR-0007).

use super::*;

#[tokio::test]
async fn sec_08_the_rows_made_for_a_replacement_are_each_in_the_security_log() -> TestResult {
    each_database("sec_08_replacement_in_the_log", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let (bob, cleo) = (org.person().await?, org.person().await?);
        let used = org.role("Used", &[Permission::SeeProjects]).await?;
        let other = org.role("Other", &[Permission::SeeProjects]).await?;
        let old_bob = org.give(&bob, used.id, acme).await?;
        let old_cleo = org.give(&cleo, used.id, acme).await?;
        let before = rig.database.security_log_entries(0, 10_000).await?.len();
        org.rig
            .api()
            .delete_role(
                Rig::info(),
                DeleteRole {
                    session_token: org.admin.session.clone(),
                    role: used.id,
                    replacement: Some(other.id),
                },
            )
            .await?;
        // Every permission change is in the security log (ADR-0025, decision 9): a row for each
        // person who had the role, then the deletion of the role itself.
        let entries = rig.database.security_log_entries(0, 10_000).await?;
        let new: Vec<_> = entries.iter().skip(before).collect();
        let actions: Vec<&str> = new.iter().map(|entry| entry.action.as_str()).collect();
        assert_eq!(
            actions,
            [
                SecurityAction::AssignmentCreated.code(),
                SecurityAction::AssignmentCreated.code(),
                SecurityAction::RoleDeleted.code(),
            ]
        );
        let made: Vec<_> = new[..2].iter().map(|entry| entry.subject.clone()).collect();
        assert!(made.iter().all(Option::is_some) && made[0] != made[1]);
        // They are new rows, not the old ones under another role.
        for old in [old_bob.id, old_cleo.id] {
            assert!(
                !made
                    .iter()
                    .flatten()
                    .any(|subject| format!("{subject:?}").contains(&old.to_string()))
            );
        }
        for entry in &new {
            assert_eq!(entry.person, Some(org.admin.id));
        }
        Ok(())
    })
    .await
}

/// The last event of the outbox.
async fn last_event(rig: &Rig) -> Run<DomainEvent> {
    let consumer = bb_core::technical::TechnicalName::new("probe-reader")?;
    rig.database
        .outbox_after(&consumer, 100_000)
        .await?
        .pop()
        .map(|(_, event)| event)
        .ok_or_else(|| "no event".into())
}

#[tokio::test]
async fn adr_0007_events_of_levels_and_removed_rows_say_what_was_there_before() -> TestResult {
    each_database("adr_0007_before_values", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let bob = org.person().await?;
        let role = org.role("Reader", &[Permission::SeeProjects]).await?;
        let api = org.rig.api();
        let set = |level| {
            api.set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: org.admin.session.clone(),
                    role: role.id,
                    project: Some(acme),
                    issue_type: None,
                    field: Field::Title.into(),
                    level,
                },
            )
        };
        set(Some(FieldLevel::CanSee)).await?;
        let first = last_event(&rig).await?;
        assert_eq!(first.changes[0].before, None);
        assert_eq!(first.changes[0].after, Some("can-see".into()));
        set(Some(FieldLevel::CanChange)).await?;
        let second = last_event(&rig).await?;
        assert_eq!(second.changes[0].before, Some("can-see".into()));
        assert_eq!(second.changes[0].after, Some("can-change".into()));
        set(None).await?;
        let third = last_event(&rig).await?;
        assert_eq!(third.changes[0].before, Some("can-change".into()));
        assert_eq!(third.changes[0].after, None);

        let set_people = |level| {
            api.set_people_level(
                Rig::info(),
                SetPeopleLevel {
                    session_token: org.admin.session.clone(),
                    role: role.id,
                    project: None,
                    kind: PeopleKind::Mentions,
                    level,
                },
            )
        };
        set_people(Some(PeopleLevel::RoleOnly)).await?;
        set_people(Some(PeopleLevel::Names)).await?;
        let people = last_event(&rig).await?;
        assert_eq!(people.changes[0].before, Some("role-only".into()));
        assert_eq!(people.changes[0].after, Some("names".into()));

        // A removed row says whom it was for, which role (or *No access*), where, and how far.
        let row = org.give(&bob, role.id, acme).await?;
        remove(&org, &org.admin, row.id).await?;
        let removed = last_event(&rig).await?;
        let before = removed.changes[0]
            .before
            .clone()
            .ok_or("the row that was there")?;
        assert_eq!(before["person"], bob.id.to_string());
        assert_eq!(before["role"], role.id.to_string());
        assert_eq!(before["project"], acme.to_string());
        assert_eq!(before["applies_to"], "project-and-sub-projects");
        assert_eq!(removed.changes[0].after, None);
        let blocked = org.forbid(&bob, acme).await?;
        remove(&org, &org.admin, blocked.id).await?;
        let removed = last_event(&rig).await?;
        let before = removed.changes[0].before.clone().ok_or("the row")?;
        assert_eq!(before["role"], serde_json::Value::Null);
        Ok(())
    })
    .await
}
