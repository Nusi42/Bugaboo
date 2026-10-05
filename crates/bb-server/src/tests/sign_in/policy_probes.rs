//! Probes from the review of WP-05: each one tried to get around a rule of the management use
//! cases (give no more than you hold, a *No access* is only lifted by somebody who decides above
//! it, a level is not raised by taking an override away, *Manage groups* is not a way to lift a
//! *No access*) and must be refused. Every probe runs on both databases (ARCH-DB-PARITY).

use bb_app::access::{
    MakeStartingRoles, Permission, PolicyStore as _, StartingRole, make_starting_roles,
};
use bb_app::people::{AddGroupMember, DeleteGroup, RemoveGroupMember};
use bb_app::projects::MakeProject;
use bb_app::roles::{
    Given, ReasonView, RemoveAssignment, ResetRole, RolesQuery, SetFieldLevel, SetPeopleLevel,
    Subject,
};
use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::event::DomainEvent;
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, ProjectId, RoleId};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_testing::each_database;

use super::policy_rig::{Member, Org, Run};
use super::rig::{Outcome, Rig, refused};

/// What a person who decides roles in a project may hold there.
const LEAD: [Permission; 3] = [
    Permission::SeeProjects,
    Permission::DecideRoles,
    Permission::CheckAccess,
];

/// Refused because the person may not do it, or may not even know the project.
fn not_allowed(code: Option<ErrorCode>) -> bool {
    matches!(code, Some(ErrorCode::Forbidden | ErrorCode::NotFound))
}

async fn remove(
    org: &Org<'_>,
    by: &Member,
    row: AssignmentId,
) -> Result<(), bb_app::auth::AppError> {
    org.rig
        .api()
        .remove_assignment(
            Rig::info(),
            RemoveAssignment {
                session_token: by.session.clone(),
                assignment: row,
            },
        )
        .await
        .map(|_| ())
}

#[tokio::test]
async fn sec_02_nobody_gives_a_role_with_sub_projects_they_do_not_hold_there() -> Outcome {
    each_database("sec_02_sub_projects", async |db| {
        let rig = Rig::start(db, 10).await?;
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

#[tokio::test]
async fn sec_03_decide_roles_is_not_kept_from_a_row_at_the_same_depth_as_the_no_access() -> Outcome
{
    each_database("sec_03_same_depth", async |db| {
        let rig = Rig::start(db, 10).await?;
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

/// The Setter role: sees projects and decides what roles see, and sees only what it may.
async fn setter(org: &Org<'_>) -> Run<RoleId> {
    let setter = org
        .role(
            "Setter",
            &[Permission::SeeProjects, Permission::DecideWhatRolesSee],
        )
        .await?;
    for field in [Field::Title] {
        org.rig
            .api()
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: org.admin.session.clone(),
                    role: setter.id,
                    project: None,
                    field,
                    level: Some(FieldLevel::CanSee),
                },
            )
            .await?;
    }
    org.rig
        .api()
        .set_people_level(
            Rig::info(),
            SetPeopleLevel {
                session_token: org.admin.session.clone(),
                role: setter.id,
                project: None,
                kind: PeopleKind::TimeEntries,
                level: Some(PeopleLevel::RoleOnly),
            },
        )
        .await?;
    Ok(setter.id)
}

#[tokio::test]
async fn sec_04_nobody_raises_a_level_by_taking_an_override_away() -> Outcome {
    each_database("sec_04_remove_override", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let web = org.project(Some(acme), "WEB").await?.id;
        let bob = org.person().await?;
        let setter = setter(&org).await?;
        org.give(&bob, setter, acme).await?;
        // The role Target has its own levels high, and lower ones in ACME.
        let target = org.role("Target", &[Permission::SeeProjects]).await?;
        let api = org.rig.api();
        let admin = org.admin.session.clone();
        let field = |project: Option<ProjectId>, level| SetFieldLevel {
            session_token: admin.clone(),
            role: target.id,
            project,
            field: Field::Title,
            level,
        };
        let kind = |project: Option<ProjectId>, level| SetPeopleLevel {
            session_token: admin.clone(),
            role: target.id,
            project,
            kind: PeopleKind::TimeEntries,
            level,
        };
        api.set_field_level(Rig::info(), field(None, Some(FieldLevel::CanChange)))
            .await?;
        api.set_field_level(Rig::info(), field(Some(acme), Some(FieldLevel::Hidden)))
            .await?;
        api.set_people_level(Rig::info(), kind(None, Some(PeopleLevel::Names)))
            .await?;
        api.set_people_level(Rig::info(), kind(Some(acme), Some(PeopleLevel::Hidden)))
            .await?;

        // Bob sees titles and time entries only a little. Taking the override away would give
        // the role its own, higher level in ACME: refused.
        let bob_field = |project, level| SetFieldLevel {
            session_token: bob.session.clone(),
            ..field(project, level)
        };
        let bob_kind = |project, level| SetPeopleLevel {
            session_token: bob.session.clone(),
            ..kind(project, level)
        };
        assert_eq!(
            refused(
                api.set_field_level(Rig::info(), bob_field(Some(acme), None))
                    .await
            ),
            Some(ErrorCode::TooMuch)
        );
        assert_eq!(
            refused(
                api.set_people_level(Rig::info(), bob_kind(Some(acme), None))
                    .await
            ),
            Some(ErrorCode::TooMuch)
        );

        // An override in WEB falls back to the one in ACME when it is taken away: Hidden, so
        // that is allowed.
        api.set_field_level(Rig::info(), field(Some(web), Some(FieldLevel::CanSee)))
            .await?;
        api.set_people_level(Rig::info(), kind(Some(web), Some(PeopleLevel::RoleOnly)))
            .await?;
        api.set_field_level(Rig::info(), bob_field(Some(web), None))
            .await?;
        api.set_people_level(Rig::info(), bob_kind(Some(web), None))
            .await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_02_manage_groups_does_not_lift_a_no_access_of_the_group() -> Outcome {
    each_database("sec_02_group_no_access", async |db| {
        let rig = Rig::start(db, 10).await?;
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
async fn adr_0007_events_of_levels_and_removed_rows_say_what_was_there_before() -> Outcome {
    each_database("adr_0007_before_values", async |db| {
        let rig = Rig::start(db, 10).await?;
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
                    field: Field::Title,
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

#[tokio::test]
async fn sec_03_check_access_in_a_project_lists_what_counts_there_and_the_rows_that_reach_it()
-> Outcome {
    each_database("sec_03_check_in_project", async |db| {
        let rig = Rig::start(db, 10).await?;
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

#[tokio::test]
async fn sec_08_two_requests_for_a_role_and_a_no_access_leave_only_one_row() -> Outcome {
    each_database("sec_08_conflict_race", async |db| {
        let rig = Rig::start(db, 10).await?;
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
async fn sec_08_two_servers_starting_together_make_the_starting_roles_once() -> Outcome {
    each_database("sec_08_starting_race", async |db| {
        let rig = Rig::start_unstarted(db).await?;
        let (first, second) = (rig.services.ctx(Rig::info()), rig.services.ctx(Rig::info()));
        let (first, second) = tokio::join!(
            make_starting_roles(&first, MakeStartingRoles),
            make_starting_roles(&second, MakeStartingRoles)
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
            .filter(|entry| entry.action.as_str() == SecurityAction::StartingRolesMade.code())
            .count();
        assert_eq!(entries, 1, "the log says it once");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_04_nobody_sets_a_people_level_above_their_own() -> Outcome {
    each_database("sec_04_people_level_too_much", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let bob = org.person().await?;
        // Bob's own level for time entries is *Role only*.
        let setter = setter(&org).await?;
        org.give(&bob, setter, acme).await?;
        let target = org.role("Target", &[Permission::SeeProjects]).await?;
        let api = org.rig.api();
        let set = |level| {
            api.set_people_level(
                Rig::info(),
                SetPeopleLevel {
                    session_token: bob.session.clone(),
                    role: target.id,
                    project: Some(acme),
                    kind: PeopleKind::TimeEntries,
                    level,
                },
            )
        };
        assert_eq!(
            refused(set(Some(PeopleLevel::Names)).await),
            Some(ErrorCode::TooMuch)
        );
        set(Some(PeopleLevel::RoleOnly)).await?;
        set(Some(PeopleLevel::Hidden)).await?;
        Ok(())
    })
    .await
}

/// Gives the role the highest level of every field.
async fn all_fields(org: &Org<'_>, role: RoleId) -> Run<()> {
    for field in Field::ALL {
        org.rig
            .api()
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: org.admin.session.clone(),
                    role,
                    project: None,
                    field: *field,
                    level: Some(FieldLevel::CanChange),
                },
            )
            .await?;
    }
    Ok(())
}

/// Gives the role the highest level of every kind of people information.
async fn all_kinds(org: &Org<'_>, role: RoleId) -> Run<()> {
    for kind in PeopleKind::ALL {
        org.rig
            .api()
            .set_people_level(
                Rig::info(),
                SetPeopleLevel {
                    session_token: org.admin.session.clone(),
                    role,
                    project: None,
                    kind: *kind,
                    level: Some(PeopleLevel::Names),
                },
            )
            .await?;
    }
    Ok(())
}

#[tokio::test]
async fn sec_04_nobody_resets_a_role_to_levels_above_their_own() -> Outcome {
    // Once for each half of the rule: the field levels and the levels of people information.
    for (fields_first, name) in [
        (true, "sec_04_reset_fields_first"),
        (false, "sec_04_reset_kinds_first"),
    ] {
        each_database(name, async |db| {
            let rig = Rig::start(db, 10).await?;
            let org = Org::start(&rig).await?;
            let kim = org.person().await?;
            // Kim manages roles and holds every permission of the Viewer role, but sees nothing.
            let keeper = org
                .role(
                    "Role keeper",
                    &[Permission::ManageRoles, Permission::SeeProjects],
                )
                .await?;
            org.give_everywhere(&kim, keeper.id).await?;
            let viewer = org
                .rig
                .api()
                .roles(
                    Rig::info(),
                    RolesQuery {
                        session_token: org.admin.session.clone(),
                        place: None,
                    },
                )
                .await?
                .into_value()
                .roles
                .into_iter()
                .find(|role| role.name == "Viewer")
                .ok_or("the Viewer role")?;
            let reset = || {
                org.rig.api().reset_role(
                    Rig::info(),
                    ResetRole {
                        session_token: kim.session.clone(),
                        role: viewer.id,
                    },
                )
            };
            // The Viewer role starts with levels she does not have.
            assert_eq!(refused(reset().await), Some(ErrorCode::TooMuch));
            // Holding the one half is not enough...
            if fields_first {
                all_fields(&org, keeper.id).await?;
            } else {
                all_kinds(&org, keeper.id).await?;
            }
            assert_eq!(refused(reset().await), Some(ErrorCode::TooMuch));
            // ...holding both is.
            if fields_first {
                all_kinds(&org, keeper.id).await?;
            } else {
                all_fields(&org, keeper.id).await?;
            }
            reset().await?;
            Ok(())
        })
        .await?;
    }
    Ok(())
}

#[tokio::test]
async fn sec_02_only_rows_that_can_exist_are_made() -> Outcome {
    each_database("sec_02_valid_rows", async |db| {
        let rig = Rig::start(db, 10).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let bob = org.person().await?;
        let reader = org.role("Reader", &[Permission::SeeProjects]).await?;
        let make = |given, project, applies_to| {
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
        assert_eq!(refused(make(role, None, AppliesTo::Project).await), invalid);
        assert_eq!(
            refused(make(role, None, AppliesTo::ProjectAndSubProjects).await),
            invalid
        );
        for applies_to in [
            AppliesTo::Organization,
            AppliesTo::Project,
            AppliesTo::ProjectAndSubProjects,
        ] {
            assert_eq!(
                refused(make(Given::NoAccess, None, applies_to).await),
                invalid,
                "No access for the whole organization: {applies_to:?}"
            );
        }
        assert_eq!(
            refused(make(Given::NoAccess, Some(acme), AppliesTo::Project).await),
            invalid
        );
        assert_eq!(
            refused(make(Given::NoAccess, Some(acme), AppliesTo::Organization).await),
            invalid
        );
        assert_eq!(
            refused(make(role, Some(acme), AppliesTo::Organization).await),
            invalid
        );
        // The three that can exist (a role and *No access* for one person on one project are a
        // conflict of their own, so *No access* is for somebody else).
        make(role, None, AppliesTo::Organization).await?;
        make(role, Some(acme), AppliesTo::Project).await?;
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

/// What a lead of one project holds: sees it, decides who has which role there, makes
/// sub-projects.
const SECTION_LEAD: [Permission; 3] = [
    Permission::SeeProjects,
    Permission::DecideRoles,
    Permission::MakeSubProjects,
];

#[tokio::test]
async fn sec_02_a_role_with_sub_projects_needs_a_row_that_reaches_sub_projects() -> Outcome {
    each_database("sec_02_future_sub_projects", async |db| {
        let rig = Rig::start(db, 10).await?;
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
            .make_project(
                Rig::info(),
                MakeProject {
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
-> Outcome {
    each_database("sec_02_group_future_sub_projects", async |db| {
        let rig = Rig::start(db, 10).await?;
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
