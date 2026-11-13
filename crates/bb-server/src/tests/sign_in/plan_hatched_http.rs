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
use super::plan_site::{
    MONDAY, TIMELINE, ask, by_effort, id_of_key, issue, page, planned, rows_of, warnings_on,
    work_times,
};

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

/// The hatched days of a bar of a person the reader does not see by name: every day without work is
/// the one neutral kind, with no word, so away, holiday and day off are not told apart (the
/// maintainer's decision of 2026-11-13).
fn assert_no_words(bar: &Value) {
    let days = bar["hatched"].as_array().cloned().unwrap_or_default();
    assert!(!days.is_empty(), "{bar}");
    for day in &days {
        assert_eq!(day["kind"], "no-work", "{bar}");
        assert_eq!(day["holiday"], Value::Null, "{bar}");
        assert_eq!(day["morning"], false, "{bar}");
        assert_eq!(day["afternoon"], false, "{bar}");
    }
}

#[tokio::test]
async fn plan_11_a_reader_who_does_not_see_the_person_sees_every_day_without_work_alike()
-> TestResult {
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
        // The weekend of the bar is a day off for a reader who sees the person.
        assert!(
            kinds(&named["hatched"]).contains(&("day-off".to_owned(), Value::Null)),
            "{named}"
        );
        // Nobody on the bar: every day without work is the one neutral kind, with no word.
        site.show_people(&mut ada, StartingRole::Contributor, "assigned-to", "hidden")
            .await?;
        let nobody = bar(&page(&mut reader, body.clone()).await?, &key);
        assert_eq!(nobody["person"], Value::Null, "{nobody}");
        assert_no_words(&nobody);
        assert_eq!(nobody["work_time"], Value::Null, "{nobody}");
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

/// The warnings of a kind on an issue of a page.
fn of_kind<'a>(page: &'a Value, id: &str, kind: &str) -> Vec<&'a Value> {
    warnings_on(page, id)
        .into_iter()
        .filter(|each| each["kind"] == kind)
        .collect()
}

/// Accepts the warning of `kind` of an issue, as `browser`.
async fn accept(browser: &mut Browser, key: &str, kind: &str) -> Result<(), Failure> {
    let version = issue(browser, key).await?["version"].clone();
    ok(&browser
        .send(
            "POST",
            &format!("{TIMELINE}/accepted-warnings"),
            Some(json!({"issue": key, "version": version, "kind": kind, "timeline": {"zoom": "week"}})),
        )
        .await?)?;
    Ok(())
}

#[tokio::test]
async fn plan_13_a_reader_who_does_not_see_the_person_gets_only_days_without_work() -> TestResult {
    each_database("plan_13_neutral_warning", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site
            .person_called(StartingRole::TeamMember, "lea", "Lea Lang")
            .await?;
        let key = over_good_friday(&site, &mut ada, &mut lea, lea_id).await?;
        let id = id_of_key(&mut ada, &key).await?;
        let body = ask(&site, MONDAY, END, json!({}));
        // Ada sees Lea: Away, and no warning of days without work.
        let named = page(&mut ada, body.clone()).await?;
        assert_eq!(of_kind(&named, &id, "away").len(), 1, "{named}");
        assert!(
            of_kind(&named, &id, "days-without-work").is_empty(),
            "{named}"
        );
        // Ada accepts it for everybody; the reader sees who accepted by the kind *Authors*.
        accept(&mut ada, &key, "days-without-work").await?;
        let (mut reader, _) = site.person(StartingRole::Contributor, "reader").await?;
        site.show_people(&mut ada, StartingRole::Contributor, "work-times", "names")
            .await?;
        for level in ["role-only", "hidden"] {
            site.show_people(&mut ada, StartingRole::Contributor, "assigned-to", level)
                .await?;
            site.show_people(&mut ada, StartingRole::Contributor, "authors", level)
                .await?;
            let seen = page(&mut reader, body.clone()).await?;
            // The bar tells no more than the warning: its days are hatched without a word, and the
            // person's time, which would tell which weekdays are absences, is not sent.
            let shown = bar(&seen, &key);
            assert_no_words(&shown);
            assert_eq!(shown["work_time"], Value::Null, "{shown}");
            // No *Away*: only *Days without work*, over the bar, kept and accepted.
            assert!(of_kind(&seen, &id, "away").is_empty(), "{seen}");
            let found = of_kind(&seen, &id, "days-without-work");
            let warning = found.first().copied().unwrap_or(&Value::Null);
            assert_eq!(
                warning["sentence"], "plan-warning.days-without-work",
                "{warning}"
            );
            assert_eq!(
                warning["days"],
                json!({"first": "2027-03-22", "last": END}),
                "{warning}"
            );
            assert_eq!(warning["ways_out"], json!(["keep-and-accept"]), "{warning}");
            assert_eq!(warning["move_after"], Value::Null, "{warning}");
            assert_eq!(warning["due_needed"], Value::Null, "{warning}");
            let by = &warning["accepted"]["by"];
            assert!(
                by.get("name").is_none() && by.get("id").is_none(),
                "{warning}"
            );
            let shown_as = if level == "hidden" { "someone" } else { "role" };
            assert_eq!(by["shown"], shown_as, "{warning}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_13_days_without_work_fire_for_a_holiday_as_for_an_away_day() -> TestResult {
    each_database("plan_13_neutral_holiday", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_lea, lea_id) = site
            .person_called(StartingRole::TeamMember, "lea", "Lea Lang")
            .await?;
        // Thursday and Good Friday 2027: no away day, one public holiday.
        let key = planned(
            &site,
            &mut ada,
            "Before Easter",
            json!({"start_date": "2027-03-25", "due_date": "2027-03-26", "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        let id = id_of_key(&mut ada, &key).await?;
        let body = ask(&site, MONDAY, END, json!({}));
        let named = page(&mut ada, body.clone()).await?;
        assert!(of_kind(&named, &id, "away").is_empty(), "{named}");
        let (mut reader, _) = site.person(StartingRole::Contributor, "reader").await?;
        site.show_people(&mut ada, StartingRole::Contributor, "work-times", "names")
            .await?;
        site.show_people(&mut ada, StartingRole::Contributor, "assigned-to", "hidden")
            .await?;
        let seen = page(&mut reader, body).await?;
        assert_eq!(of_kind(&seen, &id, "days-without-work").len(), 1, "{seen}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_13_too_much_at_once_about_a_person_not_named_is_left_out() -> TestResult {
    each_database("plan_13_too_much_unnamed", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_lea, lea_id) = site
            .person_called(StartingRole::TeamMember, "lea", "Lea Lang")
            .await?;
        // 6 hours at 50 %: 3 hours a day. Two issues of 15 hours each over the same week plan 6.
        work_times(&site, &mut ada, lea_id, 360, 50).await?;
        for title in ["API", "Login screen"] {
            planned(
                &site,
                &mut ada,
                title,
                json!({
                    "start_date": MONDAY,
                    "due_date": "2027-03-05",
                    "estimated_time": 900,
                    "assigned_to": [lea_id.to_string()],
                }),
            )
            .await?;
        }
        let body = by_effort(&site, MONDAY, END);
        let too_much = |page: &Value| {
            page["warnings"].as_array().map_or(0, |all| {
                all.iter()
                    .filter(|each| each["kind"] == "too-much-at-once")
                    .count()
            })
        };
        let (mut reader, _) = site.person(StartingRole::Contributor, "reader").await?;
        site.show_people(&mut ada, StartingRole::Contributor, "work-times", "names")
            .await?;
        // By name: Lea has too much at once.
        assert!(too_much(&page(&mut reader, body.clone()).await?) > 0);
        // By role only and not at all: the warning is not listed.
        for level in ["role-only", "hidden"] {
            site.show_people(&mut ada, StartingRole::Contributor, "assigned-to", level)
                .await?;
            let seen = page(&mut reader, body.clone()).await?;
            assert_eq!(too_much(&seen), 0, "{seen}");
        }
        Ok(())
    })
    .await
}
