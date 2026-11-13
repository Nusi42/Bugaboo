//! What the screen of the timeline reads of the reader's rights (PLAN-03, PLAN-04, PLAN-08,
//! ADR-0043): whether a bar may be dragged follows the rule of the change itself, for every kind
//! of right and for own date fields; a goal tells only what the reader may read of it. Every flow
//! runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_core::access::FieldLevel;
use bb_policy::Permission;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::{create_field, reader_of};
use super::http::{Browser, rig_and_app};
use super::issues_site::{PROJECTS, Site, key_of, ok};
use super::plan_privacy_http::{field_level, see_only_key};
use super::plan_site::{MONDAY, TIMELINE, ask, goal_of, id_of_key, page, planned, rows_of, under};

/// Whether the row of the issue with this readable ID may be dragged.
fn may_move(page: &Value, key: &str) -> Value {
    rows_of(page, key)[0]["may_move"].clone()
}

/// An issue in a second project SEC, which only Ada sees, with these members: its readable ID.
async fn secret_issue(
    ada: &mut Browser,
    kind: &str,
    title: &str,
    more: Value,
) -> Result<String, Failure> {
    let projects = ada.send("GET", PROJECTS, None).await?;
    let found = ok(&projects)?["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["short_name"] == "SEC"))
        .and_then(|item| item["id"].as_str().map(str::to_owned));
    let secret = match found {
        Some(id) => id,
        None => ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "SEC", "name": "Secret"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project ID")?
            .to_owned(),
    };
    let mut body = json!({"issue_type": kind, "title": title});
    if let (Some(body), Some(more)) = (body.as_object_mut(), more.as_object()) {
        body.extend(more.clone());
    }
    key_of(
        &ada.send("POST", &format!("{PROJECTS}/{secret}/issues"), Some(body))
            .await?,
    )
}

#[tokio::test]
async fn plan_04_a_goal_tells_only_what_the_reader_may_read() -> TestResult {
    each_database("plan_04_goal_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let goal = planned(
            &site,
            &mut ada,
            "Mobile app 2.0",
            json!({"due_date": "2027-03-12"}),
        )
        .await?;
        let seen = planned(
            &site,
            &mut ada,
            "API",
            json!({"start_date": "2027-03-02", "due_date": "2027-03-04"}),
        )
        .await?;
        under(&mut ada, &seen, &goal).await?;
        // A sub-issue only Ada sees, which ends later: it widens the bracket for her only.
        let hidden = secret_issue(
            &mut ada,
            &site.kind,
            "Hidden work",
            json!({"start_date": "2027-03-08", "due_date": "2027-03-20"}),
        )
        .await?;
        under(&mut ada, &hidden, &goal).await?;
        // A goal only Ada sees, over an issue of ACME.
        let hidden_goal = secret_issue(&mut ada, &site.kind, "Hidden goal", json!({})).await?;
        let orphan = planned(
            &site,
            &mut ada,
            "Orphan",
            json!({"start_date": MONDAY, "due_date": MONDAY}),
        )
        .await?;
        under(&mut ada, &orphan, &hidden_goal).await?;
        let (mut team, _) = site
            .person_in_acme(StartingRole::TeamMember, "team")
            .await?;
        field_level(
            &site,
            &mut ada,
            StartingRole::TeamMember,
            "due-date",
            "hidden",
        )
        .await?;
        field_level(&site, &mut ada, StartingRole::TeamMember, "title", "hidden").await?;
        let body = ask(&site, MONDAY, "2027-03-31", json!({"group_by": "parent"}));
        let goal_id = id_of_key(&mut ada, &goal).await?;
        let hidden_goal_id = id_of_key(&mut ada, &hidden_goal).await?;
        let adas = page(&mut ada, body.clone()).await?;
        let found = goal_of(&adas, &goal_id).ok_or("no goal for Ada")?;
        assert_eq!(
            found["span"],
            json!({"first": "2027-03-02", "last": "2027-03-20"}),
            "{adas}"
        );
        assert_eq!(found["title"], "Mobile app 2.0");
        let theirs = page(&mut team, body).await?;
        let found = goal_of(&theirs, &goal_id).ok_or("no goal for the team member")?;
        assert_eq!(found["due"], Value::Null, "{theirs}");
        assert_eq!(found["title"], Value::Null, "{theirs}");
        // The hidden sub-issue does not widen it; without the Due date, API is one day long.
        assert_eq!(
            found["span"],
            json!({"first": "2027-03-02", "last": "2027-03-02"})
        );
        assert!(goal_of(&theirs, &hidden_goal_id).is_none(), "{theirs}");
        assert_eq!(
            rows_of(&theirs, &orphan)[0]["group"],
            json!({"type": "empty"})
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_03_may_move_follows_the_rule_of_the_change() -> TestResult {
    each_database("plan_03_may_move_rules", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let dates = json!({"start_date": MONDAY, "due_date": "2027-03-05"});
        let adas = planned(&site, &mut ada, "Ada's work", dates.clone()).await?;
        let body = ask(&site, MONDAY, "2027-03-31", json!({}));
        // A Team member whose Start date is only to be seen may not drag the bar.
        let (mut team, _) = site
            .person_in_acme(StartingRole::TeamMember, "team")
            .await?;
        assert_eq!(
            may_move(&page(&mut team, body.clone()).await?, &adas),
            json!(true)
        );
        field_level(
            &site,
            &mut ada,
            StartingRole::TeamMember,
            "start-date",
            "can-see",
        )
        .await?;
        assert_eq!(
            may_move(&page(&mut team, body.clone()).await?, &adas),
            json!(false)
        );
        // A key that may only look changes nothing.
        let secret = see_only_key(&mut ada).await?;
        let keyed = ada
            .script("POST", TIMELINE, Some(body.clone()), &secret)
            .await?;
        assert_eq!(may_move(ok(&keyed)?, &adas), json!(false));
        // A Contributor may change only the issues they reported.
        for field in ["start-date", "due-date"] {
            field_level(
                &site,
                &mut ada,
                StartingRole::Contributor,
                field,
                "can-change",
            )
            .await?;
        }
        let (mut own, _) = site
            .person_in_acme(StartingRole::Contributor, "own")
            .await?;
        let mut new = site.new_issue("Their own");
        if let (Some(new), Some(more)) = (new.as_object_mut(), dates.as_object()) {
            new.extend(more.clone());
        }
        let made = own
            .send(
                "POST",
                &format!("{PROJECTS}/{}/issues", site.acme),
                Some(new),
            )
            .await?;
        assert!(
            made.body.get("readable_id").is_some(),
            "made: {}",
            made.body
        );
        let theirs = key_of(&made)?;
        let read = page(&mut own, body).await?;
        assert_eq!(may_move(&read, &theirs), json!(true), "{read}");
        assert_eq!(may_move(&read, &adas), json!(false));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_03_may_move_follows_the_own_date_fields_of_the_view() -> TestResult {
    each_database("plan_03_may_move_own", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let field = |name: &str| json!({"name": name, "kind": "date", "applies_to": "issues"});
        let begins = create_field(&mut ada, field("Begins")).await?["id"]
            .as_str()
            .ok_or("no field")?
            .to_owned();
        let ends = create_field(&mut ada, field("Ends")).await?["id"]
            .as_str()
            .ok_or("no field")?
            .to_owned();
        let key = planned(
            &site,
            &mut ada,
            "Own dates",
            json!({"fields": {&begins: MONDAY, &ends: "2027-03-05"}}),
        )
        .await?;
        let body = ask(
            &site,
            MONDAY,
            "2027-03-31",
            json!({"timeline": {"zoom": "week", "start_field": "begins", "end_field": "ends"}}),
        );
        assert_eq!(
            may_move(&page(&mut ada, body.clone()).await?, &key),
            json!(true)
        );
        // Ends is only to be seen for this role: the bar cannot be dragged, though Start date and
        // Due date could be changed.
        let mut reader = reader_of(
            &site,
            &rig,
            "reader@example.test",
            &[
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::ChangeIssues,
            ],
            &[
                (&ends, None, FieldLevel::CanSee),
                (&begins, None, FieldLevel::CanChange),
            ],
        )
        .await?;
        let read = page(&mut reader, body).await?;
        assert_eq!(may_move(&read, &key), json!(false), "{read}");
        Ok(())
    })
    .await
}
