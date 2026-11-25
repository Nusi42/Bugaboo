//! What the tests of the board and the calendar share: the routes, an issue of a type in a project,
//! a workflow of a project's own, and the parts of a board's answer.

use bb_testing::Failure;
use serde_json::{Value, json};

use super::http::Browser;
use super::issues_site::{PROJECTS, Site, key_of, ok, text_in};
use super::workflows_http::lock_workflow;

/// The route of the board.
pub(super) const BOARD: &str = "/api/v1/board";
/// The route of the calendar.
pub(super) const CALENDAR: &str = "/api/v1/calendar";
/// The route of the workflows.
const WORKFLOWS: &str = "/api/v1/workflows";

/// The ID of the issue type with this name.
pub(super) async fn type_named(browser: &mut Browser, name: &str) -> Result<String, Failure> {
    let types = browser.send("GET", "/api/v1/issue-types", None).await?;
    Ok(ok(&types)?["items"]
        .as_array()
        .and_then(|items| items.iter().find(|each| each["name"] == name))
        .and_then(|found| found["id"].as_str())
        .ok_or_else(|| format!("no issue type {name}"))?
        .to_owned())
}

/// An issue of the type `kind` in the project (by ID or short name) with these members: its
/// readable ID.
pub(super) async fn issue_in(
    browser: &mut Browser,
    (project, kind): (&str, &str),
    title: &str,
    members: Value,
) -> Result<String, Failure> {
    let mut body = json!({"issue_type": kind, "title": title});
    if let (Some(body), Some(members)) = (body.as_object_mut(), members.as_object()) {
        body.extend(members.clone());
    }
    key_of(
        &browser
            .send("POST", &format!("{PROJECTS}/{project}/issues"), Some(body))
            .await?,
    )
}

/// A project (by its short name), made by `browser`: its ID.
pub(super) async fn project(browser: &mut Browser, short_name: &str) -> Result<String, Failure> {
    let made = browser
        .send(
            "POST",
            PROJECTS,
            Some(json!({"short_name": short_name, "name": short_name})),
        )
        .await?;
    text_in(ok(&made)?, "id")
}

/// Gives the project (by ID) a workflow of its own with these statuses and changes, made and
/// published by the administrator.
pub(super) async fn own_workflow(
    ada: &mut Browser,
    project: &str,
    (statuses, changes): (Value, Value),
) -> Result<(), Failure> {
    let editor = ok(&ada
        .send("POST", WORKFLOWS, Some(json!({"project": project})))
        .await?)?
    .clone();
    let workflow = text_in(&editor["workflow"], "id")?;
    let read = ada
        .send(
            "GET",
            &format!("{WORKFLOWS}/editor?project={project}"),
            None,
        )
        .await?;
    let version = ok(&read)?["workflow"]["version"].clone();
    let lock = lock_workflow(ada, &workflow).await?;
    ok(&ada
        .send(
            "PUT",
            &format!("{WORKFLOWS}/{workflow}"),
            Some(json!({
                "version": version,
                "lock_token": lock,
                "name": "Own",
                "statuses": statuses,
                "changes": changes,
                "moves": [],
            })),
        )
        .await?)?;
    Ok(())
}

/// The board of `body`, as `browser` reads it.
pub(super) async fn board(browser: &mut Browser, body: Value) -> Result<Value, Failure> {
    let reply = browser.send("POST", BOARD, Some(body)).await?;
    Ok(ok(&reply)?.clone())
}

/// The keys of the columns of a board, in order.
pub(super) fn column_keys(board: &Value) -> Vec<String> {
    board["columns"]
        .as_array()
        .map(|columns| {
            columns
                .iter()
                .map(|column| column["key"].as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The place of the column with this key.
pub(super) fn column_at(board: &Value, key: &str) -> Result<usize, Failure> {
    column_keys(board)
        .iter()
        .position(|each| each == key)
        .ok_or_else(|| format!("no column {key} in {board}").into())
}

/// The column with this key.
pub(super) fn column<'a>(board: &'a Value, key: &str) -> Result<&'a Value, Failure> {
    let at = column_at(board, key)?;
    Ok(&board["columns"][at])
}

/// Every card of a board, in the order of the cells.
pub(super) fn cards(board: &Value) -> Vec<&Value> {
    board["cells"]
        .as_array()
        .map(|cells| {
            cells
                .iter()
                .filter_map(|cell| cell["cards"].as_array())
                .flatten()
                .collect()
        })
        .unwrap_or_default()
}

/// The card of the issue with this readable ID.
pub(super) fn card<'a>(board: &'a Value, key: &str) -> Result<&'a Value, Failure> {
    cards(board)
        .into_iter()
        .find(|card| card["issue"]["readable_id"] == key)
        .ok_or_else(|| format!("no card {key} in {board}").into())
}

/// The move of a card to the column with this key.
pub(super) fn move_to<'a>(board: &'a Value, key: &str, to: &str) -> Result<&'a Value, Failure> {
    let at = column_at(board, to)?;
    card(board, key)?["moves"]
        .as_array()
        .and_then(|moves| moves.iter().find(|each| each["column"] == at))
        .ok_or_else(|| format!("no move of {key} to {to} in {board}").into())
}

/// The readable IDs of the cards of one column (all rows), in order.
pub(super) fn keys_in(board: &Value, key: &str) -> Result<Vec<String>, Failure> {
    let at = column_at(board, key)?;
    Ok(board["cells"]
        .as_array()
        .map(|cells| {
            cells
                .iter()
                .filter(|cell| cell["column"] == at)
                .filter_map(|cell| cell["cards"].as_array())
                .flatten()
                .filter_map(|card| card["issue"]["readable_id"].as_str())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default())
}

/// The ACME board of every issue, with these options of the view.
pub(super) fn acme_board(site: &Site<'_>, options: &Value) -> Value {
    json!({"search_text": "", "project": site.acme, "board": options})
}
