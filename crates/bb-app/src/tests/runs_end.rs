//! Work that ends tells the person who asked for it (M3-F125, M3-F150, NTF-01): the end of a run of
//! many changes, a run that failed, and the renewal of issue IDs whose job died.

use bb_core::access::AppliesTo;
use bb_core::notification::NotificationFilter;
use bb_core::notification::NotificationKind;
use serde_json::json;

use super::many_changes::run_with_a_step_waiting;
use super::notifications::Crew;
use super::scenarios::token;
use super::world::{Outcome, World};
use crate::access::ProjectLabel;
use crate::notifications::{NotificationWhat, NotificationsQuery, notifications};
use crate::roles::{AssignRole, Given, Subject, assign_role};

async fn heard(world: &World) -> Result<Vec<crate::notifications::NotificationItem>, String> {
    Ok(notifications(
        &world.ctx(),
        NotificationsQuery {
            session_token: token(world),
            filter: NotificationFilter::All,
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map_err(|error| error.to_string())?
    .into_value()
    .items)
}

#[tokio::test]
async fn ntf_01_a_run_of_many_changes_that_is_done_tells_the_person_who_asked() -> Outcome {
    let world = World::new().await?;
    let run = run_with_a_step_waiting(&world).await?;
    assert!(heard(&world).await?.is_empty());
    while world.ctx().run_due_jobs().await? > 0 {}
    let told = heard(&world).await?;
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(told[0].kind, NotificationKind::RunEnded);
    let NotificationWhat::RunEnded { run: id, done } = &told[0].what else {
        return Err("no end of a run".into());
    };
    assert!(*done && *id == run.id.into());
    assert!(told[0].issue.is_none());
    Ok(())
}

#[tokio::test]
async fn ntf_01_a_run_that_failed_tells_the_person_who_asked() -> Outcome {
    let world = World::new().await?;
    run_with_a_step_waiting(&world).await?;
    // The store cannot save the progress for every start of the step: the job dies and the run
    // ends as failed.
    world
        .store
        .fail_many_changes_progress(usize::try_from(crate::jobs::JOB_ATTEMPTS_MAX)?);
    for _ in 0..crate::jobs::JOB_ATTEMPTS_MAX {
        world.clock.advance(60);
        world.ctx().run_due_jobs().await?;
    }
    let told = heard(&world).await?;
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(matches!(
        told[0].what,
        NotificationWhat::RunEnded { done: false, .. }
    ));
    Ok(())
}

#[tokio::test]
async fn ntf_01_the_person_who_stops_their_own_run_is_not_told_of_it() -> Outcome {
    use bb_core::many_changes::ManyChangeStatus;

    use crate::many_changes::{ManyChangeRunQuery, stop_many_changes_run};

    let world = World::new().await?;
    let run = run_with_a_step_waiting(&world).await?;
    stop_many_changes_run(
        &world.ctx(),
        ManyChangeRunQuery {
            session_token: token(&world),
            run: run.id,
        },
    )
    .await?;
    assert_eq!(
        world.store.lock().state.many_changes_runs[0].status,
        ManyChangeStatus::Stopped
    );
    assert!(heard(&world).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn ntf_01_a_renewal_of_issue_ids_that_died_tells_the_person_who_changed_the_mode() -> Outcome
{
    let crew = Crew::new().await?;
    let payload =
        json!({"project": crew.project.to_string(), "by": crew.carol.person.id.to_string()});
    crew.world.ctx().renewal_gave_up(&payload).await?;
    let told = crew.heard(&crew.carol.session_token).await?;
    assert_eq!(told.len(), 1, "{told:?}");
    // The notification names the project, as the person may name it.
    assert!(
        matches!(
            &told[0].what,
            NotificationWhat::IssueIdsFailed { project: ProjectLabel::Visible { id, .. } }
                if *id == crew.project
        ),
        "{told:?}"
    );
    assert!(crew.heard(&crew.bob.session_token).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn m4_f144_a_renewal_that_died_does_not_name_a_project_the_person_cannot_see_any_more()
-> Outcome {
    let crew = Crew::new().await?;
    let payload =
        json!({"project": crew.project.to_string(), "by": crew.carol.person.id.to_string()});
    crew.world.ctx().renewal_gave_up(&payload).await?;
    // Carol loses access to the project before she reads the notification.
    assign_role(
        &crew.world.ctx(),
        AssignRole {
            session_token: token(&crew.world),
            subject: Subject::Person(crew.carol.person.id),
            given: Given::NoAccess,
            project: Some(crew.project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    let told = crew.heard(&crew.carol.session_token).await?;
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(matches!(
        &told[0].what,
        NotificationWhat::IssueIdsFailed {
            project: ProjectLabel::Hidden
        }
    ));
    Ok(())
}
