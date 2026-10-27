//! The filter and the name of a group of a search, for every kind of group and for the names that
//! stand for more than one thing (QRY-04, M5 WP-08): a filter is given only when its name resolves,
//! by the one resolution of names of a search, to exactly the thing of the group; the filter run as
//! a search finds the group's issues.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{PROJECTS, Site, id_in, key_of, ok};
use super::search_groups_http::{GROUPS, create_in, encoded, every_group_selects_its_issues};

const LABELS: &str = "/api/v1/labels";
const FIELDS: &str = "/api/v1/fields";
const ISSUES: &str = "/api/v1/issues";

/// The groups of the field `by`, as the server answers them.
async fn groups_of(who: &mut Browser, by: &str) -> Result<Vec<Value>, Failure> {
    let reply = who
        .send("GET", &format!("{GROUPS}?by={by}&q={}", encoded("")), None)
        .await?;
    Ok(ok(&reply)?["groups"].as_array().ok_or("no groups")?.clone())
}

/// A field of issues, made as the administrator; its answer.
async fn field(ada: &mut Browser, name: &str, kind: &str, more: Value) -> Result<Value, Failure> {
    let mut body = json!({"name": name, "kind": kind, "applies_to": "issues"});
    if let (Some(body), Some(more)) = (body.as_object_mut(), more.as_object()) {
        body.extend(more.clone());
    }
    let made = ada.send("POST", FIELDS, Some(body)).await?;
    Ok(ok(&made)?.clone())
}

#[tokio::test]
async fn qry_04_two_people_of_one_name_have_no_filter_and_a_third_has() -> TestResult {
    each_database("search_group_two_people", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut first, _) = site
            .person_called(StartingRole::TeamMember, "ana1", "Ana Bell")
            .await?;
        let (mut second, _) = site
            .person_called(StartingRole::TeamMember, "ana2", "Ana Bell")
            .await?;
        let (mut bob, _) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Builder")
            .await?;
        for (who, title) in [
            (&mut first, "One"),
            (&mut second, "Two"),
            (&mut bob, "Three"),
        ] {
            create_in(who, &site.acme, site.new_issue(title)).await?;
        }
        let groups = groups_of(&mut ada, "reportedBy").await?;
        assert_eq!(groups.len(), 3, "{groups:?}");
        for group in &groups {
            let name = group["name"].as_str().ok_or("a group without a name")?;
            match name {
                // A name that two people have cannot select one of them.
                "Ana Bell" => assert!(group["filter"].is_null(), "{group}"),
                _ => assert!(group["filter"].is_string(), "{group}"),
            }
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_04_the_groups_of_own_values_have_a_filter_that_selects_their_issues() -> TestResult {
    every_kind(
        "search_group_own_values",
        &[
            "severity", "points", "weight", "effort", "urgent", "due", "note",
        ],
    )
    .await
}

#[tokio::test]
async fn qry_04_the_groups_of_things_have_a_filter_that_selects_their_issues() -> TestResult {
    every_kind(
        "search_group_things",
        &["labels", "tags", "parent", "project", "type", "status"],
    )
    .await
}

/// Issues with a value in each kind of field, and the groups by these fields: every group has a
/// filter that selects exactly its issues (a search is allowed a few a minute, so a test asks for
/// some of the fields).
async fn every_kind(name: &str, fields: &[&str]) -> TestResult {
    each_database(name, async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let severity = field(
            &mut ada,
            "Severity",
            "pick-one",
            json!({"choices": [{"name": "Minor"}, {"name": "Major"}]}),
        )
        .await?;
        let (severity_key, minor, major) = (
            id_in(&severity)?,
            id_in(&severity["choices"][0])?,
            id_in(&severity["choices"][1])?,
        );
        let points = id_in(&field(&mut ada, "Points", "whole-number", json!({})).await?)?;
        let weight = id_in(&field(&mut ada, "Weight", "decimal", json!({})).await?)?;
        let effort = id_in(&field(&mut ada, "Effort", "duration", json!({})).await?)?;
        let urgent = id_in(&field(&mut ada, "Urgent", "yes-no", json!({})).await?)?;
        let due = id_in(&field(&mut ada, "Due", "date", json!({})).await?)?;
        let moment = id_in(&field(&mut ada, "Seen", "date-time", json!({})).await?)?;
        let note = id_in(&field(&mut ada, "Note", "text", json!({})).await?)?;
        let label = id_in(ok(&ada
            .send(
                "POST",
                LABELS,
                Some(json!({"name": "Ready", "label_group": null})),
            )
            .await?)?)?;
        let parent = key_of(&site.create(&mut ada, "The parent").await?)?;
        for at in 0..3_u32 {
            let mut body = site.new_issue(&format!("Issue {at}"));
            body["fields"] = json!({
                &severity_key: if at == 0 { &minor } else { &major },
                &points: i64::from(at) + 1,
                &weight: format!("{at}.5"),
                &effort: i64::from(at + 1) * 90 * 60,
                &urgent: at == 1,
                &due: format!("2026-10-{:02}", at + 10),
                &moment: "2026-10-10T10:00:00Z",
                &note: format!("Note {at}"),
            });
            body["parent"] = json!(parent);
            let made = ada
                .send(
                    "POST",
                    &format!("{PROJECTS}/{}/issues", site.acme),
                    Some(body),
                )
                .await?;
            let key = key_of(&made)?;
            ok(&ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{key}/labels"),
                    Some(json!({"label": label})),
                )
                .await?)?;
            ok(&ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{key}/tags"),
                    Some(json!({"name": format!("tag-{at}")})),
                )
                .await?)?;
        }
        for by in fields {
            every_group_selects_its_issues(&mut ada, by, "")
                .await
                .map_err(|error| format!("by {by}: {error}"))?;
        }
        // A moment is no value to ask for: its groups say which issues they hold, never as a day.
        let by_moment = groups_of(&mut ada, "seen").await?;
        assert!(
            by_moment
                .iter()
                .filter(|group| group["key"]["type"] == "whole")
                .all(|group| group["filter"].is_null()),
            "{by_moment:?}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_04_over_the_cap_the_groups_are_cut_and_the_ones_without_a_value_stay() -> TestResult {
    each_database("search_group_cap", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let note = id_in(&field(&mut ada, "Note", "text", json!({})).await?)?;
        for at in 0..205_u32 {
            let mut body = site.new_issue(&format!("Issue {at}"));
            if at > 0 {
                body["fields"] = json!({&note: format!("Note {at:03}")});
            }
            create_in(&mut ada, &site.acme, body).await?;
        }
        let reply = ada.send("GET", &format!("{GROUPS}?by=note"), None).await?;
        let body = ok(&reply)?;
        let groups = body["groups"].as_array().ok_or("no groups")?;
        assert_eq!(groups.len(), 200);
        assert_eq!(body["truncated"], true);
        assert!(
            groups
                .last()
                .is_some_and(|group| group["filter"] == "note is empty")
        );
        Ok(())
    })
    .await
}

/// The groups by `by` whose name is `name`.
fn named<'a>(groups: &'a [Value], name: &str) -> Vec<&'a Value> {
    groups
        .iter()
        .filter(|group| group["name"].as_str() == Some(name))
        .collect()
}

#[tokio::test]
async fn qry_04_two_labels_of_one_name_in_two_label_groups_have_no_filter() -> TestResult {
    each_database("search_group_two_labels", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut labels = Vec::new();
        for group in ["Area", "Kind"] {
            let made = ok(&ada
                .send(
                    "POST",
                    "/api/v1/label-groups",
                    Some(json!({"name": group, "pick_one": false})),
                )
                .await?)?
            .clone();
            let label = ok(&ada
                .send(
                    "POST",
                    LABELS,
                    Some(json!({"name": "Urgent", "label_group": id_in(&made)?})),
                )
                .await?)?
            .clone();
            labels.push(id_in(&label)?);
        }
        for (at, label) in labels.iter().enumerate() {
            let key = key_of(&site.create(&mut ada, &format!("Issue {at}")).await?)?;
            ok(&ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{key}/labels"),
                    Some(json!({"label": label})),
                )
                .await?)?;
        }
        let groups = groups_of(&mut ada, "labels").await?;
        let urgent = named(&groups, "Urgent");
        assert_eq!(urgent.len(), 2, "{groups:?}");
        assert!(urgent.iter().all(|group| group["filter"].is_null()));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_04_two_types_of_one_name_in_sibling_projects_have_no_filter() -> TestResult {
    each_database("search_group_two_types", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "BETA", "name": "Beta"})),
            )
            .await?)?;
        for project in [site.acme.as_str(), "BETA"] {
            let made = ok(&ada
                .send(
                    "POST",
                    "/api/v1/issue-types",
                    Some(json!({"name": "Question", "icon": "star", "project": project})),
                )
                .await?)?
            .clone();
            let body = json!({"issue_type": id_in(&made)?, "title": "Asked"});
            create_in(&mut ada, project, body).await?;
        }
        let groups = groups_of(&mut ada, "type").await?;
        let questions = named(&groups, "Question");
        assert_eq!(questions.len(), 2, "{groups:?}");
        assert!(questions.iter().all(|group| group["filter"].is_null()));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_04_a_group_of_people_field_has_a_filter_that_selects_its_issues() -> TestResult {
    each_database("search_group_of_people", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let team = id_in(&field(&mut ada, "Team", "group", json!({})).await?)?;
        for (at, name) in ["Core", "Edge"].into_iter().enumerate() {
            let group = id_in(ok(&ada
                .send("POST", "/api/v1/groups", Some(json!({"name": name})))
                .await?)?)?;
            let mut body = site.new_issue(&format!("Issue {at}"));
            body["fields"] = json!({&team: group});
            create_in(&mut ada, &site.acme, body).await?;
        }
        create_in(&mut ada, &site.acme, site.new_issue("Alone")).await?;
        let found = every_group_selects_its_issues(&mut ada, "team", "").await?;
        assert_eq!(found, 3);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_04_a_person_with_a_quote_in_the_name_is_found_by_the_filter_of_their_group()
-> TestResult {
    each_database("search_group_quoted_name", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut ana, _) = site
            .person_called(StartingRole::TeamMember, "ana", "Ana \"Bell\" O'Neil")
            .await?;
        create_in(&mut ana, &site.acme, site.new_issue("By Ana")).await?;
        let found = every_group_selects_its_issues(&mut ada, "reportedBy", "").await?;
        assert_eq!(found, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_04_a_renamed_project_and_a_renamed_type_give_their_new_names() -> TestResult {
    each_database("search_group_renames", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let made = ok(&ada
            .send(
                "POST",
                "/api/v1/issue-types",
                Some(json!({"name": "Query", "icon": "star", "project": site.acme})),
            )
            .await?)?
        .clone();
        let body = json!({"issue_type": id_in(&made)?, "title": "Asked"});
        create_in(&mut ada, &site.acme, body).await?;
        ok(&ada
            .send(
                "PATCH",
                &format!("/api/v1/issue-types/{}", id_in(&made)?),
                Some(json!({"version": made["version"], "name": "Ask"})),
            )
            .await?)?;
        ok(&ada
            .send(
                "PATCH",
                &format!("{PROJECTS}/{}", site.acme),
                Some(json!({"version": 1, "short_name": "ASKME", "name": "Ask me"})),
            )
            .await?)?;
        for (by, name) in [("type", "Ask"), ("project", "ASKME")] {
            let groups = groups_of(&mut ada, by).await?;
            let found = named(&groups, name);
            assert_eq!(found.len(), 1, "{by}: {groups:?}");
            assert!(found[0]["filter"].is_string(), "{by}: {groups:?}");
        }
        every_group_selects_its_issues(&mut ada, "type", "").await?;
        Ok(())
    })
    .await
}
