//! A hidden project answers not found like a missing one (PRJ-01).

use super::*;

#[tokio::test]
async fn prj_01_a_hidden_project_answers_not_found_like_a_missing_one() -> TestResult {
    each_database("prj_01_hidden_not_found", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let hidden = create(&org, &admin, None, "HIDDEN").await?;
        let inner = create(&org, &admin, Some(hidden.id), "INNER").await?;
        let bob = org.person().await?;
        let viewer = org.role("Looker", &VIEW).await?;
        let shown = create(&org, &admin, None, "SHOWN").await?;
        org.give(&bob, viewer.id, shown.id).await?;

        let not_found = Some(AppError::Refused(ErrorCode::NotFound));
        // By short name, by ID, and for what is inside a hidden project: the same as for a
        // project that does not exist.
        for address in [
            "HIDDEN",
            "INNER",
            &hidden.id.to_string(),
            &inner.id.to_string(),
        ] {
            assert_eq!(asked(&org, &bob, address).await, not_found, "{address}");
        }
        for address in [
            "MISSING",
            &missing_id().to_string(),
            "not an address",
            "hidden",
        ] {
            assert_eq!(asked(&org, &bob, address).await, not_found, "{address}");
        }
        // Changing, previewing, moving and looking inside answer the same.
        for project in [&hidden, &inner] {
            assert_eq!(
                refused(change(&org, &bob, project, Some("X"), None).await),
                Some(ErrorCode::NotFound)
            );
            assert_eq!(
                refused(preview(&org, &bob, project.id, None).await),
                Some(ErrorCode::NotFound)
            );
            assert_eq!(
                refused(relocate(&org, &bob, project.id, None, "x").await),
                Some(ErrorCode::NotFound)
            );
            assert_eq!(
                refused(list(&org, &bob, Some(project.id), None).await),
                Some(ErrorCode::NotFound)
            );
        }
        assert_eq!(
            refused(list(&org, &bob, Some(missing_id()), None).await),
            Some(ErrorCode::NotFound)
        );
        // He is told about no sub-project of one he may not see, and finds none by searching.
        assert_eq!(names(&list(&org, &bob, None, None).await?), ["SHOWN"]);
        assert!(
            list(&org, &bob, None, Some("inner"))
                .await?
                .items
                .is_empty()
        );
        // Putting a project into a hidden one is as if it did not exist, too.
        assert_eq!(
            refused(preview(&org, &bob, shown.id, Some(hidden.id)).await),
            refused(preview(&org, &bob, shown.id, Some(missing_id())).await)
        );
        // The same for somebody who is not signed in as anybody known.
        assert_eq!(
            refused(
                api(&org)
                    .project(
                        Rig::info(),
                        ProjectQuery {
                            session_token: "nothing".to_owned(),
                            address: "SHOWN".to_owned()
                        },
                    )
                    .await
            ),
            Some(ErrorCode::NotSignedIn)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_a_visible_sub_project_of_a_hidden_project_does_not_name_it() -> TestResult {
    each_database("prj_01_hidden_parent", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let acme = create(&org, &admin, None, "ACME").await?;
        let web = create(&org, &admin, Some(acme.id), "WEB").await?;
        let api_project = create(&org, &admin, Some(web.id), "API").await?;
        let bea = org.person().await?;
        let viewer = org.role("Looker", &VIEW).await?;
        // Bea sees WEB and what is inside it, but not ACME above it.
        org.give(&bea, viewer.id, web.id).await?;

        // WEB is at the top of her tree; API inside it.
        let top = list(&org, &bea, None, None).await?;
        assert_eq!(names(&top), ["WEB"]);
        assert!(top.items[0].parent.is_none());
        assert!(top.items[0].has_sub_projects);
        assert_eq!(names(&list(&org, &bea, Some(web.id), None).await?), ["API"]);
        let found = detail(&org, &bea, "WEB").await?;
        assert!(found.project.parent.is_none());
        assert!(found.trail.is_empty());
        let deeper = detail(&org, &bea, "API").await?;
        assert_eq!(deeper.project.parent, Some(web.id));
        assert_eq!(
            deeper
                .trail
                .iter()
                .map(|step| step.short_name.as_str())
                .collect::<Vec<_>>(),
            ["WEB"]
        );
        // Nothing she is told names ACME: not in the JSON, not by name, short name or ID.
        let everything = serde_json::to_string(&(
            top,
            found,
            deeper,
            list(&org, &bea, None, Some("web")).await?,
        ))?;
        for hidden in ["ACME", &acme.id.to_string(), "Project ACME"] {
            assert!(!everything.contains(hidden), "{hidden} in {everything}");
        }
        // A change made by her names nothing above either.
        let changed = change(
            &org,
            &admin,
            &api_project,
            Some("Programming interface"),
            None,
        )
        .await?;
        assert_eq!(changed.parent, Some(web.id));
        // Seeing ACME and API but not WEB between them puts API at the top of her tree.
        let cleo = org.person().await?;
        org.give(&cleo, viewer.id, acme.id).await?;
        let no_access = org.forbid(&cleo, web.id).await;
        assert!(no_access.is_ok());
        org.assign(
            &org.admin,
            Subject::Person(cleo.id),
            Given::Role(viewer.id),
            Some(api_project.id),
            AppliesTo::Project,
        )
        .await?;
        assert_eq!(
            names(&list(&org, &cleo, None, None).await?),
            ["API", "ACME"]
        );
        assert!(detail(&org, &cleo, "API").await?.trail.is_empty());
        Ok(())
    })
    .await
}
