//! What a person is offered to search by when the answer depends on what they may know or see:
//! the names of types and statuses, the fields that hold people, the owner of a word, the places a
//! search looks at (QRY-05, ADR-0005 amendment M5, decision 9).

use bb_core::ErrorCode;
use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::access_key::KeyChoice;
use bb_core::fields::{Field, FieldKey};
use bb_core::issue::{IssueIcon, StatusGroup};
use bb_core::own_fields::{FieldType, GROUP_FIELD_KIND, PERSON_FIELD_KIND, Scope};
use bb_core::workflow::starter::OPEN;
use bb_core::workflow::{ChangeDraft, StatusDraft, StatusRules};

use super::offers::{groups, lead_of};
use super::scenarios::{a_project, token};
use super::scenarios_fields::{a_field, new_field};
use super::scenarios_issues::a_type;
use super::scenarios_workflows::{a_workflow, published_with};
use super::search_fields::{bob_with_a_role, every_field, has, set_field, set_people, words_of};
use super::world::{Outcome, PASSWORD, World};
use crate::auth::{AppError, CreateAccessKey, create_access_key};
use crate::fields::create_field;
use crate::issues::{CreateIssueType, create_issue_type};
use crate::projects::{CreateProject, create_project};
use crate::roles::{SetFieldLevel, set_level};
use crate::search::{FieldTarget, ReadableFieldsQuery, SearchScope, readable_fields};

async fn choices_of(
    world: &World,
    session: &str,
    scope: SearchScope,
    name: &str,
) -> Result<Vec<String>, AppError> {
    let fields = readable_fields(
        &world.ctx(),
        ReadableFieldsQuery {
            session_token: session.to_owned(),
            scope,
        },
    )
    .await?
    .into_value()
    .0;
    Ok(fields
        .field(name)
        .map(|field| field.choices.clone())
        .unwrap_or_default())
}

async fn targets_of(world: &World, session: &str) -> Result<Vec<(String, FieldTarget)>, AppError> {
    let ctx = world.ctx();
    let actor = ctx.actor(session).await?;
    Ok(ctx
        .searchables_of(&actor, SearchScope::Everywhere)
        .await?
        .into_iter()
        .map(|each| (each.field.word, each.target))
        .collect())
}

fn offers(names: &[String], name: &str) -> bool {
    names.iter().any(|each| each == name)
}

#[tokio::test]
async fn qry_05_the_types_offered_are_those_the_person_may_know() -> Outcome {
    let g = groups().await?;
    let sam = lead_of(&g.world, "sam@example.test", g.other).await?;
    create_issue_type(
        &g.world.ctx(),
        CreateIssueType {
            session_token: g.admin.clone(),
            project: Some("OTHER".to_owned()),
            name: "Secret kind".to_owned(),
            icon: IssueIcon::CircleHelp,
        },
    )
    .await?;
    // The lead of ACME does not know OTHER, so a type made there is not among the choices.
    let hidden = choices_of(&g.world, &g.lead, SearchScope::Everywhere, "type").await?;
    let known = choices_of(
        &g.world,
        &sam.session_token,
        SearchScope::Everywhere,
        "type",
    )
    .await?;
    assert!(!offers(&hidden, "Secret kind"), "{hidden:?}");
    assert!(offers(&known, "Secret kind"), "{known:?}");
    Ok(())
}

#[tokio::test]
async fn qry_05_the_statuses_offered_are_those_of_the_workflows_in_force_where_the_person_looks()
-> Outcome {
    let g = groups().await?;
    let editor = a_workflow(&g.world, "OTHER").await?;
    published_with(&g.world, &editor, |sent| {
        sent.statuses.push(StatusDraft {
            key: None,
            name: "Parked".to_owned(),
            group: StatusGroup::InProgress,
        });
        sent.changes.push(ChangeDraft {
            from: OPEN.to_owned(),
            to: "Parked".to_owned(),
            button: "Park".to_owned(),
            rules: StatusRules::default(),
        });
    })
    .await?;
    let sam = lead_of(&g.world, "sam@example.test", g.other).await?;
    let own = choices_of(
        &g.world,
        &sam.session_token,
        SearchScope::Everywhere,
        "status",
    )
    .await?;
    let lena = choices_of(&g.world, &g.lead, SearchScope::Everywhere, "status").await?;
    assert!(offers(&own, "Parked"), "{own:?}");
    assert!(!offers(&lena, "Parked"), "{lena:?}");
    Ok(())
}

#[tokio::test]
async fn qry_05_a_field_of_people_follows_who_sees_people_like_the_built_in_ones() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let reviewer = a_field(&world, "Reviewer", FieldType::Person, &[]).await?;
    let team = a_field(&world, "Team", FieldType::Group, &[]).await?;
    let mut shown = every_field();
    shown.extend([FieldKey::Own(reviewer.id), FieldKey::Own(team.id)]);
    let (bob, role) = bob_with_a_role(&world, &shown).await?;
    set_people(&world, role, GROUP_FIELD_KIND, PeopleLevel::Names).await?;
    let names = words_of(&world, &bob, SearchScope::Everywhere).await?;
    assert!(has(&names, "reviewer") && has(&names, "team"), "{names:?}");
    set_people(&world, role, PERSON_FIELD_KIND, PeopleLevel::RoleOnly).await?;
    set_people(&world, role, GROUP_FIELD_KIND, PeopleLevel::RoleOnly).await?;
    let hidden = words_of(&world, &bob, SearchScope::Everywhere).await?;
    assert!(!has(&hidden, "reviewer"), "{hidden:?}");
    assert!(!has(&hidden, "team"), "{hidden:?}");
    Ok(())
}

#[tokio::test]
async fn qry_05_an_own_field_never_takes_the_word_of_a_built_in_field_not_even_an_unreadable_one()
-> Outcome {
    let world = World::new().await?;
    let closed = a_project(&world, "CLOSED").await?;
    // Three own fields named like built-in fields: one the person may not read in one project,
    // one that cannot be searched yet, and one that fits a word every issue has.
    let priority = a_field(&world, "Priority", FieldType::Text, &[]).await?;
    let due = a_field(&world, "Due date", FieldType::Date, &[]).await?;
    let created = a_field(&world, "Created", FieldType::Date, &[]).await?;
    let mut shown = every_field();
    shown.extend([priority.id, due.id, created.id].map(FieldKey::Own));
    let (bob, role) = bob_with_a_role(&world, &shown).await?;
    set_field(
        &world,
        role,
        Some(closed),
        Field::Priority.into(),
        Some(FieldLevel::Hidden),
    )
    .await?;
    let found = targets_of(&world, &bob).await?;
    for word in ["priority", "dueDate"] {
        assert!(
            !found
                .iter()
                .any(|(each, target)| each == word && matches!(target, FieldTarget::Own { .. })),
            "{word} is a built-in word"
        );
    }
    let created_words: Vec<_> = found.iter().filter(|(word, _)| word == "created").collect();
    assert_eq!(created_words.len(), 1);
    assert_eq!(created_words[0].1, FieldTarget::Created);
    Ok(())
}

#[tokio::test]
async fn qry_05_between_own_fields_the_first_keeps_the_word_for_every_person() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let first = a_field(&world, "Story points", FieldType::WholeNumber, &[]).await?;
    // Another name, the same word: the names are different names, the words are the same word.
    let second = a_field(&world, "storyPoints", FieldType::WholeNumber, &[]).await?;
    let mut shown = every_field();
    shown.push(FieldKey::Own(second.id));
    let (bob, _) = bob_with_a_role(&world, &shown).await?;
    let for_bob = targets_of(&world, &bob).await?;
    // The first field is hidden from Bob, so nobody takes its word in its place.
    assert!(!for_bob.iter().any(|(word, _)| word == "storyPoints"));
    let for_admin = targets_of(&world, &token(&world)).await?;
    let owner = for_admin.iter().find(|(word, _)| word == "storyPoints");
    assert_eq!(
        owner.map(|found| found.1),
        Some(FieldTarget::Own {
            field: first.id,
            kind: FieldType::WholeNumber
        })
    );
    Ok(())
}

#[tokio::test]
async fn qry_05_the_list_of_a_project_the_person_does_not_see_is_not_found() -> Outcome {
    let g = groups().await?;
    let asked = async |project| {
        readable_fields(
            &g.world.ctx(),
            ReadableFieldsQuery {
                session_token: g.lead.clone(),
                scope: SearchScope::Within(project),
            },
        )
        .await
    };
    assert_eq!(
        asked(g.other).await.err(),
        Some(AppError::Refused(ErrorCode::NotFound))
    );
    assert!(asked(g.acme).await.is_ok());
    assert!(asked(g.sub).await.is_ok());
    Ok(())
}

#[tokio::test]
async fn qry_05_a_field_hidden_in_a_sub_project_is_not_readable_in_the_list_above_it() -> Outcome {
    let world = World::new().await?;
    let top = a_project(&world, "TOP").await?;
    let low = create_project(
        &world.ctx(),
        CreateProject {
            session_token: token(&world),
            parent: Some(top),
            short_name: "LOW".to_owned(),
            name: "Low".to_owned(),
        },
    )
    .await?
    .into_value()
    .id;
    let (bob, role) = bob_with_a_role(&world, &every_field()).await?;
    set_field(
        &world,
        role,
        Some(low),
        Field::Priority.into(),
        Some(FieldLevel::Hidden),
    )
    .await?;
    let in_top = words_of(&world, &bob, SearchScope::Within(top)).await?;
    let in_low = words_of(&world, &bob, SearchScope::Within(low)).await?;
    assert!(!has(&in_top, "priority") && !has(&in_low, "priority"));
    assert!(has(&in_top, "title"));
    Ok(())
}

#[tokio::test]
async fn qry_05_an_own_field_of_one_project_is_offered_where_it_is_used_only() -> Outcome {
    let world = World::new().await?;
    let used = a_project(&world, "USED").await?;
    let other = a_project(&world, "OTHER").await?;
    let mut input = new_field(&world, "Rack", FieldType::Text, &[]);
    input.scope = Scope {
        projects: vec![used],
        issue_types: Vec::new(),
    };
    let rack = create_field(&world.ctx(), input).await?.into_value();
    let (bob, role) = bob_with_a_role(&world, &every_field()).await?;
    // Hidden where the field is not used: that does not matter.
    set_field(
        &world,
        role,
        Some(other),
        FieldKey::Own(rack.id),
        Some(FieldLevel::Hidden),
    )
    .await?;
    set_field(
        &world,
        role,
        None,
        FieldKey::Own(rack.id),
        Some(FieldLevel::CanSee),
    )
    .await?;
    assert!(has(
        &words_of(&world, &bob, SearchScope::Everywhere).await?,
        "rack"
    ));
    // Hidden where it is used: the field is gone.
    set_field(
        &world,
        role,
        Some(used),
        FieldKey::Own(rack.id),
        Some(FieldLevel::Hidden),
    )
    .await?;
    assert!(!has(
        &words_of(&world, &bob, SearchScope::Everywhere).await?,
        "rack"
    ));
    Ok(())
}

#[tokio::test]
async fn qry_05_a_level_for_one_issue_type_hides_the_field_for_all_types() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let (bob, role) = bob_with_a_role(&world, &every_field()).await?;
    let task = a_type(&world).await?;
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: token(&world),
            role,
            project: None,
            issue_type: Some(task),
            field: Field::Priority.into(),
            level: Some(FieldLevel::Hidden),
        },
    )
    .await?;
    let words = words_of(&world, &bob, SearchScope::Everywhere).await?;
    assert!(!has(&words, "priority"), "{words:?}");
    assert!(has(&words, "status"));
    Ok(())
}

#[tokio::test]
async fn qry_05_a_key_for_one_project_reads_the_fields_of_that_project_only() -> Outcome {
    let g = groups().await?;
    let (bob_key_owner, role) = bob_with_a_role(&g.world, &every_field()).await?;
    set_field(
        &g.world,
        role,
        Some(g.other),
        Field::Priority.into(),
        Some(FieldLevel::Hidden),
    )
    .await?;
    let key = create_access_key(
        &g.world.ctx(),
        CreateAccessKey {
            session_token: bob_key_owner.clone(),
            password: PASSWORD.to_owned(),
            name: "For one project".to_owned(),
            choice: KeyChoice::Everything,
            permissions: Vec::new(),
            projects: vec![g.acme],
            expires_at: None,
        },
    )
    .await?
    .into_value()
    .secret;
    let with_session = words_of(&g.world, &bob_key_owner, SearchScope::Everywhere).await?;
    let with_key = words_of(&g.world, &key, SearchScope::Everywhere).await?;
    assert!(!has(&with_session, "priority"), "{with_session:?}");
    assert!(has(&with_key, "priority"), "{with_key:?}");
    Ok(())
}
