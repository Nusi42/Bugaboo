//! Deadline drivers, the warnings *Loop* and *Cannot fit*, and releases on the timeline over HTTP
//! (PLAN-04, PLAN-05, ISS-08, ADR-0043 amendment of 2026-11-24), on SQLite and PostgreSQL. The
//! rules themselves are tables of cases in `bb_core::plan::drivers`.

use bb_app::access::StartingRole;
use bb_core::id::{IssueId, RelationId, RelationKindId};
use bb_core::relation::Relation;
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{PROJECTS, Site, key_of, ok, text_in};
use super::plan_site::{
    MONDAY, TIMELINE, ask, blocks, by_effort, change, id_of_key, issue, page, planned, under,
    warnings_on, work_times,
};
use super::views_site::{acme_board, board};

/// The last day of every page of these tests.
const LAST: &str = "2027-03-31";

/// The options of a timeline with *Deadline drivers* on.
fn drivers_on() -> Value {
    json!({"timeline": {"zoom": "week", "deadline_drivers": true}})
}

/// The ends each issue of a page sets, by its internal ID: `goal:<id>`, `release:<id>` or `view`.
fn ends_of(page: &Value, id: &str) -> Vec<String> {
    page["drivers"]
        .as_array()
        .and_then(|all| all.iter().find(|driver| driver["issue"] == id))
        .and_then(|driver| driver["ends"].as_array())
        .map(|ends| {
            ends.iter()
                .map(|end| match end["id"].as_str() {
                    Some(id) => format!("{}:{id}", end["kind"].as_str().unwrap_or_default()),
                    None => end["kind"].as_str().unwrap_or_default().to_owned(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The internal IDs of the drivers of a page, sorted.
fn driver_ids(page: &Value) -> Vec<String> {
    let mut ids: Vec<String> = page["drivers"]
        .as_array()
        .map(|all| {
            all.iter()
                .filter_map(|driver| driver["issue"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    ids.sort();
    ids
}

/// The internal IDs of these readable IDs, sorted.
async fn sorted_ids(browser: &mut Browser, keys: &[&str]) -> Result<Vec<String>, Failure> {
    let mut ids = Vec::with_capacity(keys.len());
    for key in keys {
        ids.push(id_of_key(browser, key).await?);
    }
    ids.sort();
    Ok(ids)
}

/// Makes a release in ACME with this name and *Release date*; its ID.
async fn release(
    site: &Site<'_>,
    browser: &mut Browser,
    name: &str,
    date: &str,
) -> Result<String, Failure> {
    let made = browser
        .send(
            "POST",
            &format!("{PROJECTS}/{}/releases", site.acme),
            Some(json!({"name": name, "release_date": date})),
        )
        .await?;
    text_in(ok(&made)?, "id")
}

#[tokio::test]
async fn plan_05_the_chain_that_sets_the_end_of_a_goal_is_the_drivers() -> TestResult {
    each_database("plan_05_goal_drivers", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let goal = planned(
            &site,
            &mut ada,
            "Mobile app 2.0",
            json!({"due_date": "2027-03-12"}),
        )
        .await?;
        let dated = |first: &str, last: &str| json!({"start_date": first, "due_date": last});
        let api = planned(&site, &mut ada, "API", dated(MONDAY, "2027-03-03")).await?;
        let early = planned(&site, &mut ada, "Design", dated(MONDAY, "2027-03-02")).await?;
        let login = planned(&site, &mut ada, "Login", dated("2027-03-04", "2027-03-10")).await?;
        let menu = planned(&site, &mut ada, "Menu", dated(MONDAY, "2027-03-05")).await?;
        let goal_id = id_of_key(&mut ada, &goal).await?;
        under(&mut ada, &login, &goal_id).await?;
        under(&mut ada, &menu, &goal_id).await?;
        // *API* ends the day before *Login* starts: no slack. *Design* ends two days before.
        blocks(&mut ada, &api, &login).await?;
        blocks(&mut ada, &early, &login).await?;
        let on = page(&mut ada, ask(&site, MONDAY, LAST, drivers_on())).await?;
        let goal_end = format!("goal:{goal_id}");
        for key in [&api, &login] {
            let id = id_of_key(&mut ada, key).await?;
            assert_eq!(
                ends_of(&on, &id),
                std::slice::from_ref(&goal_end),
                "{key}: {on}"
            );
        }
        for key in [&early, &menu] {
            let id = id_of_key(&mut ada, key).await?;
            assert!(ends_of(&on, &id).is_empty(), "{key}: {on}");
        }
        // The dependency with no slack is a link of the chain; the one with slack is not.
        let (api_id, early_id) = (
            id_of_key(&mut ada, &api).await?,
            id_of_key(&mut ada, &early).await?,
        );
        let drives = |first: &str| {
            on["dependencies"]
                .as_array()
                .and_then(|all| all.iter().find(|each| each["first"] == first))
                .map(|each| each["drives"].clone())
        };
        assert_eq!(drives(&api_id), Some(json!(true)), "{on}");
        assert_eq!(drives(&early_id), Some(json!(false)), "{on}");
        // The goal's own day is the last of the view.
        assert_eq!(ends_of(&on, &goal_id), ["view"], "{on}");
        // Off, the page names no driver.
        let off = page(&mut ada, ask(&site, MONDAY, LAST, json!({}))).await?;
        assert_eq!(off["drivers"], json!([]));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_05_drivers_are_worked_out_with_plan_by_effort_when_it_is_on() -> TestResult {
    each_database("plan_05_drivers_by_effort", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, lea_id) = site.person(StartingRole::TeamMember, "lea").await?;
        // 8 hours, 100 %: 24 hours are three days.
        work_times(&site, &mut ada, lea_id, 480, 100).await?;
        let estimated = planned(
            &site,
            &mut ada,
            "Estimated",
            json!({"start_date": MONDAY, "estimated_time": 1440, "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        let then = planned(
            &site,
            &mut ada,
            "Then",
            json!({"start_date": "2027-03-04", "due_date": "2027-03-04"}),
        )
        .await?;
        blocks(&mut ada, &estimated, &then).await?;
        let mut effort = by_effort(&site, MONDAY, LAST);
        effort["timeline"]["deadline_drivers"] = json!(true);
        // By effort *Estimated* ends on Wednesday, the day before *Then* starts: both drive.
        let planned_by_effort = page(&mut ada, effort).await?;
        assert_eq!(
            driver_ids(&planned_by_effort),
            sorted_ids(&mut ada, &[&estimated, &then]).await?,
            "{planned_by_effort}"
        );
        // Without it, *Estimated* is one day on Monday, with slack before *Then*.
        let plain = page(&mut ada, ask(&site, MONDAY, LAST, drivers_on())).await?;
        assert_eq!(
            driver_ids(&plain),
            sorted_ids(&mut ada, &[&then]).await?,
            "{plain}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_05_no_driver_chain_passes_through_an_issue_the_reader_cannot_see() -> TestResult {
    each_database("plan_05_drivers_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let secret = text_in(
            ok(&ada
                .send(
                    "POST",
                    PROJECTS,
                    Some(json!({"short_name": "SEC", "name": "Secret"})),
                )
                .await?)?,
            "id",
        )?;
        let first = planned(
            &site,
            &mut ada,
            "First",
            json!({"start_date": MONDAY, "due_date": "2027-03-02"}),
        )
        .await?;
        let hidden = key_of(
            &ada.send(
                "POST",
                &format!("{PROJECTS}/{secret}/issues"),
                Some(json!({
                    "issue_type": site.kind,
                    "title": "Hidden work",
                    "start_date": "2027-03-03",
                    "due_date": "2027-03-04",
                })),
            )
            .await?,
        )?;
        let last = planned(
            &site,
            &mut ada,
            "Last",
            json!({"start_date": "2027-03-05", "due_date": "2027-03-05"}),
        )
        .await?;
        blocks(&mut ada, &first, &hidden).await?;
        blocks(&mut ada, &hidden, &last).await?;
        let everywhere = json!({
            "search_text": "",
            "timeline": {"zoom": "week", "deadline_drivers": true},
            "first": MONDAY,
            "last": LAST,
        });
        // Ada sees the whole chain.
        let all = page(&mut ada, everywhere.clone()).await?;
        assert_eq!(
            driver_ids(&all),
            sorted_ids(&mut ada, &[&first, &hidden, &last]).await?,
            "{all}"
        );
        // A Team member of ACME only: the chain stops at the issue they cannot see, and nothing
        // tells that it goes on.
        let (mut team, _) = site
            .person_in_acme(StartingRole::TeamMember, "team")
            .await?;
        let theirs = page(&mut team, everywhere).await?;
        assert_eq!(
            driver_ids(&theirs),
            sorted_ids(&mut ada, &[&last]).await?,
            "{theirs}"
        );
        assert!(!theirs.to_string().contains(hidden.as_str()), "{theirs}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_05_a_stored_loop_of_dependencies_shows_the_warning_loop() -> TestResult {
    each_database("plan_05_loop", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let one = planned(
            &site,
            &mut ada,
            "One",
            json!({"start_date": MONDAY, "due_date": "2027-03-02"}),
        )
        .await?;
        let other = planned(
            &site,
            &mut ada,
            "Other",
            json!({"start_date": "2027-03-03", "due_date": "2027-03-04"}),
        )
        .await?;
        blocks(&mut ada, &one, &other).await?;
        // The way back, stored as a loop was before loops were refused.
        let kinds = ok(&ada.send("GET", "/api/v1/relation-kinds", None).await?)?.clone();
        let blocks_kind = kinds["items"]
            .as_array()
            .and_then(|items| items.iter().find(|kind| kind["forward"] == "blocks"))
            .and_then(|kind| kind["id"].as_str())
            .ok_or("no kind blocks")?;
        let (one_id, other_id) = (
            id_of_key(&mut ada, &one).await?,
            id_of_key(&mut ada, &other).await?,
        );
        rig.database
            .store_relation_as_it_is(&Relation {
                id: RelationId::from_parts(Timestamp::UNIX_EPOCH, [7; 10])?,
                kind: blocks_kind.parse::<RelationKindId>()?,
                from: other_id.parse::<IssueId>()?,
                to: one_id.parse::<IssueId>()?,
                added_by: site.org.admin.id,
                added_at: Timestamp::UNIX_EPOCH,
            })
            .await?;
        let found = page(&mut ada, ask(&site, MONDAY, LAST, json!({}))).await?;
        for (on, waits_for) in [(&one_id, &other), (&other_id, &one)] {
            let warnings = warnings_on(&found, on);
            let looped = warnings
                .iter()
                .find(|warning| warning["kind"] == "loop")
                .ok_or_else(|| format!("no loop on {on}: {found}"))?;
            assert_eq!(looped["sentence"], "plan-warning.loop");
            assert_eq!(looped["other"]["readable_id"], waits_for.as_str());
            assert_eq!(
                looped["ways_out"],
                json!(["open-the-relation", "keep-and-accept"])
            );
        }
        // *Keep and accept* keeps it faded, as every warning.
        let version = issue(&mut ada, &one).await?["version"].clone();
        let accepted = ada
            .send(
                "POST",
                &format!("{TIMELINE}/accepted-warnings"),
                Some(json!({"issue": one, "version": version, "kind": "loop", "timeline": {"zoom": "week"}})),
            )
            .await?;
        ok(&accepted)?;
        let faded = page(&mut ada, ask(&site, MONDAY, LAST, json!({}))).await?;
        let kept = warnings_on(&faded, &one_id)
            .into_iter()
            .find(|warning| warning["kind"] == "loop")
            .ok_or("no loop")?;
        assert!(kept["accepted"]["by"].is_object(), "{kept}");
        Ok(())
    })
    .await
}

/// *API* from Monday to Thursday must finish before *Login screen*, which is due on Wednesday:
/// their readable IDs.
async fn cannot_fit(site: &Site<'_>, ada: &mut Browser) -> Result<(String, String), Failure> {
    let first = planned(
        site,
        ada,
        "API",
        json!({"start_date": MONDAY, "due_date": "2027-03-04"}),
    )
    .await?;
    let then = planned(
        site,
        ada,
        "Login screen",
        json!({"start_date": MONDAY, "due_date": "2027-03-03"}),
    )
    .await?;
    blocks(ada, &first, &then).await?;
    Ok((first, then))
}

#[tokio::test]
async fn plan_05_a_due_date_before_what_it_waits_for_can_end_shows_cannot_fit() -> TestResult {
    each_database("plan_05_cannot_fit", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (first, then) = cannot_fit(&site, &mut ada).await?;
        let found = page(&mut ada, ask(&site, MONDAY, LAST, json!({}))).await?;
        let then_id = id_of_key(&mut ada, &then).await?;
        let warnings = warnings_on(&found, &then_id);
        let short = warnings
            .iter()
            .find(|warning| warning["kind"] == "cannot-fit")
            .ok_or_else(|| format!("no cannot-fit: {found}"))?;
        assert_eq!(short["sentence"], "plan-warning.cannot-fit");
        assert_eq!(short["other"]["readable_id"], first.as_str());
        assert_eq!(short["waits_until"], "2027-03-04");
        assert_eq!(short["days"]["last"], "2027-03-03");
        // Three days after the day *API* ends.
        assert_eq!(short["due_needed"], "2027-03-07");
        assert_eq!(
            short["ways_out"],
            json!(["make-it-longer", "keep-and-accept"])
        );
        let first_id = id_of_key(&mut ada, &first).await?;
        assert!(warnings_on(&found, &first_id).is_empty(), "{found}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_05_make_it_longer_moves_the_due_date_only_when_the_person_confirms() -> TestResult {
    each_database("plan_05_make_it_longer", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, then) = cannot_fit(&site, &mut ada).await?;
        let then_id = id_of_key(&mut ada, &then).await?;
        let body = ask(&site, MONDAY, LAST, json!({}));
        let found = page(&mut ada, body.clone()).await?;
        let needed = warnings_on(&found, &then_id)
            .into_iter()
            .find(|warning| warning["kind"] == "cannot-fit")
            .map(|warning| warning["due_needed"].clone())
            .ok_or("no cannot-fit")?;
        // Reading the plan, and reading it again, moves nothing.
        page(&mut ada, body.clone()).await?;
        assert_eq!(issue(&mut ada, &then).await?["due_date"], "2027-03-03");
        // The person confirms the due date it needs: the usual change of the issue.
        ok(&change(&mut ada, &then, json!({"due_date": needed})).await?)?;
        assert_eq!(issue(&mut ada, &then).await?["due_date"], needed);
        let after = page(&mut ada, body).await?;
        assert!(
            warnings_on(&after, &then_id)
                .iter()
                .all(|warning| warning["kind"] != "cannot-fit"),
            "{after}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_04_rows_group_by_planned_for_release_with_none_last() -> TestResult {
    // The group of none is put last by the screen over all pages (`linesOf`, its test
    // `plan_04_none_comes_last_over_two_pages`); the answer gives each row its group.
    each_database("plan_04_group_by_release", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let next = release(&site, &mut ada, "2.5", "2027-03-10").await?;
        let dated = json!({"start_date": MONDAY, "due_date": "2027-03-02"});
        // The issue without a release comes first in the order of the search.
        let loose = planned(&site, &mut ada, "Loose end", dated.clone()).await?;
        let shipped = planned(&site, &mut ada, "Shipped", dated).await?;
        ok(&change(&mut ada, &shipped, json!({"planned_for_release": next})).await?)?;
        let grouped = page(
            &mut ada,
            ask(
                &site,
                MONDAY,
                LAST,
                json!({"group_by": "planned for release"}),
            ),
        )
        .await?;
        // Each row in the group of its release, or of none; the screen shows that group last.
        let group_of = |key: &str| {
            super::plan_site::rows_of(&grouped, key)
                .first()
                .map(|row| row["group"].clone())
        };
        assert_eq!(
            group_of(&shipped),
            Some(json!({"type": "release", "value": next})),
            "{grouped}"
        );
        assert_eq!(
            group_of(&loose),
            Some(json!({"type": "empty"})),
            "{grouped}"
        );
        // A board groups its rows by it too.
        let rows = board(
            &mut ada,
            acme_board(&site, &json!({"rows": "planned for release"})),
        )
        .await?;
        let mut keys: Vec<String> = rows["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|row| row["key"]["type"].as_str().unwrap_or_default().to_owned())
            .collect();
        keys.sort();
        assert_eq!(keys, ["empty", "release"], "{rows}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_08_a_release_with_a_release_date_is_a_diamond_on_the_timeline() -> TestResult {
    each_database("iss_08_release_diamond", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let next = release(&site, &mut ada, "2.5", "2027-03-10").await?;
        let work = planned(
            &site,
            &mut ada,
            "Work",
            json!({"start_date": MONDAY, "due_date": "2027-03-02"}),
        )
        .await?;
        ok(&change(&mut ada, &work, json!({"planned_for_release": next})).await?)?;
        let found = page(&mut ada, ask(&site, MONDAY, LAST, json!({}))).await?;
        let releases = found["releases"].as_array().ok_or("no releases")?;
        assert_eq!(releases.len(), 1, "{found}");
        assert_eq!(releases[0]["id"], next.as_str());
        assert_eq!(releases[0]["name"], "2.5");
        assert_eq!(releases[0]["release_date"], "2027-03-10");
        // Its end is one the drivers set.
        let on = page(&mut ada, ask(&site, MONDAY, LAST, drivers_on())).await?;
        let work_id = id_of_key(&mut ada, &work).await?;
        assert!(
            ends_of(&on, &work_id).contains(&format!("release:{next}")),
            "{on}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_08_an_issue_that_ends_after_its_release_date_shows_delay() -> TestResult {
    each_database("iss_08_release_delay", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let next = release(&site, &mut ada, "2.5", "2027-03-10").await?;
        let late = planned(
            &site,
            &mut ada,
            "Late",
            json!({"start_date": "2027-03-08", "due_date": "2027-03-12"}),
        )
        .await?;
        ok(&change(&mut ada, &late, json!({"planned_for_release": next})).await?)?;
        let late_id = id_of_key(&mut ada, &late).await?;
        let body = ask(&site, MONDAY, LAST, json!({}));
        let found = page(&mut ada, body.clone()).await?;
        let warnings = warnings_on(&found, &late_id);
        assert_eq!(warnings.len(), 1, "{found}");
        let delay = warnings[0];
        assert_eq!(delay["kind"], "delay");
        assert_eq!(delay["sentence"], "plan-warning.delay-release");
        assert_eq!(delay["release"], next.as_str());
        assert_eq!(delay["late_days"], 2);
        assert_eq!(delay["due_needed"], "2027-03-12");
        assert_eq!(
            delay["days"],
            json!({"first": "2027-03-10", "last": "2027-03-12"})
        );
        assert_eq!(
            delay["ways_out"],
            json!(["make-it-longer", "keep-and-accept"])
        );
        // *Make it longer* moves the release date: only for a person who may manage releases.
        let (mut team, _) = site
            .person_in_acme(StartingRole::TeamMember, "team")
            .await?;
        let theirs = page(&mut team, body).await?;
        assert_eq!(
            warnings_on(&theirs, &late_id)[0]["ways_out"],
            json!(["keep-and-accept"]),
            "{theirs}"
        );
        Ok(())
    })
    .await
}
