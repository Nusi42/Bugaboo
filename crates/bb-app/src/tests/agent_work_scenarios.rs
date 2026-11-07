//! The scenarios of the table of events for agent work: each says how many events its call
//! appended.

use bb_core::agent_work::{CheckResult, ReviewAnswer};
use bb_policy::StartingRole;

use super::agent_work::{Handed, handed, plain_hand_over};
use super::agent_work_review::{check, ready, report, review, show};
use super::agents::{an_agent, key_of};
use super::scenarios::{Added, a_project, measured, text, token};
use super::scenarios_issues::an_issue;
use super::workflows_rules::member;
use super::world::World;
use crate::agent_work::{
    AnswerQuestion, ApprovePlan, AskQuestion, LookAtWork, answer_question, approve_plan,
    ask_question, attach_result, end_attempt, hand_over, look_at_work, report_attempt, review_work,
};

pub(crate) async fn hand_over_scenario(world: &World) -> Added {
    let project = a_project(world, "WEB").await?;
    let agent = an_agent(world, &token(world), "Fixer")
        .await
        .map_err(text)?;
    let issue = an_issue(world, project).await?;
    measured(
        &world.store,
        hand_over(
            &world.ctx(),
            plain_hand_over(&token(world), agent, &[issue]),
        ),
    )
    .await
}

// ---- More scenarios of the table of events ----

async fn started(world: &World) -> Result<Handed, String> {
    let given = handed(world).await.map_err(text)?;
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5))
        .await
        .map_err(text)?;
    Ok(given)
}

pub(crate) async fn report_attempt_scenario(world: &World) -> Added {
    let given = handed(world).await.map_err(text)?;
    measured(
        &world.store,
        report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)),
    )
    .await
}

pub(crate) async fn ask_question_scenario(world: &World) -> Added {
    let given = started(world).await?;
    measured(
        &world.store,
        ask_question(
            &world.ctx(),
            AskQuestion {
                session_token: given.key,
                issue: given.issue.to_string(),
                text: "Which browser?".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn attach_result_scenario(world: &World) -> Added {
    let given = started(world).await?;
    measured(
        &world.store,
        attach_result(
            &world.ctx(),
            check(&given.key, given.issue, CheckResult::Passed),
        ),
    )
    .await
}

pub(crate) async fn end_attempt_scenario(world: &World) -> Added {
    let given = started(world).await?;
    measured(
        &world.store,
        end_attempt(&world.ctx(), ready(&given.key, given.issue)),
    )
    .await
}

pub(crate) async fn answer_question_scenario(world: &World) -> Added {
    let given = started(world).await?;
    let asked = ask_question(
        &world.ctx(),
        AskQuestion {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            text: "Which browser?".to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value();
    measured(
        &world.store,
        answer_question(
            &world.ctx(),
            AnswerQuestion {
                session_token: given.team.session_token.clone(),
                work: asked.id,
                question: asked.questions[0].id,
                reply: "Firefox".to_owned(),
                version: asked.version,
            },
        ),
    )
    .await
}

pub(crate) async fn approve_plan_scenario(world: &World) -> Added {
    let project = a_project(world, "WEB").await?;
    let team = member(
        world,
        "team@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await
    .map_err(text)?;
    let agent = an_agent(world, &team.session_token, "Fixer")
        .await
        .map_err(text)?;
    let key = key_of(world, &team.session_token, agent)
        .await
        .map_err(text)?;
    let issue_id = an_issue(world, project).await?;
    let mut asked = plain_hand_over(&team.session_token, agent, &[issue_id]);
    asked.plan_needs_approval = true;
    hand_over(&world.ctx(), asked).await.map_err(text)?;
    let waiting = report_attempt(&world.ctx(), report(&key, issue_id, Some("Fix it"), 1))
        .await
        .map_err(text)?
        .into_value();
    measured(
        &world.store,
        approve_plan(
            &world.ctx(),
            ApprovePlan {
                session_token: team.session_token,
                work: waiting.id,
                version: waiting.version,
                note: None,
            },
        ),
    )
    .await
}

pub(crate) async fn review_work_scenario(world: &World) -> Added {
    let given = started(world).await?;
    end_attempt(&world.ctx(), ready(&given.key, given.issue))
        .await
        .map_err(text)?;
    let work = show(world, &given).await.map_err(text)?;
    measured(
        &world.store,
        review_work(
            &world.ctx(),
            review(&given.team.session_token, &work, ReviewAnswer::Accept),
        ),
    )
    .await
}

pub(crate) async fn look_at_work_scenario(world: &World) -> Added {
    let given = handed(world).await.map_err(text)?;
    let work = show(world, &given).await.map_err(text)?;
    measured(
        &world.store,
        look_at_work(
            &world.ctx(),
            LookAtWork {
                session_token: given.team.session_token,
                work: work.id,
            },
        ),
    )
    .await
}
