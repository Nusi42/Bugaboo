//! Probes from the review of WP-05: each one tried to get around a rule of the management use
//! cases (give no more than you hold, a *No access* is only lifted by somebody who decides above
//! it, a level is not raised by taking an override away, *Manage groups* is not a way to lift a
//! *No access*) and must be refused. Every probe runs on both databases (ARCH-DB-PARITY).

use bb_app::access::Permission;
use bb_app::people::{DeleteGroup, RemoveGroupMember};
use bb_app::roles::{Given, ReasonView, RemoveAssignment, SetFieldLevel, SetPeopleLevel, Subject};
use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::event::DomainEvent;
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, ProjectId, RoleId};
use bb_core::people_kinds::PeopleKind;
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
                bb_app::roles::RolesQuery {
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
