//! The structured answer to a stale save on every route that takes a version (ADR-0019 amendment
//! M3, ISS-17): `edit.version-changed` with the `conflict.version` the record has now, whatever
//! the kind of record. One table, one row for each such route of the API description; a route that
//! is added without a row fails the second test. Every row runs on SQLite and PostgreSQL.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::FIELDS;
use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, ok, text_in};
use super::layouts_http::{AN_EDITOR, LAYOUTS, LOCKS, customize};

/// How far past the current version a stale save is.
const AHEAD: i64 = 50;

/// The routes of the API description that take a `version`, by operation ID: the rows of the table.
const VERSIONED: [&str; 31] = [
    "changeSignUpPolicy",
    "changeMySetting",
    "saveComment",
    "changeField",
    "addChoice",
    "reorderChoices",
    "changeChoice",
    "combineChoices",
    "changeGroup",
    "changeSavedSearch",
    "changeIssueType",
    "changeIssueTypeOffers",
    "changeIssue",
    "saveIssueDescription",
    "saveIssueFieldText",
    "moveIssue",
    "setIssueParent",
    "changeLabelGroup",
    "changeLabelGroupOffers",
    "changeLabel",
    "combineLabels",
    "saveLayout",
    "deleteLayout",
    "publishWorkflow",
    "deleteWorkflow",
    "changeProject",
    "changeRelationKind",
    "changeRole",
    "resetRole",
    "changeTag",
    "combineTags",
];

/// A save that is about to be sent on a version that is not the current one.
struct Save {
    method: &'static str,
    path: String,
    /// The body without the `version` (and the `lock_token`).
    body: Value,
    /// The version the record has now.
    version: i64,
    /// The part to take the edit lock on first, for a long text.
    lock: Option<Value>,
    /// The route takes the version in the query (`?version=`), not in a body.
    in_query: bool,
}

impl Save {
    fn new(
        method: &'static str,
        path: String,
        body: Value,
        version: &Value,
    ) -> Result<Self, Failure> {
        Ok(Self {
            method,
            path,
            body,
            version: version.as_i64().ok_or("the record shows no version")?,
            lock: None,
            in_query: false,
        })
    }

    fn in_the_query(mut self) -> Self {
        self.in_query = true;
        self
    }

    fn under_lock(mut self, record: &Value, part: &str) -> Self {
        self.lock = Some(json!({"record": record, "part": part}));
        self
    }
}

async fn made(browser: &mut Browser, path: &str, body: Value) -> Result<Value, Failure> {
    Ok(ok(&browser.send("POST", path, Some(body)).await?)?.clone())
}

async fn read(browser: &mut Browser, path: &str) -> Result<Value, Failure> {
    Ok(ok(&browser.send("GET", path, None).await?)?.clone())
}

/// The record a route changes, made, and the save that is stale.
async fn prepare(id: &str, site: &Site<'_>, ada: &mut Browser) -> Result<Save, Failure> {
    match id {
        "changeField" | "addChoice" | "reorderChoices" | "changeChoice" | "combineChoices" => {
            prepare_fields(id, ada).await
        }
        "saveComment"
        | "changeIssue"
        | "saveIssueDescription"
        | "saveIssueFieldText"
        | "moveIssue"
        | "setIssueParent" => prepare_issues(id, site, ada).await,
        "changeLabelGroup"
        | "changeLabelGroupOffers"
        | "changeLabel"
        | "combineLabels"
        | "changeTag"
        | "combineTags" => prepare_labels(id, site, ada).await,
        "saveLayout" | "deleteLayout" => prepare_layout(id, ada).await,
        "publishWorkflow" | "deleteWorkflow" => prepare_workflow(id, ada).await,
        "changeProject" | "changeRelationKind" | "changeRole" | "resetRole" => {
            prepare_records(id, ada).await
        }
        _ => prepare_settings(id, ada).await,
    }
}

async fn prepare_settings(id: &str, ada: &mut Browser) -> Result<Save, Failure> {
    match id {
        "changeSignUpPolicy" => {
            let now = read(ada, "/api/v1/auth/sign-up-policy").await?;
            Save::new(
                "PUT",
                "/api/v1/auth/sign-up-policy".to_owned(),
                json!({"enabled": false, "domains": []}),
                &now["version"],
            )
        }
        "changeMySetting" => {
            let now = read(ada, "/api/v1/me/settings?topic=appearance.mode").await?;
            Save::new(
                "PUT",
                "/api/v1/me/settings/appearance.mode".to_owned(),
                json!({"value": "dark"}),
                &now["settings"][0]["version"],
            )
        }
        "changeGroup" => {
            let group = made(ada, "/api/v1/groups", json!({"name": "Backend"})).await?;
            Save::new(
                "PATCH",
                format!("/api/v1/groups/{}", text_in(&group, "id")?),
                json!({"name": "Frontend"}),
                &group["version"],
            )
        }
        "changeSavedSearch" => {
            let search = made(
                ada,
                "/api/v1/saved-searches",
                json!({"name": "Open bugs", "search_text": ""}),
            )
            .await?;
            Save::new(
                "PATCH",
                format!("/api/v1/saved-searches/{}", text_in(&search, "id")?),
                json!({"name": "Closed bugs"}),
                &search["version"],
            )
        }
        "changeIssueTypeOffers" => {
            let types = read(ada, "/api/v1/issue-types").await?;
            let one = &types["items"][0];
            Save::new(
                "PUT",
                format!("/api/v1/issue-types/{}/offers", text_in(one, "id")?),
                json!({"changes": [{"offered": true}]}),
                &one["version"],
            )
        }
        "changeIssueType" => {
            let types = read(ada, "/api/v1/issue-types").await?;
            let one = &types["items"][0];
            Save::new(
                "PATCH",
                format!("/api/v1/issue-types/{}", text_in(one, "id")?),
                json!({"name": "Defect"}),
                &one["version"],
            )
        }
        other => Err(format!("no row for {other}").into()),
    }
}

async fn prepare_records(id: &str, ada: &mut Browser) -> Result<Save, Failure> {
    match id {
        "changeProject" => {
            let project = made(
                ada,
                PROJECTS,
                json!({"short_name": "GAMMA", "name": "Gamma"}),
            )
            .await?;
            Save::new(
                "PATCH",
                format!("{PROJECTS}/{}", text_in(&project, "id")?),
                json!({"name": "Gamma Corp"}),
                &project["version"],
            )
        }
        "changeRelationKind" => {
            let kinds = read(ada, "/api/v1/relation-kinds").await?;
            let one = &kinds["items"][0];
            Save::new(
                "PATCH",
                format!("/api/v1/relation-kinds/{}", text_in(one, "id")?),
                json!({"forward": "holds up"}),
                &one["version"],
            )
        }
        "changeRole" | "resetRole" => {
            let roles = read(ada, "/api/v1/roles").await?;
            let role = roles["items"]
                .as_array()
                .and_then(|items| items.iter().find(|role| role["starting"] == "viewer"))
                .ok_or("no Viewer role")?;
            let path = format!("/api/v1/roles/{}", text_in(role, "id")?);
            if id == "changeRole" {
                Save::new("PATCH", path, json!({"name": "Looker"}), &role["version"])
            } else {
                Save::new("POST", format!("{path}/reset"), json!({}), &role["version"])
            }
        }
        other => Err(format!("no row for {other}").into()),
    }
}

async fn prepare_fields(id: &str, ada: &mut Browser) -> Result<Save, Failure> {
    match id {
        "changeField" | "addChoice" | "reorderChoices" | "changeChoice" | "combineChoices" => {
            let field = made(
                ada,
                FIELDS,
                json!({
                    "name": format!("Severity of {id}"),
                    "kind": "pick-one",
                    "applies_to": "issues",
                    "choices": [{"name": "Minor"}, {"name": "Major"}, {"name": "Critical"}],
                }),
            )
            .await?;
            let key = text_in(&field, "id")?;
            let minor = text_in(&field["choices"][0], "id")?;
            let major = text_in(&field["choices"][1], "id")?;
            let base = format!("{FIELDS}/{key}");
            let version = &field["version"];
            let (method, path, body) = match id {
                "changeField" => ("PATCH", base, json!({"name": "Gravity"})),
                "addChoice" => (
                    "POST",
                    format!("{base}/choices"),
                    json!({"name": "Blocker"}),
                ),
                "reorderChoices" => {
                    let order: Vec<Value> = field["choices"]
                        .as_array()
                        .ok_or("no choices")?
                        .iter()
                        .rev()
                        .map(|choice| choice["id"].clone())
                        .collect();
                    (
                        "PUT",
                        format!("{base}/choices/order"),
                        json!({"order": order}),
                    )
                }
                "changeChoice" => (
                    "PATCH",
                    format!("{base}/choices/{minor}"),
                    json!({"name": "Tiny"}),
                ),
                _ => (
                    "POST",
                    format!("{base}/choices/{major}/combine"),
                    json!({"into": minor}),
                ),
            };
            Save::new(method, path, body, version)
        }
        other => Err(format!("no row for {other}").into()),
    }
}

async fn prepare_issues(id: &str, site: &Site<'_>, ada: &mut Browser) -> Result<Save, Failure> {
    match id {
        "saveComment" => {
            let issue = ok(&site.create(ada, "Commented").await?)?.clone();
            let comment = made(
                ada,
                &format!("{ISSUES}/{}/comments", text_in(&issue, "readable_id")?),
                json!({"text": "First"}),
            )
            .await?;
            let record = json!({"kind": "comment", "id": text_in(&comment, "id")?});
            Ok(Save::new(
                "PUT",
                format!("/api/v1/comments/{}", text_in(&comment, "id")?),
                json!({"text": "Second"}),
                &comment["version"],
            )?
            .under_lock(&record, "text"))
        }
        "changeIssue"
        | "saveIssueDescription"
        | "moveIssue"
        | "setIssueParent"
        | "saveIssueFieldText" => {
            let issue = ok(&site.create(ada, "An issue").await?)?.clone();
            let key = text_in(&issue, "readable_id")?;
            let record = json!({"kind": "issue", "id": text_in(&issue, "id")?});
            let version = &issue["version"];
            match id {
                "changeIssue" => Save::new(
                    "PATCH",
                    format!("{ISSUES}/{key}"),
                    json!({"title": "Renamed"}),
                    version,
                ),
                "saveIssueDescription" => Ok(Save::new(
                    "PUT",
                    format!("{ISSUES}/{key}/description"),
                    json!({"text": "Words"}),
                    version,
                )?
                .under_lock(&record, "description")),
                "moveIssue" => {
                    let other =
                        made(ada, PROJECTS, json!({"short_name": "BETA", "name": "Beta"})).await?;
                    Save::new(
                        "POST",
                        format!("{ISSUES}/{key}/move"),
                        json!({"project": text_in(&other, "id")?}),
                        version,
                    )
                }
                "setIssueParent" => {
                    let parent = ok(&site.create(ada, "Parent").await?)?.clone();
                    Save::new(
                        "PUT",
                        format!("{ISSUES}/{key}/parent"),
                        json!({"parent": text_in(&parent, "readable_id")?}),
                        version,
                    )
                }
                _ => {
                    let notes = made(
                        ada,
                        FIELDS,
                        json!({"name": "Notes", "kind": "long-text", "applies_to": "issues"}),
                    )
                    .await?;
                    let field = text_in(&notes, "id")?;
                    Ok(Save::new(
                        "PUT",
                        format!("{ISSUES}/{key}/fields/{field}/text"),
                        json!({"text": "Steps"}),
                        version,
                    )?
                    .under_lock(&record, &format!("field-{field}")))
                }
            }
        }
        other => Err(format!("no row for {other}").into()),
    }
}

async fn prepare_labels(id: &str, site: &Site<'_>, ada: &mut Browser) -> Result<Save, Failure> {
    match id {
        "changeLabelGroup" => {
            let group = made(
                ada,
                "/api/v1/label-groups",
                json!({"name": "Area", "pick_one": false}),
            )
            .await?;
            Save::new(
                "PATCH",
                format!("/api/v1/label-groups/{}", text_in(&group, "id")?),
                json!({"name": "Part"}),
                &group["version"],
            )
        }
        "changeLabelGroupOffers" => {
            let group = made(
                ada,
                "/api/v1/label-groups",
                json!({"name": "Platform", "pick_one": false}),
            )
            .await?;
            Save::new(
                "PUT",
                format!("/api/v1/label-groups/{}/offers", text_in(&group, "id")?),
                json!({"changes": [{"offered": true}]}),
                &group["version"],
            )
        }
        "changeLabel" | "combineLabels" => {
            let label = made(
                ada,
                "/api/v1/labels",
                json!({"name": format!("ui for {id}"), "label_group": null}),
            )
            .await?;
            let path = format!("/api/v1/labels/{}", text_in(&label, "id")?);
            if id == "changeLabel" {
                return Save::new("PATCH", path, json!({"name": "screen"}), &label["version"]);
            }
            let other = made(
                ada,
                "/api/v1/labels",
                json!({"name": format!("web for {id}"), "label_group": null}),
            )
            .await?;
            Save::new(
                "POST",
                format!("{path}/combine"),
                json!({"into": text_in(&other, "id")?, "into_version": other["version"]}),
                &label["version"],
            )
        }
        "changeTag" | "combineTags" => {
            let issue = ok(&site.create(ada, "Tagged").await?)?.clone();
            for name in ["slow", "fast"] {
                made(
                    ada,
                    &format!("{ISSUES}/{}/tags", text_in(&issue, "readable_id")?),
                    json!({"name": name}),
                )
                .await?;
            }
            let list = read(ada, "/api/v1/tags").await?;
            let find = |name: &str| {
                list["items"]
                    .as_array()
                    .and_then(|items| items.iter().find(|tag| tag["name"] == name))
                    .cloned()
                    .ok_or_else(|| Failure::from(format!("no tag {name} in {list}")))
            };
            let (slow, fast) = (find("slow")?, find("fast")?);
            let path = format!("/api/v1/tags/{}", text_in(&slow, "id")?);
            if id == "changeTag" {
                Save::new("PATCH", path, json!({"name": "sluggish"}), &slow["version"])
            } else {
                Save::new(
                    "POST",
                    format!("{path}/combine"),
                    json!({"into": text_in(&fast, "id")?, "into_version": fast["version"]}),
                    &slow["version"],
                )
            }
        }
        other => Err(format!("no row for {other}").into()),
    }
}

async fn prepare_layout(id: &str, ada: &mut Browser) -> Result<Save, Failure> {
    match id {
        "deleteLayout" => {
            let project = made(
                ada,
                PROJECTS,
                json!({"short_name": "DELTA", "name": "Delta"}),
            )
            .await?;
            let editor = customize(ada, &text_in(&project, "id")?).await?;
            Ok(Save::new(
                "DELETE",
                format!("{LAYOUTS}/{}", text_in(&editor["layout"], "id")?),
                Value::Null,
                &editor["layout"]["version"],
            )?
            .in_the_query())
        }
        "saveLayout" => {
            let editor = customize(ada, "ACME").await?;
            let layout = text_in(&editor["layout"], "id")?;
            Ok(Save::new(
                "PUT",
                format!("{LAYOUTS}/{layout}"),
                json!({"sections": []}),
                &editor["layout"]["version"],
            )?
            .under_lock(&json!({"kind": "layout", "id": layout}), "layout"))
        }
        other => Err(format!("no row for {other}").into()),
    }
}

async fn prepare_workflow(id: &str, ada: &mut Browser) -> Result<Save, Failure> {
    let short_name = if id == "deleteWorkflow" {
        "FLOWD"
    } else {
        "FLOWP"
    };
    let project = made(
        ada,
        PROJECTS,
        json!({"short_name": short_name, "name": short_name}),
    )
    .await?;
    let editor = made(
        ada,
        "/api/v1/workflows",
        json!({"project": text_in(&project, "id")?}),
    )
    .await?;
    let workflow = text_in(&editor["workflow"], "id")?;
    let path = format!("/api/v1/workflows/{workflow}");
    let version = &editor["workflow"]["version"];
    match id {
        "deleteWorkflow" => Ok(Save::new("DELETE", path, Value::Null, version)?.in_the_query()),
        "publishWorkflow" => Ok(Save::new(
            "PUT",
            path,
            json!({
                "name": "Simple",
                "statuses": [{"key": "open", "name": "Open", "status_group": "not-started"}],
                "changes": [],
            }),
            version,
        )?
        .under_lock(&json!({"kind": "workflow", "id": workflow}), "workflow")),
        other => Err(format!("no row for {other}").into()),
    }
}

#[tokio::test]
async fn iss_17_every_route_with_a_version_answers_a_stale_save_with_the_current_version()
-> TestResult {
    each_database("iss_17_conflict_everywhere", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        for id in VERSIONED {
            let save = prepare(id, &site, &mut ada)
                .await
                .map_err(|error| format!("{id}: setting up: {error}"))?;
            let mut body = save.body.clone();
            let mut path = save.path.clone();
            if save.in_query {
                path = format!("{path}?version={}", save.version + AHEAD);
            } else {
                body["version"] = json!(save.version + AHEAD);
            }
            if let Some(what) = &save.lock {
                let taken = ada
                    .send(
                        "POST",
                        LOCKS,
                        Some(json!({"target": what, "editor_id": AN_EDITOR})),
                    )
                    .await?;
                body["lock_token"] = ok(&taken)?["lock_token"].clone();
            }
            let body = (!save.in_query).then_some(body);
            let stale = ada.send(save.method, &path, body).await?;
            assert_eq!(stale.status, StatusCode::CONFLICT, "{id}: {}", stale.body);
            assert_eq!(stale.code(), "edit.version-changed", "{id}: {}", stale.body);
            assert_eq!(
                stale.body["conflict"]["version"],
                json!(save.version),
                "{id}: the answer names the version the record has now: {}",
                stale.body
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_a_combine_with_only_a_stale_into_version_names_the_record_combined_into()
-> TestResult {
    each_database("iss_17_conflict_into_version", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        for id in ["combineLabels", "combineTags"] {
            let save = prepare(id, &site, &mut ada).await?;
            // The record it goes into changes once, so the two versions differ.
            let (kind, name) = if id == "combineLabels" {
                ("labels", "renamed label")
            } else {
                ("tags", "renamed-tag")
            };
            let renamed = ada
                .send(
                    "PATCH",
                    &format!("/api/v1/{kind}/{}", text_in(&save.body, "into")?),
                    Some(json!({"version": save.body["into_version"], "name": name})),
                )
                .await?;
            let into_version = ok(&renamed)?["version"].as_i64().ok_or("no version")?;
            assert_ne!(into_version, save.version);
            let mut body = save.body.clone();
            // The record combined away is as the person saw it; the one it goes into is not.
            body["version"] = json!(save.version);
            body["into_version"] = json!(into_version + AHEAD);
            let stale = ada.send(save.method, &save.path, Some(body)).await?;
            assert_eq!(stale.code(), "edit.version-changed", "{id}: {}", stale.body);
            assert_eq!(
                stale.body["conflict"]["version"],
                json!(into_version),
                "{id}: the answer names the record it goes into: {}",
                stale.body
            );
        }
        Ok(())
    })
    .await
}

#[test]
fn iss_17_the_table_has_a_row_for_every_route_that_takes_a_version() -> TestResult {
    let description: Value = serde_json::from_str(bb_api::openapi::json())?;
    let mut found = BTreeSet::new();
    let operations = description["paths"]
        .as_object()
        .into_iter()
        .flatten()
        .flat_map(|(_, item)| item.as_object().into_iter().flatten().map(|(_, each)| each));
    for operation in operations {
        let schema = &operation["requestBody"]["content"]["application/json"]["schema"];
        let schema = match schema["$ref"]
            .as_str()
            .and_then(|name| name.rsplit('/').next())
        {
            Some(name) => &description["components"]["schemas"][name],
            None => schema,
        };
        let in_query = operation["parameters"].as_array().is_some_and(|each| {
            each.iter()
                .any(|parameter| parameter["name"] == "version" && parameter["in"] == "query")
        });
        if (schema["properties"].get("version").is_some() || in_query)
            && let Some(id) = operation["operationId"].as_str()
        {
            found.insert(id.to_owned());
        }
    }
    let rows: BTreeSet<String> = VERSIONED.iter().map(|id| (*id).to_owned()).collect();
    assert_eq!(
        found, rows,
        "the routes with a version and the rows of the table"
    );
    Ok(())
}
