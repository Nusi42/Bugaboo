//! The exit review of M3 (WP-12), tags and labels (SEC-06, TAG-01, TAG-04, PRIV-13): a tag is
//! offered, listed or shown only where the person reads the issues and the field *Tags* or *Labels*.
//! Every flow runs on SQLite and PostgreSQL.

use bb_app::access::Permission;
use bb_core::access::FieldLevel;
use bb_core::fields::Field;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::exit_masks::{issue_of_type, other_type, person_with_levels};
use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, PROJECTS, Site, ok};

const TAGS: &str = "/api/v1/tags";

/// The names in a list of tags or labels.
fn names(list: &Value, member: &str) -> Vec<String> {
    list[member]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["name"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Puts a tag on an issue: the answer.
async fn tag(browser: &mut Browser, issue: &str, name: &str) -> Result<Value, Failure> {
    let put = browser
        .send(
            "POST",
            &format!("{ISSUES}/{issue}/tags"),
            Some(json!({"name": name})),
        )
        .await?;
    Ok(ok(&put)?.clone())
}

#[tokio::test]
async fn sec_06_a_person_who_sees_only_their_own_issues_is_offered_only_their_own_tags()
-> TestResult {
    each_database("sec_06_suggest_own", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut gus, _) = person_with_levels(
            &site,
            &rig,
            "gus",
            &[
                Permission::SeeProjects,
                Permission::MakeIssues,
                Permission::ChangeOwnIssues,
            ],
            &[
                (Field::Title.into(), None, FieldLevel::CanChange),
                (Field::Tags.into(), None, FieldLevel::CanChange),
            ],
        )
        .await?;
        let theirs = site.make(&mut gus, "Gus's problem").await?;
        let theirs = ok(&theirs)?["key"].as_str().ok_or("no key")?.to_owned();
        let others = site.make(&mut ada, "Ada's problem").await?;
        let others = ok(&others)?["key"].as_str().ok_or("no key")?.to_owned();
        tag(&mut gus, &theirs, "plan-a").await?;
        tag(&mut ada, &others, "plan-b").await?;
        let path = format!("{PROJECTS}/ACME/tag-suggestions?text=plan");
        let offered = gus.send("GET", &path, None).await?;
        assert_eq!(names(ok(&offered)?, "tags"), ["plan-a"], "{}", offered.body);
        // Ada sees every issue and is offered both.
        let all = ada.send("GET", &path, None).await?;
        assert_eq!(names(ok(&all)?, "tags"), ["plan-a", "plan-b"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_nothing_is_offered_where_the_role_hides_tags_and_none_shown_on_the_issue()
-> TestResult {
    each_database("sec_06_tags_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let task = other_type(&site, &mut ada).await?;
        // Tags and labels are seen on bugs; on tasks they are hidden.
        let (mut hal, _) = person_with_levels(
            &site,
            &rig,
            "hal",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[
                (Field::Title.into(), None, FieldLevel::CanSee),
                (Field::Tags.into(), Some(&site.bug), FieldLevel::CanSee),
                (Field::Labels.into(), Some(&site.bug), FieldLevel::CanSee),
            ],
        )
        .await?;
        let (bug, _) = issue_of_type(&site, &mut ada, &site.bug, "A bug").await?;
        let (job, _) = issue_of_type(&site, &mut ada, &task, "A task").await?;
        let label = ok(&ada
            .send(
                "POST",
                "/api/v1/labels",
                Some(json!({"name": "urgent", "group": null})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no label")?
            .to_owned();
        for issue in [&bug, &job] {
            tag(&mut ada, issue, "secret-plan").await?;
            ok(&ada
                .send(
                    "POST",
                    &format!("{ISSUES}/{issue}/labels"),
                    Some(json!({"label": label})),
                )
                .await?)?;
        }
        // Where the role sees the field, the tag and the label are on the answer.
        let seen = hal
            .send("GET", &format!("{ISSUES}/{bug}/sorting"), None)
            .await?;
        assert_eq!(names(ok(&seen)?, "tags"), ["secret-plan"]);
        assert_eq!(names(&seen.body, "labels"), ["urgent"]);
        // On a task the answer holds neither, like the history of the task.
        let hidden = hal
            .send("GET", &format!("{ISSUES}/{job}/sorting"), None)
            .await?;
        assert!(names(ok(&hidden)?, "tags").is_empty(), "{}", hidden.body);
        assert!(names(&hidden.body, "labels").is_empty(), "{}", hidden.body);
        // A person who sees the field nowhere is offered no tag at all.
        let (mut ivy, _) = person_with_levels(
            &site,
            &rig,
            "ivy",
            &[Permission::SeeProjects, Permission::SeeIssues],
            &[(Field::Title.into(), None, FieldLevel::CanSee)],
        )
        .await?;
        let offered = ivy
            .send(
                "GET",
                &format!("{PROJECTS}/ACME/tag-suggestions?text=secret"),
                None,
            )
            .await?;
        assert!(names(ok(&offered)?, "tags").is_empty(), "{}", offered.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_a_tag_manager_with_no_access_to_a_project_does_not_list_its_tags() -> TestResult {
    each_database("sec_06_manager_list", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let secret = ok(&ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "SECRET", "name": "Secret"})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no project ID")?
            .to_owned();
        let hidden = ok(&ada
            .send(
                "POST",
                &format!("{PROJECTS}/{secret}/issues"),
                Some(site.new_issue("A secret")),
            )
            .await?)?["key"]
            .as_str()
            .ok_or("no key")?
            .to_owned();
        let open = site.make(&mut ada, "Something open").await?;
        let open = ok(&open)?["key"].as_str().ok_or("no key")?.to_owned();
        tag(&mut ada, &hidden, "takeover-plan").await?;
        let made = tag(&mut ada, &open, "open-tag").await?;
        // A tag no issue has any more is still there to be cleaned up.
        let unused = tag(&mut ada, &open, "unused-tag").await?;
        let unused = unused["tags"]
            .as_array()
            .and_then(|tags| tags.iter().find(|tag| tag["name"] == "unused-tag"))
            .and_then(|tag| tag["id"].as_str())
            .ok_or("no tag")?
            .to_owned();
        ok(&ada
            .send("DELETE", &format!("{ISSUES}/{open}/tags/{unused}"), None)
            .await?)?;
        assert_eq!(names(&made, "tags"), ["open-tag"]);
        let (mut mia, member) = person_with_levels(
            &site,
            &rig,
            "mia",
            &[
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::ManageLabels,
            ],
            &[(Field::Title.into(), None, FieldLevel::CanSee)],
        )
        .await?;
        site.org
            .forbid(&member, secret.parse().map_err(|_| "no project ID")?)
            .await?;
        let listed = mia.send("GET", TAGS, None).await?;
        let mut listed = names(ok(&listed)?, "items");
        listed.sort();
        assert_eq!(listed, ["open-tag", "unused-tag"], "{}", listed.join(","));
        Ok(())
    })
    .await
}
