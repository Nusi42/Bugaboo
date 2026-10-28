//! The rights of a person are read once for a request (M4-F137, SEC-03): every place the request
//! asks about is decided from that one read, and the decision is the one the engine makes from
//! what is read for each place on its own, for people with rows on the organization, in a
//! project, in a sub-project, through a group, and with levels set for a project.

use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::fields::Field;
use bb_core::id::{ProjectId, RoleId, UserId};
use bb_core::own_fields::FieldType;
use bb_policy::{Permission, Place};

use super::scenarios::{a_project, token};
use super::scenarios_fields::a_field;
use super::world::{Outcome, World};

type Failure = Box<dyn std::error::Error + Send + Sync>;
use crate::access::evaluation_in;
use crate::people::{AddGroupMember, CreateGroup, add_group_member, create_group};
use crate::projects::{CreateProject, create_project};
use crate::roles::{
    AssignRole, CreateRole, Given, SetFieldLevel, Subject, assign_role, create_role, set_level,
};

async fn role(world: &World, name: &str, permissions: Vec<Permission>) -> Result<RoleId, String> {
    Ok(create_role(
        &world.ctx(),
        CreateRole {
            session_token: token(world),
            name: name.to_owned(),
            permissions,
            two_step: false,
        },
    )
    .await
    .map_err(|e| format!("{e:?}"))?
    .into_value()
    .id)
}

async fn give(
    world: &World,
    subject: Subject,
    role: RoleId,
    project: Option<ProjectId>,
    applies_to: AppliesTo,
) -> Outcome {
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: token(world),
            subject,
            given: Given::Role(role),
            project,
            applies_to,
        },
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

/// The roles Watcher and Worker, with a level of Worker set for the roles' own and one for the
/// project `a`: both kinds of level are read.
async fn roles(world: &World, a: ProjectId) -> Result<(RoleId, RoleId), Failure> {
    let watcher = role(
        world,
        "Watcher",
        vec![Permission::SeeProjects, Permission::SeeStatus],
    )
    .await?;
    let worker = role(
        world,
        "Worker",
        vec![
            Permission::SeeProjects,
            Permission::SeeIssues,
            Permission::SeeStatus,
        ],
    )
    .await?;
    for (project, level) in [(None, FieldLevel::CanSee), (Some(a), FieldLevel::CanChange)] {
        set_level(
            &world.ctx(),
            SetFieldLevel {
                session_token: token(world),
                role: worker,
                project,
                issue_type: None,
                field: Field::Priority.into(),
                level: Some(level),
            },
        )
        .await
        .map_err(|e| format!("{e:?}"))?;
    }
    Ok((watcher, worker))
}

/// Four people without a role: Bob, Cy, Dee and Eve.
async fn people(world: &World) -> Result<[UserId; 4], Failure> {
    let bob = world
        .person("bob@example.test")
        .await
        .map_err(|e| format!("{e:?}"))?
        .person
        .id;
    let cy = world
        .person("cy@example.test")
        .await
        .map_err(|e| format!("{e:?}"))?
        .person
        .id;
    let dee = world
        .person("dee@example.test")
        .await
        .map_err(|e| format!("{e:?}"))?
        .person
        .id;
    let nobody = world
        .person("eve@example.test")
        .await
        .map_err(|e| format!("{e:?}"))?
        .person
        .id;
    Ok([bob, cy, dee, nobody])
}

/// The people of the test and the places they are asked about: an organization with projects
/// ACME (AAA) with a sub-project, BBB, and rows of every kind (see the test).
async fn organization(world: &World) -> Result<(Vec<UserId>, [Place; 4]), Failure> {
    let admin = world.admin.person.id;
    let (a, b) = (
        a_project(world, "AAA").await?,
        a_project(world, "BBB").await?,
    );
    let sub = create_project(
        &world.ctx(),
        CreateProject {
            session_token: token(world),
            parent: Some(a),
            short_name: "SUB".to_owned(),
            name: "Below A".to_owned(),
        },
    )
    .await
    .map_err(|e| format!("{e:?}"))?
    .into_value()
    .id;
    // An own field: a role anywhere makes every own field known to the decision.
    a_field(world, "Customer", FieldType::Text, &[]).await?;
    let (watcher, worker) = roles(world, a).await?;
    let [bob, cy, dee, nobody] = people(world).await?;
    give(
        world,
        Subject::Person(bob),
        watcher,
        None,
        AppliesTo::Organization,
    )
    .await?;
    give(
        world,
        Subject::Person(bob),
        worker,
        Some(a),
        AppliesTo::ProjectAndSubProjects,
    )
    .await?;
    give(
        world,
        Subject::Person(cy),
        worker,
        Some(b),
        AppliesTo::Project,
    )
    .await?;
    let group = create_group(
        &world.ctx(),
        CreateGroup {
            session_token: token(world),
            name: "Crew".to_owned(),
        },
    )
    .await
    .map_err(|e| format!("{e:?}"))?
    .into_value()
    .id;
    add_group_member(
        &world.ctx(),
        AddGroupMember {
            session_token: token(world),
            group,
            person: dee,
        },
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    give(
        world,
        Subject::Group(group),
        worker,
        Some(sub),
        AppliesTo::Project,
    )
    .await?;

    let places = [
        Place::Organization,
        Place::Project(a),
        Place::Project(b),
        Place::Project(sub),
    ];
    Ok((vec![admin, bob, cy, dee, nobody], places))
}

#[tokio::test]
async fn sec_03_every_place_of_a_request_is_decided_from_one_read_and_as_from_one_read_each()
-> Outcome {
    let world = World::new().await?;
    let (people, places) = organization(&world).await?;
    for person in people {
        let request = world.ctx();
        let loads = world.store.policy_loads();
        let mut from_one_read = Vec::new();
        for place in places {
            from_one_read.push(request.evaluate(person, place).await?);
        }
        assert_eq!(
            world.store.policy_loads() - loads,
            1,
            "the rows of {person:?} are read once for the four places"
        );
        for (place, held) in places.into_iter().zip(from_one_read) {
            let on_its_own = evaluation_in(world.ctx().store(), person, place)
                .await
                .map_err(|e| format!("{e:?}"))?
                .ok_or("no evaluation")?;
            assert_eq!(*held, on_its_own, "{person:?} at {place:?}");
        }
    }
    Ok(())
}

#[tokio::test]
async fn sec_03_a_project_that_does_not_exist_is_not_found_whatever_was_read() -> Outcome {
    let world = World::new().await?;
    let request = world.ctx();
    let admin = world.admin.person.id;
    request.evaluate(admin, Place::Organization).await?;
    let nowhere =
        ProjectId::from_parts(jiff::Timestamp::UNIX_EPOCH, [0; 10]).map_err(|e| e.to_string())?;
    let answer = request.evaluate(admin, Place::Project(nowhere)).await;
    assert_eq!(
        answer.err(),
        Some(crate::auth::AppError::Refused(bb_core::ErrorCode::NotFound))
    );
    Ok(())
}

/// The reads a request makes for the rights do not grow with the number of places: the rows,
/// roles and levels are read twice at most (the person's own, then those set for projects), and
/// each project costs the one read that tells it is there and where it sits (PERF-05).
#[tokio::test]
async fn perf_05_the_reads_of_the_rights_do_not_grow_with_the_places_but_by_one_for_each() -> Outcome
{
    let world = World::new().await?;
    let admin = world.admin.person.id;
    let mut projects = Vec::new();
    for number in 0..12 {
        projects.push(a_project(&world, &format!("P{number}")).await?);
    }
    let mut reads = Vec::new();
    for count in [2, 12] {
        let request = world.ctx();
        let before = world.store.reads();
        request.evaluate(admin, Place::Organization).await?;
        for project in &projects[..count] {
            request.evaluate(admin, Place::Project(*project)).await?;
        }
        reads.push(world.store.reads() - before);
    }
    assert_eq!(
        reads[0], 3,
        "one read of the person's rows and levels (one snapshot) and the chain of each project"
    );
    assert_eq!(
        reads[1] - reads[0],
        10,
        "ten more projects cost one read each, not the rights again: {reads:?}"
    );
    Ok(())
}
