//! Tags and labels on both databases (TAG-01, TAG-02, TAG-03): a name is one tag whatever its
//! capitals, a label's name is used once in its group, combining keeps one link for an issue that
//! had both, and the suggestions only look at issues that are not in the trash.

use bb_app::issues::IssueStore as _;
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_app::tags::TagStore;
use bb_core::id::{IdSource, IssueId, LabelGroupId, LabelId, ProjectId, TagId};
use bb_core::own_fields::ChoiceColor;
use bb_core::tags::{IssueLabel, IssueTag, Label, LabelGroup, Tag};
use bb_core::version::FIRST_VERSION;
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{World, at, event, fixture, ids, world};

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
                AuthStorage::set_issue_deleted(tx, trashed, Some(at(3)), at(3)).await?;
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

#[tokio::test]
async fn adr_0017_more_issues_than_a_statement_can_bind_are_read_in_chunks() -> TestResult {
    use crate::db::STATEMENT_ITEMS;

    each_database("tag_01_many_issues", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 2).await;
        let login = tag(&mut source, "login");
        let links = [link(&world, 0, login.id), link(&world, 1, login.id)];
        let row = login.clone();
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_tag(tx, &row).await?;
                for link in &links {
                    AuthStorage::insert_issue_tag(tx, link).await?;
                }
                Ok(())
            })
        })
        .await?;
        // Two real issues among more IDs than one statement binds, in no order.
        let mut asked: Vec<IssueId> = (0..2 * STATEMENT_ITEMS + 7)
            .map(|_| IssueId::generate(&mut source).unwrap())
            .collect();
        asked.insert(STATEMENT_ITEMS + 3, world.issues[1].id);
        asked.push(world.issues[0].id);
        let found = storage.tags_of_issues(asked.clone()).await?;
        let mut expected = vec![world.issues[0].id, world.issues[1].id];
        expected.sort_by_key(|id| id.to_bytes());
        let mut got: Vec<IssueId> = found.iter().map(|each| each.issue).collect();
        got.sort_by_key(|id| id.to_bytes());
        assert_eq!(got, expected);
        assert!(storage.labels_of_issues(asked).await?.is_empty());
        Ok(())
    })
    .await
}
