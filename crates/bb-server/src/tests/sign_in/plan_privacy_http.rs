//! What the plan of the timeline keeps from whom (PLAN-08, PLAN-10, PLAN-13, PLAN-14, PRIV-09,
//! ADR-0030 and its amendment M8, architecture §12): access keys, hidden fields, hidden people and
//! other people's work times. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::plan_site::{
    MONDAY, TIMELINE, ask, blocks, by_effort, id_of_key, issue, overlapping, page, planned,
    rows_of, warnings_on, work_times,
};
use super::rig::PASSWORD;

/// A new access key of the browser's person that may only look: its secret.
pub(super) async fn see_only_key(browser: &mut Browser) -> Result<String, Failure> {
    let made = browser
        .send(
            "POST",
            "/api/v1/access-keys",
            Some(json!({"password": PASSWORD, "name": "Looks", "choice": "see-only"})),
        )
        .await?;
    Ok(ok(&made)?["secret"].as_str().ok_or("no secret")?.to_owned())
}

/// Sets the level of a field for a starting role in ACME, as Ada.
pub(super) async fn field_level(
    site: &Site<'_>,
    ada: &mut Browser,
    role: StartingRole,
    field: &str,
    level: &str,
) -> Result<(), Failure> {
    let set = ada
        .send(
            "PUT",
            &format!(
                "/api/v1/field-permissions/{}/{field}?project={}",
                site.role_id(role),
                site.acme
            ),
            Some(json!({"level": level})),
        )
        .await?;
    assert_eq!(set.status, StatusCode::NO_CONTENT, "{}", set.body);
    Ok(())
}

#[tokio::test]
async fn plan_14_a_key_that_may_only_look_writes_nothing_of_the_plan() -> TestResult {
    each_database("plan_14_see_only_key", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site.person(StartingRole::Contributor, "lea").await?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": MONDAY, "due_date": "2027-03-05", "estimated_time": 480, "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        let secret = see_only_key(&mut lea).await?;
        let version = issue(&mut ada, &key).await?["version"].clone();
        // Lea is the person on the bar, but her key may only look.
        let factor = lea
            .script(
                "PUT",
                &format!("{ISSUES}/{key}/time-factor"),
                Some(json!({"version": version, "percent": 150})),
                &secret,
            )
            .await?;
        assert_eq!(factor.code(), "access.forbidden", "{}", factor.body);
        let accept = lea
            .script(
                "POST",
                &format!("{TIMELINE}/accepted-warnings"),
                Some(json!({"issue": key, "version": version, "kind": "away", "timeline": {"zoom": "week"}})),
                &secret,
            )
            .await?;
        assert_eq!(accept.code(), "access.forbidden", "{}", accept.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_10_a_key_reads_no_work_times_through_the_timeline() -> TestResult {
    each_database("plan_10_key_no_work_times", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site.person(StartingRole::TeamMember, "lea").await?;
        work_times(&site, &mut ada, lea_id, 360, 50).await?;
        ok(&lea
            .send(
                "POST",
                "/api/v1/me/away-times",
                Some(json!({"first": "2027-03-02", "last": "2027-03-02"})),
            )
            .await?)?;
        let people = json!([lea_id.to_string()]);
        let fixed = planned(
            &site,
            &mut ada,
            "Fixed",
            json!({"start_date": MONDAY, "due_date": "2027-03-05", "assigned_to": people}),
        )
        .await?;
        let effort = planned(
            &site,
            &mut ada,
            "By effort",
            json!({"start_date": MONDAY, "estimated_time": 540, "assigned_to": people}),
        )
        .await?;
        let body = by_effort(&site, MONDAY, "2027-03-31");
        // In a browser Ada sees the away day, the warning and the bar by effort.
        let seen = page(&mut ada, body.clone()).await?;
        assert_eq!(seen["planned_by_effort"], true);
        assert_eq!(
            rows_of(&seen, &fixed)[0]["bar"]["hatched"][0]["kind"],
            "away"
        );
        // With a key: nobody's work times. No hatched day, no Away, and bars from the set dates.
        let secret = see_only_key(&mut ada).await?;
        let keyed = ada.script("POST", TIMELINE, Some(body), &secret).await?;
        let keyed = ok(&keyed)?.clone();
        assert_eq!(keyed["planned_by_effort"], false, "{keyed}");
        assert_eq!(rows_of(&keyed, &fixed)[0]["bar"]["hatched"], json!([]));
        assert_eq!(keyed["warnings"], json!([]));
        let bar = &rows_of(&keyed, &effort)[0]["bar"]["days"];
        assert_eq!(bar, &json!({"first": MONDAY, "last": MONDAY}));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_13_a_preview_and_an_accepted_warning_tell_no_hidden_date() -> TestResult {
    each_database("plan_13_hidden_dates", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (first, then) = overlapping(&site, &mut ada).await?;
        blocks(&mut ada, &first, &then).await?;
        field_level(&site, &mut ada, StartingRole::TeamMember, "due-date", "hidden").await?;
        let (mut team, _) = site.person(StartingRole::TeamMember, "team").await?;
        let preview = team
            .send(
                "POST",
                &format!("{TIMELINE}/move-after"),
                Some(json!({"issue": then, "after": "2027-03-04", "timeline": {"zoom": "week"}})),
            )
            .await?;
        let moves = ok(&preview)?["issues"].clone();
        let first_move = &moves[0];
        assert_eq!(first_move["start_date"], "2027-03-05", "{moves}");
        assert!(first_move.get("due_date").is_none_or(Value::is_null), "{moves}");
        // Keep and accept answers no date at all.
        let version = issue(&mut ada, &then).await?["version"].clone();
        let accepted = ada
            .send(
                "POST",
                &format!("{TIMELINE}/accepted-warnings"),
                Some(json!({"issue": then, "version": version, "kind": "delay", "timeline": {"zoom": "week"}})),
            )
            .await?;
        let answer = ok(&accepted)?;
        assert!(
            answer.get("start_date").is_none() && answer.get("due_date").is_none(),
            "{answer}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_14_a_length_tells_nothing_of_a_hidden_estimate() -> TestResult {
    each_database("plan_14_hidden_estimate", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_lea, lea_id) = site.person(StartingRole::TeamMember, "lea").await?;
        let people = json!([lea_id.to_string()]);
        let with = planned(
            &site,
            &mut ada,
            "Estimated",
            json!({"estimated_time": 480, "assigned_to": people}),
        )
        .await?;
        let without = planned(
            &site,
            &mut ada,
            "Not estimated",
            json!({"assigned_to": people}),
        )
        .await?;
        field_level(
            &site,
            &mut ada,
            StartingRole::TeamMember,
            "estimated-time",
            "hidden",
        )
        .await?;
        let (mut team, _) = site.person(StartingRole::TeamMember, "team").await?;
        let mut answers = Vec::new();
        for key in [&with, &without] {
            let version = team
                .send("GET", &format!("{ISSUES}/{key}/time-factor"), None)
                .await?
                .body["version"]
                .clone();
            let set = team
                .send(
                    "PUT",
                    &format!("{ISSUES}/{key}/time-factor"),
                    Some(json!({"version": version, "takes_minutes": 600})),
                )
                .await?;
            answers.push((set.status, set.code().to_owned()));
        }
        // The same refusal, whether or not there is an estimate.
        assert_eq!(answers[0], answers[1]);
        assert_eq!(answers[0].1, "request.invalid");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_10_a_person_on_the_bar_always_sees_their_own_work_times_and_factor() -> TestResult {
    each_database("plan_10_own_always", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site.person(StartingRole::Contributor, "lea").await?;
        ok(&lea
            .send(
                "POST",
                "/api/v1/me/away-times",
                Some(json!({"first": "2027-03-02", "last": "2027-03-02"})),
            )
            .await?)?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": MONDAY, "due_date": "2027-03-05", "estimated_time": 480, "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        // Her role sees nobody's work times and no time factor; she is the person on the bar.
        site.show_people(&mut ada, StartingRole::Contributor, "work-times", "hidden")
            .await?;
        field_level(&site, &mut ada, StartingRole::Contributor, "time-factor", "hidden").await?;
        let version = issue(&mut lea, &key).await?["version"].clone();
        let set = lea
            .send(
                "PUT",
                &format!("{ISSUES}/{key}/time-factor"),
                Some(json!({"version": version, "percent": 150})),
            )
            .await?;
        assert_eq!(ok(&set)?["own_percent"], 150, "{}", set.body);
        let read = lea
            .send("GET", &format!("{ISSUES}/{key}/time-factor"), None)
            .await?;
        assert_eq!(ok(&read)?["own_percent"], 150);
        let found = page(&mut lea, ask(&site, MONDAY, "2027-03-31", json!({}))).await?;
        let bar = &rows_of(&found, &key)[0]["bar"];
        assert_eq!(bar["hatched"][0]["kind"], "away", "{bar}");
        assert_eq!(bar["time_factor"]["percent"], 150, "{bar}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_13_a_hidden_person_on_the_bar_is_not_named_through_the_factor_or_an_acceptance()
-> TestResult {
    each_database("plan_13_hidden_person_factor", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site
            .person_called(StartingRole::TeamMember, "lea", "Lea Lang")
            .await?;
        ok(&lea
            .send(
                "POST",
                "/api/v1/me/away-times",
                Some(json!({"first": "2027-03-02", "last": "2027-03-02"})),
            )
            .await?)?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": MONDAY, "due_date": "2027-03-05", "estimated_time": 480, "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        let version = issue(&mut lea, &key).await?["version"].clone();
        ok(&lea
            .send(
                "PUT",
                &format!("{ISSUES}/{key}/time-factor"),
                Some(json!({"version": version, "percent": 150})),
            )
            .await?)?;
        // A second issue of hers whose factor Ada set.
        let by_ada = planned(
            &site,
            &mut ada,
            "Logout screen",
            json!({"start_date": MONDAY, "due_date": "2027-03-05", "estimated_time": 480, "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        let version = issue(&mut ada, &by_ada).await?["version"].clone();
        ok(&ada
            .send(
                "PUT",
                &format!("{ISSUES}/{by_ada}/time-factor"),
                Some(json!({"version": version, "percent": 150})),
            )
            .await?)?;
        let version = issue(&mut lea, &key).await?["version"].clone();
        ok(&lea
            .send(
                "POST",
                &format!("{TIMELINE}/accepted-warnings"),
                Some(json!({"issue": key, "version": version, "kind": "away", "timeline": {"zoom": "week"}})),
            )
            .await?)?;
        site.show_people(&mut ada, StartingRole::Contributor, "assigned-to", "hidden")
            .await?;
        let (mut contributor, _) = site
            .person(StartingRole::Contributor, "contributor")
            .await?;
        // Who set a factor is told no more openly than the person on the bar: the factor she set
        // and the one Ada set look alike.
        let mut factors = Vec::new();
        for each in [&key, &by_ada] {
            let factor = contributor
                .send("GET", &format!("{ISSUES}/{each}/time-factor"), None)
                .await?;
            let factor = ok(&factor)?.clone();
            assert_eq!(factor["own_estimate"], false, "{factor}");
            assert_eq!(factor["set_by"], json!({"shown": "someone"}), "{factor}");
            factors.push((factor["own_estimate"].clone(), factor["set_by"].clone()));
        }
        assert_eq!(factors[0], factors[1]);
        let found = page(&mut contributor, ask(&site, MONDAY, "2027-03-31", json!({}))).await?;
        let id = id_of_key(&mut ada, &key).await?;
        let away = warnings_on(&found, &id)[0].clone();
        assert!(away["accepted"].is_object(), "{away}");
        assert!(away["accepted"]["by"].get("name").is_none(), "{away}");
        assert_eq!(rows_of(&found, &key)[0]["bar"]["time_factor"]["own_estimate"], false);
        assert!(!found.to_string().contains("Lea Lang"), "{found}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_13_accepting_a_warning_reads_work_times_around_dates_the_reader_may_read()
-> TestResult {
    each_database("plan_13_accept_window", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site.person(StartingRole::TeamMember, "lea").await?;
        // Away on the due date, which is more than a year after the start date.
        ok(&lea
            .send(
                "POST",
                "/api/v1/me/away-times",
                Some(json!({"first": "2028-06-01", "last": "2028-06-01"})),
            )
            .await?)?;
        let key = planned(
            &site,
            &mut ada,
            "Long work",
            json!({"start_date": MONDAY, "due_date": "2028-06-01", "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        // Her role may not read the start date: her bar is the due date, and the work times are
        // read around it, not around the start date she may not read.
        field_level(&site, &mut ada, StartingRole::TeamMember, "start-date", "hidden").await?;
        let version = issue(&mut lea, &key).await?["version"].clone();
        let accepted = lea
            .send(
                "POST",
                &format!("{TIMELINE}/accepted-warnings"),
                Some(json!({"issue": key, "version": version, "kind": "away", "timeline": {"zoom": "week"}})),
            )
            .await?;
        assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.body);
        Ok(())
    })
    .await
}

/// A new own date field for issues, as Ada: its ID.
async fn date_field(ada: &mut Browser, name: &str) -> Result<String, Failure> {
    let made = ada
        .send(
            "POST",
            super::fields_http::FIELDS,
            Some(json!({"name": name, "kind": "date", "applies_to": "issues"})),
        )
        .await?;
    Ok(ok(&made)?["id"].as_str().ok_or("no ID")?.to_owned())
}

/// Gives an issue these values of own fields, as Ada.
async fn own_values(ada: &mut Browser, key: &str, values: Value) -> Result<(), Failure> {
    let version = issue(ada, key).await?["version"].clone();
    ok(&ada
        .send(
            "PATCH",
            &format!("{ISSUES}/{key}"),
            Some(json!({"version": version, "fields": values})),
        )
        .await?)?;
    Ok(())
}

#[tokio::test]
async fn plan_08_an_own_date_hidden_where_an_issue_around_the_page_is_counts_as_empty() -> TestResult
{
    each_database("plan_08_own_date_around", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let kickoff = date_field(&mut ada, "Kickoff").await?;
        let launch = date_field(&mut ada, "Launch").await?;
        let beta = ok(&ada
            .send(
                "POST",
                super::issues_site::PROJECTS,
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project ID")?
            .to_owned();
        let around = super::issues_site::key_of(
            &ada.send(
                "POST",
                &format!("{}/{beta}/issues", super::issues_site::PROJECTS),
                Some(json!({"issue_type": site.kind, "title": "Around"})),
            )
            .await?,
        )?;
        let shown = planned(&site, &mut ada, "Shown", json!({})).await?;
        own_values(
            &mut ada,
            &around,
            json!({&kickoff: MONDAY, &launch: "2027-03-04"}),
        )
        .await?;
        own_values(
            &mut ada,
            &shown,
            json!({&kickoff: "2027-03-03", &launch: "2027-03-05"}),
        )
        .await?;
        blocks(&mut ada, &around, &shown).await?;
        let body = ask(
            &site,
            MONDAY,
            "2027-03-31",
            json!({"timeline": {"zoom": "week", "start_field": "Kickoff", "end_field": "Launch"}}),
        );
        let shown_id = id_of_key(&mut ada, &shown).await?;
        // Ada reads both dates of the issue around the page: it ends after the shown one starts.
        let all = page(&mut ada, body.clone()).await?;
        assert_eq!(warnings_on(&all, &shown_id).len(), 1, "{all}");
        // A Team member who may not read *Launch* in BETA: the issue there has no end, so no
        // delay tells when it ends.
        let (mut team, _) = site.person(StartingRole::TeamMember, "team").await?;
        let hide = ada
            .send(
                "PUT",
                &format!(
                    "/api/v1/field-permissions/{}/{launch}?project={beta}",
                    site.role_id(StartingRole::TeamMember)
                ),
                Some(json!({"level": "hidden"})),
            )
            .await?;
        assert_eq!(hide.status, StatusCode::NO_CONTENT, "{}", hide.body);
        let theirs = page(&mut team, body).await?;
        assert!(warnings_on(&theirs, &shown_id).is_empty(), "{theirs}");
        Ok(())
    })
    .await
}
