//! People, groups and issues in own fields (CF-01, ISS-01): a name in a field follows the rule of
//! *Assigned to* (the people who may see the issue), the list to choose from offers the same
//! people, an issue that cannot be seen cannot be named, and *Undo* can put back what a field
//! held just before.

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_core::access::FieldLevel;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::{FIELDS, create_field, reader_of, set_levels, severity};
use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, id_in, key_in, ok, version_of};
use super::policy_rig::Member;

/// A person who does not exist.
const NOBODY: &str = "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c";

/// The IDs in a list of people.
fn ids_of(listed: &Value) -> Result<Vec<String>, Failure> {
    Ok(listed["items"]
        .as_array()
        .ok_or("no list")?
        .iter()
        .filter_map(|person| person["id"].as_str().map(str::to_owned))
        .collect())
}

/// Changes the own fields of an issue on this version; the answer.
async fn set_on_issue(
    who: &mut Browser,
    issue: &Value,
    version: i64,
    fields: Value,
) -> Result<Reply, Failure> {
    who.send(
        "PATCH",
        &format!("{ISSUES}/{}", key_in(issue)?),
        Some(json!({"version": version, "fields": fields})),
    )
    .await
}

/// The status and the code of an answer; no code for a success.
fn outcome(reply: &Reply) -> (StatusCode, &str) {
    (
        reply.status,
        if reply.status == StatusCode::OK {
            ""
        } else {
            reply.code()
        },
    )
}

#[tokio::test]
async fn cf_01_a_person_in_a_field_is_judged_like_assigned_to_and_the_list_offers_the_same()
-> TestResult {
    each_database("cf_01_people_rule", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let tester = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Tester", "kind": "person", "applies_to": "issues"}),
            )
            .await?,
        )?;
        // A Team member fills in the field but may not change *Assigned to*.
        let team = site.org.starting_roles().await?[StartingRole::TeamMember.code()];
        set_levels(
            &site,
            &rig,
            team,
            &[("assigned-to", None, FieldLevel::CanSee)],
        )
        .await?;
        let (mut filler, _) = site.person(StartingRole::TeamMember, "fil").await?;
        let (mut gus, gus_id) = site.person(StartingRole::Guest, "gus").await?;
        let (gus_id, outsider) = (gus_id.to_string(), site.outsider("olga").await?.to_string());
        let by_gus = ok(&site.create(&mut gus, "Gus found it").await?)?.clone();
        let by_ada = ok(&site.create(&mut ada, "Ada found it").await?)?.clone();
        let list = format!("{PROJECTS}/{}/assignable-people", site.acme);
        let for_gus = format!("issue={}", by_gus["readable_id"].as_str().ok_or("no key")?);
        // The list for *Assigned to* still needs the right to change *Assigned to*; the list for
        // a field needs the right to change that field.
        let plain = filler.send("GET", &list, None).await?;
        assert_eq!(plain.code(), "access.forbidden", "{}", plain.body);
        let for_field = filler
            .send("GET", &format!("{list}?field={tester}&{for_gus}"), None)
            .await?;
        // The issue was reported by the Guest, who may see it: they are on the list, which judges
        // people for the issue that is meant, not for one the reader reported.
        assert!(
            ids_of(ok(&for_field)?)?.contains(&gus_id),
            "{}",
            for_field.body
        );
        let other = ada.send("GET", &format!("{list}?{for_gus}"), None).await?;
        assert!(ids_of(ok(&other)?)?.contains(&gus_id), "{}", other.body);
        let none = ada.send("GET", &list, None).await?;
        assert!(!ids_of(ok(&none)?)?.contains(&gus_id), "{}", none.body);
        // The server judges a name the same way: the Guest in the issue they reported, not in
        // one they cannot see; nobody who has no role or does not exist.
        let version = version_of(&mut ada, &key_in(&by_gus)?).await?;
        let named = set_on_issue(&mut filler, &by_gus, version, json!({ &tester: gus_id })).await?;
        assert_eq!(outcome(&named), (StatusCode::OK, ""), "{}", named.body);
        // A person who is not there: an ID like the Guest's, with another last letter.
        let other_end = if gus_id.ends_with('a') { 'b' } else { 'a' };
        let gone = format!("{}{other_end}", &gus_id[..gus_id.len() - 1]);
        let refusals = [
            (&by_ada, gus_id.clone()),
            (&by_gus, outsider),
            (&by_gus, gone),
        ];
        for (issue, person) in refusals {
            let version = version_of(&mut ada, &key_in(issue)?).await?;
            let refused =
                set_on_issue(&mut filler, issue, version, json!({ &tester: person })).await?;
            assert_eq!(
                outcome(&refused),
                (StatusCode::BAD_REQUEST, "field.unknown-reference"),
                "{}",
                refused.body
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_08_a_person_in_a_field_of_a_project_has_to_see_the_project() -> TestResult {
    each_database("cf_08_people_rule", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let sponsor = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Sponsor", "kind": "person", "applies_to": "projects"}),
            )
            .await?,
        )?;
        let me = id_in(ok(&ada.send("GET", "/api/v1/auth/me", None).await?)?)?;
        let outsider = site.outsider("olga").await?.to_string();
        let project = format!("{PROJECTS}/{}", site.acme);
        let version = ok(&ada.send("GET", &project, None).await?)?["project"]["version"].clone();
        let refused = ada
            .send(
                "PATCH",
                &project,
                Some(json!({"version": version, "fields": { &sponsor: outsider }})),
            )
            .await?;
        assert_eq!(
            outcome(&refused),
            (StatusCode::BAD_REQUEST, "field.unknown-reference"),
            "{}",
            refused.body
        );
        let named = ada
            .send(
                "PATCH",
                &project,
                Some(json!({"version": version, "fields": { &sponsor: me }})),
            )
            .await?;
        assert_eq!(outcome(&named), (StatusCode::OK, ""), "{}", named.body);
        // The list for the field holds the people who see the project, not the outsider.
        let list = ada
            .send(
                "GET",
                &format!("{project}/assignable-people?field={sponsor}"),
                None,
            )
            .await?;
        let ids = ids_of(ok(&list)?)?;
        assert!(
            ids.contains(&me) && !ids.contains(&outsider),
            "{}",
            list.body
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_01_an_issue_in_a_field_has_to_be_one_the_setter_may_see() -> TestResult {
    each_database("cf_01_issue_rule", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let duplicate = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Duplicate of", "kind": "issue", "applies_to": "issues"}),
            )
            .await?,
        )?;
        // Somebody who sees and changes only the issues they reported.
        let mut maker = reader_of(
            &site,
            &rig,
            "max@example.test",
            &[
                Permission::SeeProjects,
                Permission::CreateIssues,
                Permission::ChangeOwnIssues,
            ],
            &[
                (&duplicate, None, FieldLevel::CanChange),
                ("title", None, FieldLevel::CanChange),
            ],
        )
        .await?;
        let theirs = ok(&site.create(&mut maker, "Max found it").await?)?.clone();
        let hidden = ok(&site.create(&mut ada, "Only Ada sees it").await?)?.clone();
        for named in [hidden["id"].clone(), json!(NOBODY)] {
            let refused = set_on_issue(
                &mut maker,
                &theirs,
                theirs["version"].as_i64().ok_or("no version")?,
                json!({ &duplicate: named }),
            )
            .await?;
            assert_eq!(
                outcome(&refused),
                (StatusCode::BAD_REQUEST, "field.unknown-reference"),
                "{}",
                refused.body
            );
        }
        // Ada sees both and names the first from the second.
        let named = set_on_issue(
            &mut ada,
            &hidden,
            hidden["version"].as_i64().ok_or("no version")?,
            json!({ &duplicate: theirs["id"] }),
        )
        .await?;
        assert_eq!(outcome(&named), (StatusCode::OK, ""), "{}", named.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_07_undo_puts_back_what_the_field_held_just_before() -> TestResult {
    each_database("cf_07_undo", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let level = severity(&mut ada).await?;
        let (key, minor, major) = (
            id_in(&level)?,
            id_in(&level["choices"][0])?,
            id_in(&level["choices"][1])?,
        );
        let tester = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Tester", "kind": "person", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let (_, vic) = site.person(StartingRole::Viewer, "vic").await?;
        let me = id_in(ok(&ada.send("GET", "/api/v1/auth/me", None).await?)?)?;
        let made = ok(&ada
            .send(
                "POST",
                &format!("{PROJECTS}/ACME/issues"),
                Some(json!({
                    "issue_type": site.kind,
                    "title": "Undo me",
                    "fields": { &key: &minor, &tester: vic.to_string() },
                })),
            )
            .await?)?
        .clone();
        // One change; then the choice it replaced is retired and the person loses access.
        let changed = set_on_issue(
            &mut ada,
            &made,
            made["version"].as_i64().ok_or("no version")?,
            json!({ &key: &major, &tester: me }),
        )
        .await?;
        ok(&changed)?;
        let retired = ada
            .send(
                "PATCH",
                &format!("{FIELDS}/{key}/choices/{minor}"),
                Some(json!({"version": level["version"], "retired": true})),
            )
            .await?;
        ok(&retired)?;
        let gone = Member {
            id: vic,
            session: String::new(),
        };
        site.org
            .forbid(
                &gone,
                site.acme.parse().map_err(|error| format!("{error}"))?,
            )
            .await?;
        // Undo sends the old values on the version the change made: accepted.
        let version = version_of(&mut ada, &key_in(&made)?).await?;
        let values = json!({ &key: &minor, &tester: vic.to_string() });
        let undone = set_on_issue(&mut ada, &made, version, values).await?;
        assert_eq!(outcome(&undone), (StatusCode::OK, ""), "{}", undone.body);
        // The same values after another change are no Undo: they are judged as new values.
        let version = version_of(&mut ada, &key_in(&made)?).await?;
        let again = json!({ &key: &major, &tester: me });
        ok(&set_on_issue(&mut ada, &made, version, again).await?)?;
        let version = version_of(&mut ada, &key_in(&made)?).await?;
        let path = format!("{ISSUES}/{}", made["readable_id"].as_str().ok_or("no key")?);
        let renamed = ada
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "title": "Renamed"})),
            )
            .await?;
        ok(&renamed)?;
        let version = version_of(&mut ada, &key_in(&made)?).await?;
        let late = set_on_issue(&mut ada, &made, version, json!({ &key: &minor })).await?;
        assert_eq!(
            outcome(&late),
            (StatusCode::BAD_REQUEST, "field.unknown-choice"),
            "{}",
            late.body
        );
        let late = set_on_issue(
            &mut ada,
            &made,
            version,
            json!({ &tester: vic.to_string() }),
        )
        .await?;
        assert_eq!(
            outcome(&late),
            (StatusCode::BAD_REQUEST, "field.unknown-reference"),
            "{}",
            late.body
        );
        Ok(())
    })
    .await
}

/// Whether the list offers each person and whether a change accepts them, for *Assigned to* and
/// for the own field of people, as this viewer: each pair is `(offered, accepted)`.
async fn offered_and_accepted(
    viewer: &mut Browser,
    ada: &mut Browser,
    site: &Site<'_>,
    tester: &str,
    people: &[String],
) -> Result<Vec<(String, bool, bool, bool, bool)>, Failure> {
    let made = ok(&site.create(ada, "Who is named").await?)?.clone();
    let key = made["readable_id"].as_str().ok_or("no key")?.to_owned();
    let list = format!("{PROJECTS}/{}/assignable-people", site.acme);
    let assigned = ok(&viewer
        .send("GET", &format!("{list}?issue={key}"), None)
        .await?)?
    .clone();
    let in_field = ok(&viewer
        .send("GET", &format!("{list}?issue={key}&field={tester}"), None)
        .await?)?
    .clone();
    let (assigned, in_field) = (ids_of(&assigned)?, ids_of(&in_field)?);
    let mut seen = Vec::new();
    for person in people {
        let version = version_of(ada, &key_in(&made)?).await?;
        let path = format!("{ISSUES}/{key}");
        let assign = viewer
            .send(
                "PATCH",
                &path,
                Some(json!({"version": version, "assigned_to": [person]})),
            )
            .await?;
        let version = version_of(ada, &key_in(&made)?).await?;
        let field = set_on_issue(viewer, &made, version, json!({ tester: person })).await?;
        seen.push((
            person.clone(),
            assigned.contains(person),
            assign.status == StatusCode::OK,
            in_field.contains(person),
            field.status == StatusCode::OK,
        ));
    }
    Ok(seen)
}

#[tokio::test]
async fn cf_01_every_person_the_list_offers_is_accepted_and_no_other() -> TestResult {
    each_database("cf_01_list_and_check_agree", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let tester = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Tester", "kind": "person", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let (mut tim, tim_id) = site.person(StartingRole::TeamMember, "tim").await?;
        let (_, cora) = site.person(StartingRole::Contributor, "cora").await?;
        let (_, vera) = site.person(StartingRole::Viewer, "vera").await?;
        let outsider = site.outsider("olga").await?;
        let me = id_in(ok(&ada.send("GET", "/api/v1/auth/me", None).await?)?)?;
        let people: Vec<String> = [tim_id, cora, vera, outsider]
            .iter()
            .map(ToString::to_string)
            .chain([me, NOBODY.to_owned()])
            .collect();
        // Shown names: the list offers those who may see the issue, and a change accepts them.
        let named = offered_and_accepted(&mut tim, &mut ada, &site, &tester, &people).await?;
        for (person, offered, accepted, in_field, in_field_accepted) in &named {
            assert_eq!(offered, accepted, "assigned to {person}");
            assert_eq!(in_field, in_field_accepted, "field {person}");
            assert_eq!(offered, in_field, "both lists {person}");
        }
        assert!(named.iter().any(|seen| seen.1) && named.iter().any(|seen| !seen.1));
        // Shown only themselves: the list offers the viewer alone, and a change accepts only them.
        site.show_people(
            &mut ada,
            StartingRole::TeamMember,
            "project-members",
            "role-only",
        )
        .await?;
        let alone = offered_and_accepted(&mut tim, &mut ada, &site, &tester, &people).await?;
        for (person, offered, accepted, in_field, in_field_accepted) in &alone {
            assert_eq!(offered, accepted, "alone, assigned to {person}");
            assert_eq!(in_field, in_field_accepted, "alone, field {person}");
            assert_eq!(*offered, *person == tim_id.to_string(), "alone {person}");
            assert_eq!(offered, in_field, "alone, both lists {person}");
        }
        Ok(())
    })
    .await
}
