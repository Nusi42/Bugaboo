//! The policy engine through the real wiring, on both databases (SEC-01, SEC-02, SEC-03, SEC-04,
//! PRIV-12, ADR-0025): roles on the project tree, *No access*, and what *Check access* says.

use bb_app::access::Permission;
use bb_app::access::ProjectLabel;
use bb_app::people::GroupMembersQuery;
use bb_app::roles::{
    AssignmentsQuery, Given, LevelPlace, LevelsQuery, ReasonView, RemoveAssignment, RolesQuery,
    SetFieldLevel, Subject, WhoView,
};
use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::fields::Field;
use bb_core::id::ProjectId;
use bb_testing::{TestResult, each_database};

use super::policy_rig::{Org, Run, decision};
use super::rig::{Rig, refused};

const VIEW: [Permission; 1] = [Permission::SeeProjects];

/// ACME, WEB inside it, API inside that, and OTHER on its own.
struct Tree {
    acme: ProjectId,
    web: ProjectId,
    api: ProjectId,
    other: ProjectId,
}

async fn tree(org: &Org<'_>) -> Run<Tree> {
    let acme = org.project(None, "ACME").await?.id;
    let web = org.project(Some(acme), "WEB").await?.id;
    let api = org.project(Some(web), "API").await?.id;
    let other = org.project(None, "OTHER").await?.id;
    Ok(Tree {
        acme,
        web,
        api,
        other,
    })
}

#[tokio::test]
async fn sec_01_a_person_without_a_role_can_do_nothing_and_sees_no_project() -> TestResult {
    each_database("sec_01_default_deny", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let projects = tree(&org).await?;
        let bea = org.person().await?;
        // Nothing is allowed anywhere, as the administrator sees it with Check access.
        for place in [None, Some(projects.acme), Some(projects.api)] {
            let answer = org.check(&org.admin, bea.id, place).await?;
            assert!(answer.permissions.iter().all(|decision| !decision.allowed));
            assert!(
                answer
                    .fields
                    .iter()
                    .all(|field| field.level == FieldLevel::Hidden)
            );
        }
        // And she cannot do anything herself.
        assert_eq!(
            refused(org.make_role(&bea, "Mine", &VIEW).await),
            Some(ErrorCode::Forbidden)
        );
        let make = |parent| {
            let session = bea.session.clone();
            async move {
                org.rig
                    .api()
                    .make_project(
                        Rig::info(),
                        bb_app::projects::MakeProject {
                            session_token: session,
                            parent,
                            short_name: "MINE".to_owned(),
                            name: "Mine".to_owned(),
                        },
                    )
                    .await
            }
        };
        assert_eq!(refused(make(None).await), Some(ErrorCode::Forbidden));
        // A project that exists but that she may not know about answers like one that does not
        // exist: the same code, the same everything.
        let hidden = refused(make(Some(projects.acme)).await);
        let missing = refused(make(Some(unknown_project())).await);
        assert_eq!(hidden, Some(ErrorCode::NotFound));
        assert_eq!(hidden, missing);
        Ok(())
    })
    .await
}

/// A project ID that no project has.
fn unknown_project() -> ProjectId {
    ProjectId::from_parts(jiff::Timestamp::UNIX_EPOCH, [7; 10])
        .unwrap_or_else(|_| unreachable!("the epoch is not before 1970"))
}

#[tokio::test]
async fn sec_03_a_role_on_a_project_applies_to_its_sub_projects_unless_given_for_this_project_only()
-> TestResult {
    each_database("sec_03_inheritance", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let projects = tree(&org).await?;
        let (bea, cy) = (org.person().await?, org.person().await?);
        let member = org.role("Looker", &VIEW).await?;
        org.give(&bea, member.id, projects.acme).await?;
        for place in [projects.acme, projects.web, projects.api] {
            assert!(org.can(&bea, Permission::SeeProjects, Some(place)).await?);
        }
        assert!(
            !org.can(&bea, Permission::SeeProjects, Some(projects.other))
                .await?
        );
        // For this project only: the sub-projects do not get it.
        org.assign(
            &org.admin,
            Subject::Person(cy.id),
            Given::Role(member.id),
            Some(projects.acme),
            AppliesTo::Project,
        )
        .await?;
        assert!(
            org.can(&cy, Permission::SeeProjects, Some(projects.acme))
                .await?
        );
        assert!(
            !org.can(&cy, Permission::SeeProjects, Some(projects.web))
                .await?
        );
        // A role for the whole organization counts in every project.
        let dee = org.person().await?;
        org.give_everywhere(&dee, member.id).await?;
        assert!(
            org.can(&dee, Permission::SeeProjects, Some(projects.other))
                .await?
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_no_access_on_a_sub_project_wins_over_a_role_from_above() -> TestResult {
    each_database("sec_03_no_access", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let projects = tree(&org).await?;
        let bea = org.person().await?;
        let member = org.role("Looker", &VIEW).await?;
        let other_role = org.role("Helper", &[Permission::MakeSubProjects]).await?;
        let from_above = org.give(&bea, member.id, projects.acme).await?;
        let no_access = org.forbid(&bea, projects.web).await?;
        assert!(org.can(&bea, Permission::SeeProjects, Some(projects.acme)).await?);
        assert!(!org.can(&bea, Permission::SeeProjects, Some(projects.web)).await?);
        assert!(!org.can(&bea, Permission::SeeProjects, Some(projects.api)).await?);
        // The reasons say why: the role from above was removed by that row.
        let answer = org.check(&org.admin, bea.id, Some(projects.web)).await?;
        let why = &decision(&answer, Permission::SeeProjects).reasons;
        assert!(
            matches!(
                why.as_slice(),
                [ReasonView::RemovedByNoAccess { row, no_access: by, .. }]
                    if row.assignment == from_above.id && by.assignment == no_access.id && row.inherited
            ),
            "{why:?}"
        );
        // The nearer row decides: a role given lower down counts again.
        org.give(&bea, other_role.id, projects.api).await?;
        assert!(org.can(&bea, Permission::MakeSubProjects, Some(projects.api)).await?);
        assert!(!org.can(&bea, Permission::SeeProjects, Some(projects.api)).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_the_same_project_rules_of_m2_q1_person_before_group() -> TestResult {
    each_database("sec_03_same_project", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let projects = tree(&org).await?;
        let member = org.role("Looker", &VIEW).await?;
        let direct = org.role("Helper", &[Permission::MakeSubProjects]).await?;
        let on_web = |subject, given| {
            org.assign(
                &org.admin,
                subject,
                given,
                Some(projects.web),
                AppliesTo::ProjectAndSubProjects,
            )
        };
        // No access for the person wins over the role of one of her groups on the same project.
        let (bea, team) = (org.person().await?, org.group("Team").await?);
        org.add_to_group(team, bea.id).await?;
        on_web(Subject::Group(team), Given::Role(member.id)).await?;
        assert!(
            org.can(&bea, Permission::SeeProjects, Some(projects.web))
                .await?
        );
        org.forbid(&bea, projects.web).await?;
        assert!(
            !org.can(&bea, Permission::SeeProjects, Some(projects.web))
                .await?
        );
        // No access for a group wins over the role of another group of the person on the same
        // project, but a role given to the person directly on that project still counts.
        let (cy, blocked, staff) = (
            org.person().await?,
            org.group("Blocked").await?,
            org.group("Staff").await?,
        );
        org.add_to_group(blocked, cy.id).await?;
        org.add_to_group(staff, cy.id).await?;
        on_web(Subject::Group(staff), Given::Role(member.id)).await?;
        on_web(Subject::Group(blocked), Given::NoAccess).await?;
        assert!(
            !org.can(&cy, Permission::SeeProjects, Some(projects.web))
                .await?
        );
        org.give(&cy, direct.id, projects.web).await?;
        assert!(
            org.can(&cy, Permission::MakeSubProjects, Some(projects.web))
                .await?
        );
        assert!(
            !org.can(&cy, Permission::SeeProjects, Some(projects.web))
                .await?
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_a_person_or_group_never_has_a_role_and_no_access_on_one_project() -> TestResult {
    each_database("sec_03_conflict", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let projects = tree(&org).await?;
        let bea = org.person().await?;
        let member = org.role("Looker", &VIEW).await?;
        let row = org.give(&bea, member.id, projects.web).await?;
        // The same project, the same person: the row in the way is told, and nothing changes.
        assert_eq!(
            refused(org.forbid(&bea, projects.web).await),
            Some(ErrorCode::AssignmentConflict)
        );
        assert_eq!(
            refused(org.give(&bea, member.id, projects.web).await),
            Some(ErrorCode::AssignmentConflict),
            "the same row twice"
        );
        // Another project is fine, and so is the same one after the row was taken away.
        org.forbid(&bea, projects.api).await?;
        org.rig
            .api()
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: org.admin.session.clone(),
                    assignment: row.id,
                },
            )
            .await?;
        org.forbid(&bea, projects.web).await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_no_access_cannot_lock_out_the_people_who_decide_roles_above() -> TestResult {
    each_database("sec_03_no_lock_out", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let projects = tree(&org).await?;
        let lea = org.person().await?;
        let lead = org
            .role(
                "Boss",
                &[
                    Permission::SeeProjects,
                    Permission::DecideRoles,
                    Permission::CheckAccess,
                    Permission::ChangeProjectSettings,
                ],
            )
            .await?;
        org.give(&lea, lead.id, projects.acme).await?;
        let no_access = org.forbid(&lea, projects.web).await?;
        // She sees nothing in WEB any more, and cannot check access there (she knows it exists)...
        assert_eq!(
            refused(org.check(&lea, lea.id, Some(projects.web)).await),
            Some(ErrorCode::Forbidden)
        );
        // ...but she still decides who has which role there: she sees the rows, with the name and
        // short name of the project, and can take the No access away again.
        let rows = org
            .rig
            .api()
            .assignments(
                Rig::info(),
                AssignmentsQuery {
                    session_token: lea.session.clone(),
                    project: Some(projects.web),
                    cursor: None,
                    limit: bb_core::paging::PAGE_SIZE_MAX,
                },
            )
            .await?
            .into_value();
        let own = rows
            .rows
            .iter()
            .find(|row| row.id == no_access.id)
            .ok_or("the No access row is listed")?;
        assert!(matches!(
            own.project,
            Some(ProjectLabel::Visible { ref short_name, .. }) if short_name == "WEB"
        ));
        assert!(matches!(own.who, WhoView::Person { .. }));
        org.rig
            .api()
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: lea.session.clone(),
                    assignment: no_access.id,
                },
            )
            .await?;
        assert!(
            org.can(&lea, Permission::SeeProjects, Some(projects.web))
                .await?
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_a_role_taken_away_counts_from_the_next_request() -> TestResult {
    each_database("sec_03_next_request", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let projects = tree(&org).await?;
        let lea = org.person().await?;
        let lead = org
            .role("Boss", &[Permission::SeeProjects, Permission::DecideRoles])
            .await?;
        let row = org.give(&lea, lead.id, projects.acme).await?;
        let list = |project| {
            let rig = org.rig;
            let session = lea.session.clone();
            async move {
                rig.api()
                    .assignments(
                        Rig::info(),
                        AssignmentsQuery {
                            session_token: session,
                            project: Some(project),
                            cursor: None,
                            limit: bb_core::paging::PAGE_SIZE_MAX,
                        },
                    )
                    .await
            }
        };
        list(projects.acme).await?;
        org.rig
            .api()
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: org.admin.session.clone(),
                    assignment: row.id,
                },
            )
            .await?;
        // The very next request already knows: she does not know the project any more.
        assert_eq!(
            refused(list(projects.acme).await),
            Some(ErrorCode::NotFound)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_03_check_access_names_no_project_the_checker_cannot_see() -> TestResult {
    each_database("sec_03_check_access_names", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let projects = tree(&org).await?;
        let (lea, bea) = (org.person().await?, org.person().await?);
        let checker = org.checker().await?;
        let member = org.role("Looker", &VIEW).await?;
        // Lea checks in WEB; Bea has her role from ACME, a project Lea cannot see.
        org.give(&lea, checker.id, projects.web).await?;
        org.give(&bea, member.id, projects.acme).await?;
        let answer = org.check(&lea, bea.id, Some(projects.web)).await?;
        let why = &decision(&answer, Permission::SeeProjects);
        assert!(why.allowed);
        let ReasonView::GivenByRole { row, .. } = &why.reasons[0] else {
            return Err(format!("{:?}", why.reasons).into());
        };
        assert!(row.inherited);
        assert_eq!(row.project, Some(ProjectLabel::Hidden));
        // Nothing in the answer says the name or the short name of ACME.
        let text = serde_json::to_string(&answer)?;
        assert!(!text.contains("ACME"), "{text}");
        assert!(text.contains("WEB"), "the project she checks in is named");
        // The administrator sees it all.
        let all = org.check(&org.admin, bea.id, Some(projects.web)).await?;
        assert!(serde_json::to_string(&all)?.contains("ACME"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_04_the_highest_field_level_of_the_roles_wins_and_projects_inherit_their_own()
-> TestResult {
    each_database("sec_04_field_levels", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let projects = tree(&org).await?;
        let bea = org.person().await?;
        let viewer = org.role("Reader", &VIEW).await?;
        let member = org.role("Looker", &VIEW).await?;
        let set = |role, project, field, level| {
            let rig = org.rig;
            let session = org.admin.session.clone();
            async move {
                rig.api()
                    .set_field_level(
                        Rig::info(),
                        SetFieldLevel {
                            session_token: session,
                            role,
                            project,
                            field,
                            level,
                        },
                    )
                    .await
            }
        };
        set(viewer.id, None, Field::CostCenter, Some(FieldLevel::Hidden)).await?;
        set(member.id, None, Field::CostCenter, Some(FieldLevel::CanSee)).await?;
        org.give(&bea, viewer.id, projects.acme).await?;
        let level = |answer: &bb_app::roles::AccessAnswer| {
            answer
                .fields
                .iter()
                .find(|field| field.subject == Field::CostCenter)
                .map(|field| field.level)
        };
        let answer = org.check(&org.admin, bea.id, Some(projects.web)).await?;
        assert_eq!(level(&answer), Some(FieldLevel::Hidden));
        // A second role never lowers a level: the highest counts.
        org.give(&bea, member.id, projects.acme).await?;
        let answer = org.check(&org.admin, bea.id, Some(projects.web)).await?;
        assert_eq!(level(&answer), Some(FieldLevel::CanSee));
        // A project sets its own level for a role; its sub-projects inherit it.
        set(
            member.id,
            Some(projects.web),
            Field::CostCenter,
            Some(FieldLevel::Hidden),
        )
        .await?;
        set(
            viewer.id,
            Some(projects.web),
            Field::CostCenter,
            Some(FieldLevel::Hidden),
        )
        .await?;
        for place in [projects.web, projects.api] {
            let answer = org.check(&org.admin, bea.id, Some(place)).await?;
            assert_eq!(level(&answer), Some(FieldLevel::Hidden), "{place}");
        }
        let answer = org.check(&org.admin, bea.id, Some(projects.acme)).await?;
        assert_eq!(
            level(&answer),
            Some(FieldLevel::CanSee),
            "ACME keeps the role's own level"
        );
        // What was set, as the screens read it.
        let levels = org
            .rig
            .api()
            .role_levels(
                Rig::info(),
                LevelsQuery {
                    session_token: org.admin.session.clone(),
                    role: member.id,
                    project: Some(projects.web),
                },
            )
            .await?
            .into_value();
        assert_eq!(levels.fields.len(), 1);
        assert_eq!(levels.fields[0].level, FieldLevel::Hidden);
        // Taking the level of the project away gives the role's own level back.
        set(member.id, Some(projects.web), Field::CostCenter, None).await?;
        set(viewer.id, Some(projects.web), Field::CostCenter, None).await?;
        let answer = org.check(&org.admin, bea.id, Some(projects.api)).await?;
        assert_eq!(level(&answer), Some(FieldLevel::CanSee));
        // A role's own level cannot be taken away, only changed.
        assert_eq!(
            refused(set(member.id, None, Field::CostCenter, None).await),
            Some(ErrorCode::InvalidRequest)
        );
        // The place a level comes from is in the reasons, with the project named only when visible.
        let answer = org.check(&org.admin, bea.id, Some(projects.api)).await?;
        let own = answer
            .fields
            .iter()
            .find(|field| field.subject == Field::CostCenter)
            .ok_or("the field is there")?;
        assert!(own.roles.iter().all(|says| says.set == LevelPlace::RoleOwn));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_04_nobody_sets_a_level_above_their_own() -> TestResult {
    each_database("sec_04_levels_no_escalation", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let projects = tree(&org).await?;
        let (mia, bea) = (org.person().await?, org.person().await?);
        let decider = org
            .role(
                "Decider",
                &[Permission::SeeProjects, Permission::DecideWhatRolesSee],
            )
            .await?;
        let target = org.role("Looker", &VIEW).await?;
        org.give(&mia, decider.id, projects.acme).await?;
        let set = |level| {
            let (rig, session, role) = (org.rig, mia.session.clone(), target.id);
            async move {
                rig.api()
                    .set_field_level(
                        Rig::info(),
                        SetFieldLevel {
                            session_token: session,
                            role,
                            project: Some(projects.acme),
                            field: Field::Title,
                            level: Some(level),
                        },
                    )
                    .await
            }
        };
        // Mia sees no field herself (her role has no levels), so she cannot open one for others.
        assert_eq!(
            refused(set(FieldLevel::CanSee).await),
            Some(ErrorCode::TooMuch)
        );
        set(FieldLevel::Hidden).await?;
        // And she cannot decide for the whole organization: that needs a row for it.
        let for_the_organization = org
            .rig
            .api()
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: mia.session.clone(),
                    role: target.id,
                    project: None,
                    field: Field::Title,
                    level: Some(FieldLevel::Hidden),
                },
            )
            .await;
        assert_eq!(refused(for_the_organization), Some(ErrorCode::Forbidden));
        let _ = bea;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_12_people_in_the_management_screens_are_shown_by_name_to_those_who_manage()
-> TestResult {
    each_database("priv_12_management_names", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let team = org.group("Team").await?;
        let bea = org.person().await?;
        org.add_to_group(team, bea.id).await?;
        let members = org
            .rig
            .api()
            .group_members(
                Rig::info(),
                GroupMembersQuery {
                    session_token: org.admin.session.clone(),
                    group: team,
                    cursor: None,
                    limit: bb_core::paging::PAGE_SIZE_MAX,
                },
            )
            .await?
            .into_value();
        assert_eq!(members.members.len(), 1);
        assert!(matches!(
            &members.members[0].person,
            bb_app::access::PersonRef::Name { id, name } if *id == bea.id && name == "Bob Builder"
        ));
        // Somebody who does not manage groups does not see the list at all.
        let result = org
            .rig
            .api()
            .group_members(
                Rig::info(),
                GroupMembersQuery {
                    session_token: bea.session.clone(),
                    group: team,
                    cursor: None,
                    limit: bb_core::paging::PAGE_SIZE_MAX,
                },
            )
            .await;
        assert_eq!(refused(result), Some(ErrorCode::Forbidden));
        // The roles list is for those who manage roles (or decide who has which role on a project).
        let roles = org
            .rig
            .api()
            .roles(
                Rig::info(),
                RolesQuery {
                    session_token: bea.session.clone(),
                    place: None,
                    cursor: None,
                    limit: bb_core::paging::PAGE_SIZE_MAX,
                    giveable: false,
                },
            )
            .await;
        assert_eq!(refused(roles), Some(ErrorCode::Forbidden));
        Ok(())
    })
    .await
}
