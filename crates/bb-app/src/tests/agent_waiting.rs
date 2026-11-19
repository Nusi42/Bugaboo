//! *Wait for work* (AI-36, ADR-0029, amendment M9): one call that answers when work waits for the
//! agent or after its seconds, with what *changes since* answers, and the waits kept fair.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use bb_core::ErrorCode;
use bb_core::agent_work::ReviewAnswer;
use serde_json::json;

use super::agent_tools::used;
use super::agent_work::{Ground, ground, plain_hand_over, start_attempt};
use super::agent_work_review::{ready, review, show};
use super::agents::{an_agent, key_of, refused};
use super::world::{Outcome, World};
use crate::agent_work::{end_attempt, hand_over, review_work};
use crate::ai::tools::ToolAnswer;
use crate::auth::AppError;

/// The time of the test's runtime, as a server gives the time of its own.
pub(super) struct RuntimePause;

impl crate::agent_work::Pause for RuntimePause {
    fn pause(&self, length: Duration) -> crate::ports::BoxFuture<'static, ()> {
        Box::pin(tokio::time::sleep(length))
    }
}

/// A listener of 1024 connections, the default.
const CONNECTIONS: usize = 1024;
/// Long enough that a call which answers in time did not run out.
const LONG: u64 = 30;
/// Answering in time: well below [`LONG`].
const IN_TIME: Duration = Duration::from_secs(10);
/// How long the helpers wait before they change something while a call waits.
const SOON: Duration = Duration::from_millis(150);

async fn waited(world: &World, key: &str, seconds: u64) -> Result<ToolAnswer, AppError> {
    used(world, key, "wait_for_work", json!({ "seconds": seconds })).await
}

/// Polls every future until all are ready (the tests run them side by side on one task).
struct All<'a> {
    futures: Vec<Option<Pin<Box<dyn Future<Output = ()> + 'a>>>>,
}

impl Future for All<'_> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let mut open = false;
        for slot in &mut self.futures {
            if let Some(future) = slot {
                if future.as_mut().poll(context).is_ready() {
                    *slot = None;
                } else {
                    open = true;
                }
            }
        }
        if open { Poll::Pending } else { Poll::Ready(()) }
    }
}

#[tokio::test]
async fn ai_36_wait_for_work_answers_at_once_when_work_waits_for_the_agent() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?.waiting(CONNECTIONS);
    let given = ground(&world).await?;
    hand_over(
        &world.ctx(),
        plain_hand_over(&given.team.session_token, given.agent, &[given.issue]),
    )
    .await?;
    let began = Instant::now();
    let answer = waited(&world, &given.key, LONG).await?;
    assert!(began.elapsed() < IN_TIME);
    assert!(
        answer.text.contains("work_waiting: true"),
        "{}",
        answer.text
    );
    assert!(answer.text.contains("waited: false"), "{}", answer.text);
    Ok(())
}

#[tokio::test]
async fn ai_36_wait_for_work_answers_when_work_is_handed_over_while_it_waits() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?.waiting(CONNECTIONS);
    let given = ground(&world).await?;
    let began = Instant::now();
    let (answer, handed) = tokio::join!(waited(&world, &given.key, LONG), async {
        tokio::time::sleep(SOON).await;
        hand_over(
            &world.ctx(),
            plain_hand_over(&given.team.session_token, given.agent, &[given.issue]),
        )
        .await
    });
    handed?;
    let answer = answer?;
    assert!(began.elapsed() < IN_TIME, "it waited out its seconds");
    assert!(answer.text.contains("waited: true"), "{}", answer.text);
    assert!(
        answer.text.contains("work_waiting: true"),
        "{}",
        answer.text
    );
    assert_eq!(world.services.work_signal.waiting_now(), 0);
    Ok(())
}

/// Work of the agent that is *Ready for review*.
async fn ready_for_review(world: &World) -> Result<super::agent_work::Handed, AppError> {
    let given = super::agent_work::handed(world).await?;
    start_attempt(world, &given.key, given.issue).await?;
    end_attempt(&world.ctx(), ready(&given.key, given.issue)).await?;
    Ok(given)
}

#[tokio::test]
async fn ai_36_wait_for_work_answers_when_work_is_sent_back_while_it_waits() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?.waiting(CONNECTIONS);
    let given = ready_for_review(&world).await?;
    let began = Instant::now();
    let (answer, sent) = tokio::join!(waited(&world, &given.key, LONG), async {
        tokio::time::sleep(SOON).await;
        let work = show(&world, &given).await?;
        let mut back = review(&given.team.session_token, &work, ReviewAnswer::SendBack);
        back.note = Some("Please also check the phone layout".to_owned());
        review_work(&world.ctx(), back).await
    });
    sent?;
    let answer = answer?;
    assert!(began.elapsed() < IN_TIME, "it waited out its seconds");
    assert!(answer.text.contains("waited: true"), "{}", answer.text);
    assert!(
        answer.text.contains("work_waiting: true"),
        "{}",
        answer.text
    );
    Ok(())
}

#[tokio::test]
async fn ai_36_wait_for_work_answers_after_its_seconds_with_what_changes_since_answers() -> Outcome
{
    let world = World::with_ai_apps_and_agents().await?.waiting(CONNECTIONS);
    let given = ground(&world).await?;
    let began = Instant::now();
    let answer = waited(&world, &given.key, 1).await?;
    assert!(began.elapsed() >= Duration::from_secs(1));
    assert!(answer.text.contains("waited: true"), "{}", answer.text);
    assert!(
        answer.text.contains("work_waiting: false"),
        "{}",
        answer.text
    );
    // The rest is what changes_since answers for the same mark: the first look gives the mark.
    let since = used(&world, &given.key, "changes_since", json!({})).await?;
    assert!(answer.text.contains("mark:"), "{}", answer.text);
    assert!(since.text.contains("mark:"), "{}", since.text);
    Ok(())
}

#[tokio::test]
async fn ai_36_wait_for_work_takes_one_to_sixty_seconds() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?.waiting(CONNECTIONS);
    let given = ground(&world).await?;
    for seconds in [0, 61] {
        let refusal = waited(&world, &given.key, seconds).await;
        assert!(refused(&refusal.unwrap_err(), ErrorCode::InvalidRequest));
    }
    // A person is not offered it.
    let person = used(
        &world,
        &given.team.session_token,
        "wait_for_work",
        json!({}),
    )
    .await;
    assert!(person.is_err());
    Ok(())
}

#[tokio::test]
async fn ai_36_a_second_wait_of_the_same_agent_does_not_wait() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?.waiting(CONNECTIONS);
    let given = ground(&world).await?;
    let (first, second) = tokio::join!(waited(&world, &given.key, 2), async {
        tokio::time::sleep(SOON).await;
        let began = Instant::now();
        let answer = waited(&world, &given.key, LONG).await;
        (answer, began.elapsed())
    });
    assert!(first?.text.contains("waited: true"));
    let (second, took) = second;
    assert!(second?.text.contains("waited: false"));
    assert!(took < IN_TIME);
    Ok(())
}

#[tokio::test]
async fn ai_36_waits_over_the_share_of_connections_do_not_wait() -> Outcome {
    // Four connections: one call may wait.
    let world = World::with_ai_apps_and_agents().await?.waiting(4);
    let given = ground(&world).await?;
    let helper = an_agent(&world, &given.team.session_token, "Helper").await?;
    let helper_key = key_of(&world, &given.team.session_token, helper).await?;
    let (first, second) = tokio::join!(waited(&world, &given.key, 2), async {
        tokio::time::sleep(SOON).await;
        waited(&world, &helper_key, LONG).await
    });
    assert!(first?.text.contains("waited: true"));
    assert!(second?.text.contains("waited: false"));
    Ok(())
}

#[tokio::test]
async fn ai_36_a_wait_ends_when_the_agent_is_paused() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?.waiting(CONNECTIONS);
    let given = ground(&world).await?;
    let session = given.team.session_token.clone();
    let version = crate::agents::agent(
        &world.ctx(),
        crate::agents::AgentQuery {
            session_token: session.clone(),
            id: given.agent,
        },
    )
    .await?
    .into_value()
    .version;
    let began = Instant::now();
    let (_answer, paused) = tokio::join!(waited(&world, &given.key, LONG), async {
        tokio::time::sleep(SOON).await;
        crate::agents::change_agent(
            &world.ctx(),
            crate::agents::ChangeAgent {
                session_token: session.clone(),
                id: given.agent,
                action: crate::agents::AgentAction::Pause,
                version,
            },
        )
        .await
    });
    paused?;
    // Whatever the paused agent is told, it is told now, not after its seconds.
    assert!(began.elapsed() < IN_TIME, "it waited out its seconds");
    assert_eq!(world.services.work_signal.waiting_now(), 0);
    Ok(())
}

#[tokio::test]
async fn ai_36_work_another_agent_holds_does_not_answer_a_wait() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?.waiting(CONNECTIONS);
    let given = ground(&world).await?;
    let session = given.team.session_token.clone();
    hand_over(
        &world.ctx(),
        plain_hand_over(&session, given.agent, &[given.issue]),
    )
    .await?;
    start_attempt(&world, &given.key, given.issue).await?;
    // The same issue for a helper: it cannot take it while the first agent works on it.
    let helper = an_agent(&world, &session, "Helper").await?;
    let helper_key = key_of(&world, &session, helper).await?;
    hand_over(
        &world.ctx(),
        plain_hand_over(&session, helper, &[given.issue]),
    )
    .await?;
    let answer = waited(&world, &helper_key, 1).await?;
    assert!(answer.text.contains("waited: true"), "{}", answer.text);
    assert!(
        answer.text.contains("work_waiting: false"),
        "{}",
        answer.text
    );
    Ok(())
}

/// The measurement of what waiting costs: many agents wait at once, a hand-over to one of them
/// ends only its wait, and each wait holds an entry of the signal and one future, nothing else.
#[tokio::test]
async fn ai_36_many_waits_cost_one_read_each_and_a_hand_over_wakes_only_its_agent() -> Outcome {
    // As many agents as one person may have.
    const AGENTS: usize = bb_core::agent::MOST_AGENTS;
    let world = World::with_ai_apps_and_agents().await?.waiting(CONNECTIONS);
    let Ground {
        team,
        agent,
        key,
        issue,
        ..
    } = ground(&world).await?;
    let mut keys = Vec::new();
    for at in 1..AGENTS {
        let other = an_agent(&world, &team.session_token, &format!("Helper {at}")).await?;
        keys.push(key_of(&world, &team.session_token, other).await?);
    }
    let woken = std::cell::Cell::new(None);
    let size = {
        let probe = waited(&world, &key, LONG);
        size_of_val(&probe)
    };
    let rest_done = std::cell::Cell::new(0_usize);
    let mut futures: Vec<Option<Pin<Box<dyn Future<Output = ()> + '_>>>> = Vec::new();
    for other in &keys {
        let world = &world;
        let rest_done = &rest_done;
        futures.push(Some(Box::pin(async move {
            let answer = waited(world, other, 3).await;
            assert!(answer.is_ok_and(|answer| answer.text.contains("work_waiting: false")));
            rest_done.set(rest_done.get() + 1);
        })));
    }
    futures.push(Some(Box::pin(async {
        let began = Instant::now();
        let answer = waited(&world, &key, LONG).await;
        assert!(answer.is_ok_and(|answer| answer.text.contains("work_waiting: true")));
        woken.set(Some(began.elapsed()));
    })));
    futures.push(Some(Box::pin(async {
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(world.services.work_signal.waiting_now(), AGENTS);
        let handed_at = Instant::now();
        let handed = hand_over(
            &world.ctx(),
            plain_hand_over(&team.session_token, agent, &[issue]),
        )
        .await;
        assert!(handed.is_ok());
        while world.services.work_signal.waiting_now() == AGENTS {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let answered = handed_at.elapsed();
        // Only the wait of that agent ended; every other one still waits.
        assert_eq!(world.services.work_signal.waiting_now(), AGENTS - 1);
        assert_eq!(rest_done.get(), 0);
        eprintln!(
            "{AGENTS} waits: one future of {size} bytes each; a hand-over answered its agent \
             in {answered:?} and woke nobody else"
        );
    })));
    All { futures }.await;
    assert_eq!(rest_done.get(), AGENTS - 1);
    assert!(woken.get().is_some_and(|took| took < IN_TIME));
    Ok(())
}
