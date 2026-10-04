//! What signing in writes down: an event for every change (ADR-0007), the security log with
//! its chain (SEC-15), and no secret anywhere but where it belongs (SEC-16).

use std::fmt::Write as _;

use bb_app::auth::{
    AuthStore as _, ChangePassword, ConfirmCodeApp, CreateInvitation, RemoveCodeApp,
    RenewBackupCodes, RevokeInvitation, SessionsQuery, SignOut, StartCodeApp,
};
use bb_app::context::Clock as _;
use bb_auth::TotpSecret;
use bb_core::technical::TechnicalName;
use bb_store::Verification;
use bb_testing::each_database;

use super::rig::{OTHER_PASSWORD, Outcome, PASSWORD, Rig, secret_of};

async fn events(rig: &Rig) -> Result<usize, bb_store::StoreError> {
    let consumer =
        TechnicalName::new("test-reader").map_err(|_| bb_store::StoreError::ReservedName)?;
    Ok(rig.database.outbox_after(&consumer, 100_000).await?.len())
}

/// Counts the events of the outbox, to say how many a command added.
struct Counter<'a> {
    rig: &'a Rig,
    seen: usize,
}

impl<'a> Counter<'a> {
    async fn start(rig: &'a Rig) -> Result<Counter<'a>, bb_store::StoreError> {
        Ok(Self {
            rig,
            seen: events(rig).await?,
        })
    }

    /// The command that just ran added exactly one event.
    async fn one_more(&mut self, what: &str) -> Outcome {
        self.seen += 1;
        assert_eq!(events(self.rig).await?, self.seen, "{what}");
        Ok(())
    }

    /// Nothing was added (a failure or a read).
    async fn no_more(&self, what: &str) -> Outcome {
        assert_eq!(events(self.rig).await?, self.seen, "{what}");
        Ok(())
    }
}

#[tokio::test]
async fn adr_0007_accounts_sessions_and_invitations_append_one_event_per_change() -> Outcome {
    each_database("adr_0007_sign_in_events", async |db| {
        let rig = Rig::start(db, 10).await?;
        let mut count = Counter::start(&rig).await?;
        let admin = rig.admin("ada@example.test").await?;
        count.one_more("setup").await?;
        let session = admin.session_token.clone();
        rig.sign_in("ada@example.test", PASSWORD).await?;
        count.one_more("sign in").await?;

        // Reading and failing change nothing, so they add nothing.
        assert!(
            rig.sign_in("ada@example.test", "Wrong-Password-1")
                .await
                .is_err()
        );
        let query = SessionsQuery {
            session_token: session.clone(),
        };
        rig.api().sessions(Rig::info(), query).await?;
        count.no_more("failures and reads add no event").await?;

        let created = rig
            .api()
            .create_invitation(
                Rig::info(),
                CreateInvitation {
                    session_token: session.clone(),
                    email: "bob@example.test".to_owned(),
                },
            )
            .await?;
        count.one_more("invite").await?;
        let revoke = RevokeInvitation {
            session_token: session.clone(),
            invitation_id: created.invitation.id,
        };
        rig.api().revoke_invitation(Rig::info(), revoke).await?;
        count.one_more("revoke").await?;

        let change = ChangePassword {
            session_token: session,
            current_password: PASSWORD.to_owned(),
            new_password: OTHER_PASSWORD.to_owned(),
        };
        let renewed = rig.api().change_password(Rig::info(), change).await?;
        count.one_more("change the password").await?;
        let out = SignOut {
            session_token: renewed.session_token,
        };
        rig.api().sign_out(Rig::info(), out).await?;
        count.one_more("sign out").await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0007_the_code_app_appends_one_event_per_change() -> Outcome {
    each_database("adr_0007_code_app_events", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
        let session = admin.session_token.clone();
        let mut count = Counter::start(&rig).await?;
        let start = StartCodeApp {
            session_token: session.clone(),
        };
        rig.api().start_code_app(Rig::info(), start).await?;
        count.one_more("start the code app").await?;
        let secret = rig
            .storage
            .totp_secret(admin.person.id)
            .await?
            .ok_or("no secret")?;
        let code = TotpSecret::from_bytes(&secret).code_at(rig.clock.now())?;
        let confirm = ConfirmCodeApp {
            session_token: session.clone(),
            code,
        };
        rig.api().confirm_code_app(Rig::info(), confirm).await?;
        count.one_more("confirm the code app").await?;
        let renew = RenewBackupCodes {
            session_token: session.clone(),
            password: PASSWORD.to_owned(),
        };
        rig.api().renew_backup_codes(Rig::info(), renew).await?;
        count.one_more("new backup codes").await?;
        let remove = RemoveCodeApp {
            session_token: session,
            password: PASSWORD.to_owned(),
        };
        rig.api().remove_code_app(Rig::info(), remove).await?;
        count.one_more("switch the code app off").await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_16_no_secret_is_written_to_an_event_or_to_the_security_log() -> Outcome {
    each_database("sec_16_no_secret_in_events", async |db| {
        let rig = Rig::start(db, 10).await?;
        let admin = rig.admin("ada@example.test").await?;
        let session = admin.session_token.clone();
        let setup = rig
            .api()
            .start_code_app(
                Rig::info(),
                StartCodeApp {
                    session_token: session.clone(),
                },
            )
            .await?;
        let secret = rig
            .storage
            .totp_secret(admin.person.id)
            .await?
            .ok_or("no secret")?;
        let code = TotpSecret::from_bytes(&secret).code_at(rig.clock.now())?;
        let backup = rig
            .api()
            .confirm_code_app(
                Rig::info(),
                ConfirmCodeApp {
                    session_token: session.clone(),
                    code,
                },
            )
            .await?;
        let created = rig
            .api()
            .create_invitation(
                Rig::info(),
                CreateInvitation {
                    session_token: session.clone(),
                    email: "bob@example.test".to_owned(),
                },
            )
            .await?;
        let _ = rig.sign_in("ada@example.test", "Wrong-Password-1").await;
        let _ = rig.sign_in("ada@example.test", PASSWORD).await?;
        let hash = rig
            .storage
            .account_by_email("ada@example.test")
            .await?
            .and_then(|account| account.password_hash)
            .ok_or("no hash")?;

        // Everything that is not a secret in the database is allowed to show up; these never.
        let mut secrets = vec![
            PASSWORD.to_owned(),
            "Wrong-Password-1".to_owned(),
            session,
            admin.csrf_token.clone(),
            rig.setup_code.clone(),
            setup.secret.clone(),
            setup.otpauth_url.clone(),
            secret_of(&created.link_path),
            hash,
        ];
        secrets.extend(backup.0);
        let consumer = TechnicalName::new("test-reader")?;
        let mut written = String::new();
        for (_, event) in rig.database.outbox_after(&consumer, 100_000).await? {
            written.push_str(&serde_json::to_string(&event)?);
        }
        for entry in rig.database.security_log_entries(0, 10_000).await? {
            write!(written, "{entry:?}")?;
        }
        assert!(written.len() > 500, "something was written");
        for secret in &secrets {
            assert!(
                !written.contains(secret.as_str()),
                "a secret was written down"
            );
        }
        // The people involved are in the log by ID, and the log never holds an address in full.
        assert!(written.contains(&admin.person.id.to_string()));
        assert!(!written.contains("203.0.113.9"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_15_the_security_log_of_signing_in_verifies_and_names_what_happened() -> Outcome {
    each_database("sec_15_sign_in_log", async |db| {
        let rig = Rig::start(db, 10).await?;
        rig.admin("ada@example.test").await?;
        let _ = rig.sign_in("ada@example.test", "Wrong-Password-1").await;
        let _ = rig.sign_in("nobody@example.test", PASSWORD).await;
        rig.sign_in("ada@example.test", PASSWORD).await?;
        let entries = rig.database.security_log_entries(0, 100).await?;
        let actions: Vec<&str> = entries.iter().map(|entry| entry.action.as_str()).collect();
        assert_eq!(
            actions,
            [
                "setup.completed",
                "sign_in.failed",
                "sign_in.failed",
                "sign_in.succeeded"
            ]
        );
        // A failed try has nobody signed in; a success names the person.
        assert!(entries[1].person.is_none() && entries[2].person.is_none());
        assert!(entries[3].person.is_some());
        match rig.database.verify_security_log(&rig.seal).await? {
            Verification::Valid {
                entries, anchored, ..
            } => {
                assert_eq!(entries, 4);
                assert!(
                    anchored,
                    "the end of the log is written down outside the database"
                );
            }
            Verification::Broken { seq, reason } => {
                return Err(format!("the log breaks at {seq}: {reason:?}").into());
            }
        }
        Ok(())
    })
    .await
}
