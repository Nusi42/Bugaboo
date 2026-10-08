//! Sessions (SEC-12): signing out, the list, timeouts, new secrets, the token against forged
//! requests, and changing the password.

use bb_app::auth::{
    AppError, AuthenticateQuery, ChangePassword, EndSession, RequestInfo, SessionsQuery, SignIn,
    SignOut, SignOutEverywhere,
};
use bb_auth::Device;
use bb_core::ErrorCode;
use bb_core::network::NetworkPrefix;
use bb_core::sign_in::SecurityAction;
use bb_testing::{TestResult, each_database};

use super::http::ADA_EMAIL;
use super::rig::{OTHER_PASSWORD, PASSWORD, Rig, Tries, refused};

const FIREFOX: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:130.0) Gecko/20100101 Firefox/130.0";

async fn signed_in(rig: &Rig, token: &str) -> bool {
    rig.api()
        .authenticate(
            Rig::info(),
            AuthenticateQuery {
                session_token: token.to_owned(),
            },
        )
        .await
        .is_ok()
}

#[tokio::test]
async fn sec_12_signing_out_ends_that_session_and_only_that_one() -> TestResult {
    each_database("sec_12_sign_out", async |db| {
        let rig = Rig::start(db).await?;
        let first = rig.admin(ADA_EMAIL).await?;
        let second = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        rig.api()
            .sign_out(
                Rig::info(),
                SignOut {
                    session_token: first.session_token.clone(),
                },
            )
            .await?;
        assert!(!signed_in(&rig, &first.session_token).await);
        assert!(signed_in(&rig, &second.session_token).await);
        // A cookie that opens nothing is signed out all the same.
        for token in ["garbage", "", &first.session_token] {
            rig.api()
                .sign_out(
                    Rig::info(),
                    SignOut {
                        session_token: token.to_owned(),
                    },
                )
                .await?;
        }
        assert!(signed_in(&rig, &second.session_token).await);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_people_list_their_sessions_and_end_any_of_them() -> TestResult {
    each_database("sec_12_session_list", async |db| {
        let rig = Rig::start(db).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let bob = rig.person(&admin.session_token, "bob@example.test").await?;
        let from_firefox = RequestInfo {
            network: Some(NetworkPrefix::from_address("203.0.113.9".parse()?)),
            device: Device::from_user_agent(FIREFOX),
        };
        let ada_second = rig
            .api()
            .sign_in(
                from_firefox,
                SignIn {
                    email: ADA_EMAIL.to_owned(),
                    password: PASSWORD.to_owned(),
                    previous_session: None,
                },
            )
            .await?;
        let list = |token: &str| {
            rig.api().sessions(
                Rig::info(),
                SessionsQuery {
                    session_token: token.to_owned(),
                },
            )
        };
        let sessions = list(&ada_second.session_token).await?.sessions;
        assert_eq!(sessions.len(), 2);
        let current: Vec<_> = sessions.iter().filter(|session| session.current).collect();
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].browser.as_deref(), Some("Firefox"));
        assert_eq!(current[0].system.as_deref(), Some("Windows"));
        assert_eq!(current[0].network.as_deref(), Some("203.0.113.0"));
        let other = sessions
            .iter()
            .find(|session| !session.current)
            .ok_or("no other session")?;

        // Somebody else's session is not found, and is not touched.
        let bobs = list(&bob.session_token).await?.sessions;
        let foreign = rig
            .api()
            .end_session(
                Rig::info(),
                EndSession {
                    session_token: ada_second.session_token.clone(),
                    session_id: bobs[0].id,
                },
            )
            .await;
        assert_eq!(refused(foreign), Some(ErrorCode::NotFound));
        assert!(signed_in(&rig, &bob.session_token).await);

        // Her own other session ends; the first sign-in is signed out.
        rig.api()
            .end_session(
                Rig::info(),
                EndSession {
                    session_token: ada_second.session_token.clone(),
                    session_id: other.id,
                },
            )
            .await?;
        assert!(!signed_in(&rig, &admin.session_token).await);
        assert_eq!(list(&ada_second.session_token).await?.sessions.len(), 1);
        assert!(
            rig.database
                .security_log_entries(0, 100)
                .await?
                .iter()
                .any(|entry| entry.action.as_str() == SecurityAction::SessionEnded.code())
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_signing_out_everywhere_ends_every_session_of_the_person() -> TestResult {
    each_database("sec_12_sign_out_everywhere", async |db| {
        let rig = Rig::start(db).await?;
        let admin = rig.admin(ADA_EMAIL).await?;
        let bob = rig.person(&admin.session_token, "bob@example.test").await?;
        let second = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        rig.api()
            .sign_out_everywhere(
                Rig::info(),
                SignOutEverywhere {
                    session_token: second.session_token.clone(),
                },
            )
            .await?;
        assert!(!signed_in(&rig, &admin.session_token).await);
        assert!(!signed_in(&rig, &second.session_token).await);
        // Bob is not affected.
        assert!(signed_in(&rig, &bob.session_token).await);
        // Without a session there is nothing to end.
        let result = rig
            .api()
            .sign_out_everywhere(
                Rig::info(),
                SignOutEverywhere {
                    session_token: second.session_token,
                },
            )
            .await;
        assert_eq!(refused(result), Some(ErrorCode::NotSignedIn));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_a_session_ends_after_it_was_idle_too_long_and_use_keeps_it_alive() -> TestResult {
    each_database("sec_12_idle_timeout", async |db| {
        let rig = Rig::start(db).await?;
        let kept = rig.admin(ADA_EMAIL).await?;
        let left = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        // The default idle time is 30 minutes. Twenty minutes, used each time, is no pause.
        for _ in 0..4 {
            rig.clock.advance(20 * 60);
            assert!(signed_in(&rig, &kept.session_token).await);
        }
        // The session that was not used for more than 30 minutes is gone.
        assert!(!signed_in(&rig, &left.session_token).await);
        rig.clock.advance(31 * 60);
        assert!(!signed_in(&rig, &kept.session_token).await);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_a_session_ends_after_the_absolute_time_however_busy_the_person_is() -> TestResult {
    each_database("sec_12_absolute_timeout", async |db| {
        let rig = Rig::start(db).await?;
        let started = rig.admin(ADA_EMAIL).await?;
        // The default is 24 hours. Used every 20 minutes, the session lives until then.
        for _ in 0..71 {
            rig.clock.advance(20 * 60);
            assert!(signed_in(&rig, &started.session_token).await);
        }
        rig.clock.advance(20 * 60 + 60);
        assert!(!signed_in(&rig, &started.session_token).await);
        // Signing in again gives a new session.
        assert!(rig.sign_in(ADA_EMAIL, PASSWORD).await.is_ok());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_every_session_has_its_own_secret_and_its_own_csrf_token() -> TestResult {
    each_database("sec_12_tokens", async |db| {
        let rig = Rig::start(db).await?;
        let first = rig.admin(ADA_EMAIL).await?;
        let second = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        assert_ne!(first.session_token, second.session_token);
        assert_ne!(first.csrf_token, second.csrf_token);
        let matches = |session: &str, token: &str| rig.services.csrf_matches(session, token);
        assert!(matches(&first.session_token, &first.csrf_token));
        assert!(matches(&second.session_token, &second.csrf_token));
        // The token is bound to the session: another session's token does not fit.
        assert!(!matches(&first.session_token, &second.csrf_token));
        for bad in ["", "x", &first.session_token, &"A".repeat(43)] {
            assert!(!matches(&first.session_token, bad), "{bad}");
        }
        assert!(!matches("not a session", &first.csrf_token));
        // The secret in the cookie is 256 bits (43 characters of base64url), shown only as cookie.
        assert_eq!(first.session_token.len(), 43);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_12_changing_the_password_ends_other_sessions_and_gives_a_new_secret() -> TestResult {
    each_database("sec_12_change_password", async |db| {
        let rig = Rig::start(db).await?;
        let first = rig.admin(ADA_EMAIL).await?;
        let other = rig.sign_in(ADA_EMAIL, PASSWORD).await?;
        let change = |current: &str, new: &str| {
            rig.api().change_password(
                Rig::info(),
                ChangePassword {
                    session_token: first.session_token.clone(),
                    current_password: current.to_owned(),
                    new_password: new.to_owned(),
                },
            )
        };
        assert_eq!(
            refused(change("Wrong-Password-1", OTHER_PASSWORD).await),
            Some(ErrorCode::WrongCredentials)
        );
        assert_eq!(
            refused(change(PASSWORD, "short").await),
            Some(ErrorCode::PasswordTooShort)
        );
        assert_eq!(
            refused(change(PASSWORD, "123456789012").await),
            Some(ErrorCode::PasswordBreached)
        );
        assert!(signed_in(&rig, &other.session_token).await);

        let renewed = change(PASSWORD, OTHER_PASSWORD).await?;
        assert_ne!(renewed.session_token, first.session_token);
        assert_ne!(renewed.csrf_token, first.csrf_token);
        assert!(signed_in(&rig, &renewed.session_token).await);
        assert!(!signed_in(&rig, &first.session_token).await);
        assert!(!signed_in(&rig, &other.session_token).await);
        assert_eq!(
            rig.sign_in(ADA_EMAIL, PASSWORD).await.unwrap_err(),
            AppError::Refused(ErrorCode::WrongCredentials)
        );
        rig.sign_in(ADA_EMAIL, OTHER_PASSWORD).await?;
        assert!(
            rig.database
                .security_log_entries(0, 100)
                .await?
                .iter()
                .any(|entry| entry.action.as_str() == SecurityAction::PasswordChanged.code())
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_02_the_list_of_sessions_has_a_cap_and_says_when_it_cut() -> TestResult {
    each_database("perf_02_sessions_cap", async |db| {
        let rig = Rig::start_with(db, Tries::Unreached).await?;
        let first = rig.admin(ADA_EMAIL).await?;
        let list = |token: &str| {
            rig.api().sessions(
                Rig::info(),
                SessionsQuery {
                    session_token: token.to_owned(),
                },
            )
        };
        let few = list(&first.session_token).await?;
        assert_eq!((few.sessions.len(), few.truncated), (1, false));
        let mut newest = first.session_token.clone();
        for _ in 0..54 {
            // Sessions that start in the same second share a time and their order would be left
            // to the random end of their IDs; one second apart, the newest is the newest.
            rig.clock.advance(1);
            newest = rig.sign_in(ADA_EMAIL, PASSWORD).await?.session_token;
        }
        // 55 sessions: the list holds the 50 newest and says that it is not everything.
        let many = list(&newest).await?;
        assert_eq!(many.sessions.len(), 50);
        assert!(many.truncated);
        assert!(many.sessions.iter().any(|session| session.current));
        Ok(())
    })
    .await
}
