//! Workflows on both databases (WF-01, ADR-0031): a workflow comes back as it was stored, a save is
//! made on the version that was looked at, the uses are found by their place, and the issues in the
//! statuses a workflow drops are found.

use bb_app::issues::IssueStore as _;
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_app::workflows::WorkflowStore;
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{IdSource, WorkflowId};
use bb_core::version::FIRST_VERSION;
use bb_core::workflow::starter::{BUG, SIMPLE};
use bb_core::workflow::{Condition, Test, Workflow, WorkflowPlace, WorkflowUse, find};
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids, world};

/// Runs `write` as a change with its event, the way a use case does.
async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

fn header(source: &mut impl IdSource, name: &str) -> Workflow {
    Workflow {
        id: WorkflowId::generate(source).unwrap(),
        name: name.to_owned(),
        version: FIRST_VERSION,
    }
}

#[tokio::test]
async fn wf_01_a_workflow_comes_back_as_stored_and_a_save_needs_the_version_it_started_from()
-> TestResult {
    each_database("wf_01_store", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let mut content = BUG.content();
        content.changes[0].rules.who.reporter = true;
        content.changes[0].rules.only_if = vec![Condition {
            field: FieldKey::BuiltIn(Field::AssignedTo),
            test: Test::Filled,
        }];
        content.changes[1].rules.ask_for = vec![FieldKey::BuiltIn(Field::Description)];
        let workflow = header(&mut source, "Bug");
        let (made, first) = (workflow.clone(), content.clone());
        changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::insert_workflow(tx, &made, &first).await })
        })
        .await?;
        let found = storage.stored_workflow(workflow.id).await?.unwrap();
        assert_eq!(found.workflow, workflow);
        assert_eq!(
            found.content, content,
            "order, names, groups and rules come back"
        );
        assert_eq!(storage.workflow_headers().await?, vec![workflow.clone()]);

        // A save on the version it started from replaces the name and the content and counts the
        // version up.
        let second = SIMPLE.content();
        let (id, saved) = (workflow.id, second.clone());
        let done = changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::replace_workflow(tx, id, "Renamed", &saved, FIRST_VERSION).await
            })
        })
        .await?;
        assert!(done);
        let found = storage.stored_workflow(id).await?.unwrap();
        assert_eq!(found.workflow.version, FIRST_VERSION + 1);
        assert_eq!(found.workflow.name, "Renamed");
        assert_eq!(found.content, second);

        // A save that started from the old version writes nothing.
        let other = content.clone();
        let done = changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::replace_workflow(tx, id, "Stale", &other, FIRST_VERSION).await
            })
        })
        .await?;
        assert!(!done);
        let found = storage.stored_workflow(id).await?.unwrap();
        assert_eq!(found.workflow.name, "Renamed");
        assert_eq!(found.content, second);

        // Taking it away needs the current version too, and leaves nothing behind.
        let wrong = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_workflow(tx, id, FIRST_VERSION).await })
        })
        .await?;
        assert!(!wrong);
        let right = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_workflow(tx, id, FIRST_VERSION + 1).await })
        })
        .await?;
        assert!(right);
        assert!(storage.stored_workflow(id).await?.is_none());
        assert!(storage.workflow_headers().await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn wf_01_a_project_finds_its_workflow_like_a_layout() -> TestResult {
    each_database("wf_01_store_uses", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let project = world(storage, &mut source, 0).await.project;
        let bug = storage.issue_types().await?[0].id;
        let (simple, own) = (header(&mut source, "Simple"), header(&mut source, "Own"));
        let (a, b) = (simple.clone(), own.clone());
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_workflow(tx, &a, &SIMPLE.content()).await?;
                AuthStorage::insert_workflow(tx, &b, &BUG.content()).await?;
                // The organization uses Simple for all types; the project uses its own for bugs.
                let whole = WorkflowUse {
                    project: None,
                    issue_type: None,
                    workflow: a.id,
                };
                AuthStorage::put_workflow_use(tx, &whole).await?;
                let special = WorkflowUse {
                    project: Some(project),
                    issue_type: Some(bug),
                    workflow: b.id,
                };
                AuthStorage::put_workflow_use(tx, &special).await
            })
        })
        .await?;
        let found = |uses: &[WorkflowUse], issue_type| {
            find(&[project], issue_type, uses).map(|each| each.workflow)
        };
        let uses = storage.workflow_uses().await?;
        assert_eq!(uses.len(), 2);
        assert_eq!(found(&uses, Some(bug)), Some(own.id));
        assert_eq!(found(&uses, None), Some(simple.id));
        // The directory brings the same uses and the workflows they name, with their content.
        let rows = storage.workflow_directory().await?;
        assert_eq!(rows.uses.len(), 2);
        let mut named: Vec<(String, usize)> = rows
            .workflows
            .iter()
            .map(|each| (each.workflow.name.clone(), each.content.statuses.len()))
            .collect();
        named.sort();
        assert_eq!(
            named,
            [
                ("Own".to_owned(), BUG.content().statuses.len()),
                ("Simple".to_owned(), SIMPLE.content().statuses.len())
            ]
        );

        // Saying the place uses another workflow replaces the use of that place only.
        let place = WorkflowPlace {
            project: Some(project),
            issue_type: Some(bug),
        };
        let a = simple.clone();
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                let again = WorkflowUse {
                    project: place.project,
                    issue_type: place.issue_type,
                    workflow: a.id,
                };
                AuthStorage::put_workflow_use(tx, &again).await
            })
        })
        .await?;
        let uses = storage.workflow_uses().await?;
        assert_eq!(uses.len(), 2);
        assert_eq!(found(&uses, Some(bug)), Some(simple.id));

        // Taking the use away makes the place follow its parent; taking a workflow away takes its
        // uses with it.
        changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::remove_workflow_use(tx, place).await })
        })
        .await?;
        assert_eq!(storage.workflow_uses().await?.len(), 1);
        let (id, version) = (simple.id, simple.version);
        changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_workflow(tx, id, version).await })
        })
        .await?;
        assert!(storage.workflow_uses().await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn wf_02_the_live_issues_of_some_projects_and_types_are_found() -> TestResult {
    each_database("wf_02_store_live_issues", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 4).await;
        let all = storage.live_issues(None, None).await?;
        assert_eq!(all.len(), 4);
        // The issues of a project, and of an issue type, and of neither.
        let project = world.project;
        let in_project = storage.live_issues(Some(vec![project]), None).await?;
        assert_eq!(in_project.len(), 4);
        assert!(
            storage
                .live_issues(Some(Vec::new()), None)
                .await?
                .is_empty()
        );
        let kind = world.issues[0].issue_type;
        let of_type = storage.live_issues(None, Some(vec![kind])).await?;
        assert!(of_type.iter().all(|issue| issue.issue_type == kind));
        assert!(
            storage
                .live_issues(None, Some(Vec::new()))
                .await?
                .is_empty()
        );

        // An issue in the trash is no part of it.
        let trashed = world.issues[0].id;
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                assert!(AuthStorage::set_issue_deleted(tx, trashed, Some(at(9)), at(9)).await?);
                Ok(())
            })
        })
        .await?;
        assert_eq!(storage.live_issues(None, None).await?.len(), 3);
        Ok(())
    })
    .await
}
