//! The cursor of the history never names a row the reader may not see (M3-F68, SEC-06): rows that
//! hold nothing for the reader are skipped by the walk that saved searches and access keys use
//! (`fill_past_hidden`), so a short page with a cursor says "there is more" and nothing about
//! what lies between.

use std::collections::{BTreeMap, BTreeSet};

use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::fields::Field;
use bb_core::history::HistoryTab;
use bb_core::id::{EventId, IssueId};
use bb_core::issue::Priority;
use bb_core::paging::LOOKED_AT_MAX;
use bb_policy::Permission;

use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::history::HistoryStore as _;
use crate::history::{HistoryPage, HistoryQuery, history};
use crate::issues::{ChangeIssue, IssueQuery, change_issue, issue};
use crate::roles::{
    AssignRole, CreateRole, Given, SetFieldLevel, Subject, assign_role, create_role, set_level,
};

/// The reader sees the titles of issues and nothing else: the priority is hidden from them.
async fn reader_of_titles(world: &World) -> Result<String, AppError> {
    let role = create_role(
        &world.ctx(),
        CreateRole {
            session_token: token(world),
            name: "Titles".to_owned(),
            permissions: vec![Permission::SeeProjects, Permission::SeeIssues],
            two_step: false,
        },
    )
    .await?
    .into_value()
    .id;
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: token(world),
            role,
            project: None,
            issue_type: None,
            field: Field::Title.into(),
            level: Some(FieldLevel::CanSee),
        },
    )
    .await?;
    let reader = world.person("reader@example.test").await?;
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: token(world),
            subject: Subject::Person(reader.person.id),
            given: Given::Role(role),
            project: None,
            applies_to: AppliesTo::Organization,
        },
    )
    .await?;
    Ok(reader.session_token)
}

/// Changes the priority (hidden from the reader) `priorities` times, then the title (shown to
/// them).
async fn changes(
    world: &World,
    issue_id: IssueId,
    priorities: usize,
    title: &str,
) -> Result<(), AppError> {
    let admin = token(world);
    for count in 0..=priorities {
        let version = issue(
            &world.ctx(),
            IssueQuery {
                session_token: admin.clone(),
                issue: issue_id.to_string(),
            },
        )
        .await?
        .into_value()
        .issue
        .version;
        let last = count == priorities;
        let priority = if count % 2 == 0 {
            Priority::Critical
        } else {
            Priority::Low
        };
        change_issue(
            &world.ctx(),
            ChangeIssue {
                session_token: admin.clone(),
                issue: issue_id.to_string(),
                version,
                issue_type: None,
                title: last.then(|| title.to_owned()),
                status: None,
                priority: (!last).then_some(priority),
                assigned_to: None,
                fields: BTreeMap::new(),
            },
        )
        .await?;
    }
    Ok(())
}

async fn page(
    world: &World,
    who: &str,
    issue_id: IssueId,
    cursor: Option<String>,
    limit: u32,
) -> Result<HistoryPage, AppError> {
    Ok(history(
        &world.ctx(),
        HistoryQuery {
            session_token: who.to_owned(),
            issue: issue_id.to_string(),
            tab: HistoryTab::Changes,
            cursor,
            limit,
        },
    )
    .await?
    .into_value())
}

/// The events of each page from the first, with the cursor that came with it.
type Pages = Vec<(Vec<EventId>, Option<String>)>;

async fn walked(
    world: &World,
    who: &str,
    issue_id: IssueId,
    limit: u32,
) -> Result<Pages, AppError> {
    let mut pages = Vec::new();
    let mut cursor = None;
    loop {
        let found = page(world, who, issue_id, cursor, limit).await?;
        let events = found.items.iter().map(|item| item.event).collect();
        cursor = found.next_cursor.clone();
        pages.push((events, found.next_cursor));
        if cursor.is_none() || pages.len() > 40 {
            return Ok(pages);
        }
    }
}

/// Where in the outbox the row of an event is: what the key of a cursor is.
async fn position_of(world: &World, issue_id: IssueId, event: EventId) -> Result<i64, String> {
    let rows = world
        .store
        .history_page(issue_id, None, None, 0, 1000)
        .await
        .map_err(|error| format!("{error:?}"))?;
    rows.iter()
        .find(|row| row.event == event)
        .map(|row| row.position)
        .ok_or_else(|| "no such row".to_owned())
}

#[tokio::test]
async fn sec_06_a_cursor_names_the_last_row_shown_and_never_a_row_the_reader_may_not_see() -> Outcome
{
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue_id = an_issue(&world, project).await?;
    let reader = reader_of_titles(&world).await?;
    // Rows the reader may not see lie between the ones they may.
    changes(&world, issue_id, 2, "Title one").await?;
    changes(&world, issue_id, 1, "Title two").await?;
    changes(&world, issue_id, 3, "Title three").await?;
    changes(&world, issue_id, 0, "Title four").await?;
    let everything = page(&world, &token(&world), issue_id, None, 100).await?;
    assert!(
        everything.items.len() >= 10,
        "the administrator sees every row"
    );

    let pages = walked(&world, &reader, issue_id, 2).await?;
    let seen: Vec<EventId> = pages
        .iter()
        .flat_map(|(events, _)| events.clone())
        .collect();
    let unique: BTreeSet<EventId> = seen.iter().copied().collect();
    assert_eq!(seen.len(), unique.len(), "no row twice");
    assert!(
        seen.len() >= 4 && seen.len() < everything.items.len(),
        "the titles are seen and the priorities are not: {}",
        seen.len()
    );
    for (events, cursor) in &pages {
        let (Some(cursor), Some(last)) = (cursor, events.last()) else {
            continue;
        };
        assert_eq!(
            cursor,
            &position_of(&world, issue_id, *last).await?.to_string(),
            "the cursor is the place of the last row shown"
        );
    }
    let (_, last_cursor) = pages.last().ok_or("no page")?;
    assert_eq!(last_cursor, &None, "the end of the history has no cursor");
    Ok(())
}

#[tokio::test]
async fn sec_06_a_long_run_of_hidden_rows_ends_a_page_with_a_cursor_that_names_no_row() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue_id = an_issue(&world, project).await?;
    let reader = reader_of_titles(&world).await?;
    changes(&world, issue_id, 0, "A title the reader sees").await?;
    // More rows the reader may not see than one request looks at, after the one they do.
    changes(&world, issue_id, LOOKED_AT_MAX + 5, "Last title").await?;

    let first = page(&world, &reader, issue_id, None, 50).await?;
    assert_eq!(
        first.items.len(),
        1,
        "the newest title, then rows that are hidden"
    );
    assert_eq!(
        first.next_cursor.as_deref(),
        Some("+500"),
        "the cursor counts a stretch and names no row: {first:?}"
    );
    // Going on finds the rest, once each, and the history ends.
    let pages = walked(&world, &reader, issue_id, 50).await?;
    let seen: Vec<EventId> = pages
        .iter()
        .flat_map(|(events, _)| events.clone())
        .collect();
    let unique: BTreeSet<EventId> = seen.iter().copied().collect();
    assert_eq!(seen.len(), unique.len());
    assert!(seen.len() >= 2, "the titles are seen: {}", seen.len());
    assert_eq!(pages.last().map(|(_, cursor)| cursor.clone()), Some(None));
    // No cursor holds more than a key of a row that was shown or whole stretches that were
    // looked at.
    for (events, cursor) in &pages {
        let Some(cursor) = cursor else { continue };
        let (key, skip) = cursor.split_once('+').unwrap_or((cursor, "0"));
        assert_eq!(skip.parse::<usize>()? % LOOKED_AT_MAX, 0, "{cursor}");
        if !key.is_empty() {
            let last = events.last().ok_or("a key after a page without rows")?;
            assert_eq!(key, position_of(&world, issue_id, *last).await?.to_string());
        }
    }
    Ok(())
}
