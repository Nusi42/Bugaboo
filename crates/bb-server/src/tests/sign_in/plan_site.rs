//! What the tests of the timeline share: an issue with dates, people and an estimate, the work
//! times of a person, a page of the timeline and the parts of its answer.

use bb_core::id::UserId;
use bb_testing::Failure;
use serde_json::{Value, json};

use super::http::{Browser, Reply};
use super::issues_site::{ISSUES, PROJECTS, Site, key_of, ok};

/// The route of the timeline.
pub(super) const TIMELINE: &str = "/api/v1/timeline";

/// Monday, 1 March 2027: a week with no public holiday in Germany.
pub(super) const MONDAY: &str = "2027-03-01";

/// An issue in ACME made by `browser` with these members (dates, people, an estimate).
pub(super) async fn planned(
    site: &Site<'_>,
    browser: &mut Browser,
    title: &str,
    members: Value,
) -> Result<String, Failure> {
    let mut body = site.new_issue(title);
    if let (Some(body), Some(members)) = (body.as_object_mut(), members.as_object()) {
        body.extend(members.clone());
    }
    let reply = browser
        .send(
            "POST",
            &format!("{PROJECTS}/{}/issues", site.acme),
            Some(body),
        )
        .await?;
    key_of(&reply)
}

/// Two issues in ACME made by `browser` whose days overlap: *API* from Monday to Thursday and
/// *Login screen* from Wednesday to Friday, as their readable IDs.
pub(super) async fn overlapping(
    site: &Site<'_>,
    browser: &mut Browser,
) -> Result<(String, String), Failure> {
    let first = planned(
        site,
        browser,
        "API",
        json!({"start_date": MONDAY, "due_date": "2027-03-04"}),
    )
    .await?;
    let then = planned(
        site,
        browser,
        "Login screen",
        json!({"start_date": "2027-03-03", "due_date": "2027-03-05"}),
    )
    .await?;
    Ok((first, then))
}

/// Gives `person` these work hours on Monday to Friday and this share in ACME, as Ada.
pub(super) async fn work_times(
    site: &Site<'_>,
    ada: &mut Browser,
    person: UserId,
    minutes: u16,
    percent: u8,
) -> Result<(), Failure> {
    let usual = ada
        .send(
            "PUT",
            &format!("/api/v1/people/{person}/work-times/usual-days"),
            Some(json!({
                "version": 0,
                "days": (0..7).map(|at| json!({"works": at < 5, "minutes": minutes})).collect::<Vec<_>>(),
            })),
        )
        .await?;
    ok(&usual)?;
    let share = ada
        .send(
            "PUT",
            &format!("{PROJECTS}/{}/shares/{person}", site.acme),
            Some(json!({"percent": percent})),
        )
        .await?;
    ok(&share)?;
    Ok(())
}

/// The body of a page of the timeline of every issue of ACME, from `first` to `last`, with these
/// members added.
pub(super) fn ask(site: &Site<'_>, first: &str, last: &str, more: Value) -> Value {
    let mut body = json!({
        "search_text": "",
        "project": site.acme,
        "timeline": {"zoom": "week"},
        "first": first,
        "last": last,
    });
    if let (Some(body), Value::Object(more)) = (body.as_object_mut(), more) {
        body.extend(more);
    }
    body
}

/// The same, planned by effort.
pub(super) fn by_effort(site: &Site<'_>, first: &str, last: &str) -> Value {
    ask(
        site,
        first,
        last,
        json!({"timeline": {"zoom": "week", "plan_by_effort": true}}),
    )
}

/// A page of the timeline as `browser` reads it.
pub(super) async fn page(browser: &mut Browser, body: Value) -> Result<Value, Failure> {
    let reply = browser.send("POST", TIMELINE, Some(body)).await?;
    Ok(ok(&reply)?.clone())
}

/// The rows of a page that show the issue with this readable ID.
pub(super) fn rows_of<'a>(page: &'a Value, key: &str) -> Vec<&'a Value> {
    page["rows"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter(|row| row["issue"]["readable_id"] == key)
                .collect()
        })
        .unwrap_or_default()
}

/// The days of the bar of the issue with this readable ID: (first, last).
pub(super) fn bar_of(page: &Value, key: &str) -> Option<(String, String)> {
    let row = rows_of(page, key).into_iter().next()?;
    let days = &row["bar"]["days"];
    Some((
        days["first"].as_str()?.to_owned(),
        days["last"].as_str()?.to_owned(),
    ))
}

/// The internal ID of the issue with this readable ID, as `browser` reads it.
pub(super) async fn id_of_key(browser: &mut Browser, key: &str) -> Result<String, Failure> {
    let reply = browser
        .send("GET", &format!("{ISSUES}/{key}"), None)
        .await?;
    Ok(ok(&reply)?["issue"]["id"]
        .as_str()
        .ok_or("no ID")?
        .to_owned())
}

/// The warnings of a page on the issue with this internal ID.
pub(super) fn warnings_on<'a>(page: &'a Value, id: &str) -> Vec<&'a Value> {
    page["warnings"]
        .as_array()
        .map(|all| {
            all.iter()
                .filter(|warning| warning["issue"] == id)
                .collect()
        })
        .unwrap_or_default()
}

/// The issue as `browser` reads it.
pub(super) async fn issue(browser: &mut Browser, key: &str) -> Result<Value, Failure> {
    let reply = browser
        .send("GET", &format!("{ISSUES}/{key}"), None)
        .await?;
    Ok(ok(&reply)?["issue"].clone())
}

/// Relates `first` to `second` with the starting kind *blocks* (`first` must finish first).
pub(super) async fn blocks(
    browser: &mut Browser,
    first: &str,
    second: &str,
) -> Result<(), Failure> {
    let kinds = browser.send("GET", "/api/v1/relation-kinds", None).await?;
    let blocks = ok(&kinds)?["items"]
        .as_array()
        .and_then(|items| items.iter().find(|kind| kind["forward"] == "blocks"))
        .and_then(|kind| kind["id"].as_str())
        .ok_or("no kind blocks")?
        .to_owned();
    let made = browser
        .send(
            "POST",
            &format!("{ISSUES}/{first}/relations"),
            Some(json!({"other": second, "relation_kind": blocks, "direction": "forward"})),
        )
        .await?;
    ok(&made)?;
    Ok(())
}

/// Changes the issue with these members, on the version it has: the answer.
pub(super) async fn change(
    browser: &mut Browser,
    key: &str,
    members: Value,
) -> Result<Reply, Failure> {
    let version = issue(browser, key).await?["version"].clone();
    let mut body = json!({"version": version});
    if let (Some(body), Some(members)) = (body.as_object_mut(), members.as_object()) {
        body.extend(members.clone());
    }
    browser
        .send("PATCH", &format!("{ISSUES}/{key}"), Some(body))
        .await
}

/// Puts `child` under `parent`, as `browser`.
pub(super) async fn under(browser: &mut Browser, child: &str, parent: &str) -> Result<(), Failure> {
    let version = issue(browser, child).await?["version"].clone();
    let reply = browser
        .send(
            "PUT",
            &format!("{ISSUES}/{child}/parent"),
            Some(json!({"version": version, "parent": parent})),
        )
        .await?;
    ok(&reply)?;
    Ok(())
}

/// The goal of a page with this internal ID.
pub(super) fn goal_of<'a>(page: &'a Value, id: &str) -> Option<&'a Value> {
    page["goals"]
        .as_array()?
        .iter()
        .find(|goal| goal["id"] == id)
}
