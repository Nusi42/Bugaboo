//! Invitations and signing up on one's own (AUTH-06).

use bb_app::auth::{
    AcceptInvitation, AuthStore as _, ChangeSignUpPolicy, CreateInvitation, InvitationLookupQuery,
    InvitationsQuery, RevokeInvitation, SignUp, SignUpPolicy, SignUpPolicyQuery, StatusQuery,
};
use bb_auth::{Purpose, SecretToken};
use bb_core::ErrorCode;
use bb_core::sign_in::SecurityAction;
use bb_testing::each_database;

use super::rig::{OTHER_PASSWORD, Outcome, PASSWORD, Rig, refused, secret_of};

const WEEK: i64 = 7 * 24 * 3600;

async fn invite(rig: &Rig, session: &str, email: &str) -> Result<String, bb_app::auth::AppError> {
    let created = rig
        .api()
        .create_invitation(
            Rig::info(),
            CreateInvitation {
                session_token: session.to_owned(),
                email: email.to_owned(),
            },
        )
        .await?;
    Ok(secret_of(&created.link_path))
}

async fn accept(
    rig: &Rig,
    secret: &str,
    password: &str,
) -> Result<bb_app::auth::Started, bb_app::auth::AppError> {
    rig.api()
        .accept_invitation(
            Rig::info(),
            AcceptInvitation {
                secret: secret.to_owned(),
                display_name: "Bob".to_owned(),
                password: password.to_owned(),
            },
        )
        .await
}

async fn lookup(rig: &Rig, secret: &str) -> Result<String, bb_app::auth::AppError> {
    Ok(rig
        .api()
        .invitation_lookup(
            Rig::info(),
            InvitationLookupQuery {
                secret: secret.to_owned(),
            },
        )
        .await?
        .email)
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

#[tokio::test]
async fn auth_06_an_administrator_invites_and_the_link_works_once() -> Outcome {
    each_database("auth_06_invite", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
        let created = rig
            .api()
            .create_invitation(
                Rig::info(),
                CreateInvitation {
                    session_token: admin.session_token.clone(),
                    email: " Bob@Example.TEST ".to_owned(),
                },
            )
            .await?;
        assert!(created.link_path.starts_with("/invitation#"));
        let secret = secret_of(&created.link_path);
        assert_eq!(secret.len(), 43, "256 bits of randomness");
        assert_eq!(created.invitation.email, "bob@example.test");
        assert_eq!(
            created.invitation.expires_at.as_second() - created.invitation.created_at.as_second(),
            WEEK
        );
        // Only the hash of the secret is stored.
        let hash = SecretToken::parse(&secret)
            .ok_or("not a secret")?
            .hash(Purpose::Invitation);
        let stored = rig
            .storage
            .invitation_by_hash(&hash)
            .await?
            .ok_or("not stored")?;
        assert_eq!(stored.token_hash, hash);
        assert_eq!(lookup(&rig, &secret).await?, "bob@example.test");

        // A weak password does not use the invitation up.
        assert_eq!(
            refused(accept(&rig, &secret, "short").await),
            Some(ErrorCode::PasswordTooShort)
        );
        let bob = accept(&rig, &secret, PASSWORD).await?;
        assert!(!bob.person.administrator);
        assert_eq!(bob.person.email, "bob@example.test");
        rig.sign_in("bob@example.test", PASSWORD).await?;

        // Used up: the link, the lookup and the list all say so, without saying why.
        assert_eq!(
            refused(accept(&rig, &secret, OTHER_PASSWORD).await),
            Some(ErrorCode::InvitationInvalid)
        );
        assert_eq!(
            refused(lookup(&rig, &secret).await),
            Some(ErrorCode::InvitationInvalid)
        );
        let open = rig
            .api()
            .invitations(
                Rig::info(),
                InvitationsQuery {
                    session_token: admin.session_token.clone(),
                },
            )
            .await?;
        assert!(open.is_empty());
        assert_eq!(
            action_count(&rig, SecurityAction::InvitationCreated).await?,
            1
        );
        assert_eq!(
            action_count(&rig, SecurityAction::InvitationAccepted).await?,
            1
        );
        assert_eq!(
            action_count(&rig, SecurityAction::InvitationRefused).await?,
            1
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_06_only_administrators_manage_invitations() -> Outcome {
    each_database("auth_06_admin_only", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
        let bob = rig.person(&admin.session_token, "bob@example.test").await?;
        let session = bob.session_token.as_str();
        assert_eq!(
            refused(invite(&rig, session, "eve@example.test").await),
            Some(ErrorCode::Forbidden)
        );
        assert_eq!(
            refused(invite(&rig, "garbage", "eve@example.test").await),
            Some(ErrorCode::NotSignedIn)
        );
        let list = rig
            .api()
            .invitations(
                Rig::info(),
                InvitationsQuery {
                    session_token: session.to_owned(),
                },
            )
            .await;
        assert_eq!(refused(list), Some(ErrorCode::Forbidden));
        let secret = invite(&rig, &admin.session_token, "eve@example.test").await?;
        let id = rig
            .api()
            .invitations(
                Rig::info(),
                InvitationsQuery {
                    session_token: admin.session_token.clone(),
                },
            )
            .await?[0]
            .id;
        let revoke = |token: &str, id| {
            rig.api().revoke_invitation(
                Rig::info(),
                RevokeInvitation {
                    session_token: token.to_owned(),
                    invitation_id: id,
                },
            )
        };
        assert_eq!(
            refused(revoke(session, id).await),
            Some(ErrorCode::Forbidden)
        );
        revoke(&admin.session_token, id).await?;
        assert_eq!(
            refused(revoke(&admin.session_token, id).await),
            Some(ErrorCode::NotFound)
        );
        // A link that was taken back does not work.
        assert_eq!(
            refused(accept(&rig, &secret, PASSWORD).await),
            Some(ErrorCode::InvitationInvalid)
        );
        assert_eq!(
            action_count(&rig, SecurityAction::InvitationRevoked).await?,
            1
        );
        // The address of somebody who has an account, and text that is no address, are refused.
        for email in ["bob@example.test", "ADA@example.test", "not an address"] {
            assert_eq!(
                refused(invite(&rig, &admin.session_token, email).await),
                Some(ErrorCode::InvalidRequest),
                "{email}"
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_06_an_invitation_runs_out_after_seven_days() -> Outcome {
    each_database("auth_06_expiry", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
        let early = invite(&rig, &admin.session_token, "bob@example.test").await?;
        let late = invite(&rig, &admin.session_token, "eve@example.test").await?;
        rig.clock.advance(WEEK - 60);
        // The administrator keeps signing in, so only the invitations run out.
        let admin = rig.sign_in("ada@example.test", PASSWORD).await?;
        assert_eq!(lookup(&rig, &early).await?, "bob@example.test");
        accept(&rig, &early, PASSWORD).await?;
        rig.clock.advance(120);
        assert_eq!(
            refused(lookup(&rig, &late).await),
            Some(ErrorCode::InvitationInvalid)
        );
        assert_eq!(
            refused(accept(&rig, &late, PASSWORD).await),
            Some(ErrorCode::InvitationInvalid)
        );
        let open = rig
            .api()
            .invitations(
                Rig::info(),
                InvitationsQuery {
                    session_token: admin.session_token,
                },
            )
            .await?;
        assert!(open.is_empty(), "an invitation that ran out is not open");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_06_two_people_with_the_same_link_make_one_account() -> Outcome {
    each_database("auth_06_race", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
        let secret = invite(&rig, &admin.session_token, "bob@example.test").await?;
        let (first, second) = tokio::join!(
            accept(&rig, &secret, PASSWORD),
            accept(&rig, &secret, OTHER_PASSWORD)
        );
        assert_eq!(
            [first.is_ok(), second.is_ok()]
                .iter()
                .filter(|ok| **ok)
                .count(),
            1
        );
        Ok(())
    })
    .await
}

async fn sign_up(
    rig: &Rig,
    email: &str,
    password: &str,
) -> Result<bb_app::auth::Started, bb_app::auth::AppError> {
    rig.api()
        .sign_up(
            Rig::info(),
            SignUp {
                email: email.to_owned(),
                display_name: "Eve".to_owned(),
                password: password.to_owned(),
            },
        )
        .await
}

async fn policy(rig: &Rig, session: &str) -> Result<SignUpPolicy, bb_app::auth::AppError> {
    rig.api()
        .sign_up_policy(
            Rig::info(),
            SignUpPolicyQuery {
                session_token: session.to_owned(),
            },
        )
        .await
}

async fn change_policy(
    rig: &Rig,
    session: &str,
    enabled: bool,
    domains: &[&str],
) -> Result<SignUpPolicy, bb_app::auth::AppError> {
    rig.api()
        .change_sign_up_policy(
            Rig::info(),
            ChangeSignUpPolicy {
                session_token: session.to_owned(),
                policy: SignUpPolicy {
                    enabled,
                    domains: domains.iter().map(|domain| (*domain).to_owned()).collect(),
                },
            },
        )
        .await
}

#[tokio::test]
async fn auth_06_signing_up_is_off_by_default_and_only_administrators_change_the_rule() -> Outcome {
    each_database("auth_06_sign_up_rule", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
        assert!(
            !rig.api()
                .status(Rig::info(), StatusQuery)
                .await?
                .sign_up_open
        );
        assert_eq!(
            refused(sign_up(&rig, "eve@example.test", PASSWORD).await),
            Some(ErrorCode::SignUpRefused)
        );
        assert_eq!(
            policy(&rig, &admin.session_token).await?,
            SignUpPolicy {
                enabled: false,
                domains: vec![]
            }
        );
        let bob = rig.person(&admin.session_token, "bob@example.test").await?;
        assert_eq!(
            refused(policy(&rig, &bob.session_token).await),
            Some(ErrorCode::Forbidden)
        );
        assert_eq!(
            refused(change_policy(&rig, &bob.session_token, true, &[]).await),
            Some(ErrorCode::Forbidden)
        );
        // Domains that cannot be domains are refused, and nothing changes.
        for bad in ["nodot", "a b.test", "x@y.test", ".test.", "a..test"] {
            assert_eq!(
                refused(change_policy(&rig, &admin.session_token, true, &[bad]).await),
                Some(ErrorCode::InvalidRequest),
                "{bad}"
            );
        }
        assert!(!policy(&rig, &admin.session_token).await?.enabled);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_06_signing_up_follows_the_rule_of_the_administrator() -> Outcome {
    each_database("auth_06_sign_up_follows_rule", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
        rig.person(&admin.session_token, "bob@example.test").await?;
        let session = admin.session_token.as_str();

        // On, for two domains (written in any case, with or without @, once).
        let set = change_policy(
            &rig,
            session,
            true,
            &["Example.TEST", " @corp.test ", "corp.test"],
        )
        .await?;
        assert_eq!(set.domains, ["example.test", "corp.test"]);
        assert!(
            rig.api()
                .status(Rig::info(), StatusQuery)
                .await?
                .sign_up_open
        );
        for refused_email in ["mallory@evil.test", "mallory@sub.example.test"] {
            assert_eq!(
                refused(sign_up(&rig, refused_email, PASSWORD).await),
                Some(ErrorCode::SignUpRefused),
                "{refused_email}"
            );
        }
        assert_eq!(
            refused(sign_up(&rig, "eve@example.test", "short").await),
            Some(ErrorCode::PasswordTooShort)
        );
        let eve = sign_up(&rig, "Eve@Example.test", PASSWORD).await?;
        assert!(!eve.person.administrator);
        // An address that has an account gets the same answer as a closed door.
        for taken in ["eve@example.test", "bob@example.test"] {
            assert_eq!(
                refused(sign_up(&rig, taken, OTHER_PASSWORD).await),
                Some(ErrorCode::SignUpRefused),
                "{taken}"
            );
        }
        rig.sign_in("eve@example.test", PASSWORD).await?;

        // With no list, every address may sign up while it is on; off, nobody may.
        change_policy(&rig, session, true, &[]).await?;
        sign_up(&rig, "zed@anywhere.test", PASSWORD).await?;
        change_policy(&rig, session, false, &["example.test"]).await?;
        assert_eq!(
            refused(sign_up(&rig, "late@example.test", PASSWORD).await),
            Some(ErrorCode::SignUpRefused)
        );
        assert_eq!(action_count(&rig, SecurityAction::SignUpAccepted).await?, 2);
        assert!(action_count(&rig, SecurityAction::SignUpRefused).await? >= 5);
        assert_eq!(
            action_count(&rig, SecurityAction::SignUpPolicyChanged).await?,
            3
        );
        Ok(())
    })
    .await
}
