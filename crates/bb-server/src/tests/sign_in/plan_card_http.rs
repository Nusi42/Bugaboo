//! What the card of a bar tells (PLAN-10, PLAN-12, PLAN-14, PLAN-15, PRIV-09, ADR-0030 and its
//! amendment M8, architecture §12): the effort of the bar and the time of the person on it, each
//! only as far as the reader may know it. Every flow runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, ok};
use super::plan_privacy_http::{field_level, see_only_key};
use super::plan_site::{MONDAY, TIMELINE, by_effort, page, planned, rows_of, work_times};

/// The bar of the issue with this readable ID on a page.
fn bar(page: &Value, key: &str) -> Value {
    rows_of(page, key)
        .first()
        .map_or(Value::Null, |row| row["bar"].clone())
}

/// The page of ACME planned by effort, as `browser` reads it.
async fn planned_page(site: &Site<'_>, browser: &mut Browser) -> Result<Value, Failure> {
    page(browser, by_effort(site, MONDAY, "2027-03-31")).await
}

#[tokio::test]
async fn plan_12_the_card_of_a_bar_tells_effort_and_work_time_only_as_far_as_allowed() -> TestResult
{
    each_database("plan_12_card_of_a_bar", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_lea, lea_id) = site
            .person_called(StartingRole::TeamMember, "lea", "Lea Lang")
            .await?;
        // 6 hours a day at 50 %: 3 hours a day, so 9 hours of effort take three work days.
        work_times(&site, &mut ada, lea_id, 360, 50).await?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": MONDAY, "estimated_time": 540, "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        // Names: the effort with its work days, and Lea's usual days and share.
        let (mut team, _) = site.person(StartingRole::TeamMember, "team").await?;
        let named = bar(&planned_page(&site, &mut team).await?, &key);
        assert_eq!(
            named["effort"],
            json!({"estimate": 540, "effort": 540, "work_days": 3}),
            "{named}"
        );
        assert_eq!(named["work_time"]["share"], 50, "{named}");
        assert_eq!(
            named["work_time"]["usual"][0],
            json!({"works": true, "minutes": 360})
        );
        assert_eq!(
            named["work_time"]["usual"][5],
            json!({"works": false, "minutes": 360})
        );
        // Role only: the person is a role, and the work times are still known to the reader.
        let (mut contributor, _) = site
            .person(StartingRole::Contributor, "contributor")
            .await?;
        site.show_people(&mut ada, StartingRole::Contributor, "work-times", "role-only")
            .await?;
        let role = bar(&planned_page(&site, &mut contributor).await?, &key);
        assert_eq!(role["work_time"]["share"], 50, "{role}");
        // Hidden: no work time, while the bar keeps its length and its effort.
        site.show_people(&mut ada, StartingRole::Contributor, "work-times", "hidden")
            .await?;
        let hidden = bar(&planned_page(&site, &mut contributor).await?, &key);
        assert_eq!(hidden["work_time"], Value::Null, "{hidden}");
        assert_eq!(hidden["effort"]["work_days"], 3, "{hidden}");
        assert_eq!(hidden["days"]["last"], "2027-03-03", "{hidden}");
        // A reader who sees the person but not the estimate: no effort at all.
        field_level(
            &site,
            &mut ada,
            StartingRole::TeamMember,
            "estimated-time",
            "hidden",
        )
        .await?;
        let partial = bar(&planned_page(&site, &mut team).await?, &key);
        assert_eq!(partial["effort"], Value::Null, "{partial}");
        assert_eq!(partial["work_time"]["share"], 50, "{partial}");
        // A reader who may not read who the issue is assigned to: no person's time at all.
        field_level(&site, &mut ada, StartingRole::TeamMember, "assigned-to", "hidden").await?;
        let unassigned = bar(&planned_page(&site, &mut team).await?, &key);
        assert_eq!(unassigned["work_time"], Value::Null, "{unassigned}");
        assert_eq!(unassigned["person"], Value::Null, "{unassigned}");
        // An access key: nobody's work times, and no bar planned by effort.
        let secret = see_only_key(&mut ada).await?;
        let keyed = ada
            .script(
                "POST",
                TIMELINE,
                Some(by_effort(&site, MONDAY, "2027-03-31")),
                &secret,
            )
            .await?;
        let keyed = bar(ok(&keyed)?, &key);
        assert_eq!(keyed["work_time"], Value::Null, "{keyed}");
        assert_eq!(keyed["effort"]["work_days"], Value::Null, "{keyed}");
        Ok(())
    })
    .await
}
