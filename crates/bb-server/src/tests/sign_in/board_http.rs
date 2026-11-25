//! The board over HTTP (PLAN-06, PRJ-06, ADR-0034 and ADR-0027, amendments of 2026-11-24): columns
//! of statuses merged across workflows, where a card may go and why not, the move as the status
//! change or the field change that exists, soft card limits, the order of the search, the fields
//! of the layout *Card*, and the views of a project and its sub-projects. Every flow runs on SQLite
//! and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::create_field;
use super::http::rig_and_app;
use super::issues_site::{PROJECTS, Site, ok};
use super::plan_site::{change, issue};
use super::views_site::{
    acme_board, board, card, column, column_keys, issue_in, keys_in, move_to, own_workflow,
    project, type_named,
};

#[tokio::test]
async fn plan_06_status_columns_merge_statuses_of_the_same_name_across_workflows() -> TestResult {
    each_database("plan_06_merged_columns", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let bug = type_named(&mut ada, "Bug").await?;
        let task = issue_in(&mut ada, (&site.acme, &site.kind), "A task", json!({})).await?;
        let defect = issue_in(&mut ada, (&site.acme, &bug), "A bug", json!({})).await?;
        let read = board(&mut ada, acme_board(&site, &json!({}))).await?;
        let keys = column_keys(&read);
        // *Open*, *In progress* and *Done* of both workflows are one column each; *Testing* and
        // *Won't fix* of *Bug* stay although no issue is in them.
        for name in ["Open", "In progress", "Done", "Testing", "Won't fix"] {
            assert_eq!(
                keys.iter().filter(|key| key.as_str() == name).count(),
                1,
                "{name}: {read}"
            );
        }
        assert_eq!(keys.len(), 5, "{read}");
        assert_eq!(column(&read, "Open")?["count"], 2);
        assert_eq!(column(&read, "Testing")?["count"], 0);
        assert_eq!(keys_in(&read, "Open")?.len(), 2);
        // Each card follows its own workflow.
        assert!(card(&read, &task).is_ok() && card(&read, &defect).is_ok());
        // *Status group* has its three columns.
        let groups = board(
            &mut ada,
            acme_board(&site, &json!({"columns": "status group"})),
        )
        .await?;
        assert_eq!(
            column_keys(&groups),
            ["Not started", "In progress", "Done"],
            "{groups}"
        );
        assert_eq!(column(&groups, "Not started")?["count"], 2);
        // A field that cannot be the columns of a board is no request.
        let refused = ada
            .send(
                "POST",
                super::views_site::BOARD,
                Some(acme_board(&site, &json!({"columns": "title"}))),
            )
            .await?;
        assert_eq!(refused.code(), "request.invalid", "{}", refused.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_06_a_card_may_not_move_to_a_status_its_workflow_lacks_and_the_answer_says_why()
-> TestResult {
    each_database("plan_06_moves", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let bug = type_named(&mut ada, "Bug").await?;
        let task = issue_in(&mut ada, (&site.acme, &site.kind), "A task", json!({})).await?;
        let defect = issue_in(&mut ada, (&site.acme, &bug), "A bug", json!({})).await?;
        let read = board(&mut ada, acme_board(&site, &json!({}))).await?;
        // *Simple* has no *Testing*: the column fades with that reason.
        assert_eq!(
            move_to(&read, &task, "Testing")?["refused"],
            "no-such-status"
        );
        // *Bug* has *Testing*, but not straight from *Open*.
        assert_eq!(
            move_to(&read, &defect, "Testing")?["refused"],
            "no-such-change"
        );
        // A change the workflow allows names the status it enters.
        let start = move_to(&read, &defect, "In progress")?;
        assert_eq!(start["refused"], Value::Null, "{read}");
        assert_eq!(start["to"], "in-progress");
        // A card has no move to the column it is in.
        let open = super::views_site::column_at(&read, "Open")?;
        assert!(
            card(&read, &defect)?["moves"]
                .as_array()
                .is_some_and(|moves| moves.iter().all(|each| each["column"] != open))
        );
        // The status change itself refuses what the board faded, with the reason of the workflow.
        let refused = change(&mut ada, &defect, json!({"status": "testing"})).await?;
        assert!(refused.code().starts_with("workflow."), "{}", refused.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_06_a_move_is_the_status_change_of_the_workflow_with_ask_for_and_must_be_filled_in()
-> TestResult {
    each_database("plan_06_move_rules", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let note = create_field(
            &mut ada,
            json!({"name": "Note", "kind": "text", "applies_to": "issues"}),
        )
        .await?["id"]
            .as_str()
            .ok_or("no field ID")?
            .to_owned();
        own_workflow(
            &mut ada,
            &site.acme,
            (
                json!([
                    {"key": "open", "name": "Open", "status_group": "not-started"},
                    {"key": "done", "name": "Done", "status_group": "done"},
                ]),
                json!([
                    {"from": "open", "to": "done", "button": "Finish",
                     "rules": {"must_fill": ["assigned-to"], "ask_for": [&note]}},
                    {"from": "done", "to": "open", "button": "Reopen"},
                ]),
            ),
        )
        .await?;
        let task = issue_in(&mut ada, (&site.acme, &site.kind), "No text", json!({})).await?;
        let (_, lea) = site.person(StartingRole::TeamMember, "lea").await?;
        let read = board(&mut ada, acme_board(&site, &json!({}))).await?;
        let finish = move_to(&read, &task, "Done")?;
        assert_eq!(finish["to"], "done", "{read}");
        let needs = finish["needs"].as_array().cloned().unwrap_or_default();
        assert!(needs.contains(&json!("assigned-to")), "{finish}");
        assert!(needs.contains(&json!(note)), "{finish}");
        // The drop is the status change that exists: refused while *Must be filled in* is not.
        let refused = change(&mut ada, &task, json!({"status": "done"})).await?;
        assert_eq!(refused.code(), "field.required", "{}", refused.body);
        let made = change(
            &mut ada,
            &task,
            json!({
                "status": "done",
                "assigned_to": [lea.to_string()],
                "fields": {&note: "Checked"},
            }),
        )
        .await?;
        ok(&made)?;
        let after = board(&mut ada, acme_board(&site, &json!({}))).await?;
        assert_eq!(
            keys_in(&after, "Done")?,
            std::slice::from_ref(&task),
            "{after}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_06_a_move_by_a_choice_field_is_the_field_change_with_its_permissions() -> TestResult {
    each_database("plan_06_choice_moves", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = create_field(
            &mut ada,
            json!({
                "name": "Size",
                "kind": "pick-one",
                "applies_to": "issues",
                "choices": [{"name": "Small"}, {"name": "Large"}],
            }),
        )
        .await?;
        let field = made["id"].as_str().ok_or("no field ID")?.to_owned();
        let small = made["choices"][0]["id"]
            .as_str()
            .ok_or("no choice")?
            .to_owned();
        let large = made["choices"][1]["id"]
            .as_str()
            .ok_or("no choice")?
            .to_owned();
        let sized = issue_in(
            &mut ada,
            (&site.acme, &site.kind),
            "Sized",
            json!({"fields": {&field: &small}}),
        )
        .await?;
        issue_in(&mut ada, (&site.acme, &site.kind), "Unsized", json!({})).await?;
        let options = json!({"columns": "size"});
        let read = board(&mut ada, acme_board(&site, &options)).await?;
        assert_eq!(column_keys(&read), ["Small", "Large", ""], "{read}");
        assert_eq!(read["columns"][2]["key"], Value::Null);
        assert_eq!(read["columns"][2]["count"], 1);
        let to_large = move_to(&read, &sized, "Large")?;
        assert_eq!(to_large["refused"], Value::Null, "{read}");
        assert_eq!(to_large["to"], Value::Null);
        // The move is the change of the field.
        ok(&change(&mut ada, &sized, json!({"fields": {&field: &large}})).await?)?;
        let after = board(&mut ada, acme_board(&site, &options)).await?;
        assert_eq!(
            keys_in(&after, "Large")?,
            std::slice::from_ref(&sized),
            "{after}"
        );
        // A Viewer may not change the field: every column fades for them.
        let (mut viewer, _) = site.person(StartingRole::Viewer, "viewer").await?;
        let theirs = board(&mut viewer, acme_board(&site, &options)).await?;
        assert_eq!(
            move_to(&theirs, &sized, "Small")?["refused"],
            "cannot-change",
            "{theirs}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_06_a_card_limit_counts_only_what_the_reader_sees_and_refuses_nothing() -> TestResult {
    each_database("plan_06_card_limits", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let secret = project(&mut ada, "SEC").await?;
        for title in ["One", "Two", "Three"] {
            issue_in(&mut ada, (&site.acme, &site.kind), title, json!({})).await?;
        }
        issue_in(&mut ada, (&secret, &site.kind), "Hidden", json!({})).await?;
        let everywhere = |limit: u32| {
            json!({"search_text": "", "board": {"card_limits": [{"column": "Open", "limit": limit}]}})
        };
        let all = board(&mut ada, everywhere(3)).await?;
        assert_eq!(column(&all, "Open")?["count"], 4);
        assert_eq!(column(&all, "Open")?["limit"], 3);
        assert_eq!(column(&all, "Open")?["over"], true, "{all}");
        // A Team member of ACME only counts what they see: three, not over three.
        let (mut team, _) = site
            .person_in_acme(StartingRole::TeamMember, "team")
            .await?;
        let theirs = board(&mut team, everywhere(3)).await?;
        assert_eq!(column(&theirs, "Open")?["count"], 3);
        assert_eq!(column(&theirs, "Open")?["over"], false, "{theirs}");
        assert!(!theirs.to_string().contains("Hidden"));
        let over = board(&mut team, everywhere(2)).await?;
        assert_eq!(column(&over, "Open")?["over"], true);
        // Nothing is refused: a card still goes into a column over its limit.
        let more = issue_in(&mut ada, (&site.acme, &site.kind), "Four", json!({})).await?;
        ok(&change(&mut team, &more, json!({"status": "done"})).await?)?;
        ok(&change(&mut team, &more, json!({"status": "open"})).await?)?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_06_cards_follow_the_sort_of_the_search() -> TestResult {
    each_database("plan_06_card_order", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let low = issue_in(
            &mut ada,
            (&site.acme, &site.kind),
            "Low",
            json!({"priority": "low"}),
        )
        .await?;
        let high = issue_in(
            &mut ada,
            (&site.acme, &site.kind),
            "High",
            json!({"priority": "high"}),
        )
        .await?;
        let read = |text: &str| json!({"search_text": text, "project": site.acme});
        let by_priority = board(&mut ada, read("sort by priority")).await?;
        assert_eq!(keys_in(&by_priority, "Open")?, [high.clone(), low.clone()]);
        let by_title = board(&mut ada, read("sort by title")).await?;
        assert_eq!(keys_in(&by_title, "Open")?, [high.clone(), low.clone()]);
        let lowest = board(&mut ada, read("sort by priority lowest first")).await?;
        assert_eq!(keys_in(&lowest, "Open")?, [low, high]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_06_a_card_shows_only_the_layout_fields_the_reader_may_read() -> TestResult {
    each_database("plan_06_card_fields", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let key = issue_in(
            &mut ada,
            (&site.acme, &site.kind),
            "Dated",
            json!({"due_date": "2027-03-05"}),
        )
        .await?;
        // The starting card: *Priority*, *Assigned to*, *Due date*.
        let read = board(&mut ada, acme_board(&site, &json!({}))).await?;
        assert_eq!(
            card(&read, &key)?["fields"],
            json!(["priority", "assigned-to", "due-date"]),
            "{read}"
        );
        // A Contributor whose *Due date* is hidden is not shown it on the card, nor its value.
        let contributor = site.role_id(StartingRole::Contributor);
        let hidden = ada
            .send(
                "PUT",
                &format!("/api/v1/field-permissions/{contributor}/due-date"),
                Some(json!({"level": "hidden"})),
            )
            .await?;
        assert!(hidden.status.is_success(), "{}", hidden.body);
        let (mut them, _) = site
            .person(StartingRole::Contributor, "contributor")
            .await?;
        let theirs = board(&mut them, acme_board(&site, &json!({}))).await?;
        let shown = card(&theirs, &key)?;
        assert_eq!(
            shown["fields"],
            json!(["priority", "assigned-to"]),
            "{shown}"
        );
        assert!(shown["issue"].get("due_date").is_none(), "{shown}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_06_a_project_view_covers_its_sub_projects_and_only_this_project_narrows_it()
-> TestResult {
    each_database("prj_06_only_this_project", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{}/sub-projects", site.acme),
                Some(json!({"short_name": "KID", "name": "Kid"})),
            )
            .await?;
        ok(&made)?;
        let top = issue_in(&mut ada, (&site.acme, &site.kind), "Top", json!({})).await?;
        let below = issue_in(
            &mut ada,
            ("KID", &site.kind),
            "Below",
            json!({"priority": "high"}),
        )
        .await?;
        let whole = board(&mut ada, acme_board(&site, &json!({}))).await?;
        assert_eq!(whole["search_text"], "project within ACME", "{whole}");
        assert_eq!(keys_in(&whole, "Open")?.len(), 2);
        let mut only = acme_board(&site, &json!({}));
        only["only_this_project"] = json!(true);
        let narrowed = board(&mut ada, only).await?;
        assert_eq!(narrowed["search_text"], "project is ACME", "{narrowed}");
        assert_eq!(keys_in(&narrowed, "Open")?, std::slice::from_ref(&top));
        // The text keeps the rest of the search, and the project test is written once.
        let mut text = acme_board(&site, &json!({}));
        text["search_text"] = json!("project within ACME and priority is high");
        let found = board(&mut ada, text).await?;
        assert_eq!(keys_in(&found, "Open")?, [below], "{found}");
        assert_eq!(
            found["search_text"]
                .as_str()
                .map(|text| text.matches("ACME").count()),
            Some(1),
            "{found}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn prj_06_a_board_across_projects_shows_each_card_with_its_own_workflow() -> TestResult {
    each_database("prj_06_across_workflows", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let other = project(&mut ada, "WEB").await?;
        // ACME's tasks go from *Open* straight to *Done*; WEB's follow *Simple*.
        own_workflow(
            &mut ada,
            &site.acme,
            (
                json!([
                    {"key": "open", "name": "Open", "status_group": "not-started"},
                    {"key": "done", "name": "Done", "status_group": "done"},
                ]),
                json!([
                    {"from": "open", "to": "done", "button": "Finish"},
                    {"from": "done", "to": "open", "button": "Reopen"},
                ]),
            ),
        )
        .await?;
        let in_acme = issue_in(&mut ada, (&site.acme, &site.kind), "Acme", json!({})).await?;
        let in_web = issue_in(&mut ada, (&other, &site.kind), "Web", json!({})).await?;
        let read = board(&mut ada, json!({"search_text": ""})).await?;
        assert_eq!(
            move_to(&read, &in_acme, "In progress")?["refused"],
            "no-such-status",
            "{read}"
        );
        assert_eq!(move_to(&read, &in_web, "In progress")?["to"], "in-progress");
        assert_eq!(move_to(&read, &in_acme, "Done")?["to"], "done");
        // The issue page agrees: the workflow of each issue is its own.
        let page = issue(&mut ada, &in_acme).await?;
        assert_eq!(page["status"]["key"], "open");
        Ok(())
    })
    .await
}
