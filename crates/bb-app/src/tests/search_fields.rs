//! The readable fields of a person (QRY-05, ADR-0005 amendment M5, decision 5): a field the person
//! may not read is not among them, so the checker answers for it as for a field that does not
//! exist.

use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{ProjectId, RoleId};
use bb_core::own_fields::FieldType;
use bb_core::people_kinds::PeopleKind;
use bb_policy::Permission;
use bb_searchlang::{ReadableFields, check, parse_with};

use super::scenarios::{a_project, token};
use super::scenarios_fields::a_field;
use super::world::{Outcome, World};
use crate::roles::{
    AssignRole, CreateRole, Given, SetFieldLevel, SetPeopleLevel, Subject, assign_role,
    create_role, set_level,
};
use crate::search::{ReadableFieldsQuery, SearchScope, readable_fields};

/// Bob sees every project and its issues. His role can see every field and every kind of people
/// by name, except what the test changes afterwards.
pub(super) async fn bob_with_a_role(
    world: &World,
    fields: &[FieldKey],
) -> Result<(String, RoleId), String> {
    let admin = token(world);
    let role = create_role(
        &world.ctx(),
        CreateRole {
            session_token: admin.clone(),
            name: "Reader".to_owned(),
            permissions: vec![Permission::SeeProjects, Permission::SeeIssues],
            two_step: false,
        },
    )
    .await
    .map_err(|error| error.to_string())?
    .into_value()
    .id;
    for field in fields {
        set_field(world, role, None, *field, Some(FieldLevel::CanSee)).await?;
    }
    for kind in [PeopleKind::AssignedTo, PeopleKind::ReportedBy] {
        set_people(world, role, kind, PeopleLevel::Names).await?;
    }
    let bob = world
        .person("bob@example.test")
        .await
        .map_err(|error| error.to_string())?;
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: admin,
            subject: Subject::Person(bob.person.id),
            given: Given::Role(role),
            project: None,
            applies_to: AppliesTo::Organization,
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    Ok((bob.session_token, role))
}

pub(super) async fn set_field(
    world: &World,
    role: RoleId,
    project: Option<ProjectId>,
    field: FieldKey,
    level: Option<FieldLevel>,
) -> Result<(), String> {
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: token(world),
            role,
            project,
            issue_type: None,
            field,
            level,
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) async fn set_people(
    world: &World,
    role: RoleId,
    kind: PeopleKind,
    level: PeopleLevel,
) -> Result<(), String> {
    set_level(
        &world.ctx(),
        SetPeopleLevel {
            session_token: token(world),
            role,
            project: None,
            kind,
            level: Some(level),
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn every_field() -> Vec<FieldKey> {
    Field::ALL.iter().map(|field| (*field).into()).collect()
}

pub(super) async fn words_of(
    world: &World,
    session: &str,
    scope: SearchScope,
) -> Result<Vec<String>, String> {
    let fields = readable_fields(
        &world.ctx(),
        ReadableFieldsQuery {
            session_token: session.to_owned(),
            scope,
        },
    )
    .await
    .map_err(|error| error.to_string())?
    .into_value()
    .0;
    Ok(fields.fields.into_iter().map(|field| field.word).collect())
}

pub(super) fn has(words: &[String], word: &str) -> bool {
    words.iter().any(|each| each == word)
}

#[tokio::test]
async fn qry_05_a_person_reads_the_fields_their_role_shows_and_the_ones_every_issue_has() -> Outcome
{
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let customer = a_field(&world, "Story points", FieldType::WholeNumber, &[]).await?;
    let mut shown = every_field();
    shown.push(FieldKey::Own(customer.id));
    let (bob, _) = bob_with_a_role(&world, &shown).await?;
    let words = words_of(&world, &bob, SearchScope::Everywhere).await?;
    for word in [
        "title",
        "status",
        "priority",
        "assignedTo",
        "reportedBy",
        "labels",
        "tags",
        "project",
        "type",
        "statusGroup",
        "parent",
        "created",
        "changed",
        "closed",
        "storyPoints",
    ] {
        assert!(has(&words, word), "{word} is readable: {words:?}");
    }
    // A field whose values the issues do not hold yet is not offered.
    assert!(!has(&words, "dueDate"));
    Ok(())
}

#[tokio::test]
async fn qry_05_a_hidden_field_is_checked_like_a_field_that_does_not_exist() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    a_field(&world, "Secret code", FieldType::Text, &[]).await?;
    // Bob can see the built-in fields and not the own one.
    let (bob, _) = bob_with_a_role(&world, &every_field()).await?;
    let fields = ReadableFields {
        fields: readable_fields(
            &world.ctx(),
            ReadableFieldsQuery {
                session_token: bob,
                scope: SearchScope::Everywhere,
            },
        )
        .await?
        .into_value()
        .0
        .fields,
    };
    let answer = |text: &str| {
        check(
            parse_with(text, &fields).map_err(|mistake| vec![mistake])?,
            &fields,
        )
        .map(|_| ())
    };
    let hidden = answer("secretCode is abc");
    let unknown = answer("nonsenseWord is abc");
    let (Err(hidden), Err(unknown)) = (hidden, unknown) else {
        return Err("both are mistakes".into());
    };
    assert_eq!(hidden.len(), unknown.len());
    assert_eq!(hidden[0].key, unknown[0].key);
    assert!(answer("title is abc").is_ok());
    Ok(())
}

#[tokio::test]
async fn qry_05_a_field_hidden_in_one_project_is_not_readable_across_projects() -> Outcome {
    let world = World::new().await?;
    let open = a_project(&world, "OPEN").await?;
    let closed = a_project(&world, "CLOSED").await?;
    let (bob, role) = bob_with_a_role(&world, &every_field()).await?;
    set_field(
        &world,
        role,
        Some(closed),
        Field::Priority.into(),
        Some(FieldLevel::Hidden),
    )
    .await?;
    let everywhere = words_of(&world, &bob, SearchScope::Everywhere).await?;
    assert!(!has(&everywhere, "priority"), "{everywhere:?}");
    assert!(has(&everywhere, "title"));
    let in_open = words_of(&world, &bob, SearchScope::Within(open)).await?;
    assert!(has(&in_open, "priority"));
    let in_closed = words_of(&world, &bob, SearchScope::Within(closed)).await?;
    assert!(!has(&in_closed, "priority"));
    Ok(())
}

#[tokio::test]
async fn qry_05_a_kind_of_people_whose_names_are_hidden_cannot_be_searched() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let (bob, role) = bob_with_a_role(&world, &every_field()).await?;
    set_people(&world, role, PeopleKind::AssignedTo, PeopleLevel::RoleOnly).await?;
    let words = words_of(&world, &bob, SearchScope::Everywhere).await?;
    assert!(!has(&words, "assignedTo"));
    assert!(has(&words, "reportedBy"));
    Ok(())
}

#[tokio::test]
async fn qry_05_a_person_who_sees_no_project_reads_no_field() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let nobody = world.person("eve@example.test").await?.session_token;
    let words = words_of(&world, &nobody, SearchScope::Everywhere).await?;
    assert!(words.is_empty(), "{words:?}");
    Ok(())
}

#[tokio::test]
async fn qry_05_an_own_field_named_like_a_built_in_word_keeps_the_built_in_one() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "OPEN").await?;
    let title = a_field(&world, "Title", FieldType::Text, &[]).await?;
    let mut shown = every_field();
    shown.push(FieldKey::Own(title.id));
    let (bob, _) = bob_with_a_role(&world, &shown).await?;
    let words = words_of(&world, &bob, SearchScope::Everywhere).await?;
    assert_eq!(words.iter().filter(|word| *word == "title").count(), 1);
    Ok(())
}
