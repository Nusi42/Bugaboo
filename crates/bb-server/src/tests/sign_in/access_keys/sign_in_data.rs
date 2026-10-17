//! A key never reaches the sign-in data of its owner (AUTH-07).

use super::*;

/// Every use case of the sign-in data (password, second step, sessions, keys) turns the key away.
async fn the_sign_in_data_turns_the_key_away(rig: &Rig, key: &str) -> Run<()> {
    let api = rig.api();
    let not_allowed = Some(ErrorCode::AccessKeyNotAllowed);
    let info = Rig::info;
    let token = || key.to_owned();
    // The password: not changed, not even with the right one.
    let changed = api
        .change_password(
            info(),
            ChangePassword {
                session_token: token(),
                current_password: PASSWORD.to_owned(),
                new_password: "Another-Long-Password-7".to_owned(),
            },
        )
        .await;
    assert_eq!(refused(changed), not_allowed);
    // The second step, sessions and the keys themselves.
    let started = api
        .start_code_app(
            info(),
            StartCodeApp {
                session_token: token(),
                password: PASSWORD.to_owned(),
            },
        )
        .await;
    assert_eq!(refused(started), not_allowed);
    let status = api
        .two_step(
            info(),
            TwoStepQuery {
                session_token: token(),
            },
        )
        .await;
    assert_eq!(refused(status), not_allowed);
    let listed = api
        .sessions(
            info(),
            bb_app::auth::SessionsQuery {
                session_token: token(),
            },
        )
        .await;
    assert_eq!(refused(listed), not_allowed);
    let ended = api
        .sign_out_everywhere(
            info(),
            bb_app::auth::SignOutEverywhere {
                session_token: token(),
            },
        )
        .await;
    assert_eq!(refused(ended), not_allowed);
    let out = api
        .sign_out(
            info(),
            bb_app::auth::SignOut {
                session_token: token(),
            },
        )
        .await;
    assert_eq!(refused(out), not_allowed);
    let who = api
        .authenticate(
            info(),
            bb_app::auth::AuthenticateQuery {
                session_token: token(),
            },
        )
        .await;
    assert_eq!(refused(who), not_allowed);
    let create_another = api
        .create_access_key(
            info(),
            CreateAccessKey {
                session_token: token(),
                password: PASSWORD.to_owned(),
                name: "Another".to_owned(),
                choice: KeyChoice::Everything,
                permissions: vec![],
                projects: vec![],
                expires_at: None,
            },
        )
        .await;
    assert_eq!(refused(create_another), not_allowed);
    let own = api
        .access_keys(
            info(),
            AccessKeysQuery {
                session_token: token(),
                cursor: None,
                limit: 10,
            },
        )
        .await;
    assert_eq!(refused(own), not_allowed);
    Ok(())
}

#[tokio::test]
async fn auth_07_a_key_cannot_change_the_owners_sign_in_data() -> TestResult {
    each_database("auth_07_sign_in_data", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let key = create(&rig, &bob, KeyChoice::Everything, &[], &[])
            .await?
            .secret;
        the_sign_in_data_turns_the_key_away(&rig, &key).await?;
        // Nothing changed: the password is the old one, there is still one key, the session
        // still works, and the person has no second step set up.
        rig.sign_in("person1@example.test", PASSWORD).await?;
        assert_eq!(keys_of(&rig, &bob).await?.len(), 1);
        assert!(rig.permissions(&bob.session).await.is_ok());
        let status = rig
            .api()
            .two_step(
                Rig::info(),
                TwoStepQuery {
                    session_token: bob.session.clone(),
                },
            )
            .await?;
        assert!(!status.code_app && status.passkeys.is_empty());
        Ok(())
    })
    .await
}
