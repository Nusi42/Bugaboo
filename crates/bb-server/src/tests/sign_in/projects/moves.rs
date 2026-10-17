//! Moving a project: the preview, the rights in both parents and the no access it must not lift (PRJ-03).

use super::*;

#[tokio::test]
async fn prj_03_the_move_preview_names_who_gains_and_loses_access() -> TestResult {
    each_database("prj_03_preview", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = create(&org, &admin, None, "ACME").await?;
        let other = create(&org, &admin, None, "OTHER").await?;
        let web = create(&org, &admin, Some(other.id), "WEB").await?;
        create(&org, &admin, Some(web.id), "API").await?;
        let viewer = org.role("Looker", &VIEW).await?;
        let bob = org.person().await?;
        let cleo = org.person().await?;
        org.give(&bob, viewer.id, acme.id).await?;
        org.give(&cleo, viewer.id, other.id).await?;

        // WEB, with API inside it, moves from OTHER into ACME.
        let shown = preview(&org, &admin, web.id, Some(acme.id)).await?;
        assert!(!shown.blocked && shown.lifts.is_empty() && !shown.truncated);
        assert_eq!(
            shown.to.as_ref().map(|to| to.short_name.as_str()),
            Some("ACME")
        );
        // Two people change, by name, because the administrator manages people.
        assert_eq!(shown.people.len(), 2);
        let line = |who: &Member| {
            shown
                .people
                .iter()
                .find(
                    |change| matches!(&change.person, PersonRef::Name { id, .. } if *id == who.id),
                )
                .ok_or("a person is not in the preview")
        };
        let gains = line(&bob)?;
        let mut at: Vec<&str> = gains
            .projects
            .iter()
            .map(|change| change.project.short_name.as_str())
            .collect();
        at.sort_unstable();
        assert_eq!(at, ["API", "WEB"]);
        assert!(
            gains
                .projects
                .iter()
                .all(|change| !change.now.sees && change.after.sees)
        );
        assert!(
            gains
                .projects
                .iter()
                .all(|change| change.after.roles == ["Looker"])
        );
        assert!(
            gains
                .projects
                .iter()
                .all(|change| change.gains.contains(&Permission::SeeProjects))
        );
        let loses = line(&cleo)?;
        assert_eq!(loses.projects.len(), 2);
        assert!(
            loses
                .projects
                .iter()
                .all(|change| change.now.sees && !change.after.sees)
        );
        assert!(
            loses
                .projects
                .iter()
                .all(|change| change.now.roles == ["Looker"] && change.after.roles.is_empty())
        );
        // Moving with the version of the preview does it; the project is inside ACME after.
        let moved = relocate(&org, &admin, web.id, Some(acme.id), &shown.preview).await?;
        assert_eq!(moved.parent, Some(acme.id));
        assert_eq!(moved.version, web.version + 1);
        assert_eq!(detail(&org, &bob, "API").await?.trail.len(), 2);
        assert!(
            asked(&org, &cleo, "API").await.is_some(),
            "Cleo no longer sees it"
        );
        // One event and one entry of the log, for the move.
        let logged = rig
            .database
            .security_log_entries(0, 10_000)
            .await?
            .iter()
            .filter(|entry| entry.action.as_str() == SecurityAction::ProjectMoved.code())
            .count();
        assert_eq!(logged, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_a_project_cannot_move_below_itself() -> TestResult {
    each_database("prj_03_below_itself", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = create(&org, &admin, None, "ACME").await?;
        let web = create(&org, &admin, Some(acme.id), "WEB").await?;
        let api_project = create(&org, &admin, Some(web.id), "API").await?;
        for to in [acme.id, web.id, api_project.id] {
            assert_eq!(
                refused(preview(&org, &admin, acme.id, Some(to)).await),
                Some(ErrorCode::MoveIntoItself)
            );
            assert_eq!(
                refused(relocate(&org, &admin, acme.id, Some(to), "x").await),
                Some(ErrorCode::MoveIntoItself)
            );
        }
        // Where it is already is no move.
        assert_eq!(
            refused(preview(&org, &admin, web.id, Some(acme.id)).await),
            Some(ErrorCode::InvalidRequest)
        );
        // The projects above are where it may go; API can go to the top.
        let shown = preview(&org, &admin, api_project.id, None).await?;
        relocate(&org, &admin, api_project.id, None, &shown.preview).await?;
        assert!(detail(&org, &admin, "API").await?.trail.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_moving_needs_the_right_in_the_old_and_the_new_parent() -> TestResult {
    each_database("prj_03_both_parents", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = create(&org, &admin, None, "ACME").await?;
        let other = create(&org, &admin, None, "OTHER").await?;
        let web = create(&org, &admin, Some(acme.id), "WEB").await?;
        let lead = org.role("Section lead", &LEAD).await?;
        let viewer = org.role("Looker", &VIEW).await?;
        let bob = org.person().await?;
        // Bob leads ACME and only looks at OTHER.
        org.give(&bob, lead.id, acme.id).await?;
        org.give(&bob, viewer.id, other.id).await?;
        // Out of ACME, into OTHER: he may move in ACME, not in OTHER.
        assert_eq!(
            refused(preview(&org, &bob, web.id, Some(other.id)).await),
            Some(ErrorCode::Forbidden)
        );
        // To the top level counts as the organization, where he holds nothing.
        assert_eq!(
            refused(preview(&org, &bob, web.id, None).await),
            Some(ErrorCode::Forbidden)
        );
        // Now he leads OTHER too: both parents are his, and the move can be looked at and done.
        org.give(&bob, lead.id, other.id).await?;
        let shown = preview(&org, &bob, web.id, Some(other.id)).await?;
        relocate(&org, &bob, web.id, Some(other.id), &shown.preview).await?;
        assert_eq!(
            detail(&org, &bob, "WEB").await?.project.parent,
            Some(other.id)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_a_move_after_the_tree_changed_asks_again() -> TestResult {
    each_database("prj_03_stale_preview", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = create(&org, &admin, None, "ACME").await?;
        let web = create(&org, &admin, None, "WEB").await?;
        let viewer = org.role("Looker", &VIEW).await?;
        let bob = org.person().await?;
        let shown = preview(&org, &admin, web.id, Some(acme.id)).await?;
        assert!(shown.people.is_empty(), "nobody has a role yet");
        // Somebody gives Bob a role on ACME after the preview was made: the move would now change
        // what Bob sees, so the preview of the person who confirms is out of date.
        org.give(&bob, viewer.id, acme.id).await?;
        assert_eq!(
            refused(relocate(&org, &admin, web.id, Some(acme.id), &shown.preview).await),
            Some(ErrorCode::MovePreviewChanged)
        );
        // Nothing moved; looking again and confirming works.
        assert!(detail(&org, &admin, "WEB").await?.trail.is_empty());
        let again = preview(&org, &admin, web.id, Some(acme.id)).await?;
        assert_eq!(again.people.len(), 1);
        assert_ne!(again.preview, shown.preview);
        relocate(&org, &admin, web.id, Some(acme.id), &again.preview).await?;
        // The same goes for a rename of one of the two parents, and for an unknown version.
        let other = create(&org, &admin, None, "OTHER").await?;
        let third = preview(&org, &admin, web.id, Some(other.id)).await?;
        change(&org, &admin, &other, Some("Renamed"), None).await?;
        assert_eq!(
            refused(relocate(&org, &admin, web.id, Some(other.id), &third.preview).await),
            Some(ErrorCode::MovePreviewChanged)
        );
        assert_eq!(
            refused(relocate(&org, &admin, web.id, Some(other.id), "made up").await),
            Some(ErrorCode::MovePreviewChanged)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_03_a_move_cannot_let_somebody_lift_a_no_access_they_are_subject_to() -> TestResult {
    each_database("prj_03_lift", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = create(&org, &admin, None, "ACME").await?;
        let x = create(&org, &admin, None, "XRAY").await?;
        let inside = create(&org, &admin, Some(x.id), "XIN").await?;
        let lead = org.role("Section lead", &LEAD).await?;
        let bob = org.person().await?;
        // Bob leads ACME with its sub-projects, and has a No access on X.
        org.give(&bob, lead.id, acme.id).await?;
        let no_access = org.forbid(&bob, x.id).await?;
        assert!(!org.can(&bob, Permission::DecideRoles, Some(x.id)).await?);

        // The preview says who it would be, in words, and that the move is refused.
        let shown = preview(&org, &admin, x.id, Some(acme.id)).await?;
        assert!(shown.blocked);
        assert_eq!(shown.lifts.len(), 1);
        assert!(
            matches!(&shown.lifts[0].person, PersonRef::Name { name, .. } if name == "Bob Builder")
        );
        assert_eq!(shown.lifts[0].project.short_name.as_str(), "XRAY");
        // The move is refused, with the right preview and with a made up one.
        for version in [shown.preview.as_str(), "made up"] {
            assert_eq!(
                refused(relocate(&org, &admin, x.id, Some(acme.id), version).await),
                Some(ErrorCode::MoveLiftsNoAccess),
                "{version}"
            );
        }
        // Nothing moved, and Bob still cannot take the No access away or see X.
        assert!(detail(&org, &admin, "XRAY").await?.trail.is_empty());
        assert!(!org.can(&bob, Permission::DecideRoles, Some(x.id)).await?);
        let lifted = api(&org)
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: bob.session.clone(),
                    assignment: no_access.id,
                },
            )
            .await;
        assert!(matches!(
            refused(lifted),
            Some(ErrorCode::NotFound | ErrorCode::Forbidden)
        ));
        assert!(asked(&org, &bob, "XRAY").await.is_some());
        assert!(asked(&org, &bob, "XIN").await.is_some());

        // The same for a No access inside what moves, deeper down.
        let deep = org.person().await?;
        org.give(&deep, lead.id, acme.id).await?;
        org.forbid(&deep, inside.id).await?;
        let shown = preview(&org, &admin, x.id, Some(acme.id)).await?;
        assert_eq!(shown.lifts.len(), 2);
        assert_eq!(
            refused(relocate(&org, &admin, x.id, Some(acme.id), &shown.preview).await),
            Some(ErrorCode::MoveLiftsNoAccess)
        );

        // Take both No access rows away first: then the move is fine.
        for project in [x.id, inside.id] {
            take_no_access_away(&org, project).await?;
        }
        let shown = preview(&org, &admin, x.id, Some(acme.id)).await?;
        assert!(!shown.blocked);
        relocate(&org, &admin, x.id, Some(acme.id), &shown.preview).await?;
        Ok(())
    })
    .await
}

/// The administrator takes away every *No access* that is on the project itself.
async fn take_no_access_away(org: &Org<'_>, project: ProjectId) -> TestResult {
    let rows = api(org)
        .assignments(
            Rig::info(),
            AssignmentsQuery {
                session_token: org.admin.session.clone(),
                project: Some(project),
                cursor: None,
                limit: bb_core::paging::PAGE_SIZE_MAX,
            },
        )
        .await?
        .into_value()
        .rows;
    for row in rows
        .iter()
        .filter(|row| row.role.is_none() && !row.inherited)
    {
        api(org)
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: org.admin.session.clone(),
                    assignment: row.id,
                },
            )
            .await?;
    }
    Ok(())
}

#[tokio::test]
async fn prj_03_a_move_out_from_under_a_no_access_is_named_in_the_preview() -> TestResult {
    each_database("prj_03_out_from_under", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = create(&org, &admin, None, "ACME").await?;
        let web = create(&org, &admin, Some(acme.id), "WEB").await?;
        let viewer = org.role("Looker", &VIEW).await?;
        let bob = org.person().await?;
        org.give_everywhere(&bob, viewer.id).await?;
        org.forbid(&bob, acme.id).await?;
        assert!(asked(&org, &bob, "WEB").await.is_some());
        let shown = preview(&org, &admin, web.id, None).await?;
        let change = &shown.people[0].projects[0];
        assert!(change.now.held_back && !change.after.held_back);
        assert!(!change.now.sees && change.after.sees);
        relocate(&org, &admin, web.id, None, &shown.preview).await?;
        assert!(asked(&org, &bob, "WEB").await.is_none(), "Bob now sees it");
        Ok(())
    })
    .await
}
