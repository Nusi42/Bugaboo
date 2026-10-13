//! Tags and labels over HTTP (TAG-01, TAG-02, TAG-03, SEC-06, PRIV-13): the tags offered while a
//! person types never come from a project they cannot see, a *Pick one* group keeps one label of
//! the group on an issue, and combining or renaming changes every issue once and says so in the
//! history of each. Every flow runs on SQLite and PostgreSQL.

use bb_app::access::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, ok};

const LABELS: &str = "/api/v1/labels";
const GROUPS: &str = "/api/v1/label-groups";
const TAGS: &str = "/api/v1/tags";

fn key_of(reply: &super::http::Reply) -> Result<String, Failure> {
    ok(reply)?["key"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "the answer holds no issue ID".into())
}

fn id_of(reply: &super::http::Reply) -> Result<String, Failure> {
    ok(reply)?["id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "the answer holds no ID".into())
}

/// The names in a list of tags or labels of an issue.
fn names(sorting: &Value, member: &str) -> Vec<String> {
    sorting[member]
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
        let open = key_of(&site.make(&mut ada, "Something open").await?)?;
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
        let issue = key_of(&site.make(&mut ada, "Login fails").await?)?;
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
        let sorting = ada
            .send("GET", &format!("{ISSUES}/{issue}/sorting"), None)
            .await?;
        // The clock of the rig stands still, so the order they came in is not told by the time.
        let mut held = names(ok(&sorting)?, "labels");
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
            key_of(&site.make(&mut ada, "Only the old label").await?)?,
            key_of(&site.make(&mut ada, "Both labels").await?)?,
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
            let sorting = ada
                .send("GET", &format!("{ISSUES}/{issue}/sorting"), None)
                .await?;
            assert_eq!(names(ok(&sorting)?, "labels"), vec!["sign-in".to_owned()]);
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
        let issue = key_of(&site.make(&mut ada, "Slow page").await?)?;
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
        let sorting = ada
            .send("GET", &format!("{ISSUES}/{issue}/sorting"), None)
            .await?;
        assert_eq!(names(ok(&sorting)?, "tags"), vec!["sluggish".to_owned()]);
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
