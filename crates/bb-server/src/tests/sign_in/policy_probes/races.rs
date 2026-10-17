//! Two requests or two servers at the same moment leave one row (SEC-08).

use super::*;

#[tokio::test]
async fn sec_08_two_requests_for_a_role_and_a_no_access_leave_only_one_row() -> TestResult {
    each_database("sec_08_conflict_race", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let bob = org.person().await?;
        let reader = org.role("Reader", &[Permission::SeeProjects]).await?;
        // Both asks look at the rows of ACME at the same time: the check for a role and a *No
        // access* on one project for one person runs inside the change, after the lock, so the
        // second one finds the first.
        let role = org.assign(
            &org.admin,
            Subject::Person(bob.id),
            Given::Role(reader.id),
            Some(acme),
            AppliesTo::ProjectAndSubProjects,
        );
        let no_access = org.assign(
            &org.admin,
            Subject::Person(bob.id),
            Given::NoAccess,
            Some(acme),
            AppliesTo::ProjectAndSubProjects,
        );
        let (role, no_access) = tokio::join!(role, no_access);
        let refusals = [refused(role.map(|_| ())), refused(no_access.map(|_| ()))];
        assert_eq!(
            refusals.iter().filter(|code| code.is_none()).count(),
            1,
            "exactly one of them is made: {refusals:?}"
        );
        assert!(refusals.contains(&Some(ErrorCode::AssignmentConflict)));
        let rows = rig
            .api()
            .assignments(
                Rig::info(),
                bb_app::roles::AssignmentsQuery {
                    session_token: org.admin.session.clone(),
                    project: Some(acme),
cursor: None,
limit: bb_core::paging::PAGE_SIZE_MAX,
},
            )
            .await?
            .into_value()
            .rows;
        let on_acme = rows
            .iter()
            .filter(|row| !row.inherited)
            .filter(|row| matches!(&row.who, bb_app::roles::WhoView::Person { person: bb_app::access::PersonRef::Name { id, .. } } if *id == bob.id))
            .count();
        assert_eq!(on_acme, 1, "one row for Bob on ACME");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_two_servers_starting_together_create_the_starting_roles_once() -> TestResult {
    each_database("sec_08_starting_race", async |db| {
        let rig = Rig::start_unstarted(db).await?;
        let (first, second) = (rig.services.ctx(Rig::info()), rig.services.ctx(Rig::info()));
        let (first, second) = tokio::join!(
            create_starting_roles(&first, CreateStartingRoles),
            create_starting_roles(&second, CreateStartingRoles)
        );
        first?;
        second?;
        let roles = rig.storage.roles().await?;
        assert_eq!(roles.len(), StartingRole::ALL.len(), "each role once");
        let entries = rig
            .database
            .security_log_entries(0, 10_000)
            .await?
            .iter()
            .filter(|entry| entry.action.as_str() == SecurityAction::StartingRolesCreated.code())
            .count();
        assert_eq!(entries, 1, "the log says it once");
        Ok(())
    })
    .await
}
