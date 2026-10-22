//! The places that take a label or a type by what a person asked for apply the one rule of what
//! they may know (`offers/known.rs`, SEC-03): a label of a label group that a person may not know
//! is `resource.not_found` when they put it on an issue or name it in *Change many issues*, and
//! the statuses of a project name only what reaches that project.

use bb_core::ErrorCode;
use bb_core::issue::StatusGroup;
use bb_core::workflow::starter::OPEN;
use bb_core::workflow::{ChangeDraft, StatusDraft, StatusRules};

use super::many_changes::preview;
use super::offers::{Groups, groups, lead_of};
use super::scenarios_issues::an_issue;
use super::scenarios_workflows::{a_workflow, published_with};
use super::world::Outcome;
use crate::auth::AppError;
use crate::many_changes::ManyChangeAction;
use crate::tags::{AddLabel, add_label};
use crate::workflows::{ProjectStatusesQuery, project_statuses};

#[tokio::test]
async fn sec_03_putting_on_a_label_of_a_group_the_person_may_not_know_is_not_found() -> Outcome {
    let Groups {
        world,
        other,
        login,
        ..
    } = groups().await?;
    let sam = lead_of(&world, "sam@example.test", other).await?;
    let issue_id = an_issue(&world, other).await?;
    let put = add_label(
        &world.ctx(),
        AddLabel {
            session_token: sam.session_token.clone(),
            issue: issue_id.to_string(),
            label: login,
        },
    )
    .await;
    assert_eq!(put.err(), Some(AppError::Refused(ErrorCode::NotFound)));
    Ok(())
}

#[tokio::test]
async fn sec_03_change_many_issues_does_not_take_a_label_of_a_group_the_person_may_not_know()
-> Outcome {
    let Groups {
        world,
        other,
        login,
        ..
    } = groups().await?;
    let sam = lead_of(&world, "sam@example.test", other).await?;
    let issue_id = an_issue(&world, other).await?;
    let action = ManyChangeAction::AddLabel { label: login };
    let asked = preview(&world, &sam.session_token, &[issue_id], &action).await;
    assert_eq!(asked.err(), Some(AppError::Refused(ErrorCode::NotFound)));
    Ok(())
}

#[tokio::test]
async fn sec_03_the_statuses_of_a_project_leave_out_those_of_a_project_that_does_not_reach_it()
-> Outcome {
    let g = groups().await?;
    // ACME has a workflow with a status that OTHER does not reach.
    let editor = a_workflow(&g.world, "ACME").await?;
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
        sent.changes.push(ChangeDraft {
            from: "Parked".to_owned(),
            to: OPEN.to_owned(),
            button: "Unpark".to_owned(),
            rules: StatusRules::default(),
        });
    })
    .await?;
    let sam = lead_of(&g.world, "sam@example.test", g.other).await?;
    let names = async |who: &str, project: &str| -> Result<Vec<String>, AppError> {
        Ok(project_statuses(
            &g.world.ctx(),
            ProjectStatusesQuery {
                session_token: who.to_owned(),
                project: project.to_owned(),
            },
        )
        .await?
        .into_value()
        .changes
        .into_iter()
        .map(|change| change.to.name)
        .collect())
    };
    assert!(
        names(&g.admin, "ACME")
            .await?
            .contains(&"Parked".to_owned())
    );
    assert!(
        !names(&sam.session_token, "OTHER")
            .await?
            .contains(&"Parked".to_owned())
    );
    Ok(())
}
