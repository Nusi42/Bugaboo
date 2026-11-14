//! The stale saves of issues, their texts and comments: the rows of the table of `conflict_http` for these routes.

use bb_testing::Failure;
use serde_json::json;

use super::{Browser, Save, made};
use crate::sign_in_tests::fields_http::FIELDS;
use crate::sign_in_tests::issues_site::{ISSUES, PROJECTS, Site, ok, text_in};

pub(super) async fn prepare(id: &str, site: &Site<'_>, ada: &mut Browser) -> Result<Save, Failure> {
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
        "deleteComment" | "deleteIssue" => prepare_delete(id, site, ada).await,
        "acceptWarning" | "setTimeFactor" => prepare_plan(id, site, ada).await,
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

/// A comment or an issue to put into the trash on a version that is not its own.
async fn prepare_delete(id: &str, site: &Site<'_>, ada: &mut Browser) -> Result<Save, Failure> {
    if id == "deleteComment" {
        let issue = ok(&site.create(ada, "Commented").await?)?.clone();
        let comment = made(
            ada,
            &format!("{ISSUES}/{}/comments", text_in(&issue, "readable_id")?),
            json!({"text": "First"}),
        )
        .await?;
        return Ok(Save::new(
            "DELETE",
            format!("/api/v1/comments/{}", text_in(&comment, "id")?),
            json!({}),
            &comment["version"],
        )?
        .in_the_query());
    }
    let issue = ok(&site.create(ada, "To delete").await?)?.clone();
    Ok(Save::new(
        "DELETE",
        format!("{ISSUES}/{}", text_in(&issue, "readable_id")?),
        json!({}),
        &issue["version"],
    )?
    .in_the_query())
}

/// An issue with its creator as the person on the bar, and a stale *Keep and accept* or change of
/// its time factor.
async fn prepare_plan(id: &str, site: &Site<'_>, ada: &mut Browser) -> Result<Save, Failure> {
    let made = ok(&site.create(ada, "Planned").await?)?.clone();
    let key = text_in(&made, "readable_id")?;
    // Ada, who made it, is put on the bar.
    let assigned = ok(&ada
        .send(
            "PATCH",
            &format!("{ISSUES}/{key}"),
            Some(json!({"version": made["version"], "assigned_to": [made["reported_by"]["id"]]})),
        )
        .await?)?
    .clone();
    let version = &assigned["version"];
    if id == "acceptWarning" {
        Save::new(
            "POST",
            "/api/v1/timeline/accepted-warnings".to_owned(),
            json!({"issue": key, "kind": "delay", "timeline": {"zoom": "week"}}),
            version,
        )
    } else {
        Save::new(
            "PUT",
            format!("{ISSUES}/{key}/time-factor"),
            json!({"percent": 150}),
            version,
        )
    }
}
