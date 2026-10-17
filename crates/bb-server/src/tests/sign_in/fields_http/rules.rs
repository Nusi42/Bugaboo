//! Own fields refuse a wrong value with a sentence and are not offered where they do not belong (CF-03).

use super::*;

#[tokio::test]
async fn cf_03_a_required_field_and_a_rule_refuse_a_wrong_value_with_a_sentence() -> TestResult {
    each_database("cf_03_rules", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let customer = create_field(
            &mut ada,
            json!({
                "name": "Customer",
                "kind": "text",
                "applies_to": "issues",
                "rules": {"required": true, "max_length": 5, "pattern": "[A-Za-z]+"},
            }),
        )
        .await?;
        let key = id_in(&customer)?;
        // A refusal names the field it is about, so the form can show it there (M3 WP-13).
        let refused = |reply: &Reply, code: &str, field: &str| {
            assert_eq!(
                (reply.status, reply.code(), reply.body["field"].as_str()),
                (StatusCode::BAD_REQUEST, code, Some(field)),
                "{}",
                reply.body
            );
        };
        refused(
            &create_with(&mut ada, &site.kind, json!({})).await?,
            "field.required",
            &key,
        );
        refused(
            &create_with(&mut ada, &site.kind, json!({ &key: "Customer" })).await?,
            "field.too_long",
            &key,
        );
        refused(
            &create_with(&mut ada, &site.kind, json!({ &key: "Ab1" })).await?,
            "field.no_match",
            &key,
        );
        refused(
            &create_with(&mut ada, &site.kind, json!({ &key: 7 })).await?,
            "field.wrong_value",
            &key,
        );
        let made = create_with(&mut ada, &site.kind, json!({ &key: "Acme" })).await?;
        let issue = ok(&made)?;
        assert_eq!(issue["own_fields"][&key], "Acme");
        // A change is refused the same way, and a required field cannot be emptied.
        let path = format!("{ISSUES}/{}", issue["key"].as_str().ok_or("no key")?);
        let version = &issue["version"];
        refused(
            &ada.send(
                "PATCH",
                &path,
                Some(json!({"version": version, "fields": { &key: "Acmeee" }})),
            )
            .await?,
            "field.too_long",
            &key,
        );
        refused(
            &ada.send(
                "PATCH",
                &path,
                Some(json!({"version": version, "fields": { &key: null }})),
            )
            .await?,
            "field.required",
            &key,
        );
        // A choice the field does not offer.
        let level = severity(&mut ada).await?;
        let level_key = id_in(&level)?;
        refused(
            &ada.send(
                "PATCH",
                &path,
                Some(json!({
                    "version": version,
                    "fields": { id_in(&level)?: "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c" },
                })),
            )
            .await?,
            "field.unknown_choice",
            &level_key,
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_03_a_field_for_one_project_is_not_offered_elsewhere() -> TestResult {
    each_database("cf_03_scope", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let beta = ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project")?
            .to_owned();
        let field = create_field(
            &mut ada,
            json!({
                "name": "Customer",
                "kind": "text",
                "applies_to": "issues",
                "scope": {"projects": [site.acme.clone()]},
            }),
        )
        .await?;
        let key = id_in(&field)?;
        // Offered in ACME, not in BETA.
        let in_acme = ada.send("GET", &format!("{FIELDS}?project=ACME"), None).await?;
        assert_eq!(ok(&in_acme)?["fields"].as_array().map(Vec::len), Some(1));
        let in_beta = ada.send("GET", &format!("{FIELDS}?project=BETA"), None).await?;
        assert_eq!(ok(&in_beta)?["fields"].as_array().map(Vec::len), Some(0));
        let body = |project: &str| {
            json!({"issue_type": site.kind, "title": "Here", "fields": { &key: "Acme" }, "project": project})
        };
        let there = ada
            .send("POST", &format!("{PROJECTS}/{beta}/issues"), Some(body("beta")))
            .await?;
        assert_eq!(
            (there.status, there.code()),
            (StatusCode::BAD_REQUEST, "field.not_here"),
            "{}",
            there.body
        );
        let here = ada.send("POST", &acme_issues(), Some(body("acme"))).await?;
        assert_eq!(ok(&here)?["own_fields"][&key], "Acme");
        // Where the field is used, listed before a change.
        let usage = ada
            .send("GET", &format!("{FIELDS}/{key}/used-by"), None)
            .await?;
        assert_eq!(ok(&usage)?["whole_organization"], false);
        assert_eq!(ok(&usage)?["projects"][0]["short_name"], "ACME");
        Ok(())
    })
    .await
}
