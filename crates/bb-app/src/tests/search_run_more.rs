//! More of running a search (QRY-05, SEC-06, ISS-13): who may use a `contains` on its own, the
//! names of people by kind, tags and labels the person may not know, the cap of the groups.

use bb_core::fields::Field;

use super::offers::groups;
use super::scenarios::{a_project, token};
use super::search_fields::{bob_with_a_role, every_field};
use super::search_run::{asked, issue_of, refused};
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::search::{SearchScope, search};

#[tokio::test]
async fn qry_05_contains_on_its_own_needs_the_title_and_the_description_to_be_readable() -> Outcome
{
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let admin = token(&world);
    search(&world.ctx(), asked(&admin, "contains crash"))
        .await
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(world.store.lock().searches.len(), 1);
    // A role that hides the description: the search is told as a field that is not there.
    let shown: Vec<_> = every_field()
        .into_iter()
        .filter(|field| *field != Field::Description.into())
        .collect();
    let (bob, _) = bob_with_a_role(&world, &shown).await?;
    let refused = search(&world.ctx(), asked(&bob, "contains crash")).await;
    assert!(matches!(refused, Err(AppError::Mistakes(ref found)) if found.len() == 1));
    assert_eq!(world.store.lock().searches.len(), 1);
    Ok(())
}

#[tokio::test]
async fn sec_06_a_name_is_resolved_for_the_kind_of_the_field_it_stands_in() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let bob = world.person("bob@example.test").await?;
    let admin = token(&world);
    for text in [
        "assignedTo is \"bob builder\" or reportedBy is \"bob builder\"",
        "reportedBy is \"bob builder\" or assignedTo is \"bob builder\"",
    ] {
        search(&world.ctx(), asked(&admin, text))
            .await
            .map_err(|error| format!("{error:?}"))?;
        let request = world
            .store
            .lock()
            .searches
            .last()
            .cloned()
            .ok_or("the store was asked")?;
        // The same name stands once for each kind, resolved by the rule of that kind: the answer
        // for one kind is never taken for another, whatever the order of the text.
        let kinds: Vec<_> = request
            .names
            .people
            .iter()
            .filter(|(_, name, ids)| name == "bob builder" && ids == &[bob.person.id])
            .map(|(kind, _, _)| *kind)
            .collect();
        assert_eq!(kinds.len(), 2, "{text}");
        assert_ne!(kinds[0], kinds[1], "{text}");
    }
    Ok(())
}

#[tokio::test]
async fn qry_05_a_label_or_tag_the_person_may_not_know_is_as_unknown_as_one_that_does_not_exist()
-> Outcome {
    use super::offers::group_in;
    use super::scenarios_tags::{a_label, tagged};

    let g = groups().await?;
    let world = &g.world;
    // A label of a group that belongs to OTHER, and a tag on an issue of OTHER: the lead of ACME,
    // who does not see OTHER, may know neither.
    let secret_group = group_in(world, &g.admin, Some("OTHER"), "Hidden area").await?;
    a_label(world, "hushed", Some(secret_group)).await?;
    let in_other = issue_of(world, g.other).await?;
    tagged(world, in_other.id, "hushtag").await?;
    let unknown = |text: &'static str| {
        let (world, lead) = (&g.world, g.lead.clone());
        async move { refused(world, &lead, text).await }
    };
    for text in ["labels is hushed", "tags is hushtag"] {
        let found = unknown(text).await;
        assert_eq!(found.len(), 1, "{text}");
        assert_eq!(found[0].key, bb_searchlang::messages::UNKNOWN_CHOICE);
        // The same answer as a name that does not exist at all.
        let missing = unknown(if text.starts_with("labels") {
            "labels is nonesuch"
        } else {
            "tags is nonesuch"
        })
        .await;
        assert_eq!(missing[0].key, found[0].key);
        assert_eq!(missing[0].span.start, found[0].span.start);
    }
    // The administrator sees OTHER and knows both; a label of a group the lead knows is theirs too.
    for text in ["labels is hushed", "tags is hushtag"] {
        assert!(refused(world, &g.admin, text).await.is_empty(), "{text}");
    }
    assert!(refused(world, &g.lead, "labels is login").await.is_empty());
    assert!(!world.store.lock().searches.is_empty());
    Ok(())
}

#[tokio::test]
async fn iss_13_a_hidden_long_text_field_never_finds_an_issue() -> Outcome {
    use bb_core::own_fields::FieldType;

    use super::scenarios_fields::a_field;

    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    a_field(&world, "Notes", FieldType::LongText, &[]).await?;
    a_field(&world, "Rack", FieldType::Text, &[]).await?;
    let admin = token(&world);
    // The administrator reads the long text and may search it; a short text is compared, not
    // searched by its words, so `contains` is no operator of it.
    assert!(
        refused(&world, &admin, "notes contains crash")
            .await
            .is_empty()
    );
    let short = refused(&world, &admin, "rack contains crash").await;
    assert_eq!(short.len(), 1);
    assert_eq!(short[0].key, bb_searchlang::messages::OPERATOR_NOT_ALLOWED);
    // A person whose role does not show the own field is told what is told of a word that is not
    // there, and no statement is made for it.
    let (bob, _) = bob_with_a_role(&world, &every_field()).await?;
    let hidden = refused(&world, &bob, "notes contains crash").await;
    let unknown = refused(&world, &bob, "nonsense contains crash").await;
    assert_eq!(hidden.len(), 1);
    assert_eq!(hidden[0].key, unknown[0].key);
    let asked = world.store.lock().searches.len();
    assert_eq!(
        asked, 1,
        "only the administrator's search reached the store"
    );
    Ok(())
}

#[tokio::test]
async fn qry_05_over_the_cap_the_groups_are_cut_but_the_group_without_a_value_stays() -> Outcome {
    use crate::search::{GroupCount, GroupKey, SearchGroupsQuery, ShownKey, search_groups};

    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    // 205 values and the issues without one, which the store puts last.
    let mut counted: Vec<GroupCount> = (0..205)
        .map(|number| GroupCount {
            key: GroupKey::Text(format!("value{number:03}")),
            issues: 2,
        })
        .collect();
    counted.push(GroupCount {
        key: GroupKey::Empty,
        issues: 7,
    });
    world.store.lock().search_groups = counted;
    let asked = |by: &str| SearchGroupsQuery {
        session_token: token(&world),
        text: String::new(),
        scope: SearchScope::Everywhere,
        by: by.to_owned(),
    };
    let cut = search_groups(&world.ctx(), asked("title"))
        .await
        .map_err(|error| format!("{error:?}"))?
        .into_value();
    assert!(cut.truncated);
    assert_eq!(cut.groups.len(), 200);
    // The first 199 values, then the group without a value with its own count.
    let last = cut.groups.last().ok_or("a last group")?;
    assert!(matches!(last.key, ShownKey::Other(GroupKey::Empty)));
    assert_eq!(last.issues, 7);
    assert!(
        matches!(&cut.groups[198].key, ShownKey::Other(GroupKey::Text(text)) if text == "value198")
    );
    // Exactly at the cap nothing is cut.
    world.store.lock().search_groups.truncate(200);
    let whole = search_groups(&world.ctx(), asked("title"))
        .await
        .map_err(|error| format!("{error:?}"))?
        .into_value();
    assert!(!whole.truncated);
    assert_eq!(whole.groups.len(), 200);
    Ok(())
}

#[tokio::test]
async fn sec_06_a_role_that_sees_assignees_but_not_reporters_names_people_for_one_kind_only()
-> Outcome {
    use bb_core::access::PeopleLevel;
    use bb_core::people_kinds::PeopleKind;

    use super::search_fields::set_people;

    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let ann = world
        .person_named("ann@example.test", "Ann Archer")
        .await
        .map_err(|error| format!("{error:?}"))?;
    let (bob, role) = bob_with_a_role(&world, &every_field()).await?;
    set_people(&world, role, PeopleKind::ProjectMembers, PeopleLevel::Names).await?;
    set_people(&world, role, PeopleKind::ReportedBy, PeopleLevel::RoleOnly).await?;
    // Assignees are named; the field of reporters is not there for this role at all.
    search(&world.ctx(), asked(&bob, "assignedTo is \"Ann Archer\""))
        .await
        .map_err(|error| format!("{error:?}"))?;
    let request = world.store.lock().searches.last().cloned().ok_or("asked")?;
    let named: Vec<_> = request
        .names
        .people
        .iter()
        .filter(|(_, _, ids)| ids == &[ann.person.id])
        .map(|(kind, _, _)| *kind)
        .collect();
    assert_eq!(named, [PeopleKind::AssignedTo]);
    // The name that is allowed for the assignees is no name for the reporters, whichever field the
    // text names first; and nothing reaches the store for it.
    let before = world.store.lock().searches.len();
    for text in [
        "assignedTo is \"Ann Archer\" or reportedBy is \"Ann Archer\"",
        "reportedBy is \"Ann Archer\" or assignedTo is \"Ann Archer\"",
    ] {
        let found = refused(&world, &bob, text).await;
        assert_eq!(found.len(), 1, "{text}");
        assert_eq!(
            found[0].key,
            bb_searchlang::messages::UNKNOWN_FIELD,
            "{text}"
        );
    }
    assert_eq!(world.store.lock().searches.len(), before);
    Ok(())
}
