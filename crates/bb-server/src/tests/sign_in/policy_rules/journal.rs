//! Every change of permissions is in the security log (SEC-15).

use super::*;

/// What the outbox and the security log hold, to say what a change added.
struct Journal<'a> {
    rig: &'a Rig,
}

/// How many events and how many entries of one kind there were.
type Mark = (usize, usize);

impl Journal<'_> {
    async fn events(&self) -> Result<usize, bb_store::StoreError> {
        let consumer = bb_core::technical::TechnicalName::new("test-reader")
            .map_err(|_| bb_store::StoreError::ReservedName)?;
        Ok(self
            .rig
            .database
            .outbox_after(&consumer, 100_000)
            .await?
            .len())
    }

    async fn entries(&self, action: SecurityAction) -> Result<usize, bb_store::StoreError> {
        Ok(self
            .rig
            .database
            .security_log_entries(0, 10_000)
            .await?
            .iter()
            .filter(|entry| entry.action.as_str() == action.code())
            .count())
    }

    async fn mark(&self, action: SecurityAction) -> Result<Mark, bb_store::StoreError> {
        Ok((self.events().await?, self.entries(action).await?))
    }

    /// The change since `before` added one event and one entry of the log named `action`.
    async fn one_more(&self, action: SecurityAction, before: Mark) -> TestResult {
        assert_eq!(self.events().await?, before.0 + 1, "{action:?}: one event");
        assert_eq!(
            self.entries(action).await?,
            before.1 + 1,
            "{action:?}: one entry"
        );
        Ok(())
    }
}

/// Runs a use case and checks that it wrote one event and one entry of the log.
macro_rules! writes_one {
    ($journal:expr, $action:expr, $call:expr) => {{
        let before = $journal.mark($action).await?;
        let answer = $call.await?;
        $journal.one_more($action, before).await?;
        answer
    }};
}

/// Projects and groups.
async fn projects_and_groups(org: &Org<'_>, journal: &Journal<'_>, bea: &Member) -> Run<()> {
    let (api, session) = (org.rig.api(), org.admin.session.clone());
    let acme = org.project(None, "ACME").await?.id;
    writes_one!(
        journal,
        SecurityAction::ProjectMade,
        api.create_project(
            Rig::info(),
            bb_app::projects::CreateProject {
                session_token: session.clone(),
                parent: Some(acme),
                short_name: "WEB".to_owned(),
                name: "Web".to_owned(),
            },
        )
    );
    let group = writes_one!(
        journal,
        SecurityAction::GroupMade,
        api.create_group(
            Rig::info(),
            CreateGroup {
                session_token: session.clone(),
                name: "Team".to_owned(),
            },
        )
    )
    .into_value()
    .id;
    writes_one!(
        journal,
        SecurityAction::GroupRenamed,
        api.rename_group(
            Rig::info(),
            RenameGroup {
                session_token: session.clone(),
                group,
                version: org.rig.group_version(group).await?,
                name: "Crew".to_owned(),
            },
        )
    );
    writes_one!(
        journal,
        SecurityAction::GroupMemberAdded,
        api.add_group_member(
            Rig::info(),
            AddGroupMember {
                session_token: session.clone(),
                group,
                person: bea.id,
            },
        )
    );
    writes_one!(
        journal,
        SecurityAction::GroupMemberRemoved,
        api.remove_group_member(
            Rig::info(),
            RemoveGroupMember {
                session_token: session.clone(),
                group,
                person: bea.id,
            },
        )
    );
    writes_one!(
        journal,
        SecurityAction::GroupDeleted,
        api.delete_group(
            Rig::info(),
            DeleteGroup {
                session_token: session,
                group,
            },
        )
    );
    Ok(())
}

/// Making, changing and resetting a role.
async fn roles_of_the_organization(org: &Org<'_>, journal: &Journal<'_>) -> Run<RoleView> {
    let (api, session) = (org.rig.api(), org.admin.session.clone());
    let made = writes_one!(
        journal,
        SecurityAction::RoleMade,
        api.create_role(
            Rig::info(),
            CreateRole {
                session_token: session.clone(),
                name: "Own".to_owned(),
                permissions: vec![Permission::SeeProjects],
                two_step: false,
            },
        )
    )
    .into_value();
    writes_one!(
        journal,
        SecurityAction::RoleChanged,
        api.change_role(
            Rig::info(),
            ChangeRole {
                session_token: session.clone(),
                role: made.id,
                version: made.version,
                name: Some("Own role".to_owned()),
                permissions: None,
                two_step: Some(true),
            },
        )
    );
    let starting = role_named(org, "Viewer").await?;
    writes_one!(
        journal,
        SecurityAction::RoleReset,
        api.reset_role(
            Rig::info(),
            ResetRole {
                session_token: session.clone(),
                role: starting.id,
                version: starting.version,
            },
        )
    );
    Ok(made)
}

/// The rows that give the role, the levels it sets, and its end.
async fn rows_and_levels(
    org: &Org<'_>,
    journal: &Journal<'_>,
    bea: &Member,
    made: &RoleView,
) -> Run<()> {
    let (api, session) = (org.rig.api(), org.admin.session.clone());
    let shop = org.project(None, "SHOP").await?.id;
    let row = writes_one!(
        journal,
        SecurityAction::AssignmentMade,
        api.assign_role(
            Rig::info(),
            bb_app::roles::AssignRole {
                session_token: session.clone(),
                subject: Subject::Person(bea.id),
                given: Given::Role(made.id),
                project: Some(shop),
                applies_to: AppliesTo::Project,
            },
        )
    )
    .into_value();
    writes_one!(
        journal,
        SecurityAction::FieldLevelChanged,
        api.set_field_level(
            Rig::info(),
            SetFieldLevel {
                session_token: session.clone(),
                role: made.id,
                project: None,
                issue_type: None,
                field: Field::Title.into(),
                level: Some(FieldLevel::CanSee),
            },
        )
    );
    writes_one!(
        journal,
        SecurityAction::PeopleLevelChanged,
        api.set_people_level(
            Rig::info(),
            SetPeopleLevel {
                session_token: session.clone(),
                role: made.id,
                project: None,
                kind: PeopleKind::Followers,
                level: Some(PeopleLevel::Names),
            },
        )
    );
    writes_one!(
        journal,
        SecurityAction::AssignmentRemoved,
        api.remove_assignment(
            Rig::info(),
            RemoveAssignment {
                session_token: session.clone(),
                assignment: row.id,
            },
        )
    );
    writes_one!(
        journal,
        SecurityAction::RoleDeleted,
        api.delete_role(
            Rig::info(),
            DeleteRole {
                session_token: session,
                role: made.id,
                replacement: None,
            },
        )
    );
    Ok(())
}

#[tokio::test]
async fn sec_15_every_change_of_permissions_is_in_the_security_log() -> TestResult {
    each_database("sec_15_permission_log", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let journal = Journal { rig: &rig };
        let bea = org.person().await?;
        // Every management use case, once: one event and one entry of the log each.
        projects_and_groups(&org, &journal, &bea).await?;
        let made = roles_of_the_organization(&org, &journal).await?;
        rows_and_levels(&org, &journal, &bea, &made).await?;
        // A refused change writes nothing, and the log still verifies.
        let (events, entries) = (
            journal.events().await?,
            rig.database.security_log_entries(0, 10_000).await?.len(),
        );
        let refused_change = org
            .create_role(&bea, "Nope", &[Permission::SeeProjects])
            .await;
        assert_eq!(refused(refused_change), Some(ErrorCode::Forbidden));
        assert_eq!(journal.events().await?, events);
        assert_eq!(
            rig.database.security_log_entries(0, 10_000).await?.len(),
            entries
        );
        let verified = rig.database.verify_security_log(&rig.seal).await?;
        assert!(
            matches!(verified, Verification::Valid { anchored: true, .. }),
            "{verified:?}"
        );
        Ok(())
    })
    .await
}
