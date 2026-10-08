//! Passkeys (AUTH-02): adding and removing them, as the second step after the password, and
//! signing in with a passkey alone. The device is a soft authenticator that runs the real
//! checks without a browser.

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_app::auth::SecondStepStore as _;
use bb_app::auth::{
    AddPasskey, AuthenticateQuery, PasskeyRegistrationQuery, PasskeySecondStep,
    PasskeySecondStepQuery, PasskeySignIn, PasskeySignInQuery, RemovePasskey, StatusQuery,
    TwoStepQuery,
};
use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_core::sign_in::SecurityAction;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::Value;
use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator as _};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;

use super::http::ADA_EMAIL;
use super::rig::{PASSWORD, PUBLIC_URL, Rig, refused, refused_boxed};

fn origin() -> Result<Url, Failure> {
    Ok(Url::parse(PUBLIC_URL)?)
}

/// Adds a passkey on `device` for the person behind `session`.
pub(super) async fn add(
    rig: &Rig,
    session: &str,
    device: &mut SoftPasskey,
    label: &str,
) -> Result<bb_app::auth::PasskeyView, Failure> {
    let challenge = rig
        .api()
        .passkey_registration_options(
            Rig::info(),
            PasskeyRegistrationQuery {
                session_token: session.to_owned(),
                password: PASSWORD.to_owned(),
            },
        )
        .await?;
    let answer = device.do_registration(origin()?, serde_json::from_value(challenge.options)?)?;
    Ok(rig
        .api()
        .add_passkey(
            Rig::info(),
            AddPasskey {
                session_token: session.to_owned(),
                challenge_id: challenge.challenge_id,
                credential: serde_json::to_value(answer)?,
                label: label.to_owned(),
                password: PASSWORD.to_owned(),
            },
        )
        .await?)
}

async fn action_count(rig: &Rig, action: SecurityAction) -> Result<usize, bb_store::StoreError> {
    Ok(rig
        .database
        .security_log_entries(0, 1000)
        .await?
        .iter()
        .filter(|entry| entry.action.as_str() == action.code())
        .count())
}

/// The request with the passkey of the device named in it. A soft device only answers for a
/// passkey it holds, so a test that wants an answer from the wrong passkey has to ask for it.
fn naming(mut options: Value, credential_id: &[u8]) -> Value {
    options["publicKey"]["allowCredentials"] = serde_json::json!([{
        "type": "public-key",
        "id": Base64UrlUnpadded::encode_string(credential_id),
    }]);
    options
}

/// What a phone does that holds a discoverable passkey: it names the passkey in the request and
/// puts the person into the answer.
fn answer_like_a_phone(
    device: &mut SoftPasskey,
    options: Value,
    credential_id: &[u8],
    person: UserId,
) -> Result<Value, Failure> {
    let options = naming(options, credential_id);
    let answer = device.do_authentication(origin()?, serde_json::from_value(options)?)?;
    let mut answer = serde_json::to_value(answer)?;
    answer["response"]["userHandle"] =
        Value::String(Base64UrlUnpadded::encode_string(&person.to_bytes()));
    Ok(answer)
}

/// Bob adds a passkey on a device of his own, and the device with its passkey ID is returned: a
/// real passkey that is not Ada's.
async fn bobs_device(rig: &Rig, admin_session: &str) -> Result<(SoftPasskey, Vec<u8>), Failure> {
    let bob = rig.person(admin_session, "bob@example.test").await?;
    let mut device = SoftPasskey::new(true);
    add(rig, &bob.session_token, &mut device, "Phone").await?;
    let id = rig
        .storage
        .passkeys_of(bob.person.id)
        .await?
        .first()
        .ok_or("no passkey")?
        .credential_id
        .clone();
    Ok((device, id))
}

#[tokio::test]
async fn auth_02_a_passkey_is_added_listed_and_removed_with_the_password() -> TestResult {
    each_database("auth_02_manage_passkeys", async |db| {
        let rig = Rig::start(db).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let session = admin.session_token.as_str();
        let mut laptop = SoftPasskey::new(true);
        let added = add(&rig, session, &mut laptop, "  Work laptop ").await?;
        assert_eq!(added.label, "Work laptop");
        let listed = rig
            .api()
            .two_step(
                Rig::info(),
                TwoStepQuery {
                    session_token: session.to_owned(),
                },
            )
            .await?;
        assert_eq!(listed.passkeys.len(), 1);
        assert_eq!(listed.passkeys[0].id, added.id);
        assert!(!listed.code_app);

        // A name that is empty or too long is refused.
        for label in ["", "   ", &"x".repeat(65)] {
            let result = add(&rig, session, &mut SoftPasskey::new(true), label).await;
            assert_eq!(
                refused_boxed(result),
                Some(ErrorCode::InvalidRequest),
                "{label:?}"
            );
        }

        let remove = |password: &str, id| {
            rig.api().remove_passkey(
                Rig::info(),
                RemovePasskey {
                    session_token: session.to_owned(),
                    passkey_id: id,
                    password: password.to_owned(),
                },
            )
        };
        assert_eq!(
            refused(remove("Wrong-Password-1", added.id).await),
            Some(ErrorCode::WrongCredentials)
        );
        // Somebody else's passkey, or one that does not exist, is not found.
        let bob = rig.person(session, "bob@example.test").await?;
        let bobs = add(
            &rig,
            &bob.session_token,
            &mut SoftPasskey::new(true),
            "Phone",
        )
        .await?;
        assert_eq!(
            refused(remove(PASSWORD, bobs.id).await),
            Some(ErrorCode::NotFound)
        );
        remove(PASSWORD, added.id).await?;
        assert_eq!(
            refused(remove(PASSWORD, added.id).await),
            Some(ErrorCode::NotFound)
        );
        assert!(rig.storage.passkeys_of(admin.person.id).await?.is_empty());
        assert_eq!(rig.storage.passkeys_of(bob.person.id).await?.len(), 1);
        assert_eq!(action_count(&rig, SecurityAction::PasskeyAdded).await?, 2);
        assert_eq!(action_count(&rig, SecurityAction::PasskeyRemoved).await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_02_a_passkey_finishes_signing_in_after_the_password() -> TestResult {
    each_database("auth_02_passkey_second_step", async |db| {
        let rig = Rig::start(db).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let mut laptop = SoftPasskey::new(true);
        add(&rig, &admin.session_token, &mut laptop, "Laptop").await?;
        let (mut stranger, strangers_id) = bobs_device(&rig, &admin.session_token).await?;

        let pending = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        let methods = pending
            .second_step_needed
            .ok_or("a second step is needed")?;
        assert!(methods.passkey && !methods.code_app && !methods.backup_code);
        let session = pending.session_token.clone();
        let options = |token: &str| {
            rig.api().passkey_second_step_options(
                Rig::info(),
                PasskeySecondStepQuery {
                    session_token: token.to_owned(),
                },
            )
        };
        let finish = |token: &str, challenge_id: &str, credential: Value| {
            rig.api().passkey_second_step(
                Rig::info(),
                PasskeySecondStep {
                    session_token: token.to_owned(),
                    challenge_id: challenge_id.to_owned(),
                    credential,
                },
            )
        };
        // Only a half finished sign-in can ask for it.
        assert_eq!(
            refused(options(&admin.session_token).await),
            Some(ErrorCode::NotSignedIn)
        );

        // Something that is not an answer, a made-up challenge, and the answer of a passkey that
        // is not Ada's are all rejected the same way. Each try uses its challenge up.
        let challenge = options(&session).await?;
        assert_eq!(
            refused(finish(&session, &challenge.challenge_id, Value::Null).await),
            Some(ErrorCode::PasskeyRejected)
        );
        let challenge = options(&session).await?;
        let answer = serde_json::to_value(laptop.do_authentication(
            origin()?,
            serde_json::from_value(challenge.options.clone())?,
        )?)?;
        assert_eq!(
            refused(finish(&session, "not a challenge", answer).await),
            Some(ErrorCode::PasskeyRejected)
        );
        let challenge = options(&session).await?;
        let forged = naming(challenge.options.clone(), &strangers_id);
        let strangers_answer = serde_json::to_value(
            stranger.do_authentication(origin()?, serde_json::from_value(forged)?)?,
        )?;
        assert_eq!(
            refused(finish(&session, &challenge.challenge_id, strangers_answer).await),
            Some(ErrorCode::PasskeyRejected)
        );

        // The right device with a fresh challenge opens a session with a new secret.
        let challenge = options(&session).await?;
        let answer = serde_json::to_value(laptop.do_authentication(
            origin()?,
            serde_json::from_value(challenge.options.clone())?,
        )?)?;
        let opened = finish(&session, &challenge.challenge_id, answer.clone()).await?;
        assert_ne!(opened.session_token, session);
        assert!(opened.second_step_needed.is_none());
        rig.api()
            .authenticate(
                Rig::info(),
                AuthenticateQuery {
                    session_token: opened.session_token.clone(),
                },
            )
            .await?;
        // The pending session is gone, so the answer cannot be played again.
        assert_eq!(
            refused(finish(&session, &challenge.challenge_id, answer).await),
            Some(ErrorCode::NotSignedIn)
        );
        assert!(action_count(&rig, SecurityAction::PasskeySignInFailed).await? >= 3);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_02_a_passkey_signs_in_on_its_own() -> TestResult {
    each_database("auth_02_passkey_alone", async |db| {
        let rig = Rig::start(db).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let person = admin.person.id;
        let mut laptop = SoftPasskey::new(true);
        add(&rig, &admin.session_token, &mut laptop, "Laptop").await?;
        let credential_id = rig
            .storage
            .passkeys_of(person)
            .await?
            .first()
            .ok_or("no passkey")?
            .credential_id
            .clone();

        let sign_in = |challenge_id: &str, credential: Value| {
            rig.api().passkey_sign_in(
                Rig::info(),
                PasskeySignIn {
                    challenge_id: challenge_id.to_owned(),
                    credential,
                    previous_session: None,
                },
            )
        };
        let challenge = rig
            .api()
            .passkey_sign_in_options(Rig::info(), PasskeySignInQuery)
            .await?;
        assert_eq!(
            challenge.options["publicKey"]["allowCredentials"],
            serde_json::json!([]),
            "no address was typed, so the device is not told whose passkey to use"
        );
        let answer = answer_like_a_phone(
            &mut laptop,
            challenge.options.clone(),
            &credential_id,
            person,
        )?;
        let started = sign_in(&challenge.challenge_id, answer.clone()).await?;
        assert_eq!(started.person.id, person);
        assert!(started.second_step_needed.is_none());
        rig.api()
            .authenticate(
                Rig::info(),
                AuthenticateQuery {
                    session_token: started.session_token.clone(),
                },
            )
            .await?;
        // The same answer cannot be used twice.
        assert_eq!(
            refused(sign_in(&challenge.challenge_id, answer).await),
            Some(ErrorCode::PasskeyRejected)
        );
        // A passkey that is not hers, and things that are not answers, are rejected alike.
        let challenge = rig
            .api()
            .passkey_sign_in_options(Rig::info(), PasskeySignInQuery)
            .await?;
        // A real passkey that is not Ada's, with Ada named in the answer.
        let (mut stranger, strangers_id) = bobs_device(&rig, &admin.session_token).await?;
        let wrong = answer_like_a_phone(
            &mut stranger,
            challenge.options.clone(),
            &strangers_id,
            person,
        )?;
        for (id, credential) in [
            (challenge.challenge_id.as_str(), wrong),
            (challenge.challenge_id.as_str(), Value::Null),
            ("nonsense", serde_json::json!({})),
        ] {
            assert_eq!(
                refused(sign_in(id, credential).await),
                Some(ErrorCode::PasskeyRejected)
            );
        }
        assert_eq!(
            action_count(&rig, SecurityAction::PasskeySignInSucceeded).await?,
            1
        );
        assert!(action_count(&rig, SecurityAction::PasskeySignInFailed).await? >= 4);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_02_without_a_host_name_in_the_public_address_passkeys_are_off() -> TestResult {
    each_database("auth_02_passkeys_off", async |db| {
        let rig = Rig::start_without_passkeys(db).await?;
        let status = rig.api().status(Rig::info(), StatusQuery).await?;
        assert!(!status.passkeys_available);
        let options = rig
            .api()
            .passkey_sign_in_options(Rig::info(), PasskeySignInQuery)
            .await;
        assert_eq!(refused(options), Some(ErrorCode::PasskeyRejected));
        let admin = rig.admin(ADA_EMAIL).await?;
        let registration = rig
            .api()
            .passkey_registration_options(
                Rig::info(),
                PasskeyRegistrationQuery {
                    session_token: admin.session_token,
                    password: PASSWORD.to_owned(),
                },
            )
            .await;
        assert_eq!(refused(registration), Some(ErrorCode::PasskeyRejected));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_a_stolen_session_cannot_add_a_passkey_without_the_password() -> TestResult {
    each_database("sec_12_passkey_needs_password", async |db| {
        let rig = Rig::start(db).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let stolen = admin.session_token.clone();
        let options = |password: &str| {
            rig.api().passkey_registration_options(
                Rig::info(),
                PasskeyRegistrationQuery {
                    session_token: stolen.clone(),
                    password: password.to_owned(),
                },
            )
        };
        assert_eq!(
            refused(options("Wrong-Password-1").await),
            Some(ErrorCode::WrongCredentials)
        );
        // Even with a challenge the person asked for, the thief needs the password to add it.
        let challenge = options(PASSWORD).await?;
        let mut thief = SoftPasskey::new(true);
        let answer =
            thief.do_registration(origin()?, serde_json::from_value(challenge.options)?)?;
        let result = rig
            .api()
            .add_passkey(
                Rig::info(),
                AddPasskey {
                    session_token: stolen.clone(),
                    challenge_id: challenge.challenge_id,
                    credential: serde_json::to_value(answer)?,
                    label: "Thief".to_owned(),
                    password: "Wrong-Password-1".to_owned(),
                },
            )
            .await;
        assert_eq!(refused(result), Some(ErrorCode::WrongCredentials));
        assert!(rig.storage.passkeys_of(admin.person.id).await?.is_empty());
        Ok(())
    })
    .await
}
