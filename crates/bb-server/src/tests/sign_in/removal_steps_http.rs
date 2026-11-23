//! The removal that is not finished over HTTP on SQLite and PostgreSQL (PRIV-04, ADR-0044,
//! decision 6): the comments that go are hidden in the answer of the removal, a job that gives up
//! shows on the status page and keeps the person *removing…*, and the daily round finishes it.

use bb_app::auth::RequestInfo;
use bb_app::jobs::Lane;
use bb_core::id::UserId;
use bb_core::job::FailureKind;
use bb_core::technical::TechnicalName;
use bb_policy::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use jiff::SignedDuration;
use serde_json::json;

use super::http::Browser;
use super::issues_site::{Site, key_of, ok};
use super::removal_http::{comment, comments, remove, switch_off};
use super::rig::Rig;
use super::status::app_with_probe;

const STATUS: &str = "/api/v1/admin/status";

/// How many removals the status page counts as stopped.
async fn stopped(ada: &mut Browser) -> Result<serde_json::Value, Failure> {
    Ok(ok(&ada.send("GET", STATUS, None).await?)?["removals_stopped"].clone())
}

/// Whether the page of `person` says *removing…*.
async fn removing(ada: &mut Browser, person: UserId) -> Result<serde_json::Value, Failure> {
    Ok(ok(&ada
        .send("GET", &format!("/api/v1/people/{person}"), None)
        .await?)?["removing"]
        .clone())
}

/// Makes the job that finishes a removal die, as after a failure that no start can mend.
async fn kill_the_job(rig: &Rig) -> Result<(), Failure> {
    let worker = TechnicalName::new("worker.test")?;
    let now = rig.clock_now();
    let taken = rig
        .database
        .lease_jobs(
            &worker,
            now,
            SignedDuration::from_mins(1),
            10,
            &Lane::Long.kinds()?,
        )
        .await?;
    let failed = TechnicalName::new("job.failed")?;
    let mut killed = 0;
    for job in taken
        .iter()
        .filter(|job| job.kind.as_str() == "privacy.finish-removal")
    {
        rig.database
            .fail_job(job.id, &worker, &failed, FailureKind::Permanent, now)
            .await?;
        killed += 1;
    }
    assert_eq!(killed, 1, "{taken:?}");
    Ok(())
}

#[tokio::test]
async fn priv_04_a_removal_whose_job_gave_up_shows_on_the_status_page_and_still_finishes()
-> TestResult {
    each_database("priv_04_removal_steps_http", async |db| {
        let rig = Rig::start(db).await?;
        let app = app_with_probe(&rig, None);
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        let issue = key_of(&site.create(&mut ada, "Talk about it").await?)?;
        comment(&mut bob, &issue, "Bob's words").await?;
        rig.run_jobs().await?;
        switch_off(&mut ada, bob_id).await?;
        assert_eq!(stopped(&mut ada).await?, 0);
        let done = remove(&mut ada, bob_id, json!({"comments": "delete"})).await?;
        assert_eq!(done["status"], 204, "{done}");
        // Hidden in the answer of the removal, before its job ran.
        let shown = comments(&mut ada, &issue).await?;
        assert!(!shown.contains("Bob's words"), "{shown}");
        // The job gives up: the status page counts the removal, and Bob is still being removed.
        kill_the_job(&rig).await?;
        rig.run_jobs().await?;
        assert_eq!(stopped(&mut ada).await?, 1);
        assert_eq!(removing(&mut ada, bob_id).await?, true);
        // The daily round gives it a new job, which finishes it.
        rig.services
            .ctx(RequestInfo::default())
            .start_daily_jobs()
            .await
            .map_err(|error| error.to_string())?;
        rig.run_jobs().await?;
        assert_eq!(stopped(&mut ada).await?, 0);
        assert_eq!(removing(&mut ada, bob_id).await?, false);
        assert_eq!(
            rig.storage
                .columns_holding(&["Bob Builder", "bob@example.test"])
                .await?,
            Vec::<String>::new()
        );
        Ok(())
    })
    .await
}
