//! The two reads of what the engine decides from agree (M4-F137, SEC-03): `policy_inputs` for one
//! person at one place, and the rows, roles and levels of the person read once
//! (`person_policy`) with the engine choosing what applies at each place
//! (`Policy::inputs_from_rows`), which is how a request decides every place it asks about. Rows on
//! the organization, in a project, in a sub-project, through a group, and levels set for a
//! project must give the same decision both ways. Every flow runs on SQLite and PostgreSQL.

use bb_app::access::Place;
use bb_app::access::{Permission, PolicyStore as _};
use bb_app::fields::CreateField;
use bb_app::roles::{Given, SetFieldLevel, Subject};
use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::fields::Field;
use bb_core::own_fields::{FieldFor, FieldType, Rules, Scope};
use bb_policy::evaluate;
use bb_testing::{TestResult, each_database};

use super::policy_rig::{Org, Run};
use super::rig::Rig;

/// An own field: a role anywhere makes every own field known to the decision, which the read for
/// one place does only where a role reaches.
async fn customer_field(rig: &Rig, org: &Org<'_>) -> Run<()> {
    rig.api()
        .create_field(
            Rig::info(),
            CreateField {
                session_token: org.admin.session.clone(),
                name: "Customer".to_owned(),
                help: None,
                kind: FieldType::Text,
                applies_to: FieldFor::Issues,
                rules: Rules::default(),
                default_value: None,
                look: None,
                never_to_assistant: false,
                scope: Scope::default(),
                choices: Vec::new(),
            },
        )
        .await?;
    Ok(())
}

#[tokio::test]
async fn sec_03_the_rights_read_once_decide_every_place_as_the_rights_read_for_each() -> TestResult
{
    each_database("sec_03_policy_reads", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let acme = org.project(None, "ACME").await?.id;
        let web = org.project(Some(acme), "WEB").await?.id;
        let other = org.project(None, "OTHER").await?.id;
        customer_field(&rig, &org).await?;
        let worker = org
            .role(
                "Worker",
                &[
                    Permission::SeeProjects,
                    Permission::SeeIssues,
                    Permission::SeeStatus,
                ],
            )
            .await?;
        for (project, level) in [
            (None, FieldLevel::CanSee),
            (Some(acme), FieldLevel::CanChange),
        ] {
            rig.api()
                .set_field_level(
                    Rig::info(),
                    SetFieldLevel {
                        session_token: org.admin.session.clone(),
                        role: worker.id,
                        project,
                        issue_type: None,
                        field: Field::Priority.into(),
                        level: Some(level),
                    },
                )
                .await?;
        }
        let (bob, cy, dee, eve) = (
            org.person().await?,
            org.person().await?,
            org.person().await?,
            org.person().await?,
        );
        let everywhere = |person: &super::policy_rig::Member| (Subject::Person(person.id), None);
        let (subject, project) = everywhere(&bob);
        org.assign(
            &org.admin,
            subject,
            Given::Role(worker.id),
            project,
            AppliesTo::Organization,
        )
        .await?;
        org.give(&cy, worker.id, acme).await?;
        let crew = org.group("Crew").await?;
        org.add_to_group(crew, dee.id).await?;
        org.assign(
            &org.admin,
            Subject::Group(crew),
            Given::Role(worker.id),
            Some(other),
            AppliesTo::Project,
        )
        .await?;

        let places = [
            Place::Organization,
            Place::Project(acme),
            Place::Project(web),
            Place::Project(other),
        ];
        for person in [&org.admin, &bob, &cy, &dee, &eve] {
            let policy = rig.storage.person_policy(person.id).await?;
            for place in places {
                let chain = match place {
                    Place::Organization => Vec::new(),
                    Place::Project(project) => rig
                        .storage
                        .project_chain(project)
                        .await?
                        .ok_or("no such project")?,
                };
                let once = evaluate(&policy.inputs_from_rows(
                    person.id,
                    &policy.rows_of(person.id),
                    place,
                    chain,
                ));
                let each = evaluate(
                    &rig.storage
                        .policy_inputs(person.id, place)
                        .await?
                        .ok_or("no such project")?,
                );
                assert_eq!(once, each, "{:?} at {place:?}", person.id);
            }
        }
        Run::Ok(())
    })
    .await
}
