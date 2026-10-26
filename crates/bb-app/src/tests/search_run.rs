//! Running a search (QRY-04, QRY-05, SEC-17): what the use case hands the store, and what it makes
//! of the answer. The store here does not search; it keeps the request, so each test looks at what
//! the rights of the person and the text became. The compiler is tested on both databases in
//! `bb-store`.

use bb_core::ErrorCode;
use bb_core::fields::Field;
use bb_core::issue::Issue;
use bb_core::workflow::starter::OPEN;

use super::offers::groups;
use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::search_fields::{bob_with_a_role, every_field};
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::issues::IssueStore as _;
use crate::search::{
    SearchCountQuery, SearchError, SearchFailure, SearchHit, SearchPosition, SearchQuery,
    SearchScope, SortValue, search, search_count,
};

fn asked(session: &str, text: &str) -> SearchQuery {
    SearchQuery {
        session_token: session.to_owned(),
        text: text.to_owned(),
        scope: SearchScope::Everywhere,
        cursor: None,
        limit: 1,
    }
}

async fn issue_of(world: &World, project: bb_core::id::ProjectId) -> Result<Issue, String> {
    let id = an_issue(world, project).await?;
    world
        .store
        .issue(id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "the issue is there".to_owned())
}

fn hit(issue: Issue) -> SearchHit {
    let id = issue.id;
    SearchHit {
        issue,
        position: SearchPosition {
            values: vec![SortValue::Whole(5), SortValue::Text("a".to_owned())],
            id,
        },
    }
}

#[tokio::test]
async fn qry_05_a_mistake_is_answered_before_the_store_is_asked() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let answer = search(&world.ctx(), asked(&token(&world), "nonsense is x")).await;
    assert!(matches!(answer, Err(SearchError::Mistakes(ref found)) if found.len() == 1));
    assert!(world.store.lock().searches.is_empty());
    Ok(())
}

#[tokio::test]
async fn qry_05_a_hidden_field_in_a_search_is_the_mistake_of_an_unknown_field() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let shown: Vec<_> = every_field()
        .into_iter()
        .filter(|field| *field != Field::Priority.into())
        .collect();
    let (bob, _) = bob_with_a_role(&world, &shown).await?;
    let mistakes = |text: &'static str| {
        let (world, bob) = (&world, bob.clone());
        async move {
            match search(&world.ctx(), asked(&bob, text)).await {
                Err(SearchError::Mistakes(found)) => found,
                other => panic!("{text}: {other:?}"),
            }
        }
    };
    let hidden = mistakes("priority is high").await;
    let unknown = mistakes("prio is high").await;
    assert_eq!(hidden[0].key, unknown[0].key);
    assert_eq!(
        hidden[0].args.keys().collect::<Vec<_>>(),
        unknown[0].args.keys().collect::<Vec<_>>()
    );
    // Sorting and grouping by it are as unknown.
    let sorted = mistakes("sort by priority").await;
    assert_eq!(sorted[0].key, unknown[0].key);
    assert!(world.store.lock().searches.is_empty());
    Ok(())
}

#[tokio::test]
async fn qry_05_a_list_looks_only_at_the_projects_its_fields_were_judged_for() -> Outcome {
    let g = groups().await?;
    let list = SearchQuery {
        scope: SearchScope::Within(g.sub),
        ..asked(&g.lead, "")
    };
    search(&g.world.ctx(), list).await.ok();
    let everywhere = asked(&g.lead, "");
    search(&g.world.ctx(), everywhere).await.ok();
    let seen = g.world.store.lock().searches.clone();
    assert_eq!(seen.len(), 2);
    let within = &seen[0].sight;
    assert!(
        within
            .all
            .iter()
            .chain(&within.own)
            .all(|each| *each == g.sub)
    );
    let wide = &seen[1].sight;
    assert!(wide.all.contains(&g.acme) && wide.all.contains(&g.sub));
    // OTHER is not among the projects of the lead of ACME at all.
    assert!(!wide.all.contains(&g.other) && !wide.own.contains(&g.other));
    assert!(seen[0].projects.iter().all(|each| each.id == g.sub));
    Ok(())
}

#[tokio::test]
async fn perf_02_a_page_has_a_cursor_that_the_next_request_continues_from() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "OPEN").await?;
    let (first, second) = (
        issue_of(&world, project).await?,
        issue_of(&world, project).await?,
    );
    world.store.lock().search_hits = vec![hit(first.clone()), hit(second.clone())];
    let admin = token(&world);
    let page = search(&world.ctx(), asked(&admin, ""))
        .await
        .map_err(|error| format!("{error:?}"))?
        .into_value();
    assert_eq!(page.items.len(), 1);
    let cursor = page.next_cursor.ok_or("a next page")?;
    let again = SearchQuery {
        cursor: Some(cursor),
        ..asked(&admin, "")
    };
    search(&world.ctx(), again)
        .await
        .map_err(|error| format!("{error:?}"))?;
    let requests = world.store.lock().searches.clone();
    assert_eq!(
        requests[0].limit, 2,
        "one more than the page, to know there is a next one"
    );
    let after = requests[1]
        .after
        .clone()
        .ok_or("the position is read back")?;
    assert_eq!(after.id, first.id);
    assert_eq!(after.values.len(), 2);
    let bad = SearchQuery {
        cursor: Some("not a cursor".to_owned()),
        ..asked(&admin, "")
    };
    assert!(matches!(
        search(&world.ctx(), bad).await,
        Err(SearchError::Failed(AppError::Refused(
            ErrorCode::InvalidRequest
        )))
    ));
    Ok(())
}

#[tokio::test]
async fn sec_17_a_search_over_its_time_answers_too_slow() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    world.store.lock().search_fails = Some(SearchFailure::TooSlow);
    let answer = search(&world.ctx(), asked(&token(&world), "")).await;
    assert!(matches!(
        answer,
        Err(SearchError::Failed(AppError::Refused(
            ErrorCode::SearchTooSlow
        )))
    ));
    Ok(())
}

#[tokio::test]
async fn qry_05_the_names_of_a_search_are_turned_into_what_they_stand_for_before_the_store()
-> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let bob = world.person("bob@example.test").await?;
    let admin = token(&world);
    search(
        &world.ctx(),
        asked(
            &admin,
            "status is \"  OPEN\" and assignedTo is \"bob  BUILDER\"",
        ),
    )
    .await
    .ok();
    let request = world
        .store
        .lock()
        .searches
        .first()
        .cloned()
        .ok_or("the store was asked")?;
    let statuses = &request.names.statuses;
    assert!(
        statuses
            .iter()
            .any(|(name, keys)| name == "open" && keys.iter().any(|key| key == OPEN))
    );
    let people = &request.names.people;
    assert!(
        people
            .iter()
            .any(|(name, ids)| name == "bob builder" && ids == &[bob.person.id])
    );
    assert_eq!(request.me, world.admin.person.id);
    Ok(())
}

#[tokio::test]
async fn qry_05_the_count_comes_from_the_same_search() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "OPEN").await?;
    world.store.lock().search_hits = vec![hit(issue_of(&world, project).await?)];
    let counted = search_count(
        &world.ctx(),
        SearchCountQuery {
            session_token: token(&world),
            text: "title contains \"x\" or priority is high".to_owned(),
            scope: SearchScope::Everywhere,
        },
    )
    .await;
    // `contains` waits for the full-text port (WP-05): the text is checked, the count is asked
    // only when the search can be compiled by the store.
    assert!(counted.is_ok() || matches!(counted, Err(SearchError::Mistakes(_))));
    let counted = search_count(
        &world.ctx(),
        SearchCountQuery {
            session_token: token(&world),
            text: "priority is high".to_owned(),
            scope: SearchScope::Everywhere,
        },
    )
    .await
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(counted.into_value().issues, 1);
    Ok(())
}

/// The reads the store answers while a page of issues from `projects` projects is made.
async fn reads_for_a_page_over(projects: usize) -> Result<usize, String> {
    let world = World::new().await?;
    let mut hits = Vec::new();
    for number in 0..projects {
        let project = a_project(&world, &format!("PR{number}")).await?;
        hits.push(hit(issue_of(&world, project).await?));
    }
    world.store.lock().search_hits = hits;
    let admin = token(&world);
    let before = world.store.reads();
    let query = SearchQuery {
        limit: 100,
        ..asked(&admin, "")
    };
    let page = search(&world.ctx(), query)
        .await
        .map_err(|error| format!("{error:?}"))?
        .into_value();
    assert_eq!(page.items.len(), projects);
    Ok(world.store.reads() - before)
}

#[tokio::test]
async fn perf_n1_a_page_over_many_projects_reads_as_much_as_one_over_few() -> Outcome {
    let few = reads_for_a_page_over(2).await?;
    let many = reads_for_a_page_over(6).await?;
    assert_eq!(few, many, "reads grow with the projects of a page");
    Ok(())
}
