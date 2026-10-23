//! Combining and retiring the choices of an own field (CF-07).

use super::*;

#[tokio::test]
async fn cf_07_combining_two_choices_moves_every_value_and_keeps_a_record() -> TestResult {
    each_database("cf_07_combine", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let level = severity(&mut ada).await?;
        let (key, minor, major) = (
            id_in(&level)?,
            id_in(&level["choices"][0])?,
            id_in(&level["choices"][1])?,
        );
        let mut issues = Vec::new();
        for (title, choice) in [("First", &major), ("Second", &minor), ("Third", &major)] {
            let made = ok(&ada
                .send(
                    "POST",
                    &acme_issues(),
                    Some(
                        json!({"issue_type": site.kind, "title": title, "fields": { &key: choice }}),
                    ),
                )
                .await?)?
            .clone();
            issues.push((
                made["readable_id"].as_str().ok_or("no key")?.to_owned(),
                made["version"].clone(),
            ));
        }
        let combined = ada
            .send(
                "POST",
                &format!("{FIELDS}/{key}/choices/{major}/combine"),
                Some(json!({"version": level["version"], "into": minor})),
            )
            .await?;
        let after = ok(&combined)?;
        assert_eq!(
            after["version"],
            json!(level["version"].as_i64().unwrap_or(0) + 1)
        );
        assert_eq!(after["choices"][1]["retired"], true);
        for (issue, version) in &issues {
            let seen = ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?;
            assert_eq!(
                ok(&seen)?["issue"]["own_fields"][&key],
                json!(minor),
                "{issue}"
            );
            // The issue that held the combined choice has a new version: its history has a record.
            let moved = ok(&seen)?["issue"]["version"].as_i64().unwrap_or(0);
            let before = version.as_i64().unwrap_or(0);
            assert!(moved == before || moved == before + 1, "{issue}");
        }
        assert_eq!(
            ok(&ada
                .send("GET", &format!("{ISSUES}/{}", issues[0].0), None)
                .await?)?["issue"]["version"],
            json!(issues[0].1.as_i64().unwrap_or(0) + 1)
        );
        assert_eq!(
            ok(&ada
                .send("GET", &format!("{ISSUES}/{}", issues[1].0), None)
                .await?)?["issue"]["version"],
            issues[1].1
        );
        // A person who looked at the first issue before is told that the field changed.
        let stale = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{}", issues[0].0),
                Some(json!({"version": issues[0].1, "title": "Late"})),
            )
            .await?;
        assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.body);
        assert_eq!(stale.body["conflict"]["current"][&key], json!(minor));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_07_a_retired_choice_stays_on_old_issues_only() -> TestResult {
    each_database("cf_07_retire", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let level = severity(&mut ada).await?;
        let (key, major) = (id_in(&level)?, id_in(&level["choices"][1])?);
        let old = ok(&ada
            .send(
                "POST",
                &acme_issues(),
                Some(json!({"issue_type": site.kind, "title": "Old", "fields": { &key: &major }})),
            )
            .await?)?
        .clone();
        let retired = ada
            .send(
                "PATCH",
                &format!("{FIELDS}/{key}/choices/{major}"),
                Some(json!({"version": level["version"], "retired": true})),
            )
            .await?;
        ok(&retired)?;
        // The old issue keeps the choice.
        let seen = ada
            .send(
                "GET",
                &format!("{ISSUES}/{}", old["readable_id"].as_str().ok_or("no key")?),
                None,
            )
            .await?;
        assert_eq!(ok(&seen)?["issue"]["own_fields"][&key], json!(major));
        // A new issue cannot get it.
        let refused = ada
            .send(
                "POST",
                &acme_issues(),
                Some(json!({"issue_type": site.kind, "title": "New", "fields": { &key: &major }})),
            )
            .await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::BAD_REQUEST, "field.unknown-choice"),
            "{}",
            refused.body
        );
        // The old issue may be saved with the value it has.
        let kept = ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{}", old["readable_id"].as_str().ok_or("no key")?),
                Some(json!({"version": old["version"], "fields": { &key: &major }})),
            )
            .await?;
        ok(&kept)?;
        // A role that works with fields of the starting roles: a Team member sees and changes it.
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;
        let list = tim
            .send("GET", &format!("{FIELDS}?project=ACME"), None)
            .await?;
        assert_eq!(ok(&list)?["fields"].as_array().map(Vec::len), Some(1));
        Ok(())
    })
    .await
}
