//! What issue answers hold and leave out (SEC-04, SEC-06, PRIV-13): a field a role may not see is in
//! no answer and cannot be asked for, and no answer holds a count.

use axum::http::StatusCode;
use bb_app::access::{Masked, Permission, StartingRole};
use bb_app::issues::IssueView;
use bb_app::roles::SetFieldLevel;
use bb_core::access::FieldLevel;
use bb_core::fields::Field;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, ok};
use super::policy_rig::Member;
use super::rig::Rig;

/// The members of an issue that hold a field of the catalogue, with the field.
fn field_members() -> &'static [(&'static str, Field)] {
    <IssueView as Masked>::FIELDS
}

/// A person with a role of their own that sees only the title of an issue: no other field has a
/// level, so every other field is hidden from them.
async fn title_only_reader(site: &Site<'_>, rig: &Rig) -> Result<Browser, Failure> {
    let role = site
        .org
        .role(
            "Title reader",
            &[Permission::SeeProjects, Permission::SeeIssues],
        )
        .await?;
    rig.api()
        .set_field_level(
            Rig::info(),
            SetFieldLevel {
                session_token: site.org.admin.session.clone(),
                role: role.id,
                project: None,
                issue_type: None,
                field: Field::Title.into(),
                level: Some(FieldLevel::CanSee),
            },
        )
        .await?;
    let email = "reader@example.test";
    let started = rig.person(&site.org.admin.session, email).await?;
    let member = Member {
        id: started.person.id,
        session: started.session_token,
    };
    site.org.give_everywhere(&member, role.id).await?;
    Browser::signed_in_as(site.app, email).await
}

/// What the hidden members of an issue are for a person who holds this level of every field.
fn hidden_names(level: impl Fn(Field) -> FieldLevel) -> Vec<&'static str> {
    field_members()
        .iter()
        .filter(|(_, field)| level(*field) == FieldLevel::Hidden)
        .map(|(name, _)| *name)
        .collect()
}

/// What a reader of the issue `key` sees of it and may ask for, given the members that are hidden
/// from them.
async fn check_reader(
    name: &str,
    reader: &mut Browser,
    hidden: &[&str],
    key: &str,
) -> Result<(), Failure> {
    let detail = reader.send("GET", &format!("{ISSUES}/{key}"), None).await?;
    let issue = &ok(&detail)?["issue"];
    // `closed_at` is there only while the issue is closed, which this one is not.
    for (member, _) in field_members() {
        if hidden.contains(member) {
            assert!(
                issue.get(*member).is_none(),
                "{name}: `{member}` in the issue"
            );
        } else if *member != "closed_at" {
            assert!(
                issue.get(*member).is_some(),
                "{name}: `{member}` not in the issue"
            );
        }
    }
    let unknown = reader
        .send(
            "GET",
            &format!("{PROJECTS}/ACME/issues?fields=nothing"),
            None,
        )
        .await?;
    for member in hidden {
        let asked = reader
            .send(
                "GET",
                &format!("{PROJECTS}/ACME/issues?fields={member}"),
                None,
            )
            .await?;
        assert_eq!(
            (asked.status, asked.code()),
            (StatusCode::BAD_REQUEST, "request.unknown_field"),
            "{name}: `{member}` can be asked for"
        );
        assert_eq!(asked.body["code"], unknown.body["code"]);
    }
    // The members that may be seen are in the list when asked for.
    for (member, _) in field_members()
        .iter()
        .filter(|(m, _)| !hidden.contains(m) && *m != "closed_at")
    {
        let list = reader
            .send(
                "GET",
                &format!("{PROJECTS}/ACME/issues?fields={member}"),
                None,
            )
            .await?;
        assert_eq!(
            list.status,
            StatusCode::OK,
            "{name}: `{member}`: {}",
            list.body
        );
        assert!(
            list.body["items"][0].get(*member).is_some(),
            "{name}: `{member}`"
        );
    }
    Ok(())
}

#[tokio::test]
async fn sec_06_issue_answers_refuse_every_field_the_role_may_not_see() -> TestResult {
    each_database("sec_06_issue_fields", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, tim) = site.person(StartingRole::TeamMember, "tim").await?;
        let made = ada
            .send(
                "POST",
                &format!("{PROJECTS}/ACME/issues"),
                Some(json!({
                    "issue_type": site.bug,
                    "title": "Seen by everyone",
                    "description": "Hidden from some",
                    "priority": "high",
                    "assigned_to": [tim.to_string()],
                })),
            )
            .await?;
        let key = ok(&made)?["key"].as_str().ok_or("no key")?.to_owned();
        let mut readers = vec![(
            "title reader".to_owned(),
            title_only_reader(&site, &rig).await?,
        )];
        let mut starting = Vec::new();
        for role in StartingRole::ALL
            .iter()
            .copied()
            .filter(|role| *role != StartingRole::Guest)
        {
            let (browser, _) = site
                .person(role, &format!("{}-reader", role.code()))
                .await?;
            starting.push((role, browser));
        }
        for (role, browser) in starting {
            readers.push((role.code().to_owned(), browser));
        }
        for (name, mut reader) in readers {
            let hidden: Vec<&str> = if name == "title reader" {
                hidden_names(|field| {
                    if field == Field::Title {
                        FieldLevel::CanSee
                    } else {
                        FieldLevel::Hidden
                    }
                })
            } else {
                let role = StartingRole::from_code(&name).ok_or("no such role")?;
                hidden_names(|field| role.field_level(field))
            };
            check_reader(&name, &mut reader, &hidden, &key).await?;
        }
        Ok(())
    })
    .await
}

/// A person with a role that changes issues, may change the title and holds `others` for the
/// other fields a change can name.
async fn changer(
    site: &Site<'_>,
    rig: &Rig,
    email: &str,
    others: FieldLevel,
) -> Result<Browser, Failure> {
    let role = site
        .org
        .role(
            email,
            &[
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::ChangeIssues,
                Permission::CreateIssues,
            ],
        )
        .await?;
    for field in [
        Field::Title,
        Field::Description,
        Field::Status,
        Field::Priority,
        Field::AssignedTo,
    ] {
        let level = if field == Field::Title {
            FieldLevel::CanChange
        } else {
            others
        };
        rig.api()
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: site.org.admin.session.clone(),
                    role: role.id,
                    project: None,
                    issue_type: None,
                    field: field.into(),
                    level: Some(level),
                },
            )
            .await?;
    }
    let started = rig.person(&site.org.admin.session, email).await?;
    let member = Member {
        id: started.person.id,
        session: started.session_token,
    };
    site.org.give_everywhere(&member, role.id).await?;
    Browser::signed_in_as(site.app, email).await
}

#[tokio::test]
async fn sec_04_an_unchanged_value_of_a_field_i_may_not_change_is_refused_like_a_changed_one()
-> TestResult {
    each_database("sec_04_unchanged_value", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, tim) = site.person(StartingRole::TeamMember, "tim").await?;
        let made = ada
            .send(
                "POST",
                &format!("{PROJECTS}/ACME/issues"),
                Some(json!({
                    "issue_type": site.bug,
                    "title": "Guess me",
                    "description": "A secret",
                    "priority": "high",
                    "assigned_to": [tim.to_string()],
                })),
            )
            .await?;
        let path = format!("{ISSUES}/{}", ok(&made)?["key"].as_str().ok_or("no key")?);
        // The values the issue has now: sending one of them back must not tell the sender that
        // it is right, so it is refused like a value that differs.
        let now = [
            ("status", json!("open")),
            ("priority", json!("high")),
            ("assigned_to", json!([tim.to_string()])),
        ];
        for (name, level) in [("see", FieldLevel::CanSee), ("hidden", FieldLevel::Hidden)] {
            let mut who = changer(&site, &rig, &format!("{name}@example.test"), level).await?;
            let seen = ada.send("GET", &path, None).await?;
            let version = ok(&seen)?["issue"]["version"].clone();
            for (member, value) in &now {
                let mut body = json!({"version": version, "title": "Renamed"});
                body[*member] = value.clone();
                let refused = who.send("PATCH", &path, Some(body)).await?;
                assert_eq!(
                    (refused.status, refused.code()),
                    (StatusCode::FORBIDDEN, "access.forbidden"),
                    "{name}: `{member}`"
                );
            }
            // The title alone is theirs to change.
            let renamed = who
                .send(
                    "PATCH",
                    &path,
                    Some(json!({"version": version, "title": name})),
                )
                .await?;
            assert_eq!(ok(&renamed)?["title"], name);
        }
        Ok(())
    })
    .await
}

/// Every key of every object in the answer.
fn keys(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(members) => {
            for (key, inner) in members {
                found.push(key.clone());
                keys(inner, found);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| keys(item, found)),
        _ => {}
    }
}

#[tokio::test]
async fn priv_13_no_issue_answer_holds_a_count() -> TestResult {
    each_database("priv_13_no_counts", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        for title in ["One", "Two", "Three"] {
            ok(&site.create(&mut ada, title).await?)?;
        }
        ok(&site.create(&mut gus, "Gus's").await?)?;
        let key = ok(&site.create(&mut ada, "Four").await?)?["key"]
            .as_str()
            .ok_or("no key")?
            .to_owned();
        let all_fields = "id,key,project,issue_type,title,description,status,priority,reported_by,assigned_to,created_at,updated_at,closed_at,version";
        let answers = [
            ada.send(
                "GET",
                &format!("{PROJECTS}/ACME/issues?limit=2&fields={all_fields}"),
                None,
            )
            .await?,
            gus.send("GET", &format!("{PROJECTS}/ACME/issues"), None).await?,
            ada.send("GET", &format!("{ISSUES}/{key}"), None).await?,
            ada.send("GET", "/api/v1/issue-types", None).await?,
        ];
        let numbers = ["count", "total", "size", "length", "number", "sum"];
        for answer in &answers {
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            let mut found = Vec::new();
            keys(&answer.body, &mut found);
            for key in found {
                assert!(
                    !numbers.iter().any(|word| key.contains(word)),
                    "`{key}` counts something: {}",
                    answer.body
                );
            }
        }
        // A page says only whether there is a next one: a cursor, never how many are left.
        assert!(answers[0].body["next_cursor"].is_string());
        assert!(answers[1].body["next_cursor"].is_null());
        assert_eq!(answers[1].body["items"].as_array().map(Vec::len), Some(1));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_04_making_an_issue_refuses_a_field_one_may_not_set_whatever_its_value() -> TestResult {
    each_database("sec_04_create_mask_first", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, _ada) = Site::start(&rig, &app).await?;
        let (_, tim) = site.person(StartingRole::TeamMember, "tim").await?;
        let mut who = changer(&site, &rig, "maker@example.test", FieldLevel::CanSee).await?;
        let too_long = "x".repeat(bb_core::text::LONG_TEXT_MAX + 1);
        let nobody = "01900000-0000-4000-8000-000000000000";
        // A valid value, an invalid value and an empty list: the same answer for each, so the
        // answer tells nothing about the value (does this person exist, is this text too long).
        let cases = [
            ("description", json!("Fine")),
            ("description", json!(too_long)),
            ("assigned_to", json!([tim.to_string()])),
            ("assigned_to", json!([nobody])),
            ("assigned_to", json!([])),
            ("status", json!("done")),
            ("priority", json!("high")),
        ];
        for (member, value) in cases {
            let mut body = site.new_issue("Not for them");
            body[member] = value.clone();
            let reply = who
                .send("POST", &format!("{PROJECTS}/ACME/issues"), Some(body))
                .await?;
            assert_eq!(
                (reply.status, reply.code()),
                (StatusCode::FORBIDDEN, "access.forbidden"),
                "{member}: {value}"
            );
        }
        // The title alone is theirs to set.
        ok(&site.create(&mut who, "Only a title").await?)?;
        Ok(())
    })
    .await
}
