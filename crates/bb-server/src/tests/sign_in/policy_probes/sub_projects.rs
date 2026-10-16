//! Roles and rows for sub-projects: nobody gives a role with sub-projects they do not hold there (SEC-02).

use super::*;

#[tokio::test]
async fn sec_02_nobody_gives_a_role_with_sub_projects_they_do_not_hold_there() -> TestResult {
    each_database("sec_02_sub_projects", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let web = org.project(Some(acme), "WEB").await?.id;
        let bob = org.person().await?;
        let lead = org.role("Section lead", &LEAD).await?;
        // Bob leads ACME, for ACME only.
        org.assign(
            &org.admin,
            Subject::Person(bob.id),
            Given::Role(lead.id),
            Some(acme),
            AppliesTo::Project,
        )
        .await?;
        assert!(org.can(&bob, Permission::DecideRoles, Some(acme)).await?);
        assert!(!org.can(&bob, Permission::DecideRoles, Some(web)).await?);

        // He cannot give himself the role for ACME and its sub-projects: he holds it in ACME and
        // not in WEB.
        let escalate = org
            .assign(
                &bob,
                Subject::Person(bob.id),
                Given::Role(lead.id),
                Some(acme),
                AppliesTo::ProjectAndSubProjects,
            )
            .await;
        assert_eq!(refused(escalate), Some(ErrorCode::TooMuch));
        assert!(!org.can(&bob, Permission::DecideRoles, Some(web)).await?);

        // Giving the role for ACME alone is fine, and so is giving it with sub-projects to
        // somebody once he holds it there too.
        let cleo = org.person().await?;
        org.assign(
            &bob,
            Subject::Person(cleo.id),
            Given::Role(lead.id),
            Some(acme),
            AppliesTo::Project,
        )
        .await?;
        org.give(&bob, lead.id, acme).await?;
        org.assign(
            &bob,
            Subject::Person(cleo.id),
            Given::Role(lead.id),
            Some(acme),
            AppliesTo::ProjectAndSubProjects,
        )
        .await
        .map_err(|error| format!("{error:?}"))?;
        Ok(())
    })
    .await
}

/// What a lead of one project holds: sees it, decides who has which role there, makes
/// sub-projects.
const SECTION_LEAD: [Permission; 3] = [
    Permission::SeeProjects,
    Permission::DecideRoles,
    Permission::CreateSubProjects,
];

#[tokio::test]
async fn sec_02_a_role_with_sub_projects_needs_a_row_that_reaches_sub_projects() -> TestResult {
    each_database("sec_02_future_sub_projects", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let (bob, cleo) = (org.person().await?, org.person().await?);
        let lead = org.role("Section lead", &SECTION_LEAD).await?;
        // Bob leads ACME, for ACME alone. ACME has no sub-projects yet.
        org.assign(
            &org.admin,
            Subject::Person(bob.id),
            Given::Role(lead.id),
            Some(acme),
            AppliesTo::Project,
        )
        .await?;
        let with_sub_projects = |to: &Member| {
            org.assign(
                &bob,
                Subject::Person(to.id),
                Given::Role(lead.id),
                Some(acme),
                AppliesTo::ProjectAndSubProjects,
            )
        };
        // He may not give the role with sub-projects, to himself or to somebody else: there are
        // none to check it against today, but there will be, and it would reach them.
        assert_eq!(
            refused(with_sub_projects(&bob).await),
            Some(ErrorCode::TooMuch)
        );
        assert_eq!(
            refused(with_sub_projects(&cleo).await),
            Some(ErrorCode::TooMuch)
        );
        // A sub-project made afterwards (by him, as he may) is not his to decide in.
        let bobs = org
            .rig
            .api()
            .create_project(
                Rig::info(),
                CreateProject {
                    session_token: bob.session.clone(),
                    parent: Some(acme),
                    short_name: "BOBS".to_owned(),
                    name: "Bob's own".to_owned(),
                },
            )
            .await?
            .into_value();
        assert!(
            !org.can(&bob, Permission::DecideRoles, Some(bobs.id))
                .await?
        );
        assert!(
            !org.can(&cleo, Permission::DecideRoles, Some(bobs.id))
                .await?
        );

        // With a row of his own that reaches sub-projects he holds it there, and may.
        org.give(&bob, lead.id, acme).await?;
        with_sub_projects(&cleo).await?;
        let web = org.project(Some(acme), "WEB").await?.id;
        assert!(org.can(&cleo, Permission::DecideRoles, Some(web)).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_a_group_with_a_role_for_sub_projects_is_not_joined_by_a_lead_of_one_project()
-> TestResult {
    each_database("sec_02_group_future_sub_projects", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let (kim, bea) = (org.person().await?, org.person().await?);
        let keeper = org
            .role(
                "Group keeper",
                &[Permission::ManageGroups, Permission::SeeProjects],
            )
            .await?;
        org.give_everywhere(&kim, keeper.id).await?;
        let lead = org.role("Section lead", &SECTION_LEAD).await?;
        // Kim leads ACME alone.
        org.assign(
            &org.admin,
            Subject::Person(kim.id),
            Given::Role(lead.id),
            Some(acme),
            AppliesTo::Project,
        )
        .await?;
        // The group has the role for ACME and what will be in it.
        let leads = org.group("Leads").await?;
        org.assign(
            &org.admin,
            Subject::Group(leads),
            Given::Role(lead.id),
            Some(acme),
            AppliesTo::ProjectAndSubProjects,
        )
        .await?;
        let add = || {
            org.rig.api().add_group_member(
                Rig::info(),
                AddGroupMember {
                    session_token: kim.session.clone(),
                    group: leads,
                    person: bea.id,
                },
            )
        };
        assert_eq!(refused(add().await), Some(ErrorCode::TooMuch));
        // Once she leads ACME and what is below it, she may.
        org.give(&kim, lead.id, acme).await?;
        add().await?;
        Ok(())
    })
    .await
}
