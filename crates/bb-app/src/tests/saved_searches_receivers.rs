//! What a receiver sees of a saved search (QRY-06, ADR-0034, decision 4): a search that does not
//! check cleanly for them is not listed and its address is not found; a text is kept in its
//! canonical spelling; a person has at most as many searches as they may have.

use bb_core::ErrorCode;
use bb_core::id::SavedSearchId;
use bb_core::saved_search::{SearchSet, ShareTarget};
use bb_policy::StartingRole;

use super::saved_searches::{Crew, refused, shared};
use super::scenarios::a_project;
use super::scenarios_searches::{a_group_of, a_presentation, open, save, share, with_role};
use super::world::Outcome;
use crate::auth::{AppError, Started};
use crate::saved_searches::SavedSearchStore as _;
use crate::saved_searches::{ChangeSearch, SaveCopy, change_search, save_copy};

/// Ada saves a search with this text and shares it with ACME.
async fn shared_with_acme(
    crew: &Crew,
    name: &str,
    search_text: &str,
) -> Result<crate::saved_searches::SavedSearchView, AppError> {
    let ada = crew.ada();
    let made = save(&crew.world, &ada, name, search_text).await?;
    share(&crew.world, &ada, made.id, ShareTarget::Project(crew.acme)).await
}

/// Whether a person lists the search and can open it.
async fn is_there_for(crew: &Crew, who: &Started, id: SavedSearchId) -> Result<bool, AppError> {
    let listed = shared(crew, who).await?.iter().any(|view| view.id == id);
    let opened = open(&crew.world, &who.session_token, id).await.is_ok();
    assert_eq!(listed, opened, "the list and the address disagree");
    Ok(opened)
}

#[tokio::test]
async fn qry_06_a_shared_search_with_a_field_the_receiver_cannot_read_is_not_listed_and_its_address_is_not_found()
-> Outcome {
    let crew = Crew::new().await?;
    let made = shared_with_acme(&crew, "High", "priority is high").await?;
    assert!(is_there_for(&crew, &crew.dan, made.id).await?);
    // The role of Dan loses the field: the search leaves his list without a message.
    let guest = super::workflows_rules::starting_role(&crew.world, StartingRole::Guest).await?;
    crate::roles::set_level(
        &crew.world.ctx(),
        crate::roles::SetFieldLevel {
            session_token: crew.ada(),
            role: guest,
            project: None,
            issue_type: None,
            field: bb_core::fields::Field::Priority.into(),
            level: Some(bb_core::access::FieldLevel::Hidden),
        },
    )
    .await?;
    assert!(!is_there_for(&crew, &crew.dan, made.id).await?);
    // Every write answers as for a missing one, not as for a search that is there.
    let ids = [
        made.id,
        SavedSearchId::from_parts(jiff::Timestamp::UNIX_EPOCH, [8; 10])?,
    ];
    let dan = crew.dan.session_token.clone();
    let mut deletions = Vec::new();
    for id in ids {
        deletions.push(
            crate::saved_searches::delete_search(
                &crew.world.ctx(),
                crate::saved_searches::DeleteSearch {
                    session_token: dan.clone(),
                    search: id,
                },
            )
            .await,
        );
    }
    assert!(refused(&deletions[0], ErrorCode::NotFound));
    assert_eq!(deletions[0], deletions[1]);
    let hidden = open(&crew.world, &crew.dan.session_token, made.id).await;
    let missing = open(
        &crew.world,
        &crew.dan.session_token,
        SavedSearchId::from_parts(jiff::Timestamp::UNIX_EPOCH, [8; 10])?,
    )
    .await;
    assert_eq!(hidden, missing);
    // The others still have it.
    assert!(is_there_for(&crew, &crew.ben, made.id).await?);
    // A grouping on a field the receiver cannot read is the same.
    let mut shown = a_presentation();
    shown.group_by = Some("priority".to_owned());
    let grouped = crate::saved_searches::save_search(
        &crew.world.ctx(),
        crate::saved_searches::SaveSearch {
            session_token: crew.ada(),
            name: "Grouped".to_owned(),
            text: String::new(),
            presentation: shown,
        },
    )
    .await?
    .into_value();
    share(
        &crew.world,
        &crew.ada(),
        grouped.id,
        ShareTarget::Project(crew.acme),
    )
    .await?;
    assert!(!is_there_for(&crew, &crew.dan, grouped.id).await?);
    assert!(is_there_for(&crew, &crew.ben, grouped.id).await?);
    Ok(())
}

#[tokio::test]
async fn qry_06_a_shared_search_with_a_hidden_person_group_or_project_in_its_text_is_not_found()
-> Outcome {
    let crew = Crew::new().await?;
    a_group_of(&crew.world, "Night shift", &[&crew.cleo]).await?;
    a_project(&crew.world, "SECRET").await?;
    // A person: Dan sees people by their role only, so he cannot search by one.
    let person = shared_with_acme(&crew, "By person", "assignedTo is \"Ada Admin\"").await?;
    assert!(is_there_for(&crew, &crew.ben, person.id).await?);
    assert!(!is_there_for(&crew, &crew.dan, person.id).await?);
    // A group: Cleo is in it, Ben is not.
    let group = shared_with_acme(&crew, "By group", "assignedTo member of \"Night shift\"").await?;
    assert!(is_there_for(&crew, &crew.cleo, group.id).await?);
    assert!(!is_there_for(&crew, &crew.ben, group.id).await?);
    // A project that only Ada sees, and one that Ben sees.
    let project = shared_with_acme(&crew, "By project", "project within SECRET").await?;
    assert!(!is_there_for(&crew, &crew.ben, project.id).await?);
    let within = shared_with_acme(&crew, "By seen project", "project within ACME").await?;
    assert!(is_there_for(&crew, &crew.ben, within.id).await?);
    let issue = shared_with_acme(&crew, "Under", "under SECRET-1").await?;
    assert!(!is_there_for(&crew, &crew.ben, issue.id).await?);
    Ok(())
}

#[tokio::test]
async fn qry_06_a_text_is_kept_in_its_canonical_spelling_and_a_bad_one_is_refused() -> Outcome {
    let crew = Crew::new().await?;
    let ben = &crew.ben.session_token;
    let text = "  priority   is   high  and status is open ";
    let made = save(&crew.world, ben, "Spacing", text).await?;
    assert_eq!(made.text, "priority is high and status is open");
    for bad in [
        "nonsense is true",
        "priority is",
        "assignedTo member of \"No such group\"",
    ] {
        let refused_text = save(&crew.world, ben, "Bad", bad).await;
        assert!(refused(&refused_text, ErrorCode::InvalidRequest), "{bad}");
    }
    // A field the maker cannot read is refused for them too.
    let own = save(
        &crew.world,
        &crew.dan.session_token,
        "Own",
        "assignedTo is \"Ada Admin\"",
    )
    .await;
    assert!(refused(&own, ErrorCode::InvalidRequest));
    // And a change that breaks the text is refused.
    let broken = change_search(
        &crew.world.ctx(),
        ChangeSearch {
            session_token: ben.clone(),
            search: made.id,
            version: made.version,
            name: None,
            text: Some("nonsense".to_owned()),
            presentation: None,
        },
    )
    .await;
    assert!(refused(&broken, ErrorCode::InvalidRequest));
    Ok(())
}

#[tokio::test]
async fn qry_06_a_short_page_never_shows_that_searches_were_left_out() -> Outcome {
    let crew = Crew::new().await?;
    shared_with_acme(&crew, "Hidden one", "assignedTo is \"Ada Admin\"").await?;
    shared_with_acme(&crew, "Hidden two", "assignedTo is \"Ada Admin\"").await?;
    let third = shared_with_acme(&crew, "Seen three", "status is open").await?;
    let fourth = shared_with_acme(&crew, "Seen four", "status is open").await?;
    let page = |cursor: Option<String>| {
        let crew = &crew;
        async move {
            crate::saved_searches::saved_searches(
                &crew.world.ctx(),
                crate::saved_searches::SearchesQuery {
                    session_token: crew.dan.session_token.clone(),
                    scope: SearchSet::Shared,
                    cursor,
                    limit: 1,
                },
            )
            .await
            .map(bb_policy::Authorized::into_value)
        }
    };
    let one = page(None).await?;
    assert_eq!(one.items.len(), 1);
    assert_eq!(one.items[0].id, third.id);
    let two = page(one.next_cursor).await?;
    assert_eq!(two.items[0].id, fourth.id);
    assert_eq!(two.next_cursor, None);
    Ok(())
}

#[tokio::test]
async fn qry_06_a_person_has_at_most_five_hundred_saved_searches() -> Outcome {
    use bb_core::saved_search::SAVED_SEARCHES_MAX;
    let crew = Crew::new().await?;
    let ben = &crew.ben.session_token;
    let made = save(&crew.world, ben, "One", "").await?;
    {
        let mut shared = crew.world.store.lock();
        let template = shared.state.saved_searches[0].clone();
        for number in 1..SAVED_SEARCHES_MAX {
            let mut each = template.clone();
            each.id = SavedSearchId::from_parts(
                jiff::Timestamp::UNIX_EPOCH,
                [
                    u8::try_from(number % 250)?,
                    u8::try_from(number / 250)?,
                    7,
                    7,
                    7,
                    7,
                    7,
                    7,
                    7,
                    7,
                ],
            )?;
            shared.state.saved_searches.push(each);
        }
    }
    let over = save(&crew.world, ben, "Too many", "").await;
    assert!(refused(&over, ErrorCode::LimitReached));
    let copy = save_copy(
        &crew.world.ctx(),
        SaveCopy {
            session_token: ben.clone(),
            search: made.id,
            name: None,
        },
    )
    .await;
    assert!(refused(&copy, ErrorCode::LimitReached));
    // Somebody else is not held back by it.
    assert!(
        save(&crew.world, &crew.cleo.session_token, "Fine", "")
            .await
            .is_ok()
    );
    Ok(())
}

#[tokio::test]
async fn qry_06_a_shared_search_runs_with_the_rights_of_the_person_who_opens_it() -> Outcome {
    use crate::search::SearchScope;
    let crew = Crew::new().await?;
    let made = shared_with_acme(&crew, "Open", "status is open").await?;
    // Both open the same text; what each may search by is their own.
    let words = |who: &Started| {
        let crew = &crew;
        let token = who.session_token.clone();
        async move {
            let ctx = crew.world.ctx();
            let actor = ctx.actor(&token).await?;
            let fields = ctx
                .readable_fields_of(&actor, SearchScope::Everywhere)
                .await?;
            Ok::<_, AppError>(
                fields
                    .fields
                    .into_iter()
                    .map(|field| field.word)
                    .collect::<Vec<_>>(),
            )
        }
    };
    let (ben, dan) = (words(&crew.ben).await?, words(&crew.dan).await?);
    assert!(ben.contains(&"assignedTo".to_owned()));
    assert!(!dan.contains(&"assignedTo".to_owned()));
    for who in [&crew.ben, &crew.dan] {
        let seen = open(&crew.world, &who.session_token, made.id).await?;
        assert_eq!(seen.text, "status is open");
        assert!(seen.shared_with.is_empty() && !seen.mine);
    }
    Ok(())
}

#[tokio::test]
async fn qry_06_an_own_field_named_id_never_takes_the_word_of_the_id_column() -> Outcome {
    let crew = Crew::new().await?;
    super::scenarios_fields::a_field(&crew.world, "Id", bb_core::own_fields::FieldType::Text, &[])
        .await?;
    let ctx = crew.world.ctx();
    let actor = ctx.actor(&crew.ada()).await?;
    let fields = ctx
        .readable_fields_of(&actor, crate::search::SearchScope::Everywhere)
        .await?;
    assert!(fields.field("id").is_none());
    // The column of the Issue ID is a word that needs no field.
    let ben = &crew.ben.session_token;
    let mut shown = a_presentation();
    shown.columns.insert(
        0,
        bb_core::saved_search::Column {
            field: bb_core::fields::ID_WORD.to_owned(),
            width: None,
        },
    );
    let made = crate::saved_searches::save_search(
        &crew.world.ctx(),
        crate::saved_searches::SaveSearch {
            session_token: ben.clone(),
            name: "With ID".to_owned(),
            text: String::new(),
            presentation: shown,
        },
    )
    .await?;
    assert_eq!(made.into_value().presentation.columns[0].field, "id");
    Ok(())
}

#[tokio::test]
async fn qry_06_a_walk_past_more_than_five_hundred_hidden_searches_ends_short_with_a_cursor_that_names_none()
-> Outcome {
    use bb_core::paging::LOOKED_AT_MAX;
    use bb_core::saved_search::SearchShare;
    let crew = Crew::new().await?;
    // Five hundred and twenty searches that Dan cannot read, shared with ACME, older than the one
    // he can read.
    {
        let template = shared_with_acme(&crew, "Hidden", "assignedTo is \"Bob Baker\"").await?;
        let mut shared = crew.world.store.lock();
        let search = shared.state.saved_searches[0].clone();
        let share = shared.state.search_shares[0].clone();
        assert_eq!(template.id, search.id);
        shared.state.saved_searches.clear();
        shared.state.search_columns.clear();
        shared.state.search_shares.clear();
        for number in 0..LOOKED_AT_MAX + 20 {
            let mut each = search.clone();
            each.maker = bb_core::id::UserId::from_random([9; 16]);
            each.id = SavedSearchId::from_parts(
                jiff::Timestamp::UNIX_EPOCH,
                [
                    u8::try_from(number % 250)?,
                    u8::try_from(number / 250)?,
                    3,
                    3,
                    3,
                    3,
                    3,
                    3,
                    3,
                    3,
                ],
            )?;
            shared.state.search_shares.push(SearchShare {
                search: each.id,
                ..share.clone()
            });
            shared.state.saved_searches.push(each);
        }
    }
    let seen = shared_with_acme(&crew, "Seen", "status is open").await?;
    let ids: Vec<SavedSearchId> = crew
        .world
        .store
        .lock()
        .state
        .saved_searches
        .iter()
        .map(|search| search.id)
        .filter(|id| *id != seen.id)
        .collect();
    let page = |cursor: Option<String>| {
        let crew = &crew;
        async move {
            crate::saved_searches::saved_searches(
                &crew.world.ctx(),
                crate::saved_searches::SearchesQuery {
                    session_token: crew.dan.session_token.clone(),
                    scope: SearchSet::Shared,
                    cursor,
                    limit: 5,
                },
            )
            .await
            .map(bb_policy::Authorized::into_value)
        }
    };
    // The first page looked at the first five hundred and found nothing: short, with a cursor that
    // is only a count, so it names no search.
    let first = page(None).await?;
    assert!(first.items.is_empty());
    let cursor = first.next_cursor.ok_or("no cursor after the walk")?;
    assert_eq!(cursor, format!("+{LOOKED_AT_MAX}"));
    for id in &ids {
        assert!(!cursor.contains(&id.to_string()));
    }
    // The next page goes on after them, with nothing lost or repeated.
    let second = page(Some(cursor)).await?;
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].id, seen.id);
    assert_eq!(second.next_cursor, None);
    Ok(())
}

#[tokio::test]
async fn qry_06_a_cursor_with_a_skip_that_does_not_fit_is_refused_for_both_lists() -> Outcome {
    let crew = Crew::new().await?;
    for forged in ["+9223372036854776000", "+499", "+x", "not an id+500"] {
        let saved = crate::saved_searches::saved_searches(
            &crew.world.ctx(),
            crate::saved_searches::SearchesQuery {
                session_token: crew.dan.session_token.clone(),
                scope: SearchSet::Shared,
                cursor: Some(forged.to_owned()),
                limit: 5,
            },
        )
        .await;
        assert!(refused(&saved, ErrorCode::InvalidRequest), "{forged}");
        let choices = crate::auth::project_choices(
            &crew.world.ctx(),
            crate::auth::ProjectChoicesQuery {
                session_token: crew.dan.session_token.clone(),
                cursor: Some(forged.to_owned()),
                limit: 5,
            },
        )
        .await;
        assert!(refused(&choices, ErrorCode::InvalidRequest), "{forged}");
    }
    Ok(())
}

#[tokio::test]
async fn qry_06_a_search_is_shared_with_at_most_a_hundred_places_and_the_next_is_refused() -> Outcome
{
    use bb_core::saved_search::{SHARES_MAX, SearchShare};
    let crew = Crew::new().await?;
    let made = save(&crew.world, &crew.ben.session_token, "Wide", "").await?;
    {
        let mut shared = crew.world.store.lock();
        for number in 0..SHARES_MAX {
            let group = bb_core::id::GroupId::from_parts(
                jiff::Timestamp::UNIX_EPOCH,
                [u8::try_from(number)?, 4, 4, 4, 4, 4, 4, 4, 4, 4],
            )?;
            shared.state.search_shares.push(SearchShare {
                search: made.id,
                target: ShareTarget::Group(group),
                shared_at: jiff::Timestamp::UNIX_EPOCH,
            });
        }
    }
    let over = share(
        &crew.world,
        &crew.ben.session_token,
        made.id,
        ShareTarget::Project(crew.acme),
    )
    .await;
    assert!(refused(&over, ErrorCode::LimitReached));
    Ok(())
}

#[tokio::test]
async fn qry_06_the_label_of_a_role_read_for_many_people_is_the_one_of_their_evaluation() -> Outcome
{
    use bb_policy::Place;
    let crew = Crew::new().await?;
    let world = &crew.world;
    // A role from a parent project (inherited into the project below it), and a role that comes
    // from a group; the people have no other role.
    let parent = a_project(world, "PARENT").await?;
    let child = crate::projects::create_project(
        &world.ctx(),
        crate::projects::CreateProject {
            session_token: crew.ada(),
            parent: Some(parent),
            short_name: "CHILD".to_owned(),
            name: "Child".to_owned(),
        },
    )
    .await?
    .into_value()
    .id;
    let from_parent = world.person_named("pa@example.test", "Pa Rent").await?;
    with_role(world, &from_parent, StartingRole::Contributor, Some(parent)).await?;
    let from_group = world.person_named("gro@example.test", "Gro Up").await?;
    let group = a_group_of(world, "Readers", &[&from_group]).await?;
    let viewer_role = super::workflows_rules::starting_role(world, StartingRole::Viewer).await?;
    crate::roles::assign_role(
        &world.ctx(),
        crate::roles::AssignRole {
            session_token: crew.ada(),
            subject: crate::roles::Subject::Group(group),
            given: crate::roles::Given::Role(viewer_role),
            project: Some(child),
            applies_to: bb_core::access::AppliesTo::Project,
        },
    )
    .await?;
    let ctx = world.ctx();
    let actor = ctx.actor(&crew.ada()).await?;
    let shown = ctx.shown_with_levels_to(&actor).await?;
    let people = [from_parent.person.id, from_group.person.id];
    let policy = world
        .store
        .policy_of_people(&people)
        .await
        .map_err(|error| error.to_string())?;
    for person in people {
        for place in [
            Place::Project(child),
            Place::Project(parent),
            Place::Organization,
        ] {
            let wanted = ctx
                .evaluate(person, place)
                .await?
                .first_role_name()
                .map(str::to_owned);
            assert_eq!(shown.role_label_in(&policy, person, place), wanted);
        }
    }
    // The two cases are not empty: the inherited role and the one from a group both name a role.
    let at_child = |person| shown.role_label_in(&policy, person, Place::Project(child));
    assert_eq!(at_child(people[0]).as_deref(), Some("Contributor"));
    assert_eq!(at_child(people[1]).as_deref(), Some("Viewer"));
    Ok(())
}
