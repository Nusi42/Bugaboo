//! No access, and the rows that can exist (SEC-02, SEC-03).

use super::*;

#[tokio::test]
async fn sec_03_decide_roles_is_not_kept_from_a_row_at_the_same_depth_as_the_no_access()
-> TestResult {
    each_database("sec_03_same_depth", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let bob = org.person().await?;
        let lead = org.role("Section lead", &LEAD).await?;
        let leads = org.group("Leads").await?;
        org.add_to_group(leads, bob.id).await?;
        org.assign(
            &org.admin,
            Subject::Group(leads),
            Given::Role(lead.id),
            Some(acme),
            AppliesTo::ProjectAndSubProjects,
        )
        .await?;
        assert!(org.can(&bob, Permission::DecideRoles, Some(acme)).await?);

        // An administrator puts *No access* on Bob in ACME. The role of his group is at the same
        // depth, so it is removed, and with it the right to decide there.
        let no_access = org.forbid(&bob, acme).await?;
        assert!(!org.can(&bob, Permission::DecideRoles, Some(acme)).await?);
        assert!(not_allowed(refused(remove(&org, &bob, no_access.id).await)));

        // Somebody who decides from above (for the whole organization) can still lift it.
        let dana = org.person().await?;
        let administrator = org
            .rig
            .api()
            .roles(
                Rig::info(),
                RolesQuery {
                    session_token: org.admin.session.clone(),
                    place: None,
                    cursor: None,
                    limit: bb_core::paging::PAGE_SIZE_MAX,
                    giveable: false,
                },
            )
            .await?
            .into_value()
            .roles
            .into_iter()
            .find(|role| role.name == "Administrator")
            .ok_or("the Administrator role")?;
        org.give_everywhere(&dana, administrator.id).await?;
        let blocked = org.forbid(&dana, acme).await?;
        assert!(org.can(&dana, Permission::DecideRoles, Some(acme)).await?);
        remove(&org, &dana, blocked.id).await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_manage_groups_does_not_lift_a_no_access_of_the_group() -> TestResult {
    each_database("sec_02_group_no_access", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let (kim, cleo) = (org.person().await?, org.person().await?);
        let keeper = org
            .role(
                "Group keeper",
                &[Permission::ManageGroups, Permission::SeeProjects],
            )
            .await?;
        org.give_everywhere(&kim, keeper.id).await?;
        let blocked = org.group("Blocked").await?;
        org.add_to_group(blocked, cleo.id).await?;
        org.assign(
            &org.admin,
            Subject::Group(blocked),
            Given::NoAccess,
            Some(acme),
            AppliesTo::ProjectAndSubProjects,
        )
        .await?;
        let api = org.rig.api();
        let take_out = |by: &Member| {
            api.remove_group_member(
                Rig::info(),
                RemoveGroupMember {
                    session_token: by.session.clone(),
                    group: blocked,
                    person: cleo.id,
                },
            )
        };
        let delete = |by: &Member| {
            api.delete_group(
                Rig::info(),
                DeleteGroup {
                    session_token: by.session.clone(),
                    group: blocked,
                },
            )
        };
        // Kim manages groups but does not decide who has which role in ACME.
        assert!(not_allowed(refused(take_out(&kim).await)));
        assert!(not_allowed(refused(delete(&kim).await)));

        // With the right to decide there, she may.
        let lead = org.role("Section lead", &LEAD).await?;
        org.give(&kim, lead.id, acme).await?;
        take_out(&kim).await?;
        org.add_to_group(blocked, cleo.id).await?;
        delete(&kim).await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_a_group_with_a_no_access_the_person_cannot_see_is_refused_in_its_own_words()
-> TestResult {
    each_database("sec_02_group_no_access_hidden", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let (kim, cleo) = (org.person().await?, org.person().await?);
        // Kim manages groups and sees no project at all, so ACME is not hers to know about.
        let keeper = org
            .role("Group keeper", &[Permission::ManageGroups])
            .await?;
        org.give_everywhere(&kim, keeper.id).await?;
        let blocked = org.group("Blocked").await?;
        org.add_to_group(blocked, cleo.id).await?;
        org.assign(
            &org.admin,
            Subject::Group(blocked),
            Given::NoAccess,
            Some(acme),
            AppliesTo::ProjectAndSubProjects,
        )
        .await?;
        let api = org.rig.api();
        // The group is hers to see and change; what holds her back is said in its own words, and
        // not as a group that does not exist.
        let take_out = refused(
            api.remove_group_member(
                Rig::info(),
                RemoveGroupMember {
                    session_token: kim.session.clone(),
                    group: blocked,
                    person: cleo.id,
                },
            )
            .await,
        );
        assert_eq!(take_out, Some(ErrorCode::CannotLift));
        let delete = refused(
            api.delete_group(
                Rig::info(),
                DeleteGroup {
                    session_token: kim.session.clone(),
                    group: blocked,
                },
            )
            .await,
        );
        assert_eq!(delete, Some(ErrorCode::CannotLift));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_only_rows_that_can_exist_are_made() -> TestResult {
    each_database("sec_02_valid_rows", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let bob = org.person().await?;
        let reader = org.role("Reader", &[Permission::SeeProjects]).await?;
        let create = |given, project, applies_to| {
            org.assign(
                &org.admin,
                Subject::Person(bob.id),
                given,
                project,
                applies_to,
            )
        };
        let role = Given::Role(reader.id);
        let invalid = Some(ErrorCode::InvalidRequest);
        // A role for the whole organization counts everywhere and says so; *No access* is only
        // set on a project and covers it and what is below it; a role on a project is for the
        // project alone or with what is below it.
        assert_eq!(
            refused(create(role, None, AppliesTo::Project).await),
            invalid
        );
        assert_eq!(
            refused(create(role, None, AppliesTo::ProjectAndSubProjects).await),
            invalid
        );
        for applies_to in [
            AppliesTo::Organization,
            AppliesTo::Project,
            AppliesTo::ProjectAndSubProjects,
        ] {
            assert_eq!(
                refused(create(Given::NoAccess, None, applies_to).await),
                invalid,
                "No access for the whole organization: {applies_to:?}"
            );
        }
        assert_eq!(
            refused(create(Given::NoAccess, Some(acme), AppliesTo::Project).await),
            invalid
        );
        assert_eq!(
            refused(create(Given::NoAccess, Some(acme), AppliesTo::Organization).await),
            invalid
        );
        assert_eq!(
            refused(create(role, Some(acme), AppliesTo::Organization).await),
            invalid
        );
        // The three that can exist (a role and *No access* for one person on one project are a
        // conflict of their own, so *No access* is for somebody else).
        create(role, None, AppliesTo::Organization).await?;
        create(role, Some(acme), AppliesTo::Project).await?;
        let cleo = org.person().await?;
        org.assign(
            &org.admin,
            Subject::Person(cleo.id),
            Given::NoAccess,
            Some(acme),
            AppliesTo::ProjectAndSubProjects,
        )
        .await?;
        Ok(())
    })
    .await
}
