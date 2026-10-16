//! The last way to manage roles cannot be removed (SEC-08).

use super::*;

#[tokio::test]
async fn sec_08_nobody_can_remove_the_last_way_to_manage_roles() -> TestResult {
    each_database("sec_08_last_administrator", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let administrator = role_named(&org, "Administrator").await?;
        let remove = |by: &Member, assignment| {
            org.rig.api().remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: by.session.clone(),
                    assignment,
                },
            )
        };
        let own_row = ada_row(&org).await?;
        // Ada is the only one: her row stays.
        assert_eq!(
            refused(remove(&org.admin, own_row).await),
            Some(ErrorCode::WouldLockOut)
        );
        // The role cannot lose the permission or be deleted either.
        let weaker: Vec<Permission> = administrator
            .permissions
            .iter()
            .copied()
            .filter(|permission| *permission != Permission::ManageRoles)
            .collect();
        assert_eq!(
            refused(change(&org, &org.admin, administrator.id, Some(weaker)).await),
            Some(ErrorCode::WouldLockOut)
        );
        // A role that somebody has is first asked for the role that takes its place; with a
        // replacement that does not manage roles, the deletion is what would lock the organization.
        let viewer = role_named(&org, "Viewer").await?;
        let delete = |replacement| {
            org.rig.api().delete_role(
                Rig::info(),
                DeleteRole {
                    session_token: org.admin.session.clone(),
                    role: administrator.id,
                    replacement,
                },
            )
        };
        assert_eq!(refused(delete(None).await), Some(ErrorCode::RoleInUse));
        assert_eq!(
            refused(delete(Some(viewer.id)).await),
            Some(ErrorCode::WouldLockOut)
        );
        // A second administrator through a group: now Ada's row may go.
        let (bea, ops) = (org.person().await?, org.group("Operators").await?);
        org.add_to_group(ops, bea.id).await?;
        org.assign(
            &org.admin,
            Subject::Group(ops),
            Given::Role(administrator.id),
            None,
            AppliesTo::Organization,
        )
        .await?;
        // Bea is the last one through the group: she cannot be taken out, the group cannot be
        // deleted, while Ada's row is the other way. Taking Ada's row away first is fine...
        let remove_member = |person| {
            org.rig.api().remove_group_member(
                Rig::info(),
                RemoveGroupMember {
                    session_token: org.admin.session.clone(),
                    group: ops,
                    person,
                },
            )
        };
        remove(&org.admin, own_row).await?;
        // ...and then Bea and the group are the last way, and Ada is nobody any more but still
        // signed in: her session asks again at every request.
        assert_eq!(
            refused(remove_member(bea.id).await),
            Some(ErrorCode::Forbidden)
        );
        // Bea does the rest herself.
        let as_bea = |person| {
            org.rig.api().remove_group_member(
                Rig::info(),
                RemoveGroupMember {
                    session_token: bea.session.clone(),
                    group: ops,
                    person,
                },
            )
        };
        assert_eq!(refused(as_bea(bea.id).await), Some(ErrorCode::WouldLockOut));
        let delete_group = org.rig.api().delete_group(
            Rig::info(),
            DeleteGroup {
                session_token: bea.session.clone(),
                group: ops,
            },
        );
        assert_eq!(refused(delete_group.await), Some(ErrorCode::WouldLockOut));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_the_two_halves_of_managing_roles_can_come_from_two_roles() -> TestResult {
    each_database("sec_08_two_roles", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let (one, two) = (
            org.role(
                "Manages",
                &[Permission::ManageRoles, Permission::SeeProjects],
            )
            .await?,
            org.role(
                "Decides",
                &[Permission::DecideRoles, Permission::SeeProjects],
            )
            .await?,
        );
        let bea = org.person().await?;
        org.give_everywhere(&bea, one.id).await?;
        org.give_everywhere(&bea, two.id).await?;
        // Ada's row can go now: Bea holds both halves through two roles.
        let ada_row = ada_row(&org).await?;
        org.rig
            .api()
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: org.admin.session.clone(),
                    assignment: ada_row,
                },
            )
            .await?;
        // Taking one half away from the only manager is refused.
        let result = change(&org, &bea, two.id, Some(vec![Permission::SeeProjects])).await;
        assert_eq!(refused(result), Some(ErrorCode::WouldLockOut));
        Ok(())
    })
    .await
}
