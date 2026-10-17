//! The list of projects, the preview of a move and every request decide with the same engine from
//! the same rows (PRJ-02, PRJ-03): `Policy::evaluate_at` (made from what the projects port reads)
//! and the evaluation of a request (the policy port, one place at a time) agree for every person
//! and every project of an organization with a deep tree, groups, *No access* and levels.
//! Both databases (ARCH-DB-PARITY).

use bb_app::access::{Permission, Place, Tree};
use bb_app::projects::ProjectStore as _;
use bb_app::roles::{Given, SetFieldLevel, SetPeopleLevel, Subject};
use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::ProjectId;
use bb_core::people_kinds::PeopleKind;
use bb_testing::{Failure, TestResult, each_database};

use super::policy_rig::{Member, Org};
use super::projects::{LEAD, VIEW, create};
use super::rig::Rig;

/// What a request decides for a person at a project: the permissions that count, the level of
/// every field and of every kind of people information, from `Check access`.
type Decision = (
    Vec<Permission>,
    Vec<(bb_core::fields::FieldKey, FieldLevel)>,
    Vec<(PeopleKind, PeopleLevel)>,
);

async fn decided(org: &Org<'_>, person: &Member, project: ProjectId) -> Result<Decision, String> {
    let answer = org
        .check(&org.admin, person.id, Some(project))
        .await
        .map_err(|error| format!("{error:?}"))?;
    Ok((
        answer
            .permissions
            .iter()
            .filter(|decision| decision.allowed)
            .map(|decision| decision.permission)
            .collect(),
        answer
            .fields
            .iter()
            .map(|field| (field.subject, field.level))
            .collect(),
        answer
            .kinds
            .iter()
            .map(|kind| (kind.subject, kind.level))
            .collect(),
    ))
}

/// An organization with a tree eleven deep, with groups, *No access* and levels, and the people
/// who hold them (the administrator first).
async fn scenario(org: &Org<'_>) -> Result<Vec<Member>, Failure> {
    let admin = org.admin.clone();
    // A tree ten deep, with a branch inside its third project.
    let mut chain: Vec<ProjectId> = Vec::new();
    for level in 0..10 {
        let parent = chain.last().copied();
        chain.push(
            create(org, &admin, parent, &format!("DEEP{level}"))
                .await?
                .id,
        );
    }
    let branch = create(org, &admin, Some(chain[2]), "BRANCH").await?.id;
    let twig = create(org, &admin, Some(branch), "TWIG").await?.id;
    let lead = org.role("Section lead", &LEAD).await?;
    let looker = org.role("Looker", &VIEW).await?;
    let (p1, p2, p3, p4) = (
        org.person().await?,
        org.person().await?,
        org.person().await?,
        org.person().await?,
    );
    let crew = org.group("Crew").await?;
    org.add_to_group(crew, p3.id).await?;
    let sub = AppliesTo::ProjectAndSubProjects;
    let only = AppliesTo::Project;
    let who = |person: &Member| Subject::Person(person.id);
    // A lead low down, a looker everywhere with a No access in the middle, a group lead
    // with a No access of their own below, and a mix of both kinds of reach.
    org.assign(&admin, who(&p1), Given::Role(lead.id), Some(chain[3]), sub)
        .await?;
    org.assign(&admin, who(&p1), Given::NoAccess, Some(branch), sub)
        .await?;
    org.give_everywhere(&p2, looker.id).await?;
    org.forbid(&p2, chain[6]).await?;
    org.assign(
        &admin,
        Subject::Group(crew),
        Given::Role(lead.id),
        Some(chain[1]),
        sub,
    )
    .await?;
    org.forbid(&p3, chain[4]).await?;
    org.assign(&admin, who(&p3), Given::Role(lead.id), Some(chain[8]), only)
        .await?;
    org.assign(
        &admin,
        who(&p4),
        Given::Role(looker.id),
        Some(chain[0]),
        sub,
    )
    .await?;
    org.assign(&admin, who(&p4), Given::Role(lead.id), Some(chain[8]), only)
        .await?;
    org.assign(
        &admin,
        Subject::Group(crew),
        Given::NoAccess,
        Some(twig),
        sub,
    )
    .await?;
    // Levels set for a project, which the projects below inherit.
    for (project, field, kind) in [
        (chain[5], Field::Title, PeopleKind::ProjectMembers),
        (branch, Field::CostCenter, PeopleKind::AssignedTo),
    ] {
        api(org)
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: admin.session.clone(),
                    role: looker.id,
                    project: Some(project),
                    issue_type: None,
                    field: field.into(),
                    level: Some(FieldLevel::CanChange),
                },
            )
            .await?;
        api(org)
            .set_people_level(
                Rig::info(),
                SetPeopleLevel {
                    session_token: admin.session.clone(),
                    role: lead.id,
                    project: Some(project),
                    kind,
                    level: Some(PeopleLevel::RoleOnly),
                },
            )
            .await?;
    }
    Ok(vec![admin, p1, p2, p3, p4])
}

#[tokio::test]
async fn prj_02_the_list_the_preview_and_a_request_decide_alike_for_every_person_and_project()
-> TestResult {
    each_database("prj_02_engines_agree", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let people = scenario(&org).await?;
        let projects = rig.storage.project_tree().await?;
        let tree = Tree::of(&projects);
        let policy = rig.storage.policy_all().await?;
        let mut compared = 0;
        for person in &people {
            let own = rig.storage.policy_of(person.id).await?;
            let sees = own.visible(person.id, &tree);
            for project in &projects {
                let held = policy
                    .evaluate_at(person.id, &tree, Place::Project(project.id))
                    .ok_or("the project is not in the tree")?;
                let (permissions, fields, kinds) = decided(&org, person, project.id).await?;
                let known: Vec<Permission> = held
                    .permissions()
                    .iter()
                    .copied()
                    .filter(|permission| held.counts_in_this_place(*permission))
                    .collect();
                assert_eq!(
                    permissions, known,
                    "permissions of {} at {}",
                    person.id, project.short_name
                );
                assert_eq!(
                    fields,
                    Field::ALL
                        .iter()
                        .map(|f| ((*f).into(), held.field_level(*f)))
                        .collect::<Vec<_>>(),
                    "fields at {}",
                    project.short_name
                );
                assert_eq!(
                    kinds,
                    PeopleKind::ALL
                        .iter()
                        .map(|k| (*k, held.kind_level(*k)))
                        .collect::<Vec<_>>(),
                    "people at {}",
                    project.short_name
                );
                assert_eq!(sees.contains(&project.id), held.sees_project());
                compared += 1;
            }
        }
        assert_eq!(compared, people.len() * projects.len());
        Ok(())
    })
    .await
}

fn api<'o>(org: &'o Org<'_>) -> &'o dyn bb_app::auth::AuthApi {
    org.rig.api()
}
