//! Agents in the sample (M7 part 6, AI-30 to AI-35): two agents of one Team member, each with a key,
//! and hand-overs on Website issues in every state agent work can be in, with a question, an answer,
//! a plan, a sent-back attempt, an accepted one, one that was thrown away and one that ran out. A
//! standing rule over new bugs proposes and changes nothing.
//!
//! All of it goes through the use cases the screens and the tools of agents call, as the person and
//! as the agent with its key, one change a second of the sample's clock. The work is done some days
//! before the end of the sample, so that a hand-over of one day can run out (the clock moves two
//! days on and the queued jobs run), and the rule, the keys and the hand-overs are still alive when
//! the sample ends.

use bb_app::access::Permission;
use bb_app::agent_work::{
    AgentWorkQuery, AnswerQuestion, AskQuestion, AttachResult, CreateStandingRule, EndAttempt,
    HandOver, ReportAttempt, ReviewWork, TakeWork, answer_question, ask_question, attach_result,
    create_standing_rule, end_attempt, hand_over, issue_agent_work, report_attempt, review_work,
    take_work,
};
use bb_app::agents::{CreateAgent, create_agent};
use bb_app::auth::{CreateAccessKey, SignIn, create_access_key, sign_in};
use bb_app::comments::{AddComment, add_comment};
use bb_app::context::Clock as _;
use bb_app::roles::{CheckAccessQuery, check_access};
use bb_app::saved_searches::{SaveSearch, save_search};
use bb_core::access_key::KeyChoice;
use bb_core::agent_records::ResultKind;
use bb_core::agent_work::{CheckResult, MOST_HAND_OVER_DAYS, ReviewAnswer};
use bb_core::id::{AgentId, AgentNoteId, AgentWorkId, SavedSearchId, StandingRuleId};
use bb_core::saved_search::Presentation;
use bb_core::time::SECONDS_PER_DAY;
use bb_dev_words::sample::AiPlan;

use serde_json::{Value, json};

use crate::error::{SampleError, Step};
use crate::generate::{AgentKey, World, plain_text};
use crate::issues::Maker;
use crate::issues_setup::{id_in, json, picked};

/// How long before the end of the sample the agents begin to work.
const BEGINS_BEFORE_END: i64 = 10 * SECONDS_PER_DAY;
/// How long the hand-over that runs out lasts, in days.
const SHORT_HAND_OVER_DAYS: i64 = 1;
/// How long the clock moves on for that hand-over to run out and the queued jobs to run.
const MOVES_ON: i64 = 2 * SECONDS_PER_DAY;
/// How many issues the hand-overs use: one for each case of `Case`.
const ISSUES: usize = 10;
/// The minutes an attempt reports.
const MINUTES: i64 = 12;
/// The cost, in cents, an attempt reports.
const COST: i64 = 150;
/// The text of a comment an agent writes.
const COMMENT: &str = "I could reproduce this on a narrow screen; the details are in the result.";

/// What one hand-over is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    /// Nothing happens after the hand-over.
    Waiting,
    /// An attempt runs.
    Working,
    /// The agent asks a question and nobody answered.
    Asking,
    /// The agent shows a plan that waits for *Approve plan*.
    Planning,
    /// The attempt is ready and waits for the review.
    Ready,
    /// The agent gave up.
    GaveUp,
    /// The hand-over lasts a day and runs out.
    RunsOut,
    /// A question, an answer, an attempt sent back and one accepted.
    Accepted,
    /// An attempt that is thrown away.
    ThrownAway,
}

/// Which agent does the case (the place in the plan's `agents`), in the order the cases are made.
const CASES: [(usize, Case); 9] = [
    (0, Case::Waiting),
    (0, Case::Working),
    (0, Case::Asking),
    (1, Case::Planning),
    (1, Case::Ready),
    (1, Case::GaveUp),
    (0, Case::RunsOut),
    (0, Case::Accepted),
    (1, Case::ThrownAway),
];

fn failed(error: &dyn std::fmt::Display) -> SampleError {
    SampleError::step(Step::Agents, &error)
}

/// An agent that was made, with its key.
pub(crate) struct Made {
    pub(crate) id: AgentId,
    pub(crate) key: String,
}

impl Maker<'_, '_> {
    /// Makes the agents of the plan, hands issues over to them, lets them work, runs the jobs and
    /// returns the keys of the agents.
    pub(crate) async fn give_agents_work(
        &mut self,
        plan: &AiPlan,
    ) -> Result<(Vec<AgentKey>, u64), SampleError> {
        let begins = self.world.start.as_second() - BEGINS_BEFORE_END;
        self.world.move_clock_to(begins)?;
        let person = self
            .world
            .people
            .iter()
            .position(|made| made.person.name == plan.person)
            .ok_or_else(|| SampleError::NotMade(plan.person.clone()))?;
        let token = self.world.session_of(person).await?;
        let mut agents = Vec::new();
        for name in &plan.agents {
            agents.push(self.world.new_agent(&token, person, name).await?);
        }
        let issues = self.issues_of(&plan.project)?;
        for ((agent, case), (issue, _)) in CASES.iter().zip(&issues) {
            let made = agents.get(*agent).ok_or(SampleError::PersonMissing)?;
            self.world
                .carry_out_case(plan, &token, made, *case, issue)
                .await?;
        }
        self.world.standing_rule(plan, &token, &agents).await?;
        self.world
            .move_clock_to(self.world.clock.now().as_second() + MOVES_ON)?;
        self.world.run_the_jobs().await?;
        let unchangeable = self.world.cannot_change(person, &issues).await?;
        let email = self.world.people[person].person.email.clone();
        Ok((
            plan.agents
                .iter()
                .zip(agents)
                .map(|(agent, made)| AgentKey {
                    mark: plan.agent_key_mark.clone(),
                    agent: agent.clone(),
                    email: email.clone(),
                    key: made.key,
                })
                .collect(),
            unchangeable,
        ))
    }

    /// The issues of the project (and its sub-projects) that were made, the first [`ISSUES`].
    fn issues_of(&self, project: &str) -> Result<Vec<(String, String)>, SampleError> {
        let family = self
            .names
            .projects
            .iter()
            .position(|candidate| candidate.short_name == project)
            .ok_or_else(|| SampleError::NotMade(project.to_owned()))?;
        let found: Vec<(String, String)> = self
            .plan
            .issues
            .iter()
            .enumerate()
            .filter(|(_, planned)| planned.family == family)
            .filter_map(|(at, planned)| {
                let made = self.issues.get(at).and_then(Option::as_ref)?;
                Some((made.id.to_string(), planned.place.clone()))
            })
            .take(ISSUES)
            .collect();
        if found.len() < CASES.len() {
            return Err(SampleError::NotMade(project.to_owned()));
        }
        Ok(found)
    }
}

impl World<'_> {
    /// How many of the issues handed over the person may not change (*Check access* as the
    /// administrator asks it): a hand-over needs *Change issues* on each, so there must be none.
    async fn cannot_change(
        &self,
        person: usize,
        issues: &[(String, String)],
    ) -> Result<u64, SampleError> {
        let administrator = self.administrator()?;
        let token = self.session_of(administrator).await?;
        let mut cannot = 0;
        for (_, place) in issues.iter().take(CASES.len()) {
            let project = self.project_named(place).await?;
            let answer = json(
                check_access(
                    &self.services.ctx(self.request()),
                    CheckAccessQuery {
                        session_token: token.clone(),
                        person: self.people[person].id,
                        place: Some(project),
                        issue_type: None,
                    },
                )
                .await,
                Step::Agents,
            )?;
            let may = answer["permissions"].as_array().is_some_and(|all| {
                all.iter().any(|permission| {
                    permission["permission"] == Permission::ChangeIssues.code()
                        && permission["allowed"] == true
                })
            });
            cannot += u64::from(!may);
        }
        Ok(cannot)
    }

    /// A session of the person, signed in now.
    pub(crate) async fn session_of(&self, person: usize) -> Result<String, SampleError> {
        let made = self.people.get(person).ok_or(SampleError::PersonMissing)?;
        self.tick();
        let started = sign_in(
            &self.services.ctx(self.request()),
            SignIn {
                email: made.person.email.clone(),
                password: made.password.clone(),
                previous_session: None,
            },
        )
        .await
        .map_err(|error| SampleError::step(Step::SigningIn, &error))?;
        Ok(started.session_token)
    }

    /// An agent of the person, and its key (made as the person does, with the password).
    pub(crate) async fn new_agent(
        &self,
        token: &str,
        person: usize,
        name: &str,
    ) -> Result<Made, SampleError> {
        let ctx = self.services.ctx(self.request());
        self.tick();
        let made = picked(
            create_agent(
                &ctx,
                CreateAgent {
                    session_token: token.to_owned(),
                    name: name.to_owned(),
                },
            )
            .await,
            Step::Agents,
            |agent| json!({ "id": agent.id.to_string() }),
        )?;
        let id: AgentId = id_in(&made, "id")?;
        self.tick();
        let key = create_access_key(
            &ctx,
            CreateAccessKey {
                session_token: token.to_owned(),
                password: self.people[person].password.clone(),
                name: format!("{name} key"),
                choice: KeyChoice::SeeAndChange,
                permissions: Vec::new(),
                projects: Vec::new(),
                expires_at: None,
                agent: Some(id),
                renews: None,
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        let key = plain_text(key, |key| key.secret).ok_or(SampleError::KeyMissing)?;
        Ok(Made { id, key })
    }

    /// Hands the issue over to the agent and takes the case as far as it goes.
    async fn carry_out_case(
        &self,
        plan: &AiPlan,
        token: &str,
        agent: &Made,
        case: Case,
        issue: &str,
    ) -> Result<(), SampleError> {
        let work = self.hand(plan, token, agent, case, issue).await?;
        let key = agent.key.as_str();
        match case {
            Case::Waiting | Case::RunsOut => {}
            Case::Working => self.report_an_attempt(key, issue, None).await?,
            Case::Asking => {
                self.report_an_attempt(key, issue, None).await?;
                self.ask(key, issue, &plan.question).await?;
            }
            Case::Planning => self.report_an_attempt(key, issue, Some(&plan.plan)).await?,
            Case::Ready => self.finish(plan, key, issue, None).await?,
            Case::GaveUp => {
                self.report_an_attempt(key, issue, None).await?;
                self.end(key, issue, Some(plan.gave_up.clone())).await?;
            }
            Case::Accepted => {
                self.accepted_after_a_send_back(plan, token, key, issue, work)
                    .await?;
            }
            Case::ThrownAway => {
                self.finish(plan, key, issue, None).await?;
                self.review(token, work, issue, ReviewAnswer::ThrowAway, None)
                    .await?;
            }
        }
        Ok(())
    }

    /// The hand-over, and the ID of the work on the issue.
    async fn hand(
        &self,
        plan: &AiPlan,
        token: &str,
        agent: &Made,
        case: Case,
        issue: &str,
    ) -> Result<AgentWorkId, SampleError> {
        self.tick();
        let given = picked(
            hand_over(
                &self.services.ctx(self.request()),
                HandOver {
                    session_token: token.to_owned(),
                    agent: agent.id,
                    issues: vec![issue.to_owned()],
                    note: plan.note.clone(),
                    powers: Vec::new(),
                    fields: Vec::new(),
                    budget_attempts: None,
                    budget_minutes: None,
                    budget_cost: None,
                    plan_needs_approval: case == Case::Planning,
                    accept_by_itself: false,
                    days: Some(if case == Case::RunsOut {
                        SHORT_HAND_OVER_DAYS
                    } else {
                        MOST_HAND_OVER_DAYS
                    }),
                },
            )
            .await,
            Step::Agents,
            |given| json!({ "work": given.issues.first().map(|issue| issue.work.to_string()) }),
        )?;
        id_in(&given, "work")
    }

    /// The agent takes the work, which starts an attempt, and reports it (the first, or a new one
    /// after a send-back).
    pub(crate) async fn report_an_attempt(
        &self,
        key: &str,
        issue: &str,
        plan: Option<&str>,
    ) -> Result<(), SampleError> {
        self.tick();
        take_work(
            &self.services.ctx(self.request()),
            TakeWork {
                session_token: key.to_owned(),
                issue: issue.to_owned(),
                version: None,
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        self.tick();
        report_attempt(
            &self.services.ctx(self.request()),
            ReportAttempt {
                session_token: key.to_owned(),
                issue: issue.to_owned(),
                plan: plan.map(str::to_owned),
                minutes: MINUTES,
                cost: COST,
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        Ok(())
    }

    pub(crate) async fn ask(&self, key: &str, issue: &str, text: &str) -> Result<(), SampleError> {
        self.tick();
        ask_question(
            &self.services.ctx(self.request()),
            AskQuestion {
                session_token: key.to_owned(),
                issue: issue.to_owned(),
                text: text.to_owned(),
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        Ok(())
    }

    /// The agent ends the attempt: ready for the review, or given up with its reason.
    pub(crate) async fn end(
        &self,
        key: &str,
        issue: &str,
        gave_up: Option<String>,
    ) -> Result<(), SampleError> {
        self.tick();
        Box::pin(end_attempt(
            &self.services.ctx(self.request()),
            EndAttempt {
                session_token: key.to_owned(),
                issue: issue.to_owned(),
                gave_up,
            },
        ))
        .await
        .map_err(|error| failed(&error))?;
        Ok(())
    }

    /// An attempt from its report to its end: a comment, a passed check, ready for the review.
    pub(crate) async fn finish(
        &self,
        plan: &AiPlan,
        key: &str,
        issue: &str,
        note: Option<&str>,
    ) -> Result<(), SampleError> {
        self.report_an_attempt(key, issue, note).await?;
        self.tick();
        let ctx = self.services.ctx(self.request());
        add_comment(
            &ctx,
            AddComment {
                session_token: key.to_owned(),
                issue: issue.to_owned(),
                text: COMMENT.to_owned(),
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        self.tick();
        attach_result(
            &ctx,
            AttachResult {
                session_token: key.to_owned(),
                issue: issue.to_owned(),
                kind: ResultKind::Check,
                label: plan.check_label.clone(),
                link: None,
                outcome: Some(CheckResult::Passed),
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        self.end(key, issue, None).await
    }

    /// The question is answered, the first attempt is sent back and the second is accepted.
    async fn accepted_after_a_send_back(
        &self,
        plan: &AiPlan,
        token: &str,
        key: &str,
        issue: &str,
        work: AgentWorkId,
    ) -> Result<(), SampleError> {
        self.report_an_attempt(key, issue, None).await?;
        self.ask(key, issue, &plan.question).await?;
        let asked = self.work_of(token, issue).await?;
        let question: AgentNoteId = id_in(&asked, "question")?;
        self.tick();
        answer_question(
            &self.services.ctx(self.request()),
            AnswerQuestion {
                session_token: token.to_owned(),
                work,
                question,
                reply: plan.answer.clone(),
                version: asked["version"].as_i64().unwrap_or_default(),
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        self.end_with_a_failed_check(plan, key, issue).await?;
        self.review(
            token,
            work,
            issue,
            ReviewAnswer::SendBack,
            Some(&plan.sent_back_note),
        )
        .await?;
        self.finish(plan, key, issue, None).await?;
        self.review(token, work, issue, ReviewAnswer::Accept, None)
            .await
    }

    /// The first attempt goes on after the answer and ends with a check that failed.
    async fn end_with_a_failed_check(
        &self,
        plan: &AiPlan,
        key: &str,
        issue: &str,
    ) -> Result<(), SampleError> {
        self.tick();
        attach_result(
            &self.services.ctx(self.request()),
            AttachResult {
                session_token: key.to_owned(),
                issue: issue.to_owned(),
                kind: ResultKind::Check,
                label: plan.check_label.clone(),
                link: None,
                outcome: Some(CheckResult::Failed),
            },
        )
        .await
        .map_err(|error| failed(&error))?;
        self.end(key, issue, None).await
    }

    /// The work on the issue, as the person reads it.
    async fn work_of(&self, token: &str, issue: &str) -> Result<Value, SampleError> {
        let read = picked(
            issue_agent_work(
                &self.services.ctx(self.request()),
                AgentWorkQuery {
                    session_token: token.to_owned(),
                    issue: issue.to_owned(),
                },
            )
            .await,
            Step::Agents,
            |read| {
                let work = read.work.first();
                json!({
                    "version": work.map(|work| work.version),
                    "question": work.and_then(|work| work.questions.first()).map(|question| question.id.to_string()),
                })
            },
        )?;
        Ok(read)
    }

    /// The person answers the result of the work.
    async fn review(
        &self,
        token: &str,
        work: AgentWorkId,
        issue: &str,
        verdict: ReviewAnswer,
        note: Option<&str>,
    ) -> Result<(), SampleError> {
        let seen = self.work_of(token, issue).await?;
        self.tick();
        Box::pin(review_work(
            &self.services.ctx(self.request()),
            ReviewWork {
                session_token: token.to_owned(),
                work,
                verdict,
                note: note.map(str::to_owned),
                status: None,
                version: seen["version"].as_i64().unwrap_or_default(),
            },
        ))
        .await
        .map_err(|error| failed(&error))?;
        Ok(())
    }

    /// The standing rule of the plan: the person saves the search and the second agent gets the
    /// rule, which proposes and changes nothing.
    async fn standing_rule(
        &self,
        plan: &AiPlan,
        token: &str,
        agents: &[Made],
    ) -> Result<StandingRuleId, SampleError> {
        let ctx = self.services.ctx(self.request());
        self.tick();
        let saved = json(
            save_search(
                &ctx,
                SaveSearch {
                    session_token: token.to_owned(),
                    name: plan.rule.search.clone(),
                    text: plan.rule.text.clone(),
                    presentation: Presentation::default(),
                },
            )
            .await,
            Step::Agents,
        )?;
        let search: SavedSearchId = id_in(&saved, "id")?;
        let agent = agents.get(1).ok_or(SampleError::PersonMissing)?;
        let project = self.project_named(&plan.project).await?;
        self.tick();
        let rule = picked(
            create_standing_rule(
                &ctx,
                CreateStandingRule {
                    session_token: token.to_owned(),
                    agent: agent.id,
                    search,
                    project,
                    fields: Vec::new(),
                    creates_in: None,
                    days: Some(plan.rule.days),
                },
            )
            .await,
            Step::Agents,
            |rule| json!({ "id": rule.id.to_string() }),
        )?;
        id_in(&rule, "id")
    }
}
