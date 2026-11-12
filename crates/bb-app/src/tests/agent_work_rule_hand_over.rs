//! Standing rules that hand over what they find, and rules that never run out (AI-32, AI-35,
//! ADR-0029, amendment "Rules that hand over", WP-14).

use bb_core::ErrorCode;
use bb_core::agent_records::{GivingState, HandOver as HandOverRecord};
use bb_core::agent_work::{AgentWorkState, CheckResult, HandOverPower, ReviewAnswer, RuleWay};
use bb_core::id::{IssueId, ProjectId};
use bb_core::settings::SettingValue;
use serde_json::json;

use super::agent_limits::limit;
use super::agent_work::{Handed, plain_hand_over, report_attempt};
use super::agent_work_review::{check, ready, report, review, show};
use super::agent_work_rules::{RuleWorld, found_by_search, rule_request, rule_world};
use super::agent_work_runs::allow_accepting;
use super::agents::refused;
use super::scenarios::token;
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::agent_work::{
    ChangeStandingRule, CreateStandingRule, MyAgentWorkQuery, StandingRuleView, attach_result,
    change_standing_rule, create_standing_rule, end_attempt, hand_over, my_agent_work, review_work,
};
use crate::auth::AppError;
use crate::settings::{ChangeSetting, SettingsScope, change_setting};
use bb_core::agent::LimitKind;

/// A rule that hands over, with a title the agent may change.
fn handing(given: &RuleWorld) -> CreateStandingRule {
    CreateStandingRule {
        way: RuleWay::HandOver,
        powers: vec![HandOverPower::ChangeFields],
        fields: vec!["title".to_owned()],
        ..rule_request(given)
    }
}

async fn rule_that_hands_over(
    world: &World,
    given: &RuleWorld,
) -> Result<StandingRuleView, AppError> {
    Ok(create_standing_rule(&world.ctx(), handing(given))
        .await?
        .into_value())
}

/// Lets the job of the matches of the project run, after the search found everything.
async fn look(world: &World, project: ProjectId) -> Outcome {
    found_by_search(world);
    world
        .ctx()
        .run_rule_matches(&json!({ "project": project.to_string() }))
        .await?;
    Ok(())
}

fn hand_overs(world: &World) -> Vec<HandOverRecord> {
    world.store.lock().state.work.hand_overs.clone()
}

fn open_work_on(world: &World, issue: IssueId) -> usize {
    world
        .store
        .lock()
        .state
        .work
        .works
        .iter()
        .filter(|work| work.issue == issue && !work.closed && work.state.is_open())
        .count()
}

/// The agent of the rule, as `Handed` for the helpers of the review tests.
fn handed_of(given: &RuleWorld, world: &World, issue: IssueId) -> Handed {
    let record = hand_overs(world)
        .into_iter()
        .find(|found| {
            world
                .store
                .lock()
                .state
                .work
                .works
                .iter()
                .any(|work| work.hand_over == found.id && work.issue == issue)
        })
        .expect("a hand-over of the issue");
    Handed {
        project: given.project,
        team: given.team.clone(),
        agent: given.agent,
        key: given.key.clone(),
        issue,
        hand_over: crate::agent_work::HandOverView {
            id: record.id,
            agent: record.agent,
            issues: Vec::new(),
            ends_at: record.ends_at,
            state: record.state,
            version: record.version,
        },
    }
}

async fn allow_never(world: &World, on: bool) -> Result<(), AppError> {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(world),
            scope: SettingsScope::Organization,
            key: "agents.rules-never-run-out".to_owned(),
            value: SettingValue::Bool(on),
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn m7_wp14_a_rule_hands_over_what_its_search_finds() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let rule = rule_that_hands_over(&world, &given).await?;
    assert!(
        hand_overs(&world).is_empty(),
        "nothing before the rule looks"
    );
    look(&world, given.project).await?;
    // The issue it found is handed over to the agent of the rule, by the person of the rule, with
    // the powers of the rule and the usual end of a hand-over.
    let handed = hand_overs(&world);
    assert_eq!(handed.len(), 1);
    assert_eq!(handed[0].agent, given.agent);
    assert_eq!(handed[0].person, given.team.person.id);
    assert_eq!(handed[0].powers, vec![HandOverPower::ChangeFields]);
    assert_eq!(handed[0].fields, vec!["title".to_owned()]);
    assert_eq!(handed[0].state, GivingState::Open);
    assert!(!handed[0].plan_needs_approval && !handed[0].accept_by_itself);
    assert_eq!(open_work_on(&world, given.issue), 1);
    let work = show(&world, &handed_of(&given, &world, given.issue)).await?;
    assert_eq!(work.state, AgentWorkState::WaitingForTheAgent);
    // Its event names the rule, and the issue is written down as found.
    assert!(world.store.lock().events.iter().any(|event| {
        event.changes.iter().any(|change| {
            change.field == "rule" && change.after == Some(json!(rule.id.to_string()))
        })
    }));
    assert_eq!(world.store.lock().state.work.matches.len(), 1);
    // An issue that comes under the rule later is handed over too.
    let later = an_issue(&world, given.project).await?;
    look(&world, given.project).await?;
    assert_eq!(hand_overs(&world).len(), 2);
    assert_eq!(open_work_on(&world, later), 1);
    Ok(())
}

#[tokio::test]
async fn m7_wp14_one_hand_over_for_each_found_issue_and_never_again() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    rule_that_hands_over(&world, &given).await?;
    look(&world, given.project).await?;
    look(&world, given.project).await?;
    assert_eq!(hand_overs(&world).len(), 1, "not while one is open");
    // After the hand-over ended, the rule does not hand the issue over again, even though it still
    // matches (it must not trigger itself through the changes of its own agent).
    {
        let mut shared = world.store.lock();
        for record in &mut shared.state.work.hand_overs {
            record.state = GivingState::Ended;
        }
        for work in &mut shared.state.work.works {
            work.closed = true;
        }
    }
    look(&world, given.project).await?;
    assert_eq!(hand_overs(&world).len(), 1);
    // An issue a person handed over already is written down as found and not handed over twice.
    let handed = an_issue(&world, given.project).await?;
    hand_over(
        &world.ctx(),
        plain_hand_over(&given.team.session_token, given.agent, &[handed]),
    )
    .await?;
    look(&world, given.project).await?;
    assert_eq!(hand_overs(&world).len(), 2, "only the person's hand-over");
    assert_eq!(open_work_on(&world, handed), 1);
    assert_eq!(world.store.lock().state.work.matches.len(), 2);
    Ok(())
}

#[tokio::test]
async fn m7_wp14_the_limit_of_the_agent_caps_the_open_hand_overs_of_a_rule() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let second = an_issue(&world, given.project).await?;
    let third = an_issue(&world, given.project).await?;
    limit(&world, LimitKind::IssuesAtOnce, 2);
    rule_that_hands_over(&world, &given).await?;
    look(&world, given.project).await?;
    // Two issues at once: two hand-overs, and the third is neither handed over nor written down.
    assert_eq!(hand_overs(&world).len(), 2);
    assert_eq!(world.store.lock().state.work.matches.len(), 2);
    look(&world, given.project).await?;
    assert_eq!(hand_overs(&world).len(), 2, "no flood");
    // When room is made the issue that waited is handed over at the next look.
    {
        let mut shared = world.store.lock();
        shared.state.work.works.retain(|work| work.issue == second);
        let kept = shared
            .state
            .work
            .works
            .iter()
            .map(|work| work.hand_over)
            .collect::<Vec<_>>();
        shared
            .state
            .work
            .hand_overs
            .retain(|record| kept.contains(&record.id));
    }
    look(&world, given.project).await?;
    assert_eq!(hand_overs(&world).len(), 2);
    let waiting = [given.issue, second, third]
        .into_iter()
        .filter(|issue| open_work_on(&world, *issue) == 1)
        .count();
    assert_eq!(waiting, 2);
    Ok(())
}

#[tokio::test]
async fn m7_wp14_an_agent_takes_a_hand_over_of_a_rule_and_throw_away_undoes_the_attempt() -> Outcome
{
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    rule_that_hands_over(&world, &given).await?;
    look(&world, given.project).await?;
    let handed = handed_of(&given, &world, given.issue);
    let before = title_of(&world, &given).await?;
    // The agent works in an attempt, with a run: it changes the title, and Throw away undoes it.
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    retitle(&world, &given, "Changed by the agent of a rule").await?;
    assert_eq!(
        title_of(&world, &given).await?,
        "Changed by the agent of a rule"
    );
    end_attempt(&world.ctx(), ready(&given.key, given.issue)).await?;
    let work = show(&world, &handed).await?;
    review_work(
        &world.ctx(),
        review(&given.team.session_token, &work, ReviewAnswer::ThrowAway),
    )
    .await?;
    assert_eq!(title_of(&world, &given).await?, before);
    Ok(())
}

async fn title_of(world: &World, given: &RuleWorld) -> Result<String, AppError> {
    Ok(crate::issues::issue(
        &world.ctx(),
        super::agent_work::query(&given.team.session_token, given.issue),
    )
    .await?
    .into_value()
    .issue
    .title)
}

async fn retitle(world: &World, given: &RuleWorld, title: &str) -> Result<(), AppError> {
    let version = crate::issues::issue(
        &world.ctx(),
        super::agent_work::query(&given.key, given.issue),
    )
    .await?
    .into_value()
    .issue
    .version;
    crate::issues::change_issue(
        &world.ctx(),
        crate::issues::ChangeIssue {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            version,
            issue_type: None,
            title: Some(title.to_owned()),
            status: None,
            priority: None,
            start_date: None,
            due_date: None,
            estimated_time: None,
            time_left: None,
            assigned_to: None,
            fields: std::collections::BTreeMap::default(),
        },
    )
    .await
    .map(drop)
}

#[tokio::test]
async fn m7_wp14_a_rule_that_proposes_hands_over_nothing_and_its_agent_changes_nothing() -> Outcome
{
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    create_standing_rule(&world.ctx(), rule_request(&given)).await?;
    look(&world, given.project).await?;
    assert!(hand_overs(&world).is_empty());
    assert_eq!(world.store.lock().state.work.matches.len(), 1);
    assert!(refused(
        &retitle(&world, &given, "No attempt").await.unwrap_err(),
        ErrorCode::Forbidden
    ));
    // A rule that proposes holds nothing of a hand-over.
    let mut powers = rule_request(&given);
    powers.powers = vec![HandOverPower::ChangeStatus];
    assert!(refused(
        &create_standing_rule(&world.ctx(), powers)
            .await
            .unwrap_err(),
        ErrorCode::InvalidRequest
    ));
    // And a rule that hands over cannot name a member that holds people.
    let mut people = handing(&given);
    people.fields = vec!["assigned-to".to_owned()];
    assert!(refused(
        &create_standing_rule(&world.ctx(), people)
            .await
            .unwrap_err(),
        ErrorCode::InvalidRequest
    ));
    Ok(())
}

#[tokio::test]
async fn m7_wp14_a_rule_that_never_runs_out_needs_the_setting() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let never = |given: &RuleWorld| CreateStandingRule {
        never_ends: true,
        ..rule_request(given)
    };
    // Off at the start: refused, and 90 days stay the most.
    assert!(refused(
        &create_standing_rule(&world.ctx(), never(&given))
            .await
            .unwrap_err(),
        ErrorCode::Forbidden
    ));
    let mut long = rule_request(&given);
    long.days = Some(91);
    assert!(refused(
        &create_standing_rule(&world.ctx(), long).await.unwrap_err(),
        ErrorCode::InvalidRequest
    ));
    allow_never(&world, true).await?;
    let rule = create_standing_rule(&world.ctx(), never(&given))
        .await?
        .into_value();
    assert_eq!(rule.ends_at, None);
    // No job runs it out, and days beside "never" are no request.
    assert!(
        !world
            .store
            .lock()
            .state
            .jobs
            .iter()
            .any(|job| job.kind.as_str() == "agents.rule-runs-out")
    );
    let mut both = never(&given);
    both.days = Some(10);
    assert!(refused(
        &create_standing_rule(&world.ctx(), both).await.unwrap_err(),
        ErrorCode::InvalidRequest
    ));
    // It can be stopped at any time, and it still works until then.
    look(&world, given.project).await?;
    assert_eq!(world.store.lock().state.work.matches.len(), 1);
    let stopped = change_standing_rule(
        &world.ctx(),
        ChangeStandingRule {
            session_token: given.team.session_token.clone(),
            rule: rule.id,
            renew: false,
            days: None,
            never_ends: false,
            version: rule.version,
        },
    )
    .await?
    .into_value();
    assert_eq!(stopped.state, GivingState::Ended);
    // Switched off again, a renewal needs days again.
    allow_never(&world, false).await?;
    let other = create_standing_rule(&world.ctx(), rule_request(&given))
        .await?
        .into_value();
    let renewal = change_standing_rule(
        &world.ctx(),
        ChangeStandingRule {
            session_token: given.team.session_token.clone(),
            rule: other.id,
            renew: true,
            days: None,
            never_ends: true,
            version: other.version,
        },
    )
    .await;
    assert!(refused(&renewal.unwrap_err(), ErrorCode::Forbidden));
    Ok(())
}

#[tokio::test]
async fn m7_wp14_the_setting_is_for_administrators_only() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let refusal = change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: given.team.session_token.clone(),
            scope: SettingsScope::Organization,
            key: "agents.rules-never-run-out".to_owned(),
            value: SettingValue::Bool(true),
        },
    )
    .await;
    assert!(refused(&refusal.unwrap_err(), ErrorCode::Forbidden));
    // It is off at the start.
    assert!(!world.ctx().rules_never_run_out().await?);
    allow_never(&world, true).await?;
    assert!(world.ctx().rules_never_run_out().await?);
    Ok(())
}

#[tokio::test]
async fn m7_wp14_an_accept_by_itself_of_a_hand_over_of_a_rule_is_listed_in_my_agents() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    allow_accepting(&world, given.project).await?;
    let mut rule = handing(&given);
    rule.accept_by_itself = true;
    create_standing_rule(&world.ctx(), rule).await?;
    look(&world, given.project).await?;
    let handed = handed_of(&given, &world, given.issue);
    // Nobody is in the loop: the agent takes it, attaches a passing check and says it is ready.
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    attach_result(
        &world.ctx(),
        check(&given.key, given.issue, CheckResult::Passed),
    )
    .await?;
    end_attempt(&world.ctx(), ready(&given.key, given.issue)).await?;
    let work = show(&world, &handed).await?;
    assert!(work.accepted_by_itself && !work.open);
    let mine = my_agent_work(
        &world.ctx(),
        MyAgentWorkQuery {
            session_token: given.team.session_token.clone(),
        },
    )
    .await?
    .into_value();
    assert!(
        mine.items
            .iter()
            .any(|item| item.work.id == work.id && item.work.accepted_by_itself),
        "*Accepted by itself* lists it until a person looked at it"
    );
    Ok(())
}

fn jobs_of_kind(world: &World, kind: &str) -> usize {
    world
        .store
        .lock()
        .state
        .jobs
        .iter()
        .filter(|job| job.kind.as_str() == kind)
        .count()
}

#[tokio::test]
async fn m7_f236_switching_the_setting_off_gives_a_rule_that_never_runs_out_an_end() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    allow_never(&world, true).await?;
    let never = create_standing_rule(
        &world.ctx(),
        CreateStandingRule {
            never_ends: true,
            ..rule_request(&given)
        },
    )
    .await?
    .into_value();
    assert_eq!(never.ends_at, None);
    // Another change of the setting queues nothing.
    assert_eq!(jobs_of_kind(&world, "agents.rules-get-an-end"), 0);
    allow_never(&world, false).await?;
    assert_eq!(jobs_of_kind(&world, "agents.rules-get-an-end"), 1);
    // Still no end until the job ran, and the job leaves it alone while the setting is on again.
    allow_never(&world, true).await?;
    world.ctx().run_rules_get_an_end().await?;
    assert_eq!(world.store.lock().state.work.rules[0].ends_at, None);
    allow_never(&world, false).await?;
    world.ctx().run_rules_get_an_end().await?;
    let rule = world.store.lock().state.work.rules[0].clone();
    // The most days (90) from now, with the jobs of an end: it runs out, and it is told before.
    let end = rule.ends_at.expect("the rule has an end now");
    assert_eq!(
        end.as_second() - world.ctx().now().as_second(),
        90 * 24 * 3600
    );
    assert_eq!(rule.version, never.version + 1);
    assert_eq!(jobs_of_kind(&world, "agents.rule-runs-out"), 1);
    assert_eq!(jobs_of_kind(&world, "agents.rule-about-to-run-out"), 1);
    // A second run finds nothing to do.
    world.ctx().run_rules_get_an_end().await?;
    assert_eq!(world.store.lock().state.work.rules[0].version, rule.version);
    Ok(())
}

#[tokio::test]
async fn m7_f236_resetting_the_setting_to_its_default_gives_the_rules_an_end_too() -> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    allow_never(&world, true).await?;
    create_standing_rule(
        &world.ctx(),
        CreateStandingRule {
            never_ends: true,
            ..rule_request(&given)
        },
    )
    .await?;
    assert_eq!(jobs_of_kind(&world, "agents.rules-get-an-end"), 0);
    // *Reset to default* turns the setting off, as a change to off does.
    crate::settings::reset_setting(
        &world.ctx(),
        crate::settings::ResetSetting {
            session_token: token(&world),
            scope: SettingsScope::Organization,
            key: "agents.rules-never-run-out".to_owned(),
        },
    )
    .await?;
    assert_eq!(jobs_of_kind(&world, "agents.rules-get-an-end"), 1);
    world.ctx().run_rules_get_an_end().await?;
    assert!(world.store.lock().state.work.rules[0].ends_at.is_some());
    Ok(())
}

#[tokio::test]
async fn m7_wp14_a_rule_with_a_field_no_agent_can_have_is_skipped_and_the_other_rules_work()
-> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    let second = an_issue(&world, given.project).await?;
    // The first rule names a field that is no member of an issue (as a retired own field is not).
    let broken = rule_that_hands_over(&world, &given).await?;
    world.store.lock().state.work.rules[0].fields = vec!["retired-field".to_owned()];
    let working = rule_that_hands_over(&world, &given).await?;
    look(&world, given.project).await?;
    // The look did not fail; the rule that works handed both issues over, the broken one none.
    assert_eq!(hand_overs(&world).len(), 2);
    let found = world.store.lock().state.work.matches.clone();
    assert!(found.iter().all(|one| one.rule == working.id));
    assert!(found.iter().any(|one| one.issue == second));
    assert!(found.iter().all(|one| one.rule != broken.id));
    Ok(())
}
