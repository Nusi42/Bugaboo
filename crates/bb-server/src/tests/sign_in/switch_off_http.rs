//! *Switch off* and *Switch on* on SQLite and PostgreSQL (PRIV-04, ADR-0044, decision 5): the
//! session of a switched-off person is refused at once, the person signs in to nothing until they
//! are switched on, and the replay list in the data folder holds the switch-off.

use bb_app::auth::AuthenticateQuery;
use bb_app::people::{SwitchOff, SwitchOn};
use bb_core::ErrorCode;
use bb_policy::StartingRole;
use bb_testing::{TestResult, each_database};

use super::http::rig_and_app;
use super::issues_site::{Site, ok};
use super::policy_rig::Org;
use super::rig::{Rig, refused};
use bb_dev_words::accounts::PASSWORD;

#[tokio::test]
async fn priv_04_a_switched_off_person_is_refused_at_once_and_signs_in_again_when_switched_on()
-> TestResult {
    each_database("priv_04_switch_off_http", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let bob = org.person().await?;
        let session_works = |session: String| {
            let api = rig.api();
            async move {
                api.authenticate(
                    Rig::info(),
                    AuthenticateQuery {
                        session_token: session,
                    },
                )
                .await
                .map(|_| ())
            }
        };
        session_works(bob.session.clone()).await?;
        rig.api()
            .switch_off(
                Rig::info(),
                SwitchOff {
                    session_token: org.admin.session.clone(),
                    person: bob.id,
                },
            )
            .await?;
        assert_eq!(
            refused(session_works(bob.session.clone()).await),
            Some(ErrorCode::NotSignedIn)
        );
        assert_eq!(
            refused(rig.sign_in("person1@example.test", PASSWORD).await),
            Some(ErrorCode::WrongCredentials)
        );
        rig.api()
            .switch_on(
                Rig::info(),
                SwitchOn {
                    session_token: org.admin.session.clone(),
                    person: bob.id,
                },
            )
            .await?;
        rig.sign_in("person1@example.test", PASSWORD).await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_the_cookie_of_a_switched_off_person_is_refused_over_http() -> TestResult {
    each_database("priv_04_switch_off_cookie", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        ok(&bob.send("GET", "/api/v1/notifications", None).await?)?;
        let off = ada
            .send("POST", &format!("/api/v1/people/{bob_id}/switch-off"), None)
            .await?;
        assert!(off.status.is_success(), "{}", off.body);
        let refused = bob.send("GET", "/api/v1/notifications", None).await?;
        assert_eq!(refused.code(), "auth.not-signed-in", "{}", refused.body);
        Ok(())
    })
    .await
}
