//! Standing rules, proposals and what an agent may change (AI-31, AI-32, ADR-0029).

use bb_core::ErrorCode;
use bb_core::agent_records::{GivingState, ProposalState};
use bb_core::agent_work::{HandOverPower, RuleField};
use bb_core::id::{AgentId, IssueId, IssueTypeId, ProjectId};
use serde_json::json;

use super::agent_work::{Ground, ground, plain_hand_over, query};
use super::agents::refused;
use super::scenarios::{Added, measured, text};
use super::scenarios_issues::an_issue;
use super::scenarios_searches::save;
use super::world::{Outcome, World};
use crate::agent_work::{
    ChangeStandingRule, CreateStandingRule, DecideProposal, ProposeChange, StandingRuleView,
    change_standing_rule, create_standing_rule, decide_proposal, hand_over, propose_change,
};
use crate::auth::{AppError, Started};
use crate::issues::{ChangeIssue, IssueTypesQuery, change_issue, issue, issue_types};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};
use bb_core::settings::SettingValue;

fn change(key: &str, issue_id: IssueId, version: i64) -> ChangeIssue {
    ChangeIssue {
        session_token: key.to_owned(),
        issue: issue_id.to_string(),
        version,
        issue_type: None,
        title: None,
        status: None,
        priority: None,
        estimated_time: None,
        time_left: None,
        assigned_to: None,
        fields: std::collections::BTreeMap::default(),
    }
}

async fn version_of(world: &World, key: &str, issue_id: IssueId) -> Result<i64, AppError> {
    Ok(issue(&world.ctx(), query(key, issue_id))
        .await?
        .into_value()
        .issue
        .version)
}

#[tokio::test]
async fn ai_32_a_standing_rule_never_changes_people_fields() -> Outcome {
    let world = World::new().await?;
    let Ground {
        team,
        agent,
        key,
        issue: issue_id,
        ..
    } = ground(&world).await?;
    // Handed over with nothing ticked: the agent reads and reports, and changes nothing.
    hand_over(
        &world.ctx(),
        plain_hand_over(&team.session_token, agent, &[issue_id]),
    )
    .await?;
    let version = version_of(&world, &key, issue_id).await?;
    let mut title = change(&key, issue_id, version);
    title.title = Some("Changed by an agent".to_owned());
    assert!(refused(
        &change_issue(&world.ctx(), title).await.unwrap_err(),
        ErrorCode::Forbidden
    ));
    Ok(())
}

#[tokio::test]
async fn ai_31_ticked_fields_are_changed_and_people_and_the_status_never_unasked() -> Outcome {
    let world = World::new().await?;
    let Ground {
        team,
        agent,
        key,
        issue: issue_id,
        ..
    } = ground(&world).await?;
    // A hand-over cannot even name a member that holds people, or one that is not a member.
    for refused_field in ["assigned-to", "reported-by", "followers", "no-such-member"] {
        let mut asked = plain_hand_over(&team.session_token, agent, &[issue_id]);
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = vec!["title".to_owned(), refused_field.to_owned()];
        assert!(
            refused(
                &hand_over(&world.ctx(), asked).await.unwrap_err(),
                ErrorCode::InvalidRequest
            ),
            "{refused_field}"
        );
    }
    let mut asked = plain_hand_over(&team.session_token, agent, &[issue_id]);
    asked.powers = vec![HandOverPower::ChangeFields];
    asked.fields = vec!["title".to_owned()];
    hand_over(&world.ctx(), asked).await?;
    let version = version_of(&world, &key, issue_id).await?;
    let mut title = change(&key, issue_id, version);
    title.title = Some("Fixed title".to_owned());
    change_issue(&world.ctx(), title).await?;
    // A member that holds people is refused: the hand-over could not name it.
    let version = version_of(&world, &key, issue_id).await?;
    let mut people = change(&key, issue_id, version);
    people.assigned_to = Some(vec![team.person.id]);
    assert!(refused(
        &change_issue(&world.ctx(), people).await.unwrap_err(),
        ErrorCode::Forbidden
    ));
    // The status was not ticked.
    let mut priority = change(&key, issue_id, version);
    priority.priority = Some(bb_core::issue::Priority::High);
    assert!(refused(
        &change_issue(&world.ctx(), priority).await.unwrap_err(),
        ErrorCode::Forbidden
    ));
    Ok(())
}

/// A project, its team member with an agent, an issue and a saved search over the project.
pub(super) struct RuleWorld {
    pub(super) project: ProjectId,
    pub(super) team: Started,
    pub(super) agent: AgentId,
    pub(super) key: String,
    pub(super) issue: IssueId,
    pub(super) search: bb_core::id::SavedSearchId,
}

pub(super) async fn rule_world(world: &World) -> Result<RuleWorld, AppError> {
    let Ground {
        project,
        team,
        agent,
        key,
        issue,
    } = ground(world).await?;
    let search = save(world, &team.session_token, "Everything", "").await?.id;
    Ok(RuleWorld {
        project,
        team,
        agent,
        key,
        issue,
        search,
    })
}

fn rule_request(given: &RuleWorld, fields: Vec<RuleField>) -> CreateStandingRule {
    CreateStandingRule {
        session_token: given.team.session_token.clone(),
        agent: given.agent,
        search: given.search,
        project: given.project,
        fields,
        creates_in: None,
        days: None,
    }
}

pub(super) async fn made(world: &World, given: &RuleWorld) -> Result<StandingRuleView, AppError> {
    Ok(
        create_standing_rule(&world.ctx(), rule_request(given, Vec::new()))
            .await?
            .into_value(),
    )
}

/// What the saved search finds: every issue the store has (the in-memory store does not search).
fn found_by_search(world: &World) {
    let mut shared = world.store.lock();
    let hits = shared
        .state
        .issues
        .iter()
        .cloned()
        .map(|issue| crate::search::SearchHit { issue })
        .collect();
    shared.search_hits = hits;
}

pub(super) fn payload_of(rule: &StandingRuleView) -> serde_json::Value {
    json!({ "rule": rule.id.to_string(), "ends_at": rule.ends_at.to_string() })
}

#[tokio::test]
async fn ai_32_a_standing_rule_runs_out_after_30_days_and_never_after_more_than_90() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let rule = made(&world, &given).await?;
    let thirty = rule.ends_at.as_second() - world.ctx().now().as_second();
    assert_eq!(thirty, 30 * 24 * 3600);
    let mut too_long = rule_request(&given, Vec::new());
    too_long.days = Some(91);
    assert!(refused(
        &create_standing_rule(&world.ctx(), too_long)
            .await
            .unwrap_err(),
        ErrorCode::InvalidRequest
    ));
    // The job of an end that has not come leaves the rule as it is.
    world.ctx().run_rule_runs_out(&payload_of(&rule)).await?;
    assert_eq!(
        world.store.lock().state.work.rules[0].state,
        GivingState::Open
    );
    // Renewing queues a new end; the job of the old end finds another end and does nothing.
    let stale = payload_of(&rule);
    let renewed = change_standing_rule(
        &world.ctx(),
        ChangeStandingRule {
            session_token: given.team.session_token.clone(),
            rule: rule.id,
            renew: true,
            days: Some(90),
            version: rule.version,
        },
    )
    .await?
    .into_value();
    world.store.lock().state.work.rules[0].ends_at = world.ctx().now();
    world.ctx().run_rule_runs_out(&stale).await?;
    assert_eq!(
        world.store.lock().state.work.rules[0].state,
        GivingState::Open
    );
    // At its end the rule ends.
    let current = StandingRuleView {
        ends_at: world.ctx().now(),
        ..renewed
    };
    world.ctx().run_rule_runs_out(&payload_of(&current)).await?;
    assert_eq!(
        world.store.lock().state.work.rules[0].state,
        GivingState::Ended
    );
    Ok(())
}

#[tokio::test]
async fn ai_32_every_new_match_is_written_down() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let rule = made(&world, &given).await?;
    let job = json!({ "project": given.project.to_string() });
    found_by_search(&world);
    world.ctx().run_rule_matches(&job).await?;
    let matches = world.store.lock().state.work.matches.clone();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].issue, given.issue);
    assert_eq!(matches[0].rule, rule.id);
    // Written once: the same issue is not new the second time, a new one is.
    world.ctx().run_rule_matches(&job).await?;
    assert_eq!(world.store.lock().state.work.matches.len(), 1);
    an_issue(&world, given.project).await?;
    found_by_search(&world);
    world.ctx().run_rule_matches(&job).await?;
    assert_eq!(world.store.lock().state.work.matches.len(), 2);
    // The agent now sees the issues the rule found, and only those.
    assert!(
        issue(&world.ctx(), query(&given.key, given.issue))
            .await
            .is_ok()
    );
    Ok(())
}

async fn type_of(world: &World, given: &RuleWorld) -> Result<IssueTypeId, AppError> {
    Ok(
        issue(&world.ctx(), query(&given.team.session_token, given.issue))
            .await?
            .into_value()
            .issue
            .issue_type
            .id,
    )
}

async fn another_type(world: &World, current: IssueTypeId) -> Result<IssueTypeId, AppError> {
    let page = issue_types(
        &world.ctx(),
        IssueTypesQuery {
            project: None,
            session_token: super::scenarios::token(world),
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    page.items
        .into_iter()
        .map(|item| item.id)
        .find(|id| *id != current)
        .ok_or_else(|| AppError::Internal("there is only one issue type".to_owned()))
}

#[tokio::test]
async fn ai_32_a_proposal_of_a_standing_rule_changes_nothing_until_a_person_applies_it() -> Outcome
{
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let rule = made(&world, &given).await?;
    found_by_search(&world);
    world
        .ctx()
        .run_rule_matches(&json!({ "project": given.project.to_string() }))
        .await?;
    let before = issue(&world.ctx(), query(&given.team.session_token, given.issue))
        .await?
        .into_value()
        .issue
        .issue_type
        .id;
    let to = another_type(&world, before).await?;
    let proposed = propose_change(
        &world.ctx(),
        ProposeChange {
            session_token: given.key.clone(),
            rule: rule.id,
            issue: given.issue.to_string(),
            field: RuleField::Type,
            after: json!(to.to_string()),
        },
    )
    .await?
    .into_value();
    assert_eq!(
        type_of(&world, &given).await?,
        before,
        "a proposal changes nothing"
    );
    // Somebody else cannot decide it; the person who made the rule applies it.
    let stranger = decide_proposal(
        &world.ctx(),
        DecideProposal {
            session_token: super::scenarios::token(&world),
            proposal: proposed.id,
            apply: true,
        },
    )
    .await;
    assert!(refused(&stranger.unwrap_err(), ErrorCode::NotFound));
    let applied = decide_proposal(
        &world.ctx(),
        DecideProposal {
            session_token: given.team.session_token.clone(),
            proposal: proposed.id,
            apply: true,
        },
    )
    .await?
    .into_value();
    assert_eq!(applied.state, ProposalState::Applied);
    assert_eq!(type_of(&world, &given).await?, to);
    let again = decide_proposal(
        &world.ctx(),
        DecideProposal {
            session_token: given.team.session_token.clone(),
            proposal: proposed.id,
            apply: false,
        },
    )
    .await;
    assert!(refused(&again.unwrap_err(), ErrorCode::AgentWorkNotReady));
    Ok(())
}

pub(crate) async fn create_standing_rule_scenario(world: &World) -> Added {
    let given = rule_world(world).await.map_err(text)?;
    measured(
        &world.store,
        create_standing_rule(&world.ctx(), rule_request(&given, Vec::new())),
    )
    .await
}

pub(crate) async fn change_standing_rule_scenario(world: &World) -> Added {
    let given = rule_world(world).await.map_err(text)?;
    let rule = made(world, &given).await.map_err(text)?;
    measured(
        &world.store,
        change_standing_rule(
            &world.ctx(),
            ChangeStandingRule {
                session_token: given.team.session_token.clone(),
                rule: rule.id,
                renew: false,
                days: None,
                version: rule.version,
            },
        ),
    )
    .await
}

pub(crate) async fn propose_change_scenario(world: &World) -> Added {
    let given = rule_world(world).await.map_err(text)?;
    let rule = made(world, &given).await.map_err(text)?;
    found_by_search(world);
    world
        .ctx()
        .run_rule_matches(&json!({ "project": given.project.to_string() }))
        .await
        .map_err(text)?;
    measured(
        &world.store,
        propose_change(
            &world.ctx(),
            ProposeChange {
                session_token: given.key.clone(),
                rule: rule.id,
                issue: given.issue.to_string(),
                field: RuleField::Labels,
                after: json!("blocker"),
            },
        ),
    )
    .await
}

pub(crate) async fn decide_proposal_scenario(world: &World) -> Added {
    let given = rule_world(world).await.map_err(text)?;
    let rule = made(world, &given).await.map_err(text)?;
    found_by_search(world);
    world
        .ctx()
        .run_rule_matches(&json!({ "project": given.project.to_string() }))
        .await
        .map_err(text)?;
    let proposed = propose_change(
        &world.ctx(),
        ProposeChange {
            session_token: given.key.clone(),
            rule: rule.id,
            issue: given.issue.to_string(),
            field: RuleField::Labels,
            after: json!("blocker"),
        },
    )
    .await
    .map_err(text)?
    .into_value();
    measured(
        &world.store,
        decide_proposal(
            &world.ctx(),
            DecideProposal {
                session_token: given.team.session_token.clone(),
                proposal: proposed.id,
                apply: false,
            },
        ),
    )
    .await
}

#[tokio::test]
async fn ai_32_a_change_to_an_issue_where_a_rule_works_queues_the_job_of_the_matches() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    made(&world, &given).await?;
    let waiting = |world: &World| {
        world
            .store
            .lock()
            .state
            .jobs
            .iter()
            .filter(|job| job.kind.as_str() == "agents.rule-matches")
            .count()
    };
    let before = waiting(&world);
    an_issue(&world, given.project).await?;
    world.ctx().catch_up_notifications().await?;
    assert!(waiting(&world) > before, "the reader queued the job");
    // Many changes of one round queue it once for the project.
    let after_one = waiting(&world);
    an_issue(&world, given.project).await?;
    an_issue(&world, given.project).await?;
    world.ctx().catch_up_notifications().await?;
    assert_eq!(waiting(&world), after_one + 1);
    Ok(())
}

#[tokio::test]
async fn ai_32_applying_a_proposed_tag_puts_it_on_the_issue_as_the_person() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let rule = made(&world, &given).await?;
    found_by_search(&world);
    world
        .ctx()
        .run_rule_matches(&json!({ "project": given.project.to_string() }))
        .await?;
    let proposed = propose_change(
        &world.ctx(),
        ProposeChange {
            session_token: given.key.clone(),
            rule: rule.id,
            issue: given.issue.to_string(),
            field: RuleField::Tags,
            after: json!("needs-triage"),
        },
    )
    .await?
    .into_value();
    assert!(world.store.lock().state.issue_tags.is_empty());
    decide_proposal(
        &world.ctx(),
        DecideProposal {
            session_token: given.team.session_token.clone(),
            proposal: proposed.id,
            apply: true,
        },
    )
    .await?;
    assert_eq!(world.store.lock().state.issue_tags.len(), 1);
    Ok(())
}

#[tokio::test]
async fn ai_32_a_rule_proposes_only_for_an_issue_it_found() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let rule = made(&world, &given).await?;
    // The job of the matches did not run: the rule found nothing, so there is nothing to propose.
    let to = another_type(&world, type_of(&world, &given).await?).await?;
    let proposal = || ProposeChange {
        session_token: given.key.clone(),
        rule: rule.id,
        issue: given.issue.to_string(),
        field: RuleField::Type,
        after: json!(to.to_string()),
    };
    let none = propose_change(&world.ctx(), proposal()).await;
    assert!(refused(&none.unwrap_err(), ErrorCode::NotFound));
    Ok(())
}

async fn switch_agents(world: &World, scope: SettingsScope, on: bool) -> Outcome {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: super::scenarios::token(world),
            scope,
            key: "agents.enabled".to_owned(),
            value: SettingValue::Bool(on),
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn ai_32_a_rule_is_made_and_renewed_only_where_agents_are_on() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let rule = made(&world, &given).await?;
    let renew = |version: i64| ChangeStandingRule {
        session_token: given.team.session_token.clone(),
        rule: rule.id,
        renew: true,
        days: None,
        version,
    };
    // Off for the project the rule works in: no renewal, but it can be stopped.
    switch_agents(&world, SettingsScope::Project(given.project), false).await?;
    let refusal = change_standing_rule(&world.ctx(), renew(rule.version)).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    switch_agents(&world, SettingsScope::Project(given.project), true).await?;
    let later = change_standing_rule(&world.ctx(), renew(rule.version))
        .await?
        .into_value();
    assert_eq!(later.state, GivingState::Open);
    // A rule that may create issues in a project where agents are off is not made.
    let other = super::scenarios::a_project(&world, "OTHER").await?;
    let team_role =
        super::workflows_rules::starting_role(&world, bb_policy::StartingRole::TeamMember).await?;
    super::workflows_rules::assign(&world, &given.team, team_role, other).await?;
    let mut creating = rule_request(&given, Vec::new());
    creating.creates_in = Some(other);
    create_standing_rule(&world.ctx(), creating).await?;
    switch_agents(&world, SettingsScope::Project(other), false).await?;
    let mut off = rule_request(&given, Vec::new());
    off.creates_in = Some(other);
    let refusal = create_standing_rule(&world.ctx(), off).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    Ok(())
}

#[tokio::test]
async fn ai_32_a_rule_that_creates_in_a_project_gives_top_level_issues_with_no_members() -> Outcome
{
    use crate::issues::{CreateIssue, create_issue};
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let mut request = rule_request(&given, Vec::new());
    request.creates_in = Some(given.project);
    create_standing_rule(&world.ctx(), request).await?;
    let issue_type = super::scenarios_issues::a_type(&world)
        .await
        .map_err(AppError::Internal)?;
    let input = |priority| CreateIssue {
        session_token: given.key.clone(),
        project: given.project.to_string(),
        issue_type,
        title: "Found by the rule".to_owned(),
        description: None,
        status: None,
        priority,
        assigned_to: None,
        estimated_time: None,
        fields: std::collections::BTreeMap::new(),
        parent: None,
    };
    create_issue(&world.ctx(), input(None)).await?;
    // A member would need a hand-over that names it; a rule gives none.
    let refusal = create_issue(&world.ctx(), input(Some(bb_core::issue::Priority::High))).await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    Ok(())
}
