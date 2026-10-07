//! The rules that keep the organization from locking itself out and from giving more than a person
//! has, the starting roles, and the record of every change (SEC-02, SEC-08, SEC-15, ADR-0025).

use bb_app::access::{MakeStartingRoles, Permission, PersonRef, StartingRole, make_starting_roles};
use bb_app::auth::AppError;
use bb_app::people::{AddGroupMember, DeleteGroup, MakeGroup, RemoveGroupMember, RenameGroup};
use bb_app::roles::{
    AssignmentView, AssignmentsQuery, ChangeRole, DeleteRole, Given, LevelsQuery, MakeRole,
    RemoveAssignment, ResetRole, RoleView, RolesQuery, SetFieldLevel, SetPeopleLevel, Subject,
    WhoView,
};
use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, RoleId};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_store::Verification;
use bb_testing::each_database;

use super::policy_rig::{Member, Org, Run};
use super::rig::{Outcome, Rig, refused};

/// The roles, as the administrator lists them.
async fn roles(org: &Org<'_>) -> Run<Vec<RoleView>> {
    Ok(org
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
        .roles)
}

async fn role_named(org: &Org<'_>, name: &str) -> Run<RoleView> {
    roles(org)
        .await?
        .into_iter()
        .find(|role| role.name == name)
        .ok_or_else(|| format!("no role named {name}").into())
}

/// The rows of *Who has which role* for the whole organization, as the administrator lists them.
async fn rows_of_the_organization(org: &Org<'_>) -> Run<Vec<AssignmentView>> {
    Ok(org
        .rig
        .api()
        .assignments(
            Rig::info(),
            AssignmentsQuery {
                session_token: org.admin.session.clone(),
                project: None,
                cursor: None,
                limit: bb_core::paging::PAGE_SIZE_MAX,
            },
        )
        .await?
        .into_value()
        .rows)
}

/// The row of the administrator, found by who it is for (rows made in the same moment have no
/// order of their own).
async fn ada_row(org: &Org<'_>) -> Run<AssignmentId> {
    rows_of_the_organization(org)
        .await?
        .into_iter()
        .find(|row| {
            matches!(&row.who, WhoView::Person { person: PersonRef::Name { id, .. } } if *id == org.admin.id)
        })
        .map(|row| row.id)
        .ok_or_else(|| "Ada's row".into())
}

/// Changes a role as somebody.
async fn change(
    org: &Org<'_>,
    by: &Member,
    role: RoleId,
    permissions: Option<Vec<Permission>>,
) -> Result<RoleView, AppError> {
    Ok(org
        .rig
        .api()
        .change_role(
            Rig::info(),
            ChangeRole {
                session_token: by.session.clone(),
                role,
                name: None,
                permissions,
                two_step: None,
            },
        )
        .await?
        .into_value())
}

#[tokio::test]
async fn sec_08_starting_roles_are_made_once_and_can_be_changed_and_reset() -> Outcome {
    each_database("sec_08_starting_roles", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        // The six roles of architecture 7, in the starting order, as editable copies.
        let made = roles(&org).await?;
        let names: Vec<&str> = made.iter().map(|role| role.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Administrator",
                "Project lead",
                "Team member",
                "Contributor",
                "Viewer",
                "Guest"
            ]
        );
        for (role, starting) in made.iter().zip(StartingRole::ALL) {
            assert_eq!(role.starting.as_deref(), Some(starting.key()));
            let wanted: Vec<Permission> = {
                let mut all = starting.permissions();
                all.sort();
                all
            };
            assert_eq!(role.permissions, wanted, "{}", role.name);
            assert!(!role.two_step);
        }
        // Another start makes nothing new: the roles are made once.
        make_starting_roles(&rig.services.ctx(Rig::info()), MakeStartingRoles).await?;
        assert_eq!(roles(&org).await?.len(), 6);

        // A starting role can be renamed and changed like any role; its mark stays.
        let viewer = role_named(&org, "Viewer").await?;
        let renamed = org
            .rig
            .api()
            .change_role(
                Rig::info(),
                ChangeRole {
                    session_token: org.admin.session.clone(),
                    role: viewer.id,
                    name: Some("Looker".to_owned()),
                    permissions: Some(vec![Permission::SeeProjects, Permission::CheckAccess]),
                    two_step: Some(true),
                },
            )
            .await?
            .into_value();
        assert_eq!(renamed.name, "Looker");
        assert_eq!(renamed.starting.as_deref(), Some("viewer"));
        assert!(renamed.two_step);
        assert_eq!(renamed.permissions.len(), 2);

        // Reset gives it the starting permissions back and keeps the new name.
        let reset = org
            .rig
            .api()
            .reset_role(
                Rig::info(),
                ResetRole {
                    session_token: org.admin.session.clone(),
                    role: viewer.id,
                },
            )
            .await?
            .into_value();
        assert_eq!(reset.name, "Looker");
        assert_eq!(reset.permissions, vec![Permission::SeeProjects]);
        assert!(!reset.two_step);

        // A role of our own has nothing to be reset to.
        let own = org.role("Own", &[Permission::SeeProjects]).await?;
        let result = org
            .rig
            .api()
            .reset_role(
                Rig::info(),
                ResetRole {
                    session_token: org.admin.session.clone(),
                    role: own.id,
                },
            )
            .await;
        assert_eq!(refused(result), Some(ErrorCode::InvalidRequest));

        // A starting role can be deleted, and the next start does not bring it back.
        let guest = role_named(&org, "Guest").await?;
        org.rig
            .api()
            .delete_role(
                Rig::info(),
                DeleteRole {
                    session_token: org.admin.session.clone(),
                    role: guest.id,
                    replacement: None,
                },
            )
            .await?;
        make_starting_roles(&rig.services.ctx(Rig::info()), MakeStartingRoles).await?;
        assert!(roles(&org).await?.iter().all(|role| role.name != "Guest"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_a_new_role_has_no_levels_everything_is_hidden_until_somebody_decides() -> Outcome {
    each_database("sec_08_new_role_hidden", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let own = org.role("Own", &[Permission::SeeProjects]).await?;
        let levels = org
            .rig
            .api()
            .role_levels(
                Rig::info(),
                LevelsQuery {
                    session_token: org.admin.session.clone(),
                    role: own.id,
                    project: None,
                },
            )
            .await?
            .into_value();
        assert!(levels.fields.is_empty() && levels.people.is_empty());
        // The starting roles carry the levels of architecture 7.
        let guest = role_named(&org, "Guest").await?;
        let levels = org
            .rig
            .api()
            .role_levels(
                Rig::info(),
                LevelsQuery {
                    session_token: org.admin.session.clone(),
                    role: guest.id,
                    project: None,
                },
            )
            .await?
            .into_value();
        assert_eq!(levels.fields.len(), Field::ALL.len());
        assert_eq!(levels.people.len(), PeopleKind::ALL.len());
        let level = |field| {
            levels
                .fields
                .iter()
                .find(|row| row.field == field)
                .map(|row| row.level)
        };
        assert_eq!(level(Field::CostCenter), Some(FieldLevel::Hidden));
        assert_eq!(level(Field::Title), Some(FieldLevel::CanChange));
        let people = levels
            .people
            .iter()
            .find(|row| row.kind == PeopleKind::AssignedTo)
            .map(|row| row.level);
        assert_eq!(people, Some(PeopleLevel::RoleOnly));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_nobody_adds_a_permission_they_do_not_hold_to_a_role() -> Outcome {
    each_database("sec_08_role_escalation", async |db| {
        let rig = Rig::start(db, 10).await?;
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
            .make_role(&mara, "Small", &[Permission::SeeProjects])
            .await?;
        assert_eq!(
            refused(
                org.make_role(&mara, "Big", &[Permission::ManagePeople])
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
            refused(org.make_role(&cy, "Nope", &[Permission::SeeProjects]).await),
            Some(ErrorCode::Forbidden)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_nobody_gives_a_role_with_more_than_they_hold_there() -> Outcome {
    each_database("sec_02_give_escalation", async |db| {
        let rig = Rig::start(db, 10).await?;
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
async fn sec_02_nobody_puts_a_person_into_a_group_that_holds_more_than_they_hold() -> Outcome {
    each_database("sec_02_group_escalation", async |db| {
        let rig = Rig::start(db, 10).await?;
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

#[tokio::test]
async fn sec_08_nobody_can_remove_the_last_way_to_manage_roles() -> Outcome {
    each_database("sec_08_last_administrator", async |db| {
        let rig = Rig::start(db, 10).await?;
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
async fn sec_08_the_two_halves_of_managing_roles_can_come_from_two_roles() -> Outcome {
    each_database("sec_08_two_roles", async |db| {
        let rig = Rig::start(db, 10).await?;
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
    async fn one_more(&self, action: SecurityAction, before: Mark) -> Outcome {
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
        api.make_project(
            Rig::info(),
            bb_app::projects::MakeProject {
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
        api.make_group(
            Rig::info(),
            MakeGroup {
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
        api.make_role(
            Rig::info(),
            MakeRole {
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
                field: Field::Title,
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
async fn sec_15_every_change_of_permissions_is_in_the_security_log() -> Outcome {
    each_database("sec_15_permission_log", async |db| {
        let rig = Rig::start(db, 10).await?;
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
            .make_role(&bea, "Nope", &[Permission::SeeProjects])
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
