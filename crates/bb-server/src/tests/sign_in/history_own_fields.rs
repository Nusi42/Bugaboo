//! Own fields in the history of an issue (ISS-06, CF-06, SEC-06): a change of an own field is in
//! the history with the name of the field, by the same mask as a field of the catalogue, and a
//! field the reader may not see is in no row, not even as a gap. Both databases.

use bb_app::access::Permission;
use bb_core::access::FieldLevel;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::{id_of, make_field, reader_of, severity};
use super::http::rig_and_app;
use super::issues_site::{ISSUES, PROJECTS, Site, ok};

/// The items of the changes of an answer of the history.
fn changes(history: &Value) -> Vec<Value> {
    history["items"].as_array().cloned().unwrap_or_default()
}

#[tokio::test]
async fn iss_06_history_shows_an_own_field_by_the_mask_of_the_reader() -> TestResult {
    each_database("iss_06_own_fields", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let level = severity(&mut ada).await?;
        let (severity_id, minor, critical) = (
            id_of(&level)?,
            id_of(&level["choices"][0])?,
            id_of(&level["choices"][2])?,
        );
        let salary = make_field(
            &mut ada,
            json!({"name": "Salary band", "kind": "text", "applies_to": "issues"}),
        )
        .await?;
        let salary_id = id_of(&salary)?;
        let made = ada
            .send(
                "POST",
                &format!("{PROJECTS}/ACME/issues"),
                Some(json!({
                    "issue_type": site.bug,
                    "title": "An issue",
                    "fields": { &severity_id: &minor, &salary_id: "B-7" },
                })),
            )
            .await?;
        let issue = ok(&made)?["key"].as_str().ok_or("no key")?.to_owned();
        let version = ok(&made)?["version"].clone();
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({
                    "version": version,
                    "fields": { &severity_id: &critical, &salary_id: "C-9" },
                })),
            )
            .await?)?;
        // The reader sees the severity and not the salary band.
        let mut reader = reader_of(
            &site,
            &rig,
            "hist@example.test",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[
                (&severity_id, None, FieldLevel::CanSee),
                (&salary_id, None, FieldLevel::Hidden),
                ("title", None, FieldLevel::CanSee),
            ],
        )
        .await?;
        let path = format!("{ISSUES}/{issue}/history?tab=changes");
        // Ada sees both fields, each with its name and its values in words.
        let all = ada.send("GET", &path, None).await?;
        let all = changes(ok(&all)?);
        let severity_change = all
            .iter()
            .find(|item| item["field"] == json!(severity_id) && item["before_text"].is_string())
            .ok_or("no change of the severity")?;
        assert_eq!(severity_change["field_name"], "Severity");
        assert_eq!(severity_change["before_text"], "Minor");
        assert_eq!(severity_change["after_text"], "Critical");
        let salary_change = all
            .iter()
            .find(|item| item["field"] == json!(salary_id) && item["before_text"].is_string())
            .ok_or("no change of the salary band")?;
        assert_eq!(salary_change["field_name"], "Salary band");
        assert_eq!(salary_change["after_text"], "C-9");
        // The reader sees the severity, and nothing of the salary band: no key, no name, no text.
        let seen = reader.send("GET", &path, None).await?;
        let body = ok(&seen)?;
        let seen = changes(body);
        assert!(
            seen.iter()
                .any(|item| item["field"] == json!(severity_id) && item["field_name"] == "Severity"),
            "{body}"
        );
        let text = body.to_string();
        for hidden in [salary_id.as_str(), "Salary band", "B-7", "C-9"] {
            assert!(!text.contains(hidden), "{hidden} is in the history: {text}");
        }
        Ok(())
    })
    .await
}
