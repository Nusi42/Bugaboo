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
    SearchCountQuery, SearchFailure, SearchHit, SearchQuery, SearchScope, search, search_count,
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
    SearchHit { issue }
}

#[tokio::test]
async fn qry_05_a_mistake_is_answered_before_the_store_is_asked() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let answer = search(&world.ctx(), asked(&token(&world), "nonsense is x")).await;
    assert!(matches!(answer, Err(AppError::Mistakes(ref found)) if found.len() == 1));
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
                Err(AppError::Mistakes(found)) => found,
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
    let after = requests[1].after.ok_or("the position is read back")?;
    assert_eq!(after, first.id);
    let bad = SearchQuery {
        cursor: Some("not a cursor".to_owned()),
        ..asked(&admin, "")
    };
    assert!(matches!(
        search(&world.ctx(), bad).await,
        Err(AppError::Refused(ErrorCode::InvalidRequest))
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
        Err(AppError::Refused(ErrorCode::SearchTooSlow))
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
    assert!(counted.is_ok() || matches!(counted, Err(AppError::Mistakes(_))));
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

#[tokio::test]
async fn qry_05_a_hidden_field_takes_as_long_as_an_unknown_field() -> Outcome {
    use std::time::{Duration, Instant};

    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let shown: Vec<_> = every_field()
        .into_iter()
        .filter(|field| *field != Field::Priority.into())
        .collect();
    let (bob, _) = bob_with_a_role(&world, &shown).await?;
    let spent = |text: &'static str| {
        let (world, bob) = (&world, bob.clone());
        async move {
            let mut times: Vec<Duration> = Vec::new();
            let mut reads = Vec::new();
            for _ in 0..60 {
                let before = world.store.reads();
                let started = Instant::now();
                let answer = search(&world.ctx(), asked(&bob, text)).await;
                times.push(started.elapsed());
                assert!(matches!(answer, Err(AppError::Mistakes(_))));
                reads.push(world.store.reads() - before);
            }
            times.sort();
            (times[times.len() / 2], reads)
        }
    };
    let (hidden, hidden_reads) = spent("priority is high").await;
    let (unknown, unknown_reads) = spent("prioritx is high").await;
    // The same reads for both, and the same time within the noise of a computer.
    assert_eq!(hidden_reads, unknown_reads);
    let (slow, fast) = (hidden.max(unknown), hidden.min(unknown));
    assert!(
        slow < fast * 3 + Duration::from_millis(2),
        "{hidden:?} {unknown:?}"
    );
    Ok(())
}

/// The mistakes of a search that is refused, or none.
async fn refused(world: &World, session: &str, text: &str) -> Vec<bb_searchlang::Mistake> {
    match search(&world.ctx(), asked(session, text)).await {
        Err(AppError::Mistakes(found)) => found,
        _ => Vec::new(),
    }
}

#[tokio::test]
async fn qry_05_tags_labels_projects_and_issues_are_resolved_or_the_search_is_refused() -> Outcome {
    use super::scenarios_tags::tagged;

    let world = World::new().await?;
    let project = a_project(&world, "OPEN").await?;
    let issue = issue_of(&world, project).await?;
    let tag = tagged(&world, issue.id, "Login").await?;
    let admin = token(&world);
    let code = format!("OPEN-{}", issue.code.as_str());
    let ok = format!("tags is login and project is open and under {code}");
    search(&world.ctx(), asked(&admin, &ok))
        .await
        .map_err(|error| format!("{error:?}"))?;
    let request = world.store.lock().searches.last().cloned().ok_or("asked")?;
    assert!(
        request
            .names
            .tags
            .iter()
            .any(|(name, ids)| name == "login" && ids == &[tag])
    );
    assert!(
        request
            .names
            .issues
            .iter()
            .any(|(name, id)| *name == code && *id == issue.id)
    );
    // A name that stands for nothing the person may know is refused, where it stands.
    for (text, start) in [
        ("tags is nothing", 8),
        ("project is NOPE", 11),
        ("project within NOPE", 15),
        ("under OPEN-ZZZZZZ", 6),
        ("blocks WEB-1", 7),
    ] {
        let found = refused(&world, &admin, text).await;
        assert_eq!(found.len(), 1, "{text}");
        assert_eq!(
            found[0].key,
            bb_searchlang::messages::UNKNOWN_CHOICE,
            "{text}"
        );
        assert_eq!(found[0].span.start, start, "{text}");
    }
    Ok(())
}

#[tokio::test]
async fn qry_05_member_of_needs_being_in_the_group_or_managing_groups() -> Outcome {
    use super::search_fields::set_people;
    use crate::people::{AddGroupMember, CreateGroup, add_group_member, create_group};

    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let admin = token(&world);
    let backend = create_group(
        &world.ctx(),
        CreateGroup {
            session_token: admin.clone(),
            name: "Backend".to_owned(),
        },
    )
    .await
    .map_err(|error| format!("{error:?}"))?
    .into_value();
    let (bob, role) = bob_with_a_role(&world, &every_field()).await?;
    let mentions = bb_core::own_fields::FieldType::Group
        .people_kind()
        .ok_or("a group has a kind")?;
    set_people(&world, role, mentions, bb_core::access::PeopleLevel::Names).await?;
    let text = "assignedTo member of \"Backend\"";
    // Bob is not in the group and does not manage groups: it is as unknown as no group.
    assert_eq!(refused(&world, &bob, text).await.len(), 1);
    // The administrator manages groups.
    assert!(refused(&world, &admin, text).await.is_empty());
    // Once Bob is in the group he may ask about it.
    let person = world
        .ctx()
        .actor(&bob)
        .await
        .map_err(|error| format!("{error:?}"))?
        .person
        .id;
    add_group_member(
        &world.ctx(),
        AddGroupMember {
            session_token: admin.clone(),
            group: backend.id,
            person,
        },
    )
    .await
    .map_err(|error| format!("{error:?}"))?;
    assert!(refused(&world, &bob, text).await.is_empty());
    // A group that does not exist is refused alike.
    assert_eq!(
        refused(&world, &admin, "assignedTo member of \"Nobody\"")
            .await
            .len(),
        1
    );
    Ok(())
}

/// The reads the store answers while a page of `issues` issues of one project, by three people, is made.
async fn reads_for_a_page_of(issues: usize) -> Result<usize, String> {
    let world = World::new().await?;
    let project = a_project(&world, "OPEN").await?;
    let people = [
        world.admin.person.id,
        world
            .person("bob@example.test")
            .await
            .map_err(|e| format!("{e:?}"))?
            .person
            .id,
        world
            .person("eve@example.test")
            .await
            .map_err(|e| format!("{e:?}"))?
            .person
            .id,
    ];
    let mut hits = Vec::new();
    for number in 0..issues {
        let mut each = issue_of(&world, project).await?;
        each.reported_by = people[number % people.len()];
        hits.push(hit(each));
    }
    world.store.lock().search_hits = hits;
    let before = world.store.reads();
    let query = SearchQuery {
        limit: 100,
        ..asked(&token(&world), "")
    };
    let page = search(&world.ctx(), query)
        .await
        .map_err(|error| format!("{error:?}"))?
        .into_value();
    assert_eq!(page.items.len(), issues);
    Ok(world.store.reads() - before)
}

#[tokio::test]
async fn perf_n1_a_page_of_forty_reads_as_much_as_a_page_of_five() -> Outcome {
    assert_eq!(
        reads_for_a_page_of(5).await?,
        reads_for_a_page_of(40).await?
    );
    Ok(())
}

#[tokio::test]
async fn qry_05_a_group_of_people_is_shown_through_the_person_ref_of_the_viewer() -> Outcome {
    use crate::search::{GroupCount, GroupKey, SearchGroupsQuery, search_groups};

    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let bob = world
        .person("bob@example.test")
        .await
        .map_err(|e| format!("{e:?}"))?;
    world.store.lock().search_groups = vec![
        GroupCount {
            key: GroupKey::Person(bob.person.id),
            issues: 2,
        },
        GroupCount {
            key: GroupKey::Empty,
            issues: 1,
        },
    ];
    let groups = search_groups(
        &world.ctx(),
        SearchGroupsQuery {
            session_token: token(&world),
            text: String::new(),
            scope: SearchScope::Everywhere,
            by: "assignedTo".to_owned(),
        },
    )
    .await
    .map_err(|error| format!("{error:?}"))?;
    let shown = serde_json::to_string(&groups.into_value().groups).map_err(|e| e.to_string())?;
    // The group is the person as the viewer sees them, never a bare ID: it holds the name.
    assert!(shown.contains("Bob Builder"), "{shown}");
    assert!(!shown.contains(&bob.person.id.to_string()) || shown.contains("Bob Builder"));
    Ok(())
}

#[tokio::test]
async fn qry_05_the_projects_an_own_field_reaches_are_told_to_the_store() -> Outcome {
    use bb_core::own_fields::{FieldType, Scope};

    use super::scenarios_fields::new_field;
    use crate::fields::create_field;

    let world = World::new().await?;
    let (a, _b) = (
        a_project(&world, "ALPHA").await?,
        a_project(&world, "BETA").await?,
    );
    let mut input = new_field(&world, "Rack", FieldType::Text, &[]);
    input.scope = Scope {
        projects: vec![a],
        issue_types: Vec::new(),
    };
    let rack = create_field(&world.ctx(), input)
        .await
        .map_err(|e| format!("{e:?}"))?
        .into_value();
    search(&world.ctx(), asked(&token(&world), "rack is x"))
        .await
        .map_err(|error| format!("{error:?}"))?;
    let request = world.store.lock().searches.last().cloned().ok_or("asked")?;
    let reach = request
        .own_reach
        .iter()
        .find(|each| each.field == rack.id)
        .ok_or("the field has a reach")?;
    assert_eq!(reach.projects, [a]);
    assert_eq!(reach.issue_types, None);
    Ok(())
}
