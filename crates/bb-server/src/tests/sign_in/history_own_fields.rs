//! Own fields in the history of an issue (ISS-06, CF-06, SEC-06): a change of an own field is in
//! the history with the name of the field, by the same mask as a field of the catalogue, and a
//! field the reader may not see is in no row, not even as a gap. Both databases.

use bb_app::access::{Permission, StartingRole};
use bb_core::access::FieldLevel;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::{id_of, make_field, reader_of, severity};
use super::http::{Browser, rig_and_app};
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

/// Makes a second issue type, *Question*; its ID.
async fn question_type(ada: &mut Browser) -> Result<String, Failure> {
    let made = ada
        .send(
            "POST",
            "/api/v1/issue-types",
            Some(json!({"name": "Question", "icon": "star"})),
        )
        .await?;
    Ok(ok(&made)?["id"].as_str().ok_or("no type")?.to_owned())
}

/// Changes own fields of an issue, whatever version it has now; the answer has to be a success.
async fn change_fields(ada: &mut Browser, issue: &str, fields: Value) -> TestResult {
    let path = format!("{ISSUES}/{issue}");
    let version = ok(&ada.send("GET", &path, None).await?)?["issue"]["version"].clone();
    ok(&ada
        .send(
            "PATCH",
            &path,
            Some(json!({"version": version, "fields": fields})),
        )
        .await?)?;
    Ok(())
}

/// The items of the changes of an issue as `browser` reads them.
async fn history_of(browser: &mut Browser, issue: &str) -> Result<Vec<Value>, Failure> {
    let reply = browser
        .send(
            "GET",
            &format!("{ISSUES}/{issue}/history?tab=changes"),
            None,
        )
        .await?;
    Ok(changes(ok(&reply)?))
}

/// Whether an item is a change of this field.
fn is_change_of(item: &Value, field: &str) -> bool {
    item["field"] == json!(field)
}

/// The key of an issue in an answer.
fn key_in(made: &Value) -> Result<String, Failure> {
    Ok(made["key"].as_str().ok_or("no key")?.to_owned())
}

#[tokio::test]
async fn iss_06_history_follows_a_level_that_hides_a_field_for_one_issue_type_only() -> TestResult {
    each_database("iss_06_own_fields_type", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let centre = id_of(
            &make_field(
                &mut ada,
                json!({"name": "Cost centre", "kind": "text", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let question = question_type(&mut ada).await?;
        // The reader sees the field in general, but not on a Bug.
        let mut reader = reader_of(
            &site,
            &rig,
            "typed@example.test",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[
                (&centre, None, FieldLevel::CanSee),
                (&centre, Some(&site.bug), FieldLevel::Hidden),
                ("title", None, FieldLevel::CanSee),
            ],
        )
        .await?;
        let in_bug = key_in(ok(&site.make(&mut ada, "A bug").await?)?)?;
        let made = ada
            .send(
                "POST",
                &format!("{PROJECTS}/ACME/issues"),
                Some(json!({"issue_type": question, "title": "A question"})),
            )
            .await?;
        let in_question = key_in(ok(&made)?)?;
        let before = (
            history_of(&mut reader, &in_bug).await?.len(),
            history_of(&mut reader, &in_question).await?.len(),
        );
        // A change that touches only the field: on the Bug the reader gets no item for it at all.
        for issue in [&in_bug, &in_question] {
            change_fields(&mut ada, issue, json!({ &centre: "Secret-1" })).await?;
        }
        let bug = history_of(&mut reader, &in_bug).await?;
        assert_eq!(bug.len(), before.0, "{bug:?}");
        assert!(!bug.iter().any(|item| is_change_of(item, &centre)));
        assert!(!json!(bug).to_string().contains("Secret-1"));
        // On the other type the same change is there, with the name of the field.
        let other = history_of(&mut reader, &in_question).await?;
        assert_eq!(other.len(), before.1 + 1, "{other:?}");
        let shown = other
            .iter()
            .find(|item| is_change_of(item, &centre))
            .ok_or("no change of the cost centre on a Question")?;
        assert_eq!(shown["field_name"], "Cost centre");
        assert_eq!(shown["after_text"], "Secret-1");
        // Ada sees it on both.
        let all = history_of(&mut ada, &in_bug).await?;
        assert!(all.iter().any(|item| is_change_of(item, &centre)));
        Ok(())
    })
    .await
}

/// The first item that is a change of `field` and has a value after it.
fn change_to<'a>(items: &'a [Value], field: &str) -> Result<&'a Value, Failure> {
    items
        .iter()
        .find(|item| {
            is_change_of(item, field)
                && ["after_text", "after_value", "after_people"]
                    .iter()
                    .any(|side| !item[*side].is_null())
        })
        .ok_or_else(|| format!("no change to {field}: {items:?}").into())
}

#[tokio::test]
async fn iss_06_history_words_values_as_the_issue_page_does_and_hides_what_cannot_be_seen()
-> TestResult {
    each_database("iss_06_own_fields_values", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut fields = Vec::new();
        for (name, kind) in [
            ("Billable", "yes-no"),
            ("Effort", "duration"),
            ("Owner", "person"),
            ("Owners", "people"),
            ("Team", "group"),
            ("Twin", "issue"),
        ] {
            let made = make_field(
                &mut ada,
                json!({"name": name, "kind": kind, "applies_to": "issues"}),
            )
            .await?;
            fields.push(id_of(&made)?);
        }
        let [billable, effort, owner, owners, team, twin] = fields.as_slice() else {
            return Err("six fields".into());
        };
        let (mut vic, vic_id) = site
            .person_called(StartingRole::Viewer, "vic", "Vic Viewer")
            .await?;
        let group = id_of(ok(&ada
            .send("POST", "/api/v1/groups", Some(json!({"name": "Core"})))
            .await?)?)?;
        // Somebody who sees only the issues they reported, and so not the issue Twin names.
        let mut levels: Vec<(&str, Option<&str>, FieldLevel)> = fields
            .iter()
            .map(|field| (field.as_str(), None, FieldLevel::CanSee))
            .collect();
        levels.push(("title", None, FieldLevel::CanChange));
        let mut max = reader_of(
            &site,
            &rig,
            "max@example.test",
            &[
                Permission::SeeProjects,
                Permission::MakeIssues,
                Permission::ChangeOwnIssues,
            ],
            &levels,
        )
        .await?;
        let theirs = key_in(ok(&site.make(&mut max, "Max found it").await?)?)?;
        let hidden = ok(&site.make(&mut ada, "Only Ada sees it").await?)?.clone();
        let hidden_key = key_in(&hidden)?;
        let hidden_id = hidden["id"].as_str().ok_or("no id")?.to_owned();
        let issue = key_in(ok(&site.make(&mut ada, "Everyone sees it").await?)?)?;
        let values = json!({
            billable: true,
            effort: 5400,
            owner: vic_id.to_string(),
            owners: [vic_id.to_string()],
            team: &group,
            twin: &hidden_id,
        });
        change_fields(&mut ada, &issue, values.clone()).await?;
        change_fields(&mut ada, &theirs, values).await?;
        // Ada: typed values for the screen to word, people by name, a group by its name and an
        // issue by its readable ID.
        let seen = history_of(&mut ada, &issue).await?;
        let flag = change_to(&seen, billable)?;
        assert_eq!(flag["field_kind"], "yes-no", "{flag}");
        assert_eq!(flag["after_value"], true, "{flag}");
        assert!(flag["after_text"].is_null(), "{flag}");
        let time = change_to(&seen, effort)?;
        assert_eq!(time["field_kind"], "duration", "{time}");
        assert_eq!(time["after_value"], 5400, "{time}");
        for field in [owner, owners] {
            let people = &change_to(&seen, field)?["after_people"];
            assert_eq!(people[0]["name"], "Vic Viewer", "{people}");
        }
        assert_eq!(change_to(&seen, team)?["after_text"], "Core");
        assert_eq!(change_to(&seen, twin)?["after_text"], hidden_key.as_str());
        // Max cannot see the issue Twin names: no key, no value, and the ID is nowhere.
        let theirs_seen = history_of(&mut max, &theirs).await?;
        let text = json!(theirs_seen).to_string();
        assert!(!text.contains(&hidden_key), "{text}");
        assert!(!text.contains(&hidden_id), "{text}");
        assert!(
            theirs_seen
                .iter()
                .filter(|item| is_change_of(item, twin))
                .all(|item| item["after_text"].is_null()),
            "{text}"
        );
        // A viewer who may not see group names does not get the name of the group; with names, does.
        site.show_people(&mut ada, StartingRole::Viewer, "mentions", "hidden")
            .await?;
        let silent = json!(history_of(&mut vic, &issue).await?).to_string();
        assert!(!silent.contains("Core"), "{silent}");
        assert!(!silent.contains(&group), "{silent}");
        site.show_people(&mut ada, StartingRole::Viewer, "mentions", "names")
            .await?;
        let named = history_of(&mut vic, &issue).await?;
        assert_eq!(change_to(&named, team)?["after_text"], "Core");
        Ok(())
    })
    .await
}
