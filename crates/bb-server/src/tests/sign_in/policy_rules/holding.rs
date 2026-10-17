//! Nobody gives or puts in more than they hold (SEC-02, SEC-08).

use super::*;

#[tokio::test]
async fn sec_08_nobody_adds_a_permission_they_do_not_hold_to_a_role() -> TestResult {
    each_database("sec_08_role_escalation", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let mara = org.person().await?;
        let keeper = org
            .role(
                "Role keeper",
                &[Permission::ManageRoles, Permission::SeeProjects],
            )
            .await?;
        org.give_everywhere(&mara, keeper.id).await?;
        // She can make a role with what she holds, and no more.
        let small = org
            .create_role(&mara, "Small", &[Permission::SeeProjects])
            .await?;
        assert_eq!(
            refused(
                org.create_role(&mara, "Big", &[Permission::ManagePeople])
                    .await
            ),
            Some(ErrorCode::TooMuch)
        );
        // She can add what she holds, and cannot add what she does not.
        change(
            &org,
            &mara,
            small.id,
            Some(vec![Permission::SeeProjects, Permission::ManageRoles]),
        )
        .await?;
        assert_eq!(
            refused(
                change(
                    &org,
                    &mara,
                    small.id,
                    Some(vec![Permission::SeeProjects, Permission::ManageGroups])
                )
                .await
            ),
            Some(ErrorCode::TooMuch)
        );
        // Taking permissions away needs nothing, and a rename needs nothing.
        change(&org, &mara, small.id, Some(vec![])).await?;
        // A person who may not manage roles at all cannot change any.
        let bea = org.person().await?;
        assert_eq!(
            refused(change(&org, &bea, small.id, Some(vec![])).await),
            Some(ErrorCode::Forbidden)
        );
        // "Manage roles" counts only for the whole organization: given on a project it gives
        // nothing there, so a lead of a project cannot make roles from it.
        let projects = org.project(None, "ACME").await?;
        let cy = org.person().await?;
        org.give(&cy, keeper.id, projects.id).await?;
        assert_eq!(
            refused(
                org.create_role(&cy, "Nope", &[Permission::SeeProjects])
                    .await
            ),
            Some(ErrorCode::Forbidden)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_nobody_gives_a_role_with_more_than_they_hold_there() -> TestResult {
    each_database("sec_02_give_escalation", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let lea = org.person().await?;
        let bea = org.person().await?;
        let boss = org
            .role(
                "Boss",
                &[
                    Permission::SeeProjects,
                    Permission::DecideRoles,
                    Permission::CheckAccess,
                ],
            )
            .await?;
        org.give(&lea, boss.id, acme).await?;
        let reader = org.role("Reader", &[Permission::SeeProjects]).await?;
        let wide = org
            .role("Wide", &[Permission::SeeProjects, Permission::ManagePeople])
            .await?;
        let administrator = role_named(&org, "Administrator").await?;
        let give = |role: RoleId, project, applies| {
            org.assign(
                &lea,
                Subject::Person(bea.id),
                Given::Role(role),
                project,
                applies,
            )
        };
        // A role she holds all of: fine. The Administrator role: too much.
        give(reader.id, Some(acme), AppliesTo::ProjectAndSubProjects).await?;
        assert_eq!(
            refused(
                give(
                    administrator.id,
                    Some(acme),
                    AppliesTo::ProjectAndSubProjects
                )
                .await
            ),
            Some(ErrorCode::TooMuch)
        );
        // Organization permissions count nowhere in a project: a role that holds one is compared
        // without it for a row on a project, so she may give it there.
        give(wide.id, Some(acme), AppliesTo::Project).await?;
        // Whole-organization rows need a row for the whole organization.
        assert_eq!(
            refused(give(reader.id, None, AppliesTo::Organization).await),
            Some(ErrorCode::Forbidden)
        );
        // The administrator can give any role anywhere.
        org.assign(
            &org.admin,
            Subject::Person(bea.id),
            Given::Role(administrator.id),
            None,
            AppliesTo::Organization,
        )
        .await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_nobody_puts_a_person_into_a_group_that_holds_more_than_they_hold() -> TestResult {
    each_database("sec_02_group_escalation", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let (kim, bea) = (org.person().await?, org.person().await?);
        let keeper = org
            .role(
                "Group keeper",
                &[Permission::ManageGroups, Permission::SeeProjects],
            )
            .await?;
        org.give_everywhere(&kim, keeper.id).await?;
        let admins = org.group("Admins").await?;
        let readers = org.group("Readers").await?;
        let administrator = role_named(&org, "Administrator").await?;
        let reader = org.role("Reader", &[Permission::SeeProjects]).await?;
        for (group, role) in [(admins, administrator.id), (readers, reader.id)] {
            org.assign(
                &org.admin,
                Subject::Group(group),
                Given::Role(role),
                None,
                AppliesTo::Organization,
            )
            .await?;
        }
        let add = |group| {
            org.rig.api().add_group_member(
                Rig::info(),
                AddGroupMember {
                    session_token: kim.session.clone(),
                    group,
                    person: bea.id,
                },
            )
        };
        // Into a group of readers: fine. Into the group of the administrators: she holds less.
        add(readers).await?;
        assert_eq!(refused(add(admins).await), Some(ErrorCode::TooMuch));
        Ok(())
    })
    .await
}
