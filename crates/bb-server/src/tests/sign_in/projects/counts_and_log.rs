//! No answer holds a count, and every change writes its event and log entry (PRIV-13, PRJ-01).

use super::*;

#[tokio::test]
async fn priv_13_no_project_answer_holds_a_count() -> TestResult {
    each_database("priv_13_no_counts", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        // Two top-level projects, each with hidden sub-projects (three and none).
        let loud = create(&org, &admin, None, "LOUD").await?;
        let quiet = create(&org, &admin, None, "QUIET").await?;
        for short_name in ["ONE", "TWO", "THREE"] {
            create(&org, &admin, Some(loud.id), short_name).await?;
        }
        let viewer = org.role("Looker", &VIEW).await?;
        let bea = org.person().await?;
        // Bea sees the two projects themselves, and none of what is inside.
        for project in [&loud, &quiet] {
            org.assign(
                &org.admin,
                Subject::Person(bea.id),
                Given::Role(viewer.id),
                Some(project.id),
                AppliesTo::Project,
            )
            .await?;
        }
        let top = list(&org, &bea, None, None).await?;
        assert_eq!(names(&top), ["LOUD", "QUIET"]);
        // The projects that hold hidden sub-projects look exactly like the one that holds none.
        assert!(top.items.iter().all(|item| !item.has_sub_projects));
        assert!(
            list(&org, &bea, Some(loud.id), None)
                .await?
                .items
                .is_empty()
        );
        let (a, b) = (
            detail(&org, &bea, "LOUD").await?,
            detail(&org, &bea, "QUIET").await?,
        );
        assert_eq!(a.permissions, b.permissions);
        assert_eq!(a.trail, b.trail);

        // No property of any answer is a count of anything, and the administrator's answers
        // hold none either: `has_sub_projects` is a yes or no.
        let answers = [
            serde_json::to_value(&top)?,
            serde_json::to_value(&a)?,
            serde_json::to_value(preview(&org, &admin, quiet.id, Some(loud.id)).await?)?,
            serde_json::to_value(list(&org, &admin, None, None).await?)?,
            serde_json::to_value(&loud)?,
        ];
        for answer in &answers {
            assert_no_count(answer, "");
        }
        // The summary of the preview counts the people the answer lists, not one more.
        let moved = serde_json::to_value(preview(&org, &admin, quiet.id, Some(loud.id)).await?)?;
        assert_eq!(
            moved["summary"]["will_change"].as_u64(),
            moved["people"].as_array().map(|people| people.len() as u64)
        );
        assert_eq!(moved["summary"]["skipped"], 0);
        Ok(())
    })
    .await
}

/// No member of the JSON is named like a count, and no member holds a number except the version
/// of a project (a number of changes made to that one project, which counts nothing hidden) and
/// the two numbers of the summary of a preview, which count what the answer lists.
fn assert_no_count(value: &serde_json::Value, path: &str) {
    match value {
        serde_json::Value::Object(members) => {
            for (name, inner) in members {
                let at = format!("{path}/{name}");
                for counter in [
                    "count",
                    "total",
                    "number",
                    "size",
                    "length",
                    "sub_projects_count",
                ] {
                    assert!(!name.contains(counter), "{at} looks like a count");
                }
                if inner.is_number() {
                    // The summary of a preview counts only the people the answer lists
                    // (`people.len()`), which is checked where the answer is made.
                    let summary =
                        matches!(name.as_str(), "will_change" | "skipped") && path == "/summary";
                    assert!(name == "version" || summary, "{at} is a number");
                }
                assert_no_count(inner, &at);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                assert_no_count(item, path);
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn prj_01_changes_write_their_event_and_entry_of_the_log() -> TestResult {
    each_database("prj_01_event_and_log", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let admin = org.admin.clone();
        let web = create(&org, &admin, None, "WEB").await?;
        let entries = |action: SecurityAction| {
            let database = rig.database.clone();
            async move {
                Ok::<_, bb_store::StoreError>(
                    database
                        .security_log_entries(0, 10_000)
                        .await?
                        .iter()
                        .filter(|entry| entry.action.as_str() == action.code())
                        .count(),
                )
            }
        };
        let renamed = change(&org, &admin, &web, Some("The website"), None).await?;
        assert_eq!(entries(SecurityAction::ProjectRenamed).await?, 1);
        change(&org, &admin, &renamed, None, Some("SITE")).await?;
        assert_eq!(entries(SecurityAction::ProjectShortNameChanged).await?, 1);
        // A refused change writes nothing.
        let before = rig.database.security_log_entries(0, 10_000).await?.len();
        assert!(
            change(&org, &admin, &web, Some("Stale"), None)
                .await
                .is_err()
        );
        assert_eq!(
            rig.database.security_log_entries(0, 10_000).await?.len(),
            before
        );
        Ok(())
    })
    .await
}
