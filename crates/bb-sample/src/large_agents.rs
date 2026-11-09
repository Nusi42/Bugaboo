//! Agent work in the large dataset (M7 WP-13, PERF-01): one Team member has a few agents, 1 % of the
//! issues are handed over to them and some of that work has been taken, asked about or ended, and
//! standing rules over a selective search have their match record.
//!
//! It is made by the use cases of agents, as a person and as the agent with its key, so the data is
//! made by the code that the benchmarks of *changes since*, of the search of agent states and of
//! *My agents* measure. Only issues of projects that count their IDs are handed over: their
//! readable IDs follow from the counter of the project.

use bb_app::agent_work::{CreateStandingRule, HandOver, create_standing_rule, hand_over};
use bb_app::saved_searches::{SaveSearch, save_search};
use bb_core::agent_work::{MOST_HAND_OVER_DAYS, MOST_ISSUES_OF_A_HAND_OVER, MOST_RULE_DAYS};
use bb_core::saved_search::Presentation;
use bb_dev_words::sample::AiPlan;
use bb_dev_words::{LargeAgents, LargePlan};
use serde_json::json;

use crate::agents_work::Made;
use crate::ai_setup::switch_on_ai;
use crate::error::{SampleError, Step};
use crate::generate::{AgentKey, World};
use crate::issues_setup::{id_in, picked};
use crate::large_org::Org;

/// How many seconds of the clock pass after an agent worked on an issue, so it stays within its
/// limit of changes a minute.
const SECONDS_BETWEEN_ISSUES: u32 = 6;

fn failed(error: &dyn std::fmt::Display) -> SampleError {
    SampleError::step(Step::Agents, &error)
}

/// How an issue handed over to an agent goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Goes {
    /// Nothing: it waits for the agent.
    Waits,
    /// The agent took it and reported.
    Works,
    /// The agent took it and asked a question.
    Asks,
    /// The agent ended the attempt with a result for the review.
    Ends,
}

impl Goes {
    /// By the place of the issue among the hundred of its hand-over and by how many attempts the
    /// agent holds open: the first shares of every hundred end with a result, then the agent opens
    /// attempts (a work and a question in turn) up to the plan, and the rest wait. An agent holds
    /// only a few attempts at once, so most of what is handed over waits.
    fn of(at: usize, open: u32, plan: &LargeAgents) -> Self {
        if u32::try_from(at % 100).unwrap_or(100) < plan.ready_percent {
            Self::Ends
        } else if open >= plan.open_per_agent {
            Self::Waits
        } else if open % 2 == 0 {
            Self::Works
        } else {
            Self::Asks
        }
    }
}

/// The readable IDs of the issues to hand over: every `100 / work_percent`-th issue of the
/// projects that count their IDs, with the project that has the most of them.
async fn issues_to_hand_over(
    world: &World<'_>,
    org: &Org,
    plan: &LargeAgents,
) -> Result<(Vec<String>, bb_core::id::ProjectId), SampleError> {
    let step = usize::try_from(100 / plan.work_percent.max(1))
        .unwrap_or(100)
        .max(1);
    let mut keys = Vec::new();
    let mut biggest = None;
    for place in org.places.iter().filter(|place| !place.random_codes) {
        let last = world
            .storage
            .database()
            .issue_counter(place.id)
            .await?
            .unwrap_or(0);
        keys.extend(
            (1..=last)
                .step_by(step)
                .map(|number| format!("{}-{number}", place.short_name)),
        );
        if biggest.is_none_or(|(_, most)| last > most) {
            biggest = Some((place.id, last));
        }
    }
    let (project, _) = biggest.ok_or_else(|| SampleError::NotMade("a project".to_owned()))?;
    Ok((keys, project))
}

/// Makes the agents, hands the issues over, lets the agents go on as the plan says and makes the
/// standing rules and their match record. Returns the keys of the agents for `sign-in.txt`.
pub(crate) async fn give_work(
    world: &World<'_>,
    org: &Org,
    large: &LargePlan,
    ai: &AiPlan,
) -> Result<Vec<AgentKey>, SampleError> {
    let plan = &large.agents;
    let administrator = world.administrator()?;
    let password = world.people[administrator].password.clone();
    // The session of the first administrator ended long ago, while the issues were written.
    let administrators = world.session_of(administrator).await?;
    switch_on_ai(world, &administrators, &password, ai).await?;
    let person = world
        .people
        .iter()
        .position(|made| made.person.name == ai.person)
        .ok_or_else(|| SampleError::NotMade(ai.person.clone()))?;
    let token = world.session_of(person).await?;
    let mut agents = Vec::new();
    for number in 1..=plan.agents {
        agents.push(
            world
                .new_agent(&token, person, &format!("Agent {number}"))
                .await?,
        );
    }
    let (keys, project) = issues_to_hand_over(world, org, plan).await?;
    hand_over_in_chunks(world, &token, &agents, &keys, ai, plan).await?;
    create_rules(world, &token, &agents, project, plan).await?;
    world.run_the_jobs().await?;
    let email = world.people[person].person.email.clone();
    Ok(agents
        .into_iter()
        .zip(1..)
        .map(|(made, number)| AgentKey {
            mark: ai.agent_key_mark.clone(),
            agent: format!("Agent {number}"),
            email: email.clone(),
            key: made.key,
        })
        .collect())
}

/// Hands the issues over a hundred at a time, one agent after the other, and takes each issue as
/// far as its place says.
async fn hand_over_in_chunks(
    world: &World<'_>,
    token: &str,
    agents: &[Made],
    keys: &[String],
    ai: &AiPlan,
    plan: &LargeAgents,
) -> Result<(), SampleError> {
    let mut open = vec![0_u32; agents.len()];
    for (at, chunk) in keys.chunks(MOST_ISSUES_OF_A_HAND_OVER).enumerate() {
        let held = at % agents.len();
        let agent = &agents[held];
        world.tick();
        hand_over(
            &world.services.ctx(world.request()),
            HandOver {
                session_token: token.to_owned(),
                agent: agent.id,
                issues: chunk.to_vec(),
                note: ai.note.clone(),
                powers: Vec::new(),
                fields: Vec::new(),
                budget_attempts: None,
                budget_minutes: None,
                budget_cost: None,
                plan_needs_approval: false,
                accept_by_itself: false,
                days: Some(MOST_HAND_OVER_DAYS),
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        for (number, issue) in chunk.iter().enumerate() {
            let goes = Goes::of(number, open[held], plan);
            match goes {
                Goes::Waits => {}
                Goes::Works => world.report_an_attempt(&agent.key, issue, None).await?,
                Goes::Asks => {
                    world.report_an_attempt(&agent.key, issue, None).await?;
                    world.ask(&agent.key, issue, &ai.question).await?;
                }
                Goes::Ends => world.finish(ai, &agent.key, issue, None).await?,
            }
            open[held] += u32::from(matches!(goes, Goes::Works | Goes::Asks));
            if goes != Goes::Waits {
                // An agent may make a change a second at the most (its limit is 60 a minute).
                for _ in 0..SECONDS_BETWEEN_ISSUES {
                    world.tick();
                }
            }
        }
    }
    Ok(())
}

/// The standing rules: each over a saved search of the person in the biggest project, for one agent.
async fn create_rules(
    world: &World<'_>,
    token: &str,
    agents: &[Made],
    project: bb_core::id::ProjectId,
    plan: &LargeAgents,
) -> Result<(), SampleError> {
    let ctx = world.services.ctx(world.request());
    for number in 0..plan.rules {
        let agent = &agents[usize::try_from(number).unwrap_or(0) % agents.len()];
        world.tick();
        let saved = picked(
            save_search(
                &ctx,
                SaveSearch {
                    session_token: token.to_owned(),
                    name: format!("Agent rule {}", number + 1),
                    text: plan.rule_search.clone(),
                    presentation: Presentation::default(),
                },
            )
            .await,
            Step::Agents,
            |saved| json!({ "id": saved.id.to_string() }),
        )?;
        world.tick();
        create_standing_rule(
            &ctx,
            CreateStandingRule {
                session_token: token.to_owned(),
                agent: agent.id,
                search: id_in(&saved, "id")?,
                project,
                fields: Vec::new(),
                creates_in: None,
                days: Some(MOST_RULE_DAYS),
            },
        )
        .await
        .map_err(|error| failed(&error))?;
    }
    Ok(())
}
