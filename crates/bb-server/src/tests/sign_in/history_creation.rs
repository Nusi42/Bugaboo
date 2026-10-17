//! The history of a new issue (ISS-06, M3-F185): it lists what was set at creation, and leaves out
//! the values that were empty ("set Assigned to: nobody"). Both databases.

use bb_app::access::StartingRole;
use bb_testing::{TestResult, each_database};
use serde_json::{Value, json};

use super::http::rig_and_app;
use super::issues_site::{ISSUES, Site, key_in, ok};

/// The fields the changes of an answer of the history name.
fn fields_in(history: &Value) -> Vec<String> {
    history["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["field"].as_str().map(str::to_owned))
        .collect()
}

#[tokio::test]
async fn iss_06_history_leaves_out_empty_values_at_creation() -> TestResult {
    each_database("iss_06_empty_at_creation", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_in(ok(&site.create(&mut ada, "Nobody has it yet").await?)?)?;
        let reply = ada
            .send(
                "GET",
                &format!("{ISSUES}/{issue}/history?tab=changes"),
                None,
            )
            .await?;
        let fields = fields_in(ok(&reply)?);
        assert!(
            fields.iter().any(|field| field == "title"),
            "the title is set at creation: {fields:?}"
        );
        for empty in ["assigned-to", "description"] {
            assert!(
                !fields.iter().any(|field| field == empty),
                "{empty} was empty at creation and is in the history: {fields:?}"
            );
        }
        // Assigning later is a change like any other, so the field is not left out for good.
        let version =
            ok(&ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?)?["issue"]["version"]
                .clone();
        let (_, cora) = site.person(StartingRole::Contributor, "cora").await?;
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": version, "assigned_to": [cora.to_string()]})),
            )
            .await?)?;
        let later = ada
            .send(
                "GET",
                &format!("{ISSUES}/{issue}/history?tab=changes"),
                None,
            )
            .await?;
        assert!(
            fields_in(ok(&later)?)
                .iter()
                .any(|field| field == "assigned-to"),
            "{}",
            later.body
        );
        Ok(())
    })
    .await
}
