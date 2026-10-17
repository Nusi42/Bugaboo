//! A long text own field is saved under its lock and comes back as HTML (CF-01).

use super::*;

/// The lock on the text of an own field of an issue; the answer that took it.
async fn take_notes_lock(ada: &mut Browser, id: &str, field: &str) -> Result<Reply, Failure> {
    let taken = ada
        .send(
            "POST",
            "/api/v1/edit-locks",
            Some(json!({
                "what": {"record": {"kind": "issue", "id": id}, "part": format!("field-{field}")},
                "editor_id": "7c1d2e3f-4a5b-4c6d-8e7f-a0b1c2d3e4f5",
            })),
        )
        .await?;
    assert_eq!(taken.body["status"], "taken", "{}", taken.body);
    Ok(taken)
}

/// A long text own field *Notes* and an issue with it, with the lock on that text taken by this
/// browser: the field, the issue as made, its readable ID and the answer that took the lock.
async fn locked_notes(
    site: &Site<'_>,
    ada: &mut Browser,
) -> Result<(String, Value, String, Reply), Failure> {
    let notes = create_field(
        ada,
        json!({"name": "Notes", "kind": "long-text", "applies_to": "issues"}),
    )
    .await?;
    let field = id_in(&notes)?;
    let made = ok(&site.create(ada, "Has notes").await?)?.clone();
    let (id, key) = (
        id_in(&made)?,
        made["key"].as_str().ok_or("no key")?.to_owned(),
    );
    let taken = take_notes_lock(ada, &id, &field).await?;
    Ok((field, made, key, taken))
}

#[tokio::test]
async fn cf_01_a_long_text_own_field_is_saved_under_its_lock_and_comes_back_as_html() -> TestResult
{
    each_database("cf_01_long_text", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (field, made, key, taken) = locked_notes(&site, &mut ada).await?;
        let saved = ada
            .send(
                "PUT",
                &format!("{ISSUES}/{key}/fields/{field}/text"),
                Some(json!({
                    "version": made["version"],
                    "lock_token": taken.body["lock_token"],
                    "text": "Steps: **bold**",
                })),
            )
            .await?;
        assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        // The page gets the text and the HTML the server made of it, for a viewer who may see it.
        let seen = ada.send("GET", &format!("{ISSUES}/{key}"), None).await?;
        let detail = ok(&seen)?;
        assert_eq!(detail["issue"]["own_fields"][&field], "Steps: **bold**");
        assert!(
            detail["own_fields_html"][0]["html"]
                .as_str()
                .is_some_and(|html| html.contains("<strong>bold</strong>")),
            "{detail}"
        );
        assert_eq!(detail["own_fields_html"][0]["field"], json!(field));
        // A role that may not see the field gets neither.
        let mut reader = reader_of(
            &site,
            &rig,
            "rae@example.test",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[
                (&field, None, FieldLevel::Hidden),
                ("title", None, FieldLevel::CanSee),
            ],
        )
        .await?;
        let theirs = reader.send("GET", &format!("{ISSUES}/{key}"), None).await?;
        assert!(
            !ok(&theirs)?.to_string().contains("bold"),
            "{}",
            theirs.body
        );
        assert!(
            ok(&theirs)?["own_fields_html"]
                .as_array()
                .is_some_and(Vec::is_empty)
        );
        Ok(())
    })
    .await
}
