//! Making a key: the password, the name, the choice and the days it lives (AUTH-07).

use super::*;

#[tokio::test]
async fn auth_07_making_a_key_checks_the_password_the_name_and_the_choice() -> TestResult {
    each_database("auth_07_create_rules", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let web = org.project(None, "WEB").await?;
        let invalid = Some(ErrorCode::InvalidRequest);
        // A wrong password: nothing is made.
        let wrong = rig
            .api()
            .create_access_key(
                Rig::info(),
                CreateAccessKey {
                    session_token: bob.session.clone(),
                    password: "Wrong-Password-1".to_owned(),
                    name: "Report".to_owned(),
                    choice: KeyChoice::SeeOnly,
                    permissions: vec![],
                    projects: vec![],
                    expires_at: None,
                },
            )
            .await;
        assert_eq!(refused(wrong), Some(ErrorCode::WrongCredentials));
        // A name that is empty, or too long.
        for name in ["", "   ", &"x".repeat(101)] {
            let result = rig
                .api()
                .create_access_key(
                    Rig::info(),
                    CreateAccessKey {
                        session_token: bob.session.clone(),
                        password: PASSWORD.to_owned(),
                        name: name.to_owned(),
                        choice: KeyChoice::SeeOnly,
                        permissions: vec![],
                        projects: vec![],
                        expires_at: None,
                    },
                )
                .await;
            assert_eq!(refused(result), invalid, "{name:?}");
        }
        // Single permissions only with that choice, and at least one then.
        let none = create(&rig, &bob, KeyChoice::Permissions, &[], &[]).await;
        assert_eq!(refused(none), invalid);
        let both = create(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[Permission::SeeProjects],
            &[],
        )
        .await;
        assert_eq!(refused(both), invalid);
        // A project that does not exist, and a project that Bob may not know about.
        let missing = create(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[ProjectId::from_parts(rig.clock_now(), [1; 10])?],
        )
        .await;
        assert_eq!(refused(missing), Some(ErrorCode::NotFound));
        org.forbid(&bob, web.id).await?;
        let hidden = create(&rig, &bob, KeyChoice::SeeOnly, &[], &[web.id]).await;
        assert_eq!(refused(hidden), Some(ErrorCode::NotFound));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_a_key_lives_90_days_unless_a_date_is_picked_and_never_beyond_the_longest_time()
-> TestResult {
    each_database("auth_07_create_dates", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let invalid = Some(ErrorCode::InvalidRequest);
        // The default is 90 days; a date picked in the past, or beyond a year, is refused.
        let now = rig.clock_now();
        let made = create(&rig, &bob, KeyChoice::SeeOnly, &[], &[]).await?;
        assert_eq!(made.key.expires_at, later(now, 90 * DAY));
        let past = create_with(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[],
            Some(later(now, -DAY).to_string()),
        )
        .await;
        assert_eq!(refused(past), invalid);
        let too_late = create_with(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[],
            Some(later(now, 366 * DAY).to_string()),
        )
        .await;
        assert_eq!(refused(too_late), invalid);
        let not_a_date = create_with(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[],
            Some("tomorrow".to_owned()),
        )
        .await;
        assert_eq!(refused(not_a_date), invalid);
        let in_a_year = create_with(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[],
            Some(later(now, 365 * DAY).to_string()),
        )
        .await?;
        assert_eq!(in_a_year.key.expires_at, later(now, 365 * DAY));
        // The organization shortens the longest time: the default follows it, and a date beyond
        // it is refused.
        for (key, days) in [
            ("auth.access-key-longest-days", 30),
            ("auth.access-key-default-days", 60),
        ] {
            rig.set_organization_setting(key, days).await?;
        }
        let capped = create(&rig, &bob, KeyChoice::SeeOnly, &[], &[]).await?;
        assert_eq!(capped.key.expires_at, later(now, 30 * DAY));
        let beyond = create_with(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[],
            Some(later(now, 31 * DAY).to_string()),
        )
        .await;
        assert_eq!(refused(beyond), invalid);
        Ok(())
    })
    .await
}
