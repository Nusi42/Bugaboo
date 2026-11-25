//! The two release fields in the compiler on both databases (ISS-08, ADR-0049, decision 4): a
//! release by its name as the request resolved it, `in (…)`, `is empty`, the order by the
//! *Release date* and the groups by release.

use bb_app::issues::IssueStore as _;
use bb_app::ports::ChangeStore;
use bb_app::releases::ReleaseStore as _;
use bb_app::search::{FieldTarget, GroupKey, SearchRequest, Searchable};
use bb_core::id::{ReleaseId, UserId};
use bb_core::names::name_key;
use bb_core::project_release::{Release, ReleaseState, found_in_rows};
use bb_core::version::{FIRST_VERSION, Version};
use bb_searchlang::Field;
use bb_testing::{Failure, TestResult, each_database};

use super::tests::{Data, request_with, seed};
use super::tests_more::titles_of;
use crate::AuthStorage;
use crate::test_support::{at, event, fixture};

/// The two fields as a person who may read them has them, with the names of the releases.
fn release_fields(names: &[&str]) -> Vec<Searchable> {
    let choices: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
    vec![
        Searchable {
            field: Field {
                choices: choices.clone(),
                ..Field::unordered("found in release")
            },
            target: FieldTarget::FoundInRelease,
        },
        Searchable {
            field: Field {
                choices,
                ..Field::unordered("planned for release")
            },
            target: FieldTarget::PlannedForRelease,
        },
    ]
}

fn release(
    data: &mut Data,
    name: &str,
    day: Option<&str>,
    person: UserId,
) -> Result<Release, Failure> {
    Ok(Release {
        id: ReleaseId::generate(&mut data.source)?,
        project: data.project,
        name: name.to_owned(),
        release_date: day.map(str::parse).transpose()?,
        state: ReleaseState::Planned,
        released_on: None,
        made_by: person,
        made_at: at(0),
        changed_by: person,
        changed_at: at(0),
        deleted_at: None,
        version: FIRST_VERSION,
    })
}

/// Three releases of ACME: *2.4* on December 1, *2.5* on November 1, *3.0* with no date.
/// Printer jams is planned for 2.4 and was found in 2.4 and 2.5, Login broken is planned for 2.5
/// and found in 2.5, Docs typo is planned for 3.0. Slow page names *Secret* in both fields, a
/// release the person may not know (of a project they do not see: not among the releases the
/// request knows); the others name none.
async fn with_releases(storage: &AuthStorage, data: &mut Data) -> Result<Vec<Release>, Failure> {
    let made = vec![
        release(data, "2.4", Some("2026-12-01"), data.me)?,
        release(data, "2.5", Some("2026-11-01"), data.me)?,
        release(data, "3.0", None, data.me)?,
    ];
    let secret = release(data, "Secret", Some("2026-10-01"), data.me)?;
    let mut planned = data.issues.clone();
    planned[0].planned_for_release = Some(made[0].id);
    planned[1].planned_for_release = Some(made[1].id);
    planned[2].planned_for_release = Some(made[2].id);
    planned[3].planned_for_release = Some(secret.id);
    let found = vec![
        found_in_rows(data.issues[0].id, &[made[0].id, made[1].id]),
        found_in_rows(data.issues[1].id, &[made[1].id]),
        found_in_rows(data.issues[3].id, &[secret.id]),
    ];
    let mut releases = made.clone();
    releases.push(secret);
    ChangeStore::record_change(storage, &event(&mut data.source, 7), move |tx| {
        Box::pin(async move {
            for each in &releases {
                AuthStorage::insert_release(tx, each).await?;
            }
            for issue in &planned[..4] {
                AuthStorage::update_issue(tx, issue, Version::new(issue.version)).await?;
            }
            for rows in &found {
                if let Some(first) = rows.first() {
                    AuthStorage::replace_found_in(tx, first.issue, rows).await?;
                }
            }
            Ok(())
        })
    })
    .await?;
    Ok(made)
}

/// The request for the text, with the names of the releases resolved as the use case does.
fn asked(data: &Data, made: &[Release], text: &str) -> SearchRequest {
    let names: Vec<&str> = made.iter().map(|each| each.name.as_str()).collect();
    let mut request = request_with(data, text, None, 100, release_fields(&names), Vec::new());
    request.names.releases = made
        .iter()
        .map(|each| (name_key(&each.name), vec![each.id]))
        .collect();
    request.releases_known = made.iter().map(|each| each.id).collect();
    request
}

async fn found(
    storage: &AuthStorage,
    data: &Data,
    made: &[Release],
    text: &str,
) -> Result<Vec<String>, Failure> {
    Ok(titles_of(storage, &asked(data, made, text)).await?)
}

#[tokio::test]
async fn iss_08_search_filters_sorts_and_groups_by_both_release_fields() -> TestResult {
    each_database("search_releases", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut data = seed(storage).await;
        let made = with_releases(storage, &mut data).await?;
        let some = |text: &'static str| found(storage, &data, &made, text);
        assert_eq!(
            some("planned for release is \"2.4\"").await?,
            ["Printer jams"]
        );
        assert_eq!(
            some("found in release is \"2.5\" sort by title").await?,
            ["Login broken", "Printer jams"]
        );
        assert_eq!(
            some("planned for release in (\"2.5\", \"3.0\") sort by title").await?,
            ["Docs typo", "Login broken"]
        );
        assert_eq!(
            some("found in release is empty sort by title").await?,
            ["Crash on save", "Docs typo", "Idea", "Slow page"]
        );
        assert_eq!(
            some("planned for release is not \"2.4\" and planned for release is not empty sort by title")
                .await?,
            ["Docs typo", "Login broken"]
        );
        // By the *Release date*: November before December, the release with no date and the
        // issues with none last, in either direction.
        let by_date = some(
            "planned for release is not empty sort by planned for release oldest first",
        )
        .await?;
        assert_eq!(by_date, ["Login broken", "Printer jams", "Docs typo"]);
        let newest = some(
            "planned for release is not empty sort by planned for release newest first",
        )
        .await?;
        assert_eq!(newest, ["Printer jams", "Login broken", "Docs typo"]);
        // An issue found in several releases goes by the earliest of them.
        let found_order = some(
            "found in release is not empty sort by found in release oldest first, title",
        )
        .await?;
        assert_eq!(found_order, ["Login broken", "Printer jams"]);
        // The groups are the releases by their IDs, and the issues with none.
        let groups = storage
            .search_groups(&asked(&data, &made, ""), "found in release")
            .await
            .map_err(|error| format!("{error:?}"))?;
        let count = |key: GroupKey| {
            groups
                .iter()
                .find(|group| group.key == key)
                .map(|group| group.issues)
        };
        assert_eq!(count(GroupKey::Release(made[0].id)), Some(1));
        assert_eq!(count(GroupKey::Release(made[1].id)), Some(2));
        assert_eq!(count(GroupKey::Empty), Some(4));
        // A release the person may not know is no value: Slow page has none for them, in the
        // filter, the order and the groups, and no group is keyed by that release.
        assert!(
            some("planned for release is empty sort by title")
                .await?
                .contains(&"Slow page".to_owned())
        );
        assert!(groups.iter().all(|group| match group.key {
            GroupKey::Release(id) => made.iter().any(|each| each.id == id),
            _ => true,
        }));
        let planned = storage
            .search_groups(&asked(&data, &made, ""), "planned for release")
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(planned.len(), 4, "{planned:?}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_08_the_store_keeps_a_release_and_tells_only_that_an_issue_names_it() -> TestResult {
    each_database("releases_store", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut data = seed(storage).await;
        let made = with_releases(storage, &mut data).await?;
        let read = storage.release(made[0].id).await?.ok_or("no release")?;
        assert_eq!(read, made[0]);
        let of_acme = storage.releases_of_projects(vec![data.project]).await?;
        assert_eq!(of_acme.len(), 4, "{of_acme:?}");
        assert!(storage.release_named(made[0].id).await?);
        assert!(storage.release_named(made[2].id).await?);
        let me = data.me;
        let unused = release(&mut data, "4.0", None, me)?;
        let stored = unused.clone();
        ChangeStore::record_change(storage, &event(&mut data.source, 8), move |tx| {
            Box::pin(async move { AuthStorage::insert_release(tx, &stored).await })
        })
        .await?;
        assert!(!storage.release_named(unused.id).await?);
        let rows = storage.found_in_of_issues(vec![data.issues[0].id]).await?;
        assert_eq!(
            rows.iter().map(|row| row.release).collect::<Vec<_>>(),
            [made[0].id, made[1].id]
        );
        // The trash: a release there is no release of its project, and comes back by its ID.
        let mut trashed = unused.clone();
        trashed.deleted_at = Some(at(9));
        ChangeStore::record_change(storage, &event(&mut data.source, 9), move |tx| {
            Box::pin(async move {
                assert!(
                    AuthStorage::update_release(tx, &trashed, Version::new(FIRST_VERSION)).await?
                );
                Ok(())
            })
        })
        .await?;
        let listed = storage.releases_of_projects(vec![data.project]).await?;
        assert!(listed.iter().all(|each| each.id != unused.id));
        let again = storage.release(unused.id).await?.ok_or("gone")?;
        assert_eq!(again.version, FIRST_VERSION + 1);
        assert!(again.deleted_at.is_some());
        Ok(())
    })
    .await
}
