//! Projects nest, and short names are unique, redirect and are told only to those who may create (PRJ-01).

use super::*;

#[tokio::test]
async fn prj_01_projects_nest_to_any_depth_and_short_names_are_unique() -> TestResult {
    each_database("prj_01_nesting", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let mut parent = None;
        let mut made = Vec::new();
        // Twelve projects, each inside the one before.
        for number in 0..12 {
            let project = create(&org, &admin, parent, &format!("DEEP{number}")).await?;
            parent = Some(project.id);
            made.push(project);
        }
        let deepest = detail(&org, &admin, "DEEP11").await?;
        assert_eq!(deepest.trail.len(), 11);
        assert_eq!(deepest.trail[0].short_name.as_str(), "DEEP0");
        assert_eq!(deepest.project.parent, Some(made[10].id));
        // A short name is unique in the whole organization, at any depth.
        for place in [None, Some(made[3].id), Some(made[11].id)] {
            assert_eq!(
                refused(create(&org, &admin, place, "DEEP5").await),
                Some(ErrorCode::NameTaken)
            );
        }
        // The tree is walked one level at a time, and by search.
        assert_eq!(names(&list(&org, &admin, None, None).await?), ["DEEP0"]);
        let inside = list(&org, &admin, Some(made[4].id), None).await?;
        assert_eq!(names(&inside), ["DEEP5"]);
        assert!(inside.items[0].has_sub_projects);
        assert!(
            !list(&org, &admin, Some(made[11].id), None)
                .await?
                .items
                .iter()
                .any(|_| true)
        );
        // A search for nothing lists every project the person sees, wherever it sits.
        assert_eq!(list(&org, &admin, None, Some("")).await?.items.len(), 12);
        assert_eq!(
            names(&list(&org, &admin, None, Some("deep1")).await?),
            ["DEEP1", "DEEP10", "DEEP11"]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_a_short_name_breaks_no_rule_of_m2_q4_when_a_project_is_made() -> TestResult {
    each_database("prj_01_short_name_rules", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        // The server takes only capitals; the form turns typed letters into capitals.
        for bad in ["web", "W", "ABCDEFGHIJK", "2WEB", "WE B", "WEB-1", "ÄB", ""] {
            assert_eq!(
                refused(create(&org, &admin, None, bad).await),
                Some(ErrorCode::InvalidRequest),
                "{bad:?}"
            );
        }
        for good in ["WE", "APP2", "ABCDEFGHIJ"] {
            create(&org, &admin, None, good).await?;
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_an_old_short_name_redirects() -> TestResult {
    each_database("prj_01_old_short_name", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let web = create(&org, &admin, None, "WEB").await?;
        let renamed = change(&org, &admin, &web, None, Some("SITE")).await?;
        assert_eq!(renamed.short_name.as_str(), "SITE");
        assert_eq!(renamed.version, 2);
        // The old address still opens the project, and the answer holds the new short name.
        let old = detail(&org, &admin, "WEB").await?;
        assert_eq!(old.project.id, web.id);
        assert_eq!(old.project.short_name.as_str(), "SITE");
        assert_eq!(detail(&org, &admin, "SITE").await?.project.id, web.id);
        assert_eq!(
            detail(&org, &admin, &web.id.to_string()).await?.project.id,
            web.id
        );
        // Nobody else can take the old short name; the project can.
        assert_eq!(
            refused(create(&org, &admin, None, "WEB").await),
            Some(ErrorCode::NameTaken)
        );
        let other = create(&org, &admin, None, "OTHER").await?;
        assert_eq!(
            refused(change(&org, &admin, &other, None, Some("WEB")).await),
            Some(ErrorCode::NameTaken)
        );
        let back = change(&org, &admin, &renamed, None, Some("WEB")).await?;
        assert_eq!(back.short_name.as_str(), "WEB");
        // Now SITE is the old one, and stays with the project.
        assert_eq!(detail(&org, &admin, "SITE").await?.project.id, web.id);
        assert_eq!(
            refused(create(&org, &admin, None, "SITE").await),
            Some(ErrorCode::NameTaken)
        );
        // A change made on an old version is stopped, a name alone leaves the short name be.
        assert_eq!(
            refused(change(&org, &admin, &web, Some("Stale"), None).await),
            Some(ErrorCode::VersionChanged)
        );
        let named = change(&org, &admin, &back, Some("The website"), None).await?;
        assert_eq!(
            (named.name.as_str(), named.short_name.as_str()),
            ("The website", "WEB")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_01_a_taken_short_name_is_told_only_to_people_who_may_create_projects() -> TestResult {
    each_database("prj_01_taken_short_name", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let secret = create(&org, &admin, None, "SECRET").await?;
        let renamed = change(&org, &admin, &secret, None, Some("OPEN")).await?;
        assert_eq!(renamed.short_name.as_str(), "OPEN");
        let ours = create(&org, &admin, None, "OURS").await?;

        // Maker may create projects at the top level and sees none of them.
        let maker_role = org.role("Maker", &[Permission::CreateProjects]).await?;
        let maker = org.person().await?;
        org.give_everywhere(&maker, maker_role.id).await?;
        // The same answer for a hidden project's short name, for its old one, and for a short
        // name that a project she cannot see once had; a free one works.
        let current = refused(create(&org, &maker, None, "OPEN").await);
        let old = refused(create(&org, &maker, None, "SECRET").await);
        assert_eq!(current, Some(ErrorCode::NameTaken));
        assert_eq!(old, current);
        assert_eq!(
            asked(&org, &maker, "OPEN").await,
            Some(AppError::Refused(ErrorCode::NotFound))
        );
        create(&org, &maker, None, "FREE").await?;

        // Cleo leads OURS: she may change its settings, but may make no project where it sits,
        // so she is told nothing about short names of others, taken or not, and never refused
        // differently for a taken one.
        let lead_role = org.role("Section lead", &LEAD).await?;
        let cleo = org.person().await?;
        org.give(&cleo, lead_role.id, ours.id).await?;
        let taken = refused(change(&org, &cleo, &ours, None, Some("OPEN")).await);
        let free = refused(change(&org, &cleo, &ours, None, Some("NOBODYS")).await);
        assert_eq!(taken, Some(ErrorCode::Forbidden));
        assert_eq!(free, taken);
        // Everything else about the project she may change.
        let named = change(&org, &cleo, &ours, Some("Ours, renamed"), None).await?;
        assert_eq!(named.name, "Ours, renamed");
        assert!(!detail(&org, &cleo, "OURS").await?.may_change_short_name);
        assert!(detail(&org, &admin, "OURS").await?.may_change_short_name);
        Ok(())
    })
    .await
}
