//! The projects and the permissions a person chooses from for a key (AUTH-07).

use super::*;

/// The short names of the projects a person may choose for a key, a page at a time.
async fn choices(rig: &Rig, who: &Member, limit: u32) -> Run<Vec<Vec<String>>> {
    let mut pages = Vec::new();
    let mut cursor = None;
    loop {
        let page = rig
            .api()
            .project_choices(
                Rig::info(),
                ProjectChoicesQuery {
                    session_token: who.session.clone(),
                    cursor: cursor.clone(),
                    limit,
                },
            )
            .await?
            .into_value();
        pages.push(
            page.items
                .iter()
                .map(|project| project.short_name.clone())
                .collect(),
        );
        cursor = page.next_cursor;
        if cursor.is_none() {
            return Ok(pages);
        }
        assert!(pages.len() < 20, "the pages do not end");
    }
}

#[tokio::test]
async fn auth_07_a_person_chooses_the_projects_of_a_key_from_those_they_know_about() -> TestResult {
    each_database("auth_07_project_choices", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let web = org.project(None, "WEB").await?;
        org.project(Some(web.id), "API").await?;
        org.project(None, "OTHER").await?;
        org.project(None, "ZEBRA").await?;
        // Bob sees every project: by short name, in one page or in several.
        assert_eq!(
            choices(&rig, &bob, 50).await?,
            [["API", "OTHER", "WEB", "ZEBRA"]]
        );
        assert_eq!(
            choices(&rig, &bob, 3).await?,
            [vec!["API", "OTHER", "WEB"], vec!["ZEBRA"]]
        );
        // Somebody who holds a role in one project only knows that project and what is inside it.
        let role = org.role("Reader", &[Permission::SeeProjects]).await?;
        let dora = org.person().await?;
        org.give(&dora, role.id, web.id).await?;
        assert_eq!(choices(&rig, &dora, 50).await?, [["API", "WEB"]]);
        // Somebody without a role knows none, and asks for no cursor that is not one.
        let eve = org.person().await?;
        assert_eq!(choices(&rig, &eve, 50).await?, [Vec::<String>::new()]);
        let bad = rig
            .api()
            .project_choices(
                Rig::info(),
                ProjectChoicesQuery {
                    session_token: bob.session.clone(),
                    cursor: Some("not a count".to_owned()),
                    limit: 10,
                },
            )
            .await;
        assert_eq!(refused(bad), Some(ErrorCode::InvalidRequest));
        // A key cannot ask: choosing is part of making a key.
        let key = create(&rig, &bob, KeyChoice::Everything, &[], &[])
            .await?
            .secret;
        let by_key = rig
            .api()
            .project_choices(
                Rig::info(),
                ProjectChoicesQuery {
                    session_token: key,
                    cursor: None,
                    limit: 10,
                },
            )
            .await;
        assert_eq!(refused(by_key), Some(ErrorCode::AccessKeyNotAllowed));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_the_rules_of_a_key_name_its_days_and_the_permissions_to_choose_from() -> TestResult
{
    each_database("auth_07_key_rules", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let ask = || async {
            rig.api()
                .access_key_rules(
                    Rig::info(),
                    AccessKeyRulesQuery {
                        session_token: org.admin.session.clone(),
                    },
                )
                .await
                .map(bb_app::access::Authorized::into_value)
        };
        let rules = ask().await?;
        assert_eq!((rules.default_days, rules.longest_days), (90, 365));
        // Every permission of the catalogue that a key may hold, once, in its order, with its
        // topic: not inviting people or changing roles (`NOT_FOR_KEYS`).
        let keys: Vec<Permission> = rules
            .permissions
            .iter()
            .map(|(permission, _)| *permission)
            .collect();
        let may_be_held: Vec<Permission> = Permission::ALL
            .iter()
            .copied()
            .filter(|permission| permission.keys_may_hold())
            .collect();
        assert_eq!(keys, may_be_held);
        assert!(keys.len() < Permission::ALL.len());
        assert!(
            rules
                .permissions
                .iter()
                .all(|(permission, topic)| permission.topic() == *topic)
        );
        // The default is never more than the longest time.
        rig.set_organization_setting("auth.access-key-longest-days", 30)
            .await?;
        let rules = ask().await?;
        assert_eq!((rules.default_days, rules.longest_days), (30, 30));
        Ok(())
    })
    .await
}
