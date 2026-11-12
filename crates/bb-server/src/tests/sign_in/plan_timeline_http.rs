//! The timeline over HTTP (PLAN-08, PLAN-10, PLAN-12, PLAN-13, ADR-0030 and its amendment M8,
//! ADR-0043): the rows with their bars, worked out once on the server for the reader, plan by
//! effort, the people on the bars as *Who sees people* allows, the warnings and their ways out.
//! Every flow runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::http::rig_and_app;
use super::issues_site::{ISSUES, PROJECTS, Site, key_of, ok};
use super::plan_site::{
    MONDAY, TIMELINE, ask, bar_of, blocks, by_effort, change, id_of_key, issue, page, planned,
    rows_of, warnings_on, work_times,
};

#[tokio::test]
async fn plan_08_a_restricted_issue_has_no_bar_no_arrow_no_warning_and_no_count() -> TestResult {
    each_database("plan_08_restricted", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // A second project the reader has no role in.
        let secret = ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "SEC", "name": "Secret"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project ID")?
            .to_owned();
        let hidden = key_of(
            &ada.send(
                "POST",
                &format!("{PROJECTS}/{secret}/issues"),
                Some(json!({
                    "issue_type": site.kind,
                    "title": "Hidden work",
                    "start_date": MONDAY,
                    "due_date": "2027-03-05",
                })),
            )
            .await?,
        )?;
        let undated_hidden = key_of(
            &ada.send(
                "POST",
                &format!("{PROJECTS}/{secret}/issues"),
                Some(json!({"issue_type": site.kind, "title": "Hidden idea"})),
            )
            .await?,
        )?;
        let shown = planned(
            &site,
            &mut ada,
            "Shown work",
            json!({"start_date": "2027-03-03", "due_date": "2027-03-04"}),
        )
        .await?;
        planned(&site, &mut ada, "Shown idea", json!({})).await?;
        // The hidden issue must finish first, and ends after the shown one starts.
        blocks(&mut ada, &hidden, &shown).await?;
        let everywhere = json!({
            "search_text": "",
            "timeline": {"zoom": "week"},
            "first": MONDAY,
            "last": "2027-03-31",
        });
        // Ada sees both projects: two rows, the arrow, the delay, two waiting.
        let all = page(&mut ada, everywhere.clone()).await?;
        assert_eq!(all["rows"].as_array().map(Vec::len), Some(2), "{all}");
        assert_eq!(all["dependencies"].as_array().map(Vec::len), Some(1));
        assert_eq!(all["not_scheduled"], 2);
        let shown_id = id_of_key(&mut ada, &shown).await?;
        assert_eq!(warnings_on(&all, &shown_id).len(), 1, "{all}");
        // A Team member of ACME only: the hidden issue has no bar, no arrow, no warning, no count.
        let (mut team, _) = site
            .person_in_acme(StartingRole::TeamMember, "team")
            .await?;
        let theirs = page(&mut team, everywhere).await?;
        let keys: Vec<&str> = theirs["rows"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| row["issue"]["readable_id"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(keys, [shown.as_str()], "{theirs}");
        assert_eq!(theirs["dependencies"], json!([]));
        assert_eq!(theirs["warnings"], json!([]));
        assert_eq!(theirs["not_scheduled"], 1);
        let text = theirs.to_string();
        for key in [&hidden, &undated_hidden] {
            assert!(!text.contains(key.as_str()), "{key} in {text}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_10_the_timeline_shows_people_only_as_who_sees_people_allows() -> TestResult {
    each_database("plan_10_who_sees_people", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_lea, lea_id) = site
            .person_called(StartingRole::TeamMember, "lea", "Lea Lang")
            .await?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": MONDAY, "due_date": "2027-03-05", "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        let body = ask(&site, MONDAY, "2027-03-31", json!({}));
        let person = |page: &Value| rows_of(page, &key)[0]["bar"]["person"].clone();
        // By name for a Team member.
        let (mut team, _) = site.person(StartingRole::TeamMember, "team").await?;
        let named = person(&page(&mut team, body.clone()).await?);
        assert_eq!(named["name"], "Lea Lang", "{named}");
        // As a role for a Contributor who sees Assigned to as Role only; nobody with Hidden.
        let (mut contributor, _) = site
            .person(StartingRole::Contributor, "contributor")
            .await?;
        site.show_people(&mut ada, StartingRole::Contributor, "assigned-to", "role-only")
            .await?;
        let role = person(&page(&mut contributor, body.clone()).await?);
        assert!(role.get("name").is_none() && role.get("id").is_none(), "{role}");
        assert!(role.get("role").is_some(), "{role}");
        // With Role only there is no grouping by person: one row, no group.
        let grouped = page(
            &mut contributor,
            ask(&site, MONDAY, "2027-03-31", json!({"group_by": "assigned to"})),
        )
        .await?;
        let rows = rows_of(&grouped, &key);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["group"], Value::Null);
        site.show_people(&mut ada, StartingRole::Contributor, "assigned-to", "hidden")
            .await?;
        let nobody = person(&page(&mut contributor, body).await?);
        assert_eq!(nobody, Value::Null);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_10_the_first_assigned_person_shapes_the_bar() -> TestResult {
    each_database("plan_10_first_person", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_lea, lea) = site.person(StartingRole::TeamMember, "lea").await?;
        let (_ben, ben) = site.person(StartingRole::TeamMember, "ben").await?;
        work_times(&site, &mut ada, lea, 360, 50).await?;
        work_times(&site, &mut ada, ben, 480, 100).await?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({
                "start_date": MONDAY,
                "estimated_time": 540,
                "assigned_to": [lea.to_string(), ben.to_string()],
            }),
        )
        .await?;
        let body = by_effort(&site, MONDAY, "2027-03-31");
        // Lea first: 3 hours a day, 9 hours take three days.
        let lea_first = page(&mut ada, body.clone()).await?;
        assert_eq!(
            bar_of(&lea_first, &key),
            Some((MONDAY.to_owned(), "2027-03-03".to_owned()))
        );
        // Ben first: 8 hours a day, two days.
        ok(&change(
            &mut ada,
            &key,
            json!({"assigned_to": [ben.to_string(), lea.to_string()]}),
        )
        .await?)?;
        let ben_first = page(&mut ada, body).await?;
        assert_eq!(
            bar_of(&ben_first, &key),
            Some((MONDAY.to_owned(), "2027-03-02".to_owned()))
        );
        // The issue keeps no date of the plan: only the start date somebody set.
        let stored = issue(&mut ada, &key).await?;
        assert!(stored.get("due_date").is_none(), "{stored}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_10_group_by_person_marks_the_copies_of_other_assigned_people() -> TestResult {
    each_database("plan_10_group_by_person", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_lea, lea) = site.person(StartingRole::TeamMember, "lea").await?;
        let (_ben, ben) = site.person(StartingRole::TeamMember, "ben").await?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({
                "start_date": MONDAY,
                "due_date": "2027-03-03",
                "assigned_to": [lea.to_string(), ben.to_string()],
            }),
        )
        .await?;
        let grouped = page(
            &mut ada,
            ask(
                &site,
                MONDAY,
                "2027-03-31",
                json!({"group_by": "assigned to"}),
            ),
        )
        .await?;
        let rows = rows_of(&grouped, &key);
        assert_eq!(rows.len(), 2, "{grouped}");
        let in_row = |row: &Value| row["group"]["value"]["id"].as_str().map(str::to_owned);
        assert_eq!(in_row(rows[0]), Some(lea.to_string()));
        assert_eq!(rows[0]["copy"], false);
        assert_eq!(in_row(rows[1]), Some(ben.to_string()));
        assert_eq!(rows[1]["copy"], true);
        // Both rows show the same bar, the first person's.
        assert_eq!(rows[0]["bar"], rows[1]["bar"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_12_plan_by_effort_counts_work_hours_share_away_days_and_holidays() -> TestResult {
    each_database("plan_12_by_effort", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site.person(StartingRole::TeamMember, "lea").await?;
        // 6 hours, 50 %, 9 hours: three days.
        work_times(&site, &mut ada, lea_id, 360, 50).await?;
        let people = json!([lea_id.to_string()]);
        let plain = planned(
            &site,
            &mut ada,
            "Plain",
            json!({"start_date": MONDAY, "estimated_time": 540, "assigned_to": people}),
        )
        .await?;
        let body = by_effort(&site, MONDAY, "2027-05-31");
        let first = page(&mut ada, body.clone()).await?;
        assert_eq!(
            bar_of(&first, &plain),
            Some((MONDAY.to_owned(), "2027-03-03".to_owned()))
        );
        assert_eq!(rows_of(&first, &plain)[0]["bar"]["worked_out"], "due");
        // Away on Tuesday: the bar grows by a day, and the day is hatched as Away.
        let away = lea
            .send(
                "POST",
                "/api/v1/me/away-times",
                Some(json!({"first": "2027-03-02", "last": "2027-03-02"})),
            )
            .await?;
        ok(&away)?;
        let with_away = page(&mut ada, body.clone()).await?;
        assert_eq!(
            bar_of(&with_away, &plain),
            Some((MONDAY.to_owned(), "2027-03-04".to_owned()))
        );
        let hatched = &rows_of(&with_away, &plain)[0]["bar"]["hatched"];
        assert_eq!(hatched[0]["date"], "2027-03-02", "{hatched}");
        assert_eq!(hatched[0]["kind"], "away");
        // Ascension Day (6 May 2027) counts nothing: from Wednesday, Thursday is skipped.
        let holiday = planned(
            &site,
            &mut ada,
            "Around a holiday",
            json!({"start_date": "2027-05-05", "estimated_time": 540, "assigned_to": people}),
        )
        .await?;
        let around = page(&mut ada, body).await?;
        assert_eq!(
            bar_of(&around, &holiday),
            Some(("2027-05-05".to_owned(), "2027-05-10".to_owned()))
        );
        // Without plan by effort the issue with one date is one day long.
        let off = page(&mut ada, ask(&site, MONDAY, "2027-05-31", json!({}))).await?;
        assert_eq!(
            bar_of(&off, &plain),
            Some((MONDAY.to_owned(), MONDAY.to_owned()))
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_12_a_fixed_due_date_never_moves() -> TestResult {
    each_database("plan_12_fixed_due", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_lea, lea) = site.person(StartingRole::TeamMember, "lea").await?;
        work_times(&site, &mut ada, lea, 360, 50).await?;
        let key = planned(
            &site,
            &mut ada,
            "Too short",
            json!({
                "start_date": MONDAY,
                "due_date": "2027-03-02",
                "estimated_time": 540,
                "assigned_to": [lea.to_string()],
            }),
        )
        .await?;
        let found = page(&mut ada, by_effort(&site, MONDAY, "2027-03-31")).await?;
        assert_eq!(
            bar_of(&found, &key),
            Some((MONDAY.to_owned(), "2027-03-02".to_owned()))
        );
        let id = id_of_key(&mut ada, &key).await?;
        let warnings = warnings_on(&found, &id);
        assert_eq!(warnings.len(), 1, "{found}");
        let warning = warnings[0];
        assert_eq!(warning["kind"], "not-enough-days");
        assert_eq!(
            (warning["needs_days"].clone(), warning["has_days"].clone()),
            (json!(3), json!(2))
        );
        assert_eq!(warning["due_needed"], "2027-03-03");
        assert_eq!(
            warning["ways_out"],
            json!(["make-it-longer", "give-to-someone-else", "keep-and-accept"])
        );
        // Nothing moved: the due date is the one somebody set.
        assert_eq!(issue(&mut ada, &key).await?["due_date"], "2027-03-02");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_13_a_delay_is_shown_along_dependencies_and_moves_nothing() -> TestResult {
    each_database("plan_13_delay", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let first = planned(
            &site,
            &mut ada,
            "API",
            json!({"start_date": MONDAY, "due_date": "2027-03-04"}),
        )
        .await?;
        let then = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": "2027-03-03", "due_date": "2027-03-05"}),
        )
        .await?;
        blocks(&mut ada, &first, &then).await?;
        let found = page(&mut ada, ask(&site, MONDAY, "2027-03-31", json!({}))).await?;
        let (first_id, then_id) = (
            id_of_key(&mut ada, &first).await?,
            id_of_key(&mut ada, &then).await?,
        );
        assert_eq!(
            found["dependencies"],
            json!([{"first": first_id, "then": then_id}])
        );
        let warnings = warnings_on(&found, &then_id);
        assert_eq!(warnings.len(), 1, "{found}");
        let delay = warnings[0];
        assert_eq!(delay["kind"], "delay");
        assert_eq!(delay["sentence"], "plan-warning.delay");
        assert_eq!(delay["other"]["readable_id"], first.as_str());
        assert_eq!(delay["late_days"], 1);
        assert_eq!(delay["move_after"], "2027-03-04");
        assert_eq!(
            delay["ways_out"],
            json!(["move-it-after", "keep-and-accept"])
        );
        // Nothing moved by itself.
        let waiting = issue(&mut ada, &then).await?;
        assert_eq!(waiting["start_date"], "2027-03-03");
        assert!(warnings_on(&found, &first_id).is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_13_a_sub_issue_after_its_goals_due_date_gives_the_goal_a_delay() -> TestResult {
    each_database("plan_13_goal_delay", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let goal = planned(
            &site,
            &mut ada,
            "Mobile app 2.0",
            json!({"due_date": "2027-03-05"}),
        )
        .await?;
        let sub = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": MONDAY, "due_date": "2027-03-09"}),
        )
        .await?;
        let goal_id = id_of_key(&mut ada, &goal).await?;
        let version = issue(&mut ada, &sub).await?["version"].clone();
        let put = ada
            .send(
                "PUT",
                &format!("{ISSUES}/{sub}/parent"),
                Some(json!({"version": version, "parent": goal_id})),
            )
            .await?;
        ok(&put)?;
        let found = page(&mut ada, ask(&site, MONDAY, "2027-03-31", json!({}))).await?;
        let warnings = warnings_on(&found, &goal_id);
        assert_eq!(warnings.len(), 1, "{found}");
        let delay = warnings[0];
        assert_eq!(delay["kind"], "delay");
        assert_eq!(delay["sentence"], "plan-warning.delay-goal");
        assert_eq!(delay["other"]["readable_id"], sub.as_str());
        assert_eq!(delay["late_days"], 4);
        assert_eq!(delay["due_needed"], "2027-03-09");
        // The goal keeps its due date.
        assert_eq!(issue(&mut ada, &goal).await?["due_date"], "2027-03-05");
        Ok(())
    })
    .await
}

/// Each issue of a preview of *Move it after* as its readable ID and its new dates.
fn named_moves(moves: &Value) -> Vec<String> {
    moves
        .as_array()
        .map(|all| {
            all.iter()
                .map(|each| {
                    format!(
                        "{} {} {}",
                        each["readable_id"].as_str().unwrap_or_default(),
                        each["start_date"].as_str().unwrap_or_default(),
                        each["due_date"].as_str().unwrap_or_default()
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn plan_13_move_it_after_previews_then_moves_as_one_run() -> TestResult {
    each_database("plan_13_move_after", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let first = planned(
            &site,
            &mut ada,
            "API",
            json!({"start_date": MONDAY, "due_date": "2027-03-04"}),
        )
        .await?;
        let then = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": "2027-03-03", "due_date": "2027-03-05"}),
        )
        .await?;
        let after = planned(
            &site,
            &mut ada,
            "Tests",
            json!({"start_date": "2027-03-06", "due_date": "2027-03-06"}),
        )
        .await?;
        blocks(&mut ada, &first, &then).await?;
        blocks(&mut ada, &then, &after).await?;
        let body = ask(&site, MONDAY, "2027-03-31", json!({}));
        let found = page(&mut ada, body.clone()).await?;
        let then_id = id_of_key(&mut ada, &then).await?;
        let move_after = warnings_on(&found, &then_id)[0]["move_after"].clone();
        // The preview names every issue that moves and its new dates; nothing is saved.
        let preview = ada
            .send(
                "POST",
                &format!("{TIMELINE}/move-after"),
                Some(json!({"issue": then, "after": move_after, "timeline": {"zoom": "week"}})),
            )
            .await?;
        let moves = ok(&preview)?["issues"].clone();
        assert_eq!(
            named_moves(&moves),
            [
                format!("{then} 2027-03-05 2027-03-07"),
                format!("{after} 2027-03-08 2027-03-08"),
            ],
            "{moves}"
        );
        assert_eq!(issue(&mut ada, &then).await?["start_date"], "2027-03-03");
        // The move is one run of the dates.
        let issues: Vec<Value> = moves
            .as_array()
            .ok_or("no issues")?
            .iter()
            .map(|each| {
                json!({
                    "issue": each["id"],
                    "version": each["version"],
                    "start_date": each["start_date"],
                    "due_date": each["due_date"],
                })
            })
            .collect();
        let run_answer = ada
            .send(
                "POST",
                &format!("{ISSUES}/move-dates"),
                Some(json!({"issues": issues})),
            )
            .await?;
        let run = ok(&run_answer)?["run"].as_str().ok_or("no run")?.to_owned();
        let calm = page(&mut ada, body.clone()).await?;
        assert!(warnings_on(&calm, &then_id).is_empty(), "{calm}");
        // Undo takes the whole move back in one step.
        let preview = ada
            .send("GET", &format!("/api/v1/runs/{run}/undo"), None)
            .await?;
        let seen = ok(&preview)?["preview_version"].clone();
        let undone = ada
            .send(
                "POST",
                &format!("/api/v1/runs/{run}/undo"),
                Some(json!({"preview_version": seen})),
            )
            .await?;
        assert!(undone.status.is_success(), "{}", undone.body);
        assert_eq!(issue(&mut ada, &then).await?["start_date"], "2027-03-03");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_13_an_accepted_warning_comes_back_when_its_dates_change() -> TestResult {
    each_database("plan_13_accepted", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let first = planned(
            &site,
            &mut ada,
            "API",
            json!({"start_date": MONDAY, "due_date": "2027-03-04"}),
        )
        .await?;
        let then = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": "2027-03-03", "due_date": "2027-03-05"}),
        )
        .await?;
        blocks(&mut ada, &first, &then).await?;
        let body = ask(&site, MONDAY, "2027-03-31", json!({}));
        let then_id = id_of_key(&mut ada, &then).await?;
        let version = issue(&mut ada, &then).await?["version"].clone();
        let accepted = ada
            .send(
                "POST",
                &format!("{TIMELINE}/accepted-warnings"),
                Some(json!({"issue": then, "version": version, "kind": "delay"})),
            )
            .await?;
        ok(&accepted)?;
        // Everybody who sees the issue sees it faded, with who accepted it.
        let (mut team, _) = site.person(StartingRole::TeamMember, "team").await?;
        let faded = page(&mut team, body.clone()).await?;
        let warning = warnings_on(&faded, &then_id)[0].clone();
        assert!(warning["accepted"]["by"].get("name").is_some(), "{warning}");
        // A change of its dates brings it back.
        ok(&change(&mut ada, &then, json!({"due_date": "2027-03-06"})).await?)?;
        let back = page(&mut team, body).await?;
        assert_eq!(warnings_on(&back, &then_id)[0]["accepted"], Value::Null);
        // Accepting on an old version is refused.
        let stale = ada
            .send(
                "POST",
                &format!("{TIMELINE}/accepted-warnings"),
                Some(json!({"issue": then, "version": version, "kind": "delay"})),
            )
            .await?;
        assert_eq!(stale.code(), "edit.version-changed", "{}", stale.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_13_a_hidden_person_is_named_only_as_the_assigned_person() -> TestResult {
    each_database("plan_13_hidden_person", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut lea, lea_id) = site
            .person_called(StartingRole::TeamMember, "lea", "Lea Lang")
            .await?;
        let key = planned(
            &site,
            &mut ada,
            "Login screen",
            json!({"start_date": MONDAY, "due_date": "2027-03-05", "assigned_to": [lea_id.to_string()]}),
        )
        .await?;
        ok(&lea
            .send(
                "POST",
                "/api/v1/me/away-times",
                Some(json!({"first": "2027-03-02", "last": "2027-03-03"})),
            )
            .await?)?;
        let id = id_of_key(&mut ada, &key).await?;
        let body = ask(&site, MONDAY, "2027-03-31", json!({}));
        // By name to a Team member.
        let (mut team, _) = site.person(StartingRole::TeamMember, "team").await?;
        let named = page(&mut team, body.clone()).await?;
        let away = warnings_on(&named, &id)[0].clone();
        assert_eq!(away["kind"], "away");
        assert_eq!(away["sentence"], "plan-warning.away");
        assert_eq!(away["person"]["name"], "Lea Lang");
        assert_eq!(away["days"], json!({"first": "2027-03-02", "last": "2027-03-03"}));
        // A Contributor who may not see who it is: the assigned person is away, nothing more.
        site.show_people(&mut ada, StartingRole::Contributor, "assigned-to", "hidden")
            .await?;
        let (mut contributor, _) = site
            .person(StartingRole::Contributor, "contributor")
            .await?;
        let hidden = page(&mut contributor, body.clone()).await?;
        let nameless = warnings_on(&hidden, &id)[0].clone();
        assert_eq!(nameless["sentence"], "plan-warning.away-hidden");
        assert_eq!(nameless["person"], Value::Null);
        assert!(!hidden.to_string().contains("Lea Lang"), "{hidden}");
        // The hatched days are those of a reader who may know the work times.
        assert_eq!(
            rows_of(&named, &key)[0]["bar"]["hatched"][0]["kind"],
            "away"
        );
        // A reader who may not know the person's work times learns nothing of them: no hatched
        // day and no warning about the away days.
        site.show_people(&mut ada, StartingRole::Contributor, "work-times", "hidden")
            .await?;
        let unknown = page(&mut contributor, body).await?;
        assert_eq!(rows_of(&unknown, &key)[0]["bar"]["hatched"], json!([]));
        assert!(warnings_on(&unknown, &id).is_empty(), "{unknown}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_01_not_scheduled_yet_comes_from_the_same_route() -> TestResult {
    each_database("plan_01_tray", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let dated = planned(&site, &mut ada, "Dated", json!({"start_date": MONDAY})).await?;
        let waiting = planned(&site, &mut ada, "Waiting", json!({})).await?;
        let rows = page(&mut ada, ask(&site, MONDAY, "2027-03-31", json!({}))).await?;
        assert_eq!(rows["not_scheduled"], 1);
        assert_eq!(rows_of(&rows, &dated).len(), 1);
        let tray = page(
            &mut ada,
            ask(
                &site,
                MONDAY,
                "2027-03-31",
                json!({"part": "not-scheduled"}),
            ),
        )
        .await?;
        let found = rows_of(&tray, &waiting);
        assert_eq!(found.len(), 1, "{tray}");
        assert_eq!(found[0]["bar"], Value::Null);
        assert!(rows_of(&tray, &dated).is_empty());
        // A span longer than a year, or backwards, is no request.
        for (first, last) in [(MONDAY, "2028-03-31"), ("2027-03-31", MONDAY)] {
            let refused = ada
                .send("POST", TIMELINE, Some(ask(&site, first, last, json!({}))))
                .await?;
            assert_eq!(refused.code(), "request.invalid", "{}", refused.body);
        }
        Ok(())
    })
    .await
}
