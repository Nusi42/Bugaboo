//! Nobody raises a level above their own (SEC-04).

use super::*;

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
                    issue_type: None,
                    field: field.into(),
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
async fn sec_04_nobody_raises_a_level_by_taking_an_override_away() -> TestResult {
    each_database("sec_04_remove_override", async |db| {
        let rig = Rig::start(db).await?;
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
            issue_type: None,
            field: Field::Title.into(),
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
async fn sec_04_nobody_sets_a_people_level_above_their_own() -> TestResult {
    each_database("sec_04_people_level_too_much", async |db| {
        let rig = Rig::start(db).await?;
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
                    issue_type: None,
                    field: (*field).into(),
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
async fn sec_04_nobody_resets_a_role_to_levels_above_their_own() -> TestResult {
    // Once for each half of the rule: the field levels and the levels of people information.
    for (fields_first, name) in [
        (true, "sec_04_reset_fields_first"),
        (false, "sec_04_reset_kinds_first"),
    ] {
        each_database(name, async |db| {
            let rig = Rig::start(db).await?;
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
                        cursor: None,
                        limit: bb_core::paging::PAGE_SIZE_MAX,
                        giveable: false,
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
                        version: viewer.version,
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
