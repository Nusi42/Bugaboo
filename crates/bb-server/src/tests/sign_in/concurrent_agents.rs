//! Ten people and their agents working at once, through the real wiring, on both databases
//! (TEST-CONCURRENT, architecture §11 "Ten people with agents", AI-30 to AI-38, ADR-0029).
//!
//! Each of ten Team members has an agent with a key and an issue of their own that they hand over to
//! it. The agents take, report, change a priority they were allowed to change, ask and submit over
//! `/mcp` while their people answer and review over the API: three people accept, three send an
//! attempt back first and one throws an attempt away, which undoes the change of the agent. Two
//! agents take one issue at the same moment, and an agent works on while its person loses access.
//! The people write the book of what they did; what must hold afterwards is not defined here but in
//! `bb-load-check`, as for the ten people of `concurrent.rs`.

use std::sync::{Arc, Mutex};

use axum::Router;
use bb_app::access::StartingRole;
use bb_load_check::{Afterwards, Agented, Book, HandedOver, Kept, Lost, Raced, Work, check};
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};
use tokio::task::JoinSet;

use super::ai_mcp_agents::{call, failed};
use super::ai_mcp_rig::{Mcp, ai_on, an_agent_with_a_key};
use super::http::Browser;
use super::issues_site::{ISSUES, Site, ok};
use super::policy_rig::Member;
use super::rig::Rig;

/// Ten people, each with an agent.
const PEOPLE: usize = 10;
/// The person who loses access while their agent works.
const LOSES: usize = PEOPLE - 1;
const HAND_OVERS: &str = "/api/v1/hand-overs";
const AGENT_WORK: &str = "/api/v1/agent-work";

type Shared = Arc<Mutex<Book>>;

fn book(shared: &Shared) -> Result<std::sync::MutexGuard<'_, Book>, Failure> {
    shared.lock().map_err(|_| "the book is poisoned".into())
}

/// A person with the agent they answer for and the issue they hand over to it.
struct Crew {
    number: usize,
    browser: Browser,
    holder: String,
    agent: String,
    key: String,
    issue: Work,
}

/// What `/mcp` is asked and where the book is kept.
struct Floor {
    app: Router,
    book: Shared,
}

impl Floor {
    /// The agent calls a tool and has to be answered.
    async fn tool(&self, crew: &Crew, tool: &str, arguments: Value) -> Result<(), Failure> {
        let answer = call(&Mcp::new(&self.app), &crew.key, tool, arguments).await?;
        if failed(&answer) {
            return Err(format!(
                "{tool} of agent {} was refused: {}",
                crew.number, answer.json
            )
            .into());
        }
        Ok(())
    }
}

/// Whether the call was answered: the tool did its work and the key was let in.
fn kept(answer: &super::ai_mcp_rig::Answer) -> bool {
    answer.status.is_success() && !failed(answer)
}

/// The version of the issue, as the person reads it.
async fn version_of(crew: &mut Crew) -> Result<i64, Failure> {
    let read = crew
        .browser
        .send("GET", &format!("{ISSUES}/{}", crew.issue.key), None)
        .await?;
    let body = ok(&read)?;
    body["version"]
        .as_i64()
        .or_else(|| body["issue"]["version"].as_i64())
        .ok_or_else(|| "no version".into())
}

/// The work on the issue of the crew: its ID, version and first question.
async fn work_of(crew: &mut Crew) -> Result<Value, Failure> {
    let read = crew
        .browser
        .send(
            "GET",
            &format!("{ISSUES}/{}/agent-work", crew.issue.key),
            None,
        )
        .await?;
    Ok(ok(&read)?["work"][0].clone())
}

/// The person hands the issue over to their agent, with the right to change the priority.
async fn hand_over(crew: &mut Crew, shared: &Shared) -> Result<(), Failure> {
    let given = crew
        .browser
        .send(
            "POST",
            HAND_OVERS,
            Some(json!({
                "agent": crew.agent,
                "issues": [crew.issue.key],
                "note": "Please look at this",
                "powers": ["change-fields"],
                "fields": ["priority"],
            })),
        )
        .await?;
    ok(&given).map_err(|e| format!("hand-over: {e}"))?;
    book(shared)?.handed.push(HandedOver {
        agent: crew.agent.clone(),
        key: crew.issue.key.clone(),
    });
    Ok(())
}

impl Floor {
    /// The agent takes the issue, reports, changes the priority and, for some, asks a question that
    /// the person answers; then it submits.
    async fn attempt(&self, crew: &mut Crew, asks: bool, changes: bool) -> Result<(), Failure> {
        let key = crew.issue.key.clone();
        self.tool(crew, "take", json!({"issue": key})).await?;
        self.tool(
            crew,
            "report",
            json!({"issue": key, "minutes": 3, "cost_cents": 0}),
        )
        .await?;
        if changes {
            self.change_priority(crew).await?;
        }
        if asks {
            self.tool(
                crew,
                "ask",
                json!({"issue": key, "question": "Which browser?"}),
            )
            .await?;
            let work = work_of(crew).await?;
            let answered = crew
                .browser
                .send(
                    "POST",
                    &format!(
                        "{AGENT_WORK}/{}/answer",
                        work["id"].as_str().unwrap_or_default()
                    ),
                    Some(json!({
                        "question": work["questions"][0]["id"],
                        "answer": "Firefox",
                        "version": work["version"],
                    })),
                )
                .await?;
            ok(&answered).map_err(|e| format!("answer: {e}"))?;
        }
        self.tool(crew, "submit", json!({"issue": key})).await?;
        // Ending an attempt writes the short comment of the agent on the issue.
        book(&self.book)?.comments.push(key);
        Ok(())
    }

    /// The agent changes the priority the person allowed it to change.
    async fn change_priority(&self, crew: &mut Crew) -> Result<(), Failure> {
        let version = version_of(crew).await?;
        self.tool(
            crew,
            "update_issue",
            json!({"issue": crew.issue.key, "version": version, "priority": "high"}),
        )
        .await?;
        book(&self.book)?.kept.push(Kept {
            key: crew.issue.key.clone(),
            version: version + 1,
            ..Kept::default()
        });
        Ok(())
    }
}

/// The person answers the result of the work.
async fn review(crew: &mut Crew, answer: &str, note: Option<&str>) -> Result<(), Failure> {
    let work = work_of(crew).await?;
    let reviewed = crew
        .browser
        .send(
            "POST",
            &format!(
                "{AGENT_WORK}/{}/review",
                work["id"].as_str().unwrap_or_default()
            ),
            Some(json!({"answer": answer, "note": note, "version": work["version"]})),
        )
        .await?;
    ok(&reviewed).map_err(|e| format!("review: {e}"))?;
    Ok(())
}

/// One person and their agent, by their number: `number % 3` decides how the review goes.
async fn work_of_one(floor: Arc<Floor>, mut crew: Crew) -> Result<(), Failure> {
    hand_over(&mut crew, &floor.book).await?;
    let asks = crew.number % 2 == 0;
    // The change comes with the run of the attempt, so it is the agent's, within its person.
    let before = version_of(&mut crew).await?;
    floor.attempt(&mut crew, asks, true).await?;
    match crew.number % 3 {
        0 => review(&mut crew, "accept", None).await,
        1 => {
            review(
                &mut crew,
                "send-back",
                Some("Please also check the narrow layout"),
            )
            .await?;
            floor.attempt(&mut crew, false, false).await?;
            review(&mut crew, "accept", None).await
        }
        _ => {
            review(&mut crew, "throw-away", None).await?;
            // The undo wrote the old value back: one more version, kept by the person.
            let now = version_of(&mut crew).await?;
            for version in before + 2..=now {
                book(&floor.book)?.kept.push(Kept {
                    key: crew.issue.key.clone(),
                    version,
                    ..Kept::default()
                });
            }
            Ok(())
        }
    }
}

/// The person who loses access: their agent works, the administrator takes the role away, and the
/// agent's next calls are refused, none of them keeps a change.
async fn work_and_lose_access(
    floor: Arc<Floor>,
    mut crew: Crew,
    rig: &Rig,
    site: &Site<'_>,
    row: bb_app::roles::AssignmentView,
) -> Result<(), Failure> {
    hand_over(&mut crew, &floor.book).await?;
    let key = crew.issue.key.clone();
    floor.tool(&crew, "take", json!({"issue": key})).await?;
    floor
        .tool(
            &crew,
            "report",
            json!({"issue": key, "minutes": 1, "cost_cents": 0}),
        )
        .await?;
    rig.api()
        .remove_assignment(
            Rig::info(),
            bb_app::roles::RemoveAssignment {
                session_token: site.org.admin.session.clone(),
                assignment: row.id,
            },
        )
        .await?;
    let mut kept_after = 0;
    for (tool, arguments) in [
        (
            "report",
            json!({"issue": key, "minutes": 2, "cost_cents": 0}),
        ),
        ("submit", json!({"issue": key})),
    ] {
        let answer = call(&Mcp::new(&floor.app), &crew.key, tool, arguments).await?;
        kept_after += u64::from(kept(&answer));
    }
    book(&floor.book)?.lost.push(Lost {
        agent: crew.agent.clone(),
        person: crew.holder.clone(),
        kept_after,
    });
    Ok(())
}

/// Two agents of two people take the issue at the same moment: one is told it has it.
async fn the_race(floor: &Floor, crews: &mut [Crew], shared: &Work) -> Result<(), Failure> {
    let [first, second, ..] = crews else {
        return Err("fewer than two crews".into());
    };
    for crew in [&mut *first, &mut *second] {
        let given = crew
            .browser
            .send(
                "POST",
                HAND_OVERS,
                Some(json!({"agent": crew.agent, "issues": [shared.key], "note": "Race"})),
            )
            .await?;
        ok(&given).map_err(|e| format!("race hand-over: {e}"))?;
        book(&floor.book)?.handed.push(HandedOver {
            agent: crew.agent.clone(),
            key: shared.key.clone(),
        });
    }
    let input = json!({"issue": shared.key});
    let mcp = Mcp::new(&floor.app);
    let (one, other) = tokio::join!(
        call(&mcp, &first.key, "take", input.clone()),
        call(&mcp, &second.key, "take", input.clone()),
    );
    let (one, other) = (one?, other?);
    let winners = [(&first.agent, &one), (&second.agent, &other)]
        .into_iter()
        .filter(|(_, answer)| kept(answer))
        .map(|(agent, _)| agent.clone())
        .collect();
    book(&floor.book)?.races.push(Raced {
        key: shared.key.clone(),
        askers: vec![first.agent.clone(), second.agent.clone()],
        winners,
    });
    Ok(())
}

fn work_of_issue(body: &Value) -> Result<Work, Failure> {
    Ok(Work {
        id: body["id"].as_str().ok_or("no id")?.to_owned(),
        key: body["readable_id"].as_str().ok_or("no key")?.to_owned(),
        reporter: None,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_ai_30_ten_people_and_their_agents_hold_the_rules() -> TestResult {
    each_database("test_concurrent_ten_people_with_agents", async |db| {
        let (rig, app) = Mcp::start(db).await?;
        let (site, mut admin) = Site::start(&rig, &app).await?;
        ai_on(&rig).await?;
        let shared_book: Shared = Arc::default();
        let role = site.role_id(StartingRole::TeamMember).parse()?;
        let mut crews = Vec::new();
        let mut loser = None;
        for number in 0..PEOPLE {
            let email = format!("crew{number}@example.test");
            let started = rig
                .person_called(&site.org.admin.session, &email, "Bob Builder")
                .await?;
            let member = Member {
                id: started.person.id,
                session: started.session_token,
            };
            let row = site.org.give_everywhere(&member, role).await?;
            let mut browser = Browser::signed_in_as(&app, &email).await?;
            let (agent, key) =
                an_agent_with_a_key(&mut browser, &format!("Agent {number}"), None).await?;
            let made = site
                .create(&mut admin, &format!("Issue of {number}"))
                .await?;
            let issue = work_of_issue(ok(&made)?)?;
            book(&shared_book)?.issues.push(issue.clone());
            book(&shared_book)?.agents.push(Agented {
                agent: agent.clone(),
                person: member.id.to_string(),
            });
            let crew = Crew {
                number,
                browser,
                holder: member.id.to_string(),
                agent,
                key,
                issue,
            };
            if number == LOSES {
                loser = Some((crew, row));
            } else {
                crews.push(crew);
            }
        }
        let shared_issue = work_of_issue(ok(&site.create(&mut admin, "Taken by two").await?)?)?;
        book(&shared_book)?.issues.push(shared_issue.clone());
        let floor = Arc::new(Floor {
            app: app.clone(),
            book: Arc::clone(&shared_book),
        });
        the_race(&floor, &mut crews, &shared_issue).await?;
        let (lost, row) = loser.ok_or("nobody loses access")?;
        let mut working = JoinSet::new();
        for crew in crews {
            let floor = Arc::clone(&floor);
            working.spawn(work_of_one(floor, crew));
        }
        let others = async {
            while let Some(done) = working.join_next().await {
                done??;
            }
            Ok::<(), Failure>(())
        };
        let (lost_access, others) = tokio::join!(
            work_and_lose_access(Arc::clone(&floor), lost, &rig, &site, row),
            others
        );
        lost_access?;
        others?;
        let book = std::mem::take(&mut *book(&shared_book)?);
        let afterwards = Afterwards {
            now: rig.clock_now().as_second(),
            lease_seconds: bb_core::edit::LEASE_SECONDS,
            purged: false,
        };
        let found = check(&rig.database, &book, &afterwards).await?;
        assert!(found.is_empty(), "{found:#?}");
        // The rules were put to the test: a race was run, a person lost access, and agents changed
        // issues.
        assert_eq!(book.races.len(), 1);
        assert_eq!(book.lost.len(), 1);
        assert!(
            book.kept.len() >= PEOPLE - 1,
            "{} changes of agents",
            book.kept.len()
        );
        Ok(())
    })
    .await
}
