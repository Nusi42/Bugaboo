//! The scenarios of signing in, the second step, passkeys, the code app and the session (M1-F18).

use super::*;

pub(crate) async fn sign_in_scenario(world: &World) -> Added {
    world.person("bob@example.test").await.map_err(text)?;
    measured(
        &world.store,
        sign_in(
            &world.ctx(),
            SignIn {
                email: "bob@example.test".to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        ),
    )
    .await
}

/// Ada with a code app: set up and confirmed with the first code.
async fn ada_with_a_code_app(world: &World) -> Result<(), String> {
    let token = world.admin.session_token.clone();
    start_code_app(
        &world.ctx(),
        StartCodeApp {
            session_token: token.clone(),
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    confirm_code_app(
        &world.ctx(),
        ConfirmCodeApp {
            session_token: token,
            code: code_now(world).await?,
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    Ok(())
}

/// The code of the code app of Ada at this moment.
async fn code_now(world: &World) -> Result<String, String> {
    let secret = world
        .store
        .totp_secret(world.admin.person.id)
        .await
        .map_err(text)?
        .ok_or("no secret")?;
    TotpSecret::from_bytes(&secret)
        .code_at(crate::context::Clock::now(&*world.clock))
        .map_err(text)
}

/// Ada signs in with her password and still needs the second step.
async fn ada_waits_for_the_second_step(world: &World) -> Result<Started, String> {
    let started = sign_in(
        &world.ctx(),
        SignIn {
            email: "ada@example.test".to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await
    .map_err(text)?;
    if started.second_step_needed.is_none() {
        return Err("a second step was expected".into());
    }
    Ok(started)
}

pub(crate) async fn second_step_scenario(world: &World) -> Added {
    ada_with_a_code_app(world).await?;
    let pending = ada_waits_for_the_second_step(world).await?;
    // A code is good once: the next one is for the next 30 seconds.
    world.clock.advance(30);
    measured(
        &world.store,
        second_step(
            &world.ctx(),
            SecondStep {
                session_token: pending.session_token,
                code: code_now(world).await?,
            },
        ),
    )
    .await
}

/// A device with a passkey of Ada.
async fn ada_with_a_passkey(world: &World) -> Result<SoftPasskey, String> {
    let mut device = SoftPasskey::new(true);
    let challenge = passkey_registration_options(
        &world.ctx(),
        PasskeyRegistrationQuery {
            session_token: world.admin.session_token.clone(),
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    let answer = device
        .do_registration(
            origin()?,
            serde_json::from_value(challenge.options).map_err(text)?,
        )
        .map_err(text)?;
    add_passkey(
        &world.ctx(),
        AddPasskey {
            session_token: world.admin.session_token.clone(),
            challenge_id: challenge.challenge_id,
            credential: serde_json::to_value(answer).map_err(text)?,
            label: "Phone".to_owned(),
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    Ok(device)
}

pub(crate) async fn add_passkey_scenario(world: &World) -> Added {
    let mut device = SoftPasskey::new(true);
    let challenge = passkey_registration_options(
        &world.ctx(),
        PasskeyRegistrationQuery {
            session_token: world.admin.session_token.clone(),
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    let answer = device
        .do_registration(
            origin()?,
            serde_json::from_value(challenge.options).map_err(text)?,
        )
        .map_err(text)?;
    measured(
        &world.store,
        add_passkey(
            &world.ctx(),
            AddPasskey {
                session_token: world.admin.session_token.clone(),
                challenge_id: challenge.challenge_id,
                credential: serde_json::to_value(answer).map_err(text)?,
                label: "Phone".to_owned(),
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn remove_passkey_scenario(world: &World) -> Added {
    ada_with_a_passkey(world).await?;
    let passkey = world
        .store
        .passkeys_of(world.admin.person.id)
        .await
        .map_err(text)?
        .first()
        .map(|passkey| passkey.id)
        .ok_or("no passkey")?;
    measured(
        &world.store,
        remove_passkey(
            &world.ctx(),
            RemovePasskey {
                session_token: world.admin.session_token.clone(),
                passkey_id: passkey,
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn passkey_second_step_scenario(world: &World) -> Added {
    let mut device = ada_with_a_passkey(world).await?;
    let pending = ada_waits_for_the_second_step(world).await?;
    let challenge = passkey_second_step_options(
        &world.ctx(),
        PasskeySecondStepQuery {
            session_token: pending.session_token.clone(),
        },
    )
    .await
    .map_err(text)?;
    let answer = device
        .do_authentication(
            origin()?,
            serde_json::from_value(challenge.options).map_err(text)?,
        )
        .map_err(text)?;
    measured(
        &world.store,
        passkey_second_step(
            &world.ctx(),
            PasskeySecondStep {
                session_token: pending.session_token,
                challenge_id: challenge.challenge_id,
                credential: serde_json::to_value(answer).map_err(text)?,
            },
        ),
    )
    .await
}

pub(crate) async fn passkey_sign_in_scenario(world: &World) -> Added {
    let mut device = ada_with_a_passkey(world).await?;
    let credential = world
        .store
        .passkeys_of(world.admin.person.id)
        .await
        .map_err(text)?
        .first()
        .map(|passkey| passkey.credential_id.clone())
        .ok_or("no passkey")?;
    let challenge = passkey_sign_in_options(&world.ctx(), PasskeySignInQuery)
        .await
        .map_err(text)?;
    // What a phone does that holds a discoverable passkey: it names the passkey in the request and
    // puts the person into the answer.
    let mut options: Value = challenge.options;
    options["publicKey"]["allowCredentials"] = serde_json::json!([{
        "type": "public-key",
        "id": Base64UrlUnpadded::encode_string(&credential),
    }]);
    let answer = device
        .do_authentication(origin()?, serde_json::from_value(options).map_err(text)?)
        .map_err(text)?;
    let mut answer = serde_json::to_value(answer).map_err(text)?;
    answer["response"]["userHandle"] = Value::String(Base64UrlUnpadded::encode_string(
        &world.admin.person.id.to_bytes(),
    ));
    measured(
        &world.store,
        passkey_sign_in(
            &world.ctx(),
            PasskeySignIn {
                challenge_id: challenge.challenge_id,
                credential: answer,
                previous_session: None,
            },
        ),
    )
    .await
}

pub(crate) async fn sign_out_scenario(world: &World) -> Added {
    measured(
        &world.store,
        sign_out(
            &world.ctx(),
            SignOut {
                session_token: world.admin.session_token.clone(),
            },
        ),
    )
    .await
}

pub(crate) async fn sign_out_everywhere_scenario(world: &World) -> Added {
    measured(
        &world.store,
        sign_out_everywhere(
            &world.ctx(),
            SignOutEverywhere {
                session_token: world.admin.session_token.clone(),
            },
        ),
    )
    .await
}

pub(crate) async fn end_session_scenario(world: &World) -> Added {
    // A second place where Ada is signed in, ended from the first.
    let second = sign_in(
        &world.ctx(),
        SignIn {
            email: "ada@example.test".to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await
    .map_err(text)?;
    let listed = sessions(
        &world.ctx(),
        crate::auth::SessionsQuery {
            session_token: second.session_token,
        },
    )
    .await
    .map_err(text)?;
    let other = listed
        .sessions
        .iter()
        .find(|session| !session.current)
        .map(|session| session.id)
        .ok_or("no other session")?;
    measured(
        &world.store,
        end_session(
            &world.ctx(),
            EndSession {
                session_token: world.admin.session_token.clone(),
                session_id: other,
            },
        ),
    )
    .await
}

pub(crate) async fn change_password_scenario(world: &World) -> Added {
    measured(
        &world.store,
        change_password(
            &world.ctx(),
            ChangePassword {
                session_token: world.admin.session_token.clone(),
                current_password: PASSWORD.to_owned(),
                new_password: OTHER_PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn start_code_app_scenario(world: &World) -> Added {
    measured(
        &world.store,
        start_code_app(
            &world.ctx(),
            StartCodeApp {
                session_token: world.admin.session_token.clone(),
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn confirm_code_app_scenario(world: &World) -> Added {
    start_code_app(
        &world.ctx(),
        StartCodeApp {
            session_token: world.admin.session_token.clone(),
            password: PASSWORD.to_owned(),
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        confirm_code_app(
            &world.ctx(),
            ConfirmCodeApp {
                session_token: world.admin.session_token.clone(),
                code: code_now(world).await?,
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn remove_code_app_scenario(world: &World) -> Added {
    ada_with_a_code_app(world).await?;
    measured(
        &world.store,
        remove_code_app(
            &world.ctx(),
            RemoveCodeApp {
                session_token: world.admin.session_token.clone(),
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn renew_backup_codes_scenario(world: &World) -> Added {
    ada_with_a_code_app(world).await?;
    measured(
        &world.store,
        renew_backup_codes(
            &world.ctx(),
            RenewBackupCodes {
                session_token: world.admin.session_token.clone(),
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}
