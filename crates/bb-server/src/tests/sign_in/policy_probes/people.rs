//! The lists of people and the check of access in a project (PRIV-12, PERF-02, SEC-03).

use super::*;

#[tokio::test]
async fn priv_12_the_cursor_of_a_page_of_people_holds_the_id_of_the_last_one_shown_and_no_name()
-> TestResult {
    each_database("priv_12_people_cursor_id_only", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let (bob, cleo) = (org.person().await?, org.person().await?);
        let api = org.rig.api();
        let page = |cursor: Option<String>| {
            api.people(
                Rig::info(),
                PeopleQuery {
                    session_token: org.admin.session.clone(),
                    search: None,
                    cursor,
                    limit: 1,
                    groups: false,
                    roles: false,
                },
            )
        };
        let first = page(None).await?.into_value();
        let cursor = first.next_cursor.ok_or("a next page")?;
        // The ID of a person and nothing of their name; the page after it goes on from there.
        let last: UserId = cursor.parse().map_err(|_| "a cursor that is an ID")?;
        assert!([org.admin.id, bob.id, cleo.id].contains(&last));
        let second = page(Some(cursor)).await?.into_value();
        assert_eq!(second.items.len(), 1);
        assert_ne!(second.items[0].person, first.items[0].person);
        // An ID that is nobody's is no cursor.
        let nobody = refused(page(Some("00000000-0000-7000-8000-000000000000".to_owned())).await);
        assert_eq!(nobody, Some(ErrorCode::InvalidRequest));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_02_a_list_of_people_reads_groups_and_roles_only_when_they_are_asked_for() -> TestResult
{
    each_database("perf_02_people_wanted", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let bob = org.person().await?;
        let role = org.role("Reader", &[Permission::SeeProjects]).await?;
        org.give(&bob, role.id, acme).await?;
        let group = org.group("Backend").await?;
        org.add_to_group(group, bob.id).await?;
        let api = org.rig.api();
        let ask = |groups, roles| {
            api.people(
                Rig::info(),
                PeopleQuery {
                    session_token: org.admin.session.clone(),
                    search: Some("person".to_owned()),
                    cursor: None,
                    limit: 10,
                    groups,
                    roles,
                },
            )
        };
        // The administrator may see both, so nothing is missing for any other reason than that it
        // was not asked for.
        let plain = ask(false, false).await?.into_value();
        assert!(
            plain
                .items
                .iter()
                .all(|line| line.groups.is_none() && line.roles.is_empty())
        );
        let wanted = ask(true, true).await?.into_value();
        let line = wanted.items.first().ok_or("a person")?;
        assert_eq!(line.groups.as_ref().map(Vec::len), Some(1));
        assert_eq!(line.roles.len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_check_access_in_a_project_lists_what_counts_there_and_the_rows_that_reach_it()
-> TestResult {
    each_database("sec_03_check_in_project", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let web = org.project(Some(acme), "WEB").await?.id;
        let other = org.project(None, "OTHER").await?.id;
        let bob = org.person().await?;
        let reader = org.role("Reader", &[Permission::SeeProjects]).await?;
        let here_only = org
            .assign(
                &org.admin,
                Subject::Person(bob.id),
                Given::Role(reader.id),
                Some(acme),
                AppliesTo::Project,
            )
            .await?;
        let elsewhere = org.give(&bob, reader.id, other).await?;
        let reaching = org.give_everywhere(&bob, reader.id).await?;

        // In a project, only what counts in a project is listed.
        let in_web = org.check(&org.admin, bob.id, Some(web)).await?;
        assert!(
            in_web
                .permissions
                .iter()
                .all(|answer| answer.permission.counts() == Permission::SeeProjects.counts()),
            "only the permissions that count in a project"
        );
        // For the whole organization, all of them.
        let everywhere = org.check(&org.admin, bob.id, None).await?;
        assert_eq!(everywhere.permissions.len(), Permission::ALL.len());
        assert!(in_web.permissions.len() < everywhere.permissions.len());

        // Only the rows that reach WEB are named: the one for the organization, not the one for
        // ACME alone and not the one for another project.
        let rows: Vec<AssignmentId> = in_web
            .permissions
            .iter()
            .flat_map(|answer| answer.reasons.iter())
            .filter_map(|reason| match reason {
                ReasonView::GivenByRole { row, .. } => Some(row.assignment),
                _ => None,
            })
            .collect();
        assert!(rows.contains(&reaching.id));
        assert!(!rows.contains(&here_only.id));
        assert!(!rows.contains(&elsewhere.id));
        Ok(())
    })
    .await
}
