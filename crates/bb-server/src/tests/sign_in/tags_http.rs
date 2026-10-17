//! Tags and labels over HTTP (TAG-01, TAG-02, TAG-03, SEC-06, PRIV-13): the tags offered while a
//! person types never come from a project they cannot see, a *Pick one* group keeps one label of
//! the group on an issue, and combining or renaming changes every issue once and says so in the
//! history of each. Every flow runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::IssueId;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, id_of, key_of, ok, version_of};

const LABELS: &str = "/api/v1/labels";
const GROUPS: &str = "/api/v1/label-groups";
const TAGS: &str = "/api/v1/tags";

/// The names in a list of tags or labels of an issue.
fn names(labels_and_tags: &Value, member: &str) -> Vec<String> {
    labels_and_tags[member]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["name"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// A label made as `ada`: its ID.
async fn a_label(ada: &mut Browser, name: &str, group: Option<&str>) -> Result<String, Failure> {
    let made = ada
        .send("POST", LABELS, Some(json!({"name": name, "group": group})))
        .await?;
    id_of(&made)
}

#[tokio::test]
async fn tag_01_autocomplete_never_offers_a_tag_only_used_in_a_hidden_project() -> TestResult {
    each_database("tag_01_hidden_tags", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        // A project that only Ada sees, with an issue that carries a tag nobody else uses.
        let secret = ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "SECRET", "name": "Secret"})),
            )
            .await?;
        let secret_id = id_of(&secret)?;
        let hidden = ada
            .send(
                "POST",
                &format!("{PROJECTS}/{secret_id}/issues"),
                Some(site.new_issue("A secret")),
            )
            .await?;
        let hidden = key_of(&hidden)?;
        ok(&ada
            .send(
                "POST",
                &format!("{ISSUES}/{hidden}/tags"),
                Some(json!({"name": "takeover-plan"})),
            )
            .await?)?;
        // And an open one, used in ACME.
        let open = key_of(&site.create(&mut ada, "Something open").await?)?;
        ok(&ada
            .send(
                "POST",
                &format!("{ISSUES}/{open}/tags"),
                Some(json!({"name": "takeover-fix"})),
            )
            .await?)?;
        // Bob is a team member of ACME only.
        let (mut bob, _) = site.person_in_acme(StartingRole::TeamMember, "bob").await?;
        let offered = bob
            .send(
                "GET",
                &format!("{PROJECTS}/ACME/tag-suggestions?text=takeover"),
                None,
            )
            .await?;
        let tags = names(ok(&offered)?, "tags");
        assert_eq!(tags, vec!["takeover-fix".to_owned()], "{}", offered.body);
        // Ada, who sees both, is offered both.
        let all = ada
            .send(
                "GET",
                &format!("{PROJECTS}/ACME/tag-suggestions?text=takeover"),
                None,
            )
            .await?;
        assert_eq!(
            names(ok(&all)?, "tags"),
            vec!["takeover-fix".to_owned(), "takeover-plan".to_owned()]
        );
        // The hidden project is not even a project for Bob.
        let nothing = bob
            .send(
                "GET",
                &format!("{PROJECTS}/SECRET/tag-suggestions?text=x"),
                None,
            )
            .await?;
        assert_eq!(nothing.code(), "resource.not_found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn tag_02_a_pick_one_group_keeps_one_label_per_issue() -> TestResult {
    each_database("tag_02_pick_one", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Login fails").await?)?;
        let group = id_of(
            &ada.send(
                "POST",
                GROUPS,
                Some(json!({"name": "Area", "pick_one": true})),
            )
            .await?,
        )?;
        let login = a_label(&mut ada, "login", Some(&group)).await?;
        let billing = a_label(&mut ada, "billing", Some(&group)).await?;
        let urgent = a_label(&mut ada, "urgent", None).await?;
        for label in [&login, &urgent, &billing] {
            let put = ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{issue}/labels"),
                    Some(json!({"label": label})),
                )
                .await?;
            ok(&put)?;
        }
        let labels_and_tags = ada
            .send("GET", &format!("{ISSUES}/{issue}/labels-and-tags"), None)
            .await?;
        // The clock of the rig stands still, so the order they came in is not told by the time.
        let mut held = names(ok(&labels_and_tags)?, "labels");
        held.sort();
        assert_eq!(held, vec!["billing".to_owned(), "urgent".to_owned()]);
        // A group cannot become a Pick one group while an issue has two of its labels.
        let area = id_of(
            &ada.send(
                "POST",
                GROUPS,
                Some(json!({"name": "Part", "pick_one": false})),
            )
            .await?,
        )?;
        let (a, b) = (
            a_label(&mut ada, "a", Some(&area)).await?,
            a_label(&mut ada, "b", Some(&area)).await?,
        );
        for label in [&a, &b] {
            ok(&ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{issue}/labels"),
                    Some(json!({"label": label})),
                )
                .await?)?;
        }
        let refused = ada
            .send(
                "PATCH",
                &format!("{GROUPS}/{area}"),
                Some(json!({"version": 1, "pick_one": true})),
            )
            .await?;
        assert_eq!(refused.code(), "label.pick_one_conflict");
        Ok(())
    })
    .await
}

/// The changes the history of an issue shows for the field `field`, as `before → after` names.
fn history_of(history: &Value, field: &str) -> Vec<(Vec<String>, Vec<String>)> {
    history["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|item| item["field"] == field)
                .map(|item| {
                    let list = |name: &str| -> Vec<String> {
                        item[name]
                            .as_array()
                            .map(|each| {
                                each.iter()
                                    .filter_map(|text| text.as_str().map(str::to_owned))
                                    .collect()
                            })
                            .unwrap_or_default()
                    };
                    (list("before_names"), list("after_names"))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn tag_03_combining_two_labels_changes_every_issue_once_and_records_it() -> TestResult {
    each_database("tag_03_combine_labels", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (only_old, both) = (
            key_of(&site.create(&mut ada, "Only the old label").await?)?,
            key_of(&site.create(&mut ada, "Both labels").await?)?,
        );
        let (old, new) = (
            a_label(&mut ada, "login", None).await?,
            a_label(&mut ada, "sign-in", None).await?,
        );
        for (issue, label) in [(&only_old, &old), (&both, &old), (&both, &new)] {
            ok(&ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{issue}/labels"),
                    Some(json!({"label": label})),
                )
                .await?)?;
            // The next one comes a moment later, so the order they came in is the order they read.
            site.org.rig.clock.advance(1);
        }
        let preview = ada
            .send(
                "POST",
                &format!("{LABELS}/{old}/preview"),
                Some(json!({"into": new})),
            )
            .await?;
        assert_eq!(ok(&preview)?["summary"]["will_change"], 2);
        let combined = ada
            .send(
                "POST",
                &format!("{LABELS}/{old}/combine"),
                Some(json!({"into": new, "version": 1, "into_version": 1})),
            )
            .await?;
        assert_eq!(ok(&combined)?["name"], "sign-in");
        for issue in [&only_old, &both] {
            let labels_and_tags = ada
                .send("GET", &format!("{ISSUES}/{issue}/labels-and-tags"), None)
                .await?;
            assert_eq!(
                names(ok(&labels_and_tags)?, "labels"),
                vec!["sign-in".to_owned()]
            );
            // The history of the issue says so, once.
            let history = ada
                .send(
                    "GET",
                    &format!("{ISSUES}/{issue}/history?tab=changes"),
                    None,
                )
                .await?;
            let changes = history_of(ok(&history)?, "labels");
            let last = changes
                .first()
                .ok_or("no change of labels in the history")?;
            assert_eq!(last.1, vec!["sign-in".to_owned()], "{issue}");
        }
        let history = ada
            .send("GET", &format!("{ISSUES}/{both}/history?tab=changes"), None)
            .await?;
        let changes = history_of(ok(&history)?, "labels");
        assert_eq!(
            changes.first(),
            Some(&(
                vec!["login".to_owned(), "sign-in".to_owned()],
                vec!["sign-in".to_owned()]
            )),
            "newest first"
        );
        // The label that went away is not in the list any more.
        let list = ada.send("GET", LABELS, None).await?;
        assert_eq!(names(ok(&list)?, "labels"), vec!["sign-in".to_owned()]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn tag_03_renaming_a_tag_asks_for_the_version_and_changes_every_issue() -> TestResult {
    each_database("tag_03_rename_tag", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Slow page").await?)?;
        let put = ada
            .send(
                "POST",
                &format!("{ISSUES}/{issue}/tags"),
                Some(json!({"name": "slow"})),
            )
            .await?;
        let tag = ok(&put)?["tags"][0]["id"]
            .as_str()
            .ok_or("no tag")?
            .to_owned();
        let list = ada.send("GET", TAGS, None).await?;
        assert_eq!(ok(&list)?["items"][0]["name"], "slow");
        let stale = ada
            .send(
                "PATCH",
                &format!("{TAGS}/{tag}"),
                Some(json!({"version": 7, "name": "sluggish"})),
            )
            .await?;
        assert_eq!(stale.code(), "edit.version_changed");
        let renamed = ada
            .send(
                "PATCH",
                &format!("{TAGS}/{tag}"),
                Some(json!({"version": 1, "name": "sluggish"})),
            )
            .await?;
        assert_eq!(ok(&renamed)?["name"], "sluggish");
        let labels_and_tags = ada
            .send("GET", &format!("{ISSUES}/{issue}/labels-and-tags"), None)
            .await?;
        assert_eq!(
            names(ok(&labels_and_tags)?, "tags"),
            vec!["sluggish".to_owned()]
        );
        // A person without the right to look after tags is refused.
        let (mut gus, _) = site.person(StartingRole::TeamMember, "gus").await?;
        let refused = gus
            .send(
                "PATCH",
                &format!("{TAGS}/{tag}"),
                Some(json!({"version": 2, "name": "x"})),
            )
            .await?;
        assert_eq!(refused.code(), "access.forbidden");
        Ok(())
    })
    .await
}

/// The ID of an issue, as the events are about it.
async fn entity_of(ada: &mut Browser, issue: &str) -> Result<EntityRef, Failure> {
    let read = ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?;
    let id: IssueId = ok(&read)?["issue"]["id"].as_str().ok_or("no ID")?.parse()?;
    Ok(EntityRef {
        kind: RecordKind::Issue.into(),
        id: id.into(),
    })
}

/// The versions the events of an issue count up to, lowest first (the events of a test share one
/// time, so their order is not the order of the versions).
async fn counted_versions(rig: &super::rig::Rig, entity: &EntityRef) -> Result<Vec<i64>, Failure> {
    let events = rig.database.events_about(entity, 100).await?;
    let mut versions: Vec<i64> = events
        .iter()
        .flat_map(|event| event.changes.iter())
        .filter(|change| change.field == bb_core::version::FIELD)
        .filter_map(|change| change.after.as_ref().and_then(Value::as_i64))
        .collect();
    versions.sort_unstable();
    Ok(versions)
}

#[tokio::test]
async fn adr_0017_two_people_taking_tags_off_at_the_same_moment_both_succeed() -> TestResult {
    each_database("adr_0017_removes_race", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut ben, _) = site.person_in_acme(StartingRole::TeamMember, "ben").await?;
        let issue = key_of(&site.create(&mut ada, "Crowded").await?)?;
        let mut tags = Vec::new();
        for name in ["one", "two", "three", "four"] {
            let put = ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{issue}/tags"),
                    Some(json!({"name": name})),
                )
                .await?;
            tags = ok(&put)?["tags"]
                .as_array()
                .ok_or("no tags")?
                .iter()
                .filter_map(|tag| tag["id"].as_str().map(str::to_owned))
                .collect();
        }
        let entity = entity_of(&mut ada, &issue).await?;
        let before = version_of(&mut ada, &issue).await?;
        let (path, other) = (format!("{ISSUES}/{issue}/tags"), tags.clone());
        let (first, second) = tokio::join!(
            async {
                for tag in &tags[..2] {
                    let reply = ada.send("DELETE", &format!("{path}/{tag}"), None).await?;
                    ok(&reply)?;
                }
                Ok::<(), Failure>(())
            },
            async {
                for tag in &other[2..] {
                    let reply = ben.send("DELETE", &format!("{path}/{tag}"), None).await?;
                    ok(&reply)?;
                }
                Ok::<(), Failure>(())
            }
        );
        first?;
        second?;
        assert_eq!(version_of(&mut ada, &issue).await?, before + 4);
        // The creation of the issue is no count-up: one event for each version after it, in a row.
        assert_eq!(
            counted_versions(&rig, &entity).await?,
            (2..=before + 4).collect::<Vec<_>>(),
            "one event for each version, in a row"
        );
        let labels_and_tags = ada
            .send("GET", &format!("{ISSUES}/{issue}/labels-and-tags"), None)
            .await?;
        assert!(names(ok(&labels_and_tags)?, "tags").is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0017_a_tag_added_while_the_title_is_saved_lands_and_the_version_agrees() -> TestResult
{
    each_database("adr_0017_add_and_title", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut ben, _) = site.person_in_acme(StartingRole::TeamMember, "ben").await?;
        let issue = key_of(&site.create(&mut ada, "Old title").await?)?;
        let entity = entity_of(&mut ada, &issue).await?;
        let before = version_of(&mut ada, &issue).await?;
        let (tags_path, issue_path) = (
            format!("{ISSUES}/{issue}/tags"),
            format!("{ISSUES}/{issue}"),
        );
        let (added, saved) = tokio::join!(
            ada.send("POST", &tags_path, Some(json!({"name": "urgent"}))),
            ben.send(
                "PATCH",
                &issue_path,
                Some(json!({"version": before, "title": "New title"})),
            )
        );
        // The tag never conflicts with anything: it takes no version.
        let added = added?;
        ok(&added)?;
        // The title is saved from the version it read; if the tag came first, it is told so
        // and saves again from the version it is told.
        let saved = saved?;
        if saved.code() == "edit.version_changed" {
            let now = version_of(&mut ben, &issue).await?;
            ok(&ben
                .send(
                    "PATCH",
                    &format!("{ISSUES}/{issue}"),
                    Some(json!({"version": now, "title": "New title"})),
                )
                .await?)?;
        } else {
            ok(&saved)?;
        }
        assert_eq!(version_of(&mut ada, &issue).await?, before + 2);
        let read = ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?;
        assert_eq!(ok(&read)?["issue"]["title"], "New title");
        let labels_and_tags = ada
            .send("GET", &format!("{ISSUES}/{issue}/labels-and-tags"), None)
            .await?;
        assert_eq!(
            names(ok(&labels_and_tags)?, "tags"),
            vec!["urgent".to_owned()]
        );
        assert_eq!(
            counted_versions(&rig, &entity).await?,
            (2..=before + 2).collect::<Vec<_>>(),
            "the versions of the events and of the row agree"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0017_tags_and_labels_of_an_issue_in_the_trash_are_not_found() -> TestResult {
    each_database("adr_0017_trash", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.create(&mut ada, "Soon gone").await?)?;
        let label = a_label(&mut ada, "bug", None).await?;
        let trashed = ada
            .send("DELETE", &format!("{ISSUES}/{issue}"), None)
            .await?;
        assert!(trashed.status.is_success(), "{}", trashed.body);
        let tag = ada
            .send(
                "POST",
                &format!("{ISSUES}/{issue}/tags"),
                Some(json!({"name": "late"})),
            )
            .await?;
        assert_eq!(tag.code(), "resource.not_found");
        let put = ada
            .send(
                "POST",
                &format!("{ISSUES}/{issue}/labels"),
                Some(json!({"label": label})),
            )
            .await?;
        assert_eq!(put.code(), "resource.not_found");
        Ok(())
    })
    .await
}
