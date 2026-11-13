//! The days without work on the timeline as each reader may know them (PLAN-10, PLAN-11, PRIV-09,
//! ADR-0030 and its amendment M8, architecture §12): the hatched days of a bar, without a word for a
//! reader who sees nobody and not at all without *Assigned to*, and the row of each person under
//! *Group by person* (M8-F90). Every flow runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_core::id::UserId;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, ok};
use super::plan_privacy_http::{field_level, see_only_key};
use super::plan_site::{MONDAY, TIMELINE, ask, page, planned, rows_of};

/// The last day of the pages of these tests: the end of March 2027, after Good Friday (26 March).
const END: &str = "2027-03-31";

/// Lea's issue over Good Friday 2027 with an away day on its first day: its readable ID.
async fn over_good_friday(
    site: &Site<'_>,
    ada: &mut Browser,
    lea: &mut Browser,
    lea_id: UserId,
) -> Result<String, Failure> {
    away(lea, "2027-03-22").await?;
    planned(
        site,
        ada,
        "Over Easter",
        json!({"start_date": "2027-03-22", "due_date": END, "assigned_to": [lea_id.to_string()]}),
    )
    .await
}

/// An away day the browser's person enters for themselves.
async fn away(browser: &mut Browser, day: &str) -> Result<(), Failure> {
    ok(&browser
        .send(
            "POST",
            "/api/v1/me/away-times",
            Some(json!({"first": day, "last": day})),
        )
        .await?)?;
    Ok(())
}

/// The bar of the issue with this readable ID on a page.
fn bar(page: &Value, key: &str) -> Value {
    rows_of(page, key)
        .first()
        .map_or(Value::Null, |row| row["bar"].clone())
}

/// The kind and the holiday of each hatched day of a list.
fn kinds(hatched: &Value) -> Vec<(String, Value)> {
    hatched.as_array().map_or_else(Vec::new, |days| {
        days.iter()
            .map(|day| {
                let kind = day["kind"].as_str().unwrap_or_default().to_owned();
                (kind, day["holiday"].clone())
            })
            .collect()
    })
}

/// The good-friday holiday by its name.
fn good_friday() -> (String, Value) {
    ("holiday".to_owned(), json!("good-friday"))
}

#[tokio::test]
async fn plan_11_hatched_days_have_no_word_for_a_reader_who_sees_nobody() -> TestResult {
    each_database("plan_11_hatched_words", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site
            .person_called(StartingRole::TeamMember, "lea", "Lea Lang")
            .await?;
        let key = over_good_friday(&site, &mut ada, &mut lea, lea_id).await?;
        let body = ask(&site, MONDAY, END, json!({}));
        let (mut reader, _) = site.person(StartingRole::Contributor, "reader").await?;
        site.show_people(&mut ada, StartingRole::Contributor, "work-times", "names")
            .await?;
        // Names: Good Friday by its name.
        let named = bar(&page(&mut reader, body.clone()).await?, &key);
        assert!(kinds(&named["hatched"]).contains(&good_friday()), "{named}");
        // Nobody on the bar: the days stay hatched, and no word of them is sent.
        site.show_people(&mut ada, StartingRole::Contributor, "assigned-to", "hidden")
            .await?;
        let nobody = bar(&page(&mut reader, body.clone()).await?, &key);
        assert_eq!(nobody["person"], Value::Null, "{nobody}");
        let days = kinds(&nobody["hatched"]);
        assert!(
            days.contains(&("holiday".to_owned(), Value::Null)),
            "{nobody}"
        );
        assert!(
            days.iter().all(|(_, holiday)| holiday.is_null()),
            "{nobody}"
        );
        assert_eq!(nobody["hatched"][0]["kind"], "away", "{nobody}");
        assert_eq!(nobody["hatched"][0]["morning"], false, "{nobody}");
        // *Assigned to* hidden by its field level: no hatched day and no warning about the person.
        field_level(
            &site,
            &mut ada,
            StartingRole::Contributor,
            "assigned-to",
            "hidden",
        )
        .await?;
        let unread = page(&mut reader, body).await?;
        assert_eq!(bar(&unread, &key)["hatched"], json!([]), "{unread}");
        let warned = unread["warnings"].as_array().map_or(0, |all| {
            all.iter().filter(|each| each["kind"] == "away").count()
        });
        assert_eq!(warned, 0, "{unread}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_10_the_row_of_a_person_hatches_their_days_as_far_as_work_times_allow() -> TestResult {
    each_database("plan_10_person_rows", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site
            .person_called(StartingRole::TeamMember, "lea", "Lea Lang")
            .await?;
        over_good_friday(&site, &mut ada, &mut lea, lea_id).await?;
        // A second away day on which Lea has no bar at all.
        away(&mut lea, "2027-03-04").await?;
        let body = ask(&site, MONDAY, END, json!({"group_by": "assigned to"}));
        let lea_days = |page: &Value| {
            page["people_days"]
                .as_array()
                .and_then(|all| {
                    all.iter()
                        .find(|each| each["person"]["id"] == json!(lea_id.to_string()))
                        .cloned()
                })
                .unwrap_or(Value::Null)
        };
        // Names for both kinds: the away days across the span, and Good Friday by its name.
        let (mut team, _) = site.person(StartingRole::TeamMember, "team").await?;
        let row = lea_days(&page(&mut team, body.clone()).await?);
        let days = kinds(&row["hatched"]);
        assert!(days.contains(&good_friday()), "{row}");
        assert!(days.iter().all(|(kind, _)| kind != "day-off"), "{row}");
        let dates: Vec<&str> = row["hatched"].as_array().map_or_else(Vec::new, |all| {
            all.iter().filter_map(|day| day["date"].as_str()).collect()
        });
        assert!(
            dates.contains(&"2027-03-04") && dates.contains(&"2027-03-22"),
            "{row}"
        );
        // Work times hidden for the reader: the row is there, its days are not.
        site.show_people(&mut ada, StartingRole::TeamMember, "work-times", "hidden")
            .await?;
        let hidden = page(&mut team, body.clone()).await?;
        assert_eq!(lea_days(&hidden), Value::Null, "{hidden}");
        // Role only for people: no grouping by person, so nobody's row.
        let (mut contributor, _) = site
            .person(StartingRole::Contributor, "contributor")
            .await?;
        site.show_people(
            &mut ada,
            StartingRole::Contributor,
            "assigned-to",
            "role-only",
        )
        .await?;
        let role = page(&mut contributor, body.clone()).await?;
        assert_eq!(role["people_days"], json!([]), "{role}");
        // An access key: nobody's work times.
        let secret = see_only_key(&mut ada).await?;
        let keyed = ada.script("POST", TIMELINE, Some(body), &secret).await?;
        assert_eq!(ok(&keyed)?["people_days"], json!([]));
        Ok(())
    })
    .await
}
