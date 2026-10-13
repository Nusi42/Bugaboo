//! Tags and labels on both databases (TAG-01, TAG-02, TAG-03): a name is one tag whatever its
//! capitals, a label's name is used once in its group, combining keeps one link for an issue that
//! had both, and the suggestions only look at issues that are not in the trash.

use bb_app::access::PolicyStore as _;
use bb_app::issues::IssueStore as _;
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_app::tags::TagStore;
use bb_core::id::{
    IdSource, IssueId, IssueTypeId, LabelGroupId, LabelId, ProjectId, TagId, UserId,
};
use bb_core::issue::{Issue, IssueIcon, IssueType, Priority, Status};
use bb_core::issue_key::IssueCode;
use bb_core::own_fields::ChoiceColor;
use bb_core::project::{Project, ShortName};
use bb_core::tags::{IssueLabel, IssueTag, Label, LabelGroup, Tag};
use bb_core::version::FIRST_VERSION;
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids};

async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

fn tag(source: &mut impl IdSource, name: &str) -> Tag {
    Tag {
        id: TagId::generate(source).unwrap(),
        name: name.to_owned(),
        color: ChoiceColor::Cat1,
        created_at: at(1),
        version: FIRST_VERSION,
    }
}

fn issue(
    source: &mut impl IdSource,
    project: ProjectId,
    issue_type: IssueTypeId,
    number: i64,
) -> Issue {
    Issue {
        id: IssueId::generate(source).unwrap(),
        project,
        issue_type,
        code: IssueCode::counting(number),
        title: format!("Issue {number}"),
        description: String::new(),
        status: Status::Open,
        priority: Priority::Normal,
        reported_by: UserId::generate(source),
        created_at: at(1),
        updated_at: at(1),
        closed_at: None,
        deleted_at: None,
        version: FIRST_VERSION,
    }
}

/// A project, a type and the issues the tests put tags and labels on.
struct World {
    project: ProjectId,
    issues: Vec<Issue>,
    person: UserId,
}

async fn world(storage: &AuthStorage, source: &mut impl IdSource, count: i64) -> World {
    let project = ProjectId::generate(source).unwrap();
    let issue_type = IssueTypeId::generate(source).unwrap();
    let issues: Vec<Issue> = (1..=count)
        .map(|number| issue(source, project, issue_type, number))
        .collect();
    let (row, kind, rows) = (
        Project {
            id: project,
            parent: None,
            short_name: ShortName::new("ACME").unwrap(),
            name: "Acme".to_owned(),
            version: FIRST_VERSION,
        },
        IssueType {
            id: issue_type,
            name: "Bug".to_owned(),
            icon: IssueIcon::Bug,
            position: 0,
            retired_at: None,
            version: FIRST_VERSION,
        },
        issues.clone(),
    );
    changed(storage, source, move |tx| {
        Box::pin(async move {
            AuthStorage::insert_project(tx, &row).await?;
            AuthStorage::insert_issue_type(tx, &kind).await?;
            for issue in &rows {
                AuthStorage::insert_issue(tx, issue, &[]).await?;
            }
            Ok(())
        })
    })
    .await
    .unwrap();
    World {
        project,
        issues,
        person: UserId::generate(source),
    }
}

fn link(world: &World, at_issue: usize, tag: TagId) -> IssueTag {
    IssueTag {
        issue: world.issues[at_issue].id,
        tag,
        added_by: world.person,
        added_at: at(2),
    }
}

#[tokio::test]
async fn tag_01_a_name_is_one_tag_whatever_its_capitals() -> TestResult {
    each_database("tag_01_one_tag_by_name", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let first = tag(&mut source, "Login");
        let second = tag(&mut source, "login");
        let stored = first.clone();
        changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::insert_tag(tx, &stored).await })
        })
        .await?;
        let again = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::insert_tag(tx, &second).await })
        })
        .await;
        assert!(again.is_err(), "the same name twice is a duplicate key");
        assert_eq!(storage.tags().await?, vec![first]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn tag_01_suggestions_only_look_at_live_issues_of_the_projects_asked_for() -> TestResult {
    each_database("tag_01_suggestions", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 2).await;
        let (login, logout, other) = (
            tag(&mut source, "login"),
            tag(&mut source, "logout"),
            tag(&mut source, "100%_sure"),
        );
        let links = [
            link(&world, 0, login.id),
            link(&world, 0, other.id),
            link(&world, 1, logout.id),
        ];
        let (rows, trashed) = (
            [login.clone(), logout.clone(), other.clone()],
            world.issues[1].id,
        );
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                for row in &rows {
                    AuthStorage::insert_tag(tx, row).await?;
                }
                for link in &links {
                    AuthStorage::insert_issue_tag(tx, link).await?;
                }
                AuthStorage::set_issue_deleted(tx, trashed, Some(at(3))).await?;
                Ok(())
            })
        })
        .await?;
        let found = storage.used_tags(&[world.project], "LOG", 10).await?;
        assert_eq!(
            found,
            vec![login.clone()],
            "the tag on the trashed issue is not offered"
        );
        // The characters of a pattern are text.
        let found = storage.used_tags(&[world.project], "%_", 10).await?;
        assert_eq!(found, vec![other]);
        let nowhere = ProjectId::generate(&mut source).unwrap();
        assert!(storage.used_tags(&[nowhere], "log", 10).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn tag_03_combining_keeps_one_link_for_an_issue_that_had_both() -> TestResult {
    each_database("tag_03_combine_tags", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 3).await;
        let (old, new) = (tag(&mut source, "old"), tag(&mut source, "new"));
        let links = [
            link(&world, 0, old.id),
            link(&world, 1, old.id),
            link(&world, 1, new.id),
            link(&world, 2, new.id),
        ];
        let (rows, from, into) = ([old.clone(), new.clone()], old.id, new.id);
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                for row in &rows {
                    AuthStorage::insert_tag(tx, row).await?;
                }
                for link in &links {
                    AuthStorage::insert_issue_tag(tx, link).await?;
                }
                AuthStorage::retarget_issue_tags(tx, from, into).await?;
                AuthStorage::delete_tag(tx, from).await?;
                Ok(())
            })
        })
        .await?;
        for issue in &world.issues {
            let tags: Vec<TagId> = storage
                .issue_tags(issue.id)
                .await?
                .iter()
                .map(|found| found.tag)
                .collect();
            assert_eq!(tags, vec![new.id], "{}", issue.title);
        }
        assert_eq!(storage.tags().await?, vec![new]);
        Ok(())
    })
    .await
}

fn group(source: &mut impl IdSource, name: &str) -> LabelGroup {
    LabelGroup {
        id: LabelGroupId::generate(source).unwrap(),
        name: name.to_owned(),
        pick_one: true,
        position: 0,
        retired_at: None,
        version: FIRST_VERSION,
    }
}

fn label(source: &mut impl IdSource, group: Option<LabelGroupId>, name: &str) -> Label {
    Label {
        id: LabelId::generate(source).unwrap(),
        group,
        name: name.to_owned(),
        color: ChoiceColor::Cat2,
        position: 0,
        retired_at: None,
        version: FIRST_VERSION,
    }
}

#[tokio::test]
async fn tag_02_a_label_name_is_used_once_in_its_group() -> TestResult {
    each_database("tag_02_label_names", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let (area, kind) = (group(&mut source, "Area"), group(&mut source, "Kind"));
        let (a_login, k_login, a_login_again, loose, loose_again) = (
            label(&mut source, Some(area.id), "Login"),
            label(&mut source, Some(kind.id), "login"),
            label(&mut source, Some(area.id), "LOGIN"),
            label(&mut source, None, "Login"),
            label(&mut source, None, "login"),
        );
        let rows = [area.clone(), kind.clone()];
        let ok = [a_login.clone(), k_login.clone(), loose.clone()];
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                for row in &rows {
                    AuthStorage::insert_label_group(tx, row).await?;
                }
                for row in &ok {
                    AuthStorage::insert_label(tx, row).await?;
                }
                Ok(())
            })
        })
        .await?;
        for clash in [a_login_again, loose_again] {
            let refused = changed(storage, &mut source, move |tx| {
                Box::pin(async move { AuthStorage::insert_label(tx, &clash).await })
            })
            .await;
            assert!(refused.is_err());
        }
        assert_eq!(storage.labels().await?.len(), 3);
        assert_eq!(storage.label_groups().await?, vec![area, kind]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn tag_03_combining_labels_moves_every_link_once() -> TestResult {
    each_database("tag_03_combine_labels", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 3).await;
        let (old, new) = (
            label(&mut source, None, "old"),
            label(&mut source, None, "new"),
        );
        let on = |at_issue: usize, label: LabelId| IssueLabel {
            issue: world.issues[at_issue].id,
            label,
            added_by: world.person,
            added_at: at(2),
        };
        let links = [on(0, old.id), on(1, old.id), on(1, new.id), on(2, new.id)];
        let (rows, from, into) = ([old.clone(), new.clone()], old.id, new.id);
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                for row in &rows {
                    AuthStorage::insert_label(tx, row).await?;
                }
                for link in &links {
                    AuthStorage::insert_issue_label(tx, link).await?;
                }
                AuthStorage::retarget_issue_labels(tx, from, into).await?;
                AuthStorage::delete_label(tx, from).await?;
                Ok(())
            })
        })
        .await?;
        let on_new = storage.issues_with_label(new.id, None, 10).await?;
        let expected: Vec<IssueId> = world.issues.iter().map(|issue| issue.id).collect();
        assert_eq!(on_new, expected);
        assert_eq!(storage.labels().await?, vec![new]);
        Ok(())
    })
    .await
}
