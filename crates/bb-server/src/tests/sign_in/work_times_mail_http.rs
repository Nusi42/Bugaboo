//! The e-mail of *Your work times were changed* (ADR-0033, amendment M8): the sentence a person gets
//! when somebody else enters an away time for them names who and the days, and nothing else: no
//! note and no reason. Runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::http::rig_and_app;
use super::issues_site::{Site, ok};

#[tokio::test]
async fn plan_11_the_e_mail_of_changed_work_times_names_who_and_the_days() -> TestResult {
    each_database("plan_11_work_times_mail", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        rig.turn_mail_on().await?;
        rig.set_organization_switch("notifications.work-times-changed-by-mail", true)
            .await?;
        let (_bob, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Baker")
            .await?;
        let entered = ada
            .send(
                "POST",
                &format!("/api/v1/people/{bob_id}/away-times"),
                Some(json!({"first": "2027-01-25", "last": "2027-01-29"})),
            )
            .await?;
        ok(&entered)?;
        rig.clock
            .advance(bb_core::notification::MAIL_BATCH_SECONDS + 1);
        rig.run_jobs().await?;
        let mails = rig.mailer.messages();
        let mail = mails
            .iter()
            .find(|mail| mail.to == "bob@example.test")
            .ok_or("no e-mail for Bob")?;
        let text = &mail.text;
        assert!(
            text.contains("Ada Admin entered an away time for you"),
            "{text}"
        );
        assert!(text.contains("2027") && text.contains("25"), "{text}");
        assert!(text.contains("29"), "{text}");
        assert!(!text.to_lowercase().contains("reason"), "{text}");
        // Nobody else gets an e-mail about it.
        assert!(mails.iter().all(|mail| mail.to != "ada@example.test"));
        Ok(())
    })
    .await
}
