//! The part of the canonical dump that is about AI and agents (M7 WP-13): how AI is set up, what is
//! never sent to it, the agents with their work in every state, the standing rules and the marks of
//! text an agent wrote. Only counts and names: IDs are random by design.
//!
//! TEST-SAMPLE-DATA holds these numbers against the plan in `tools/dev/sample.json`
//! (`bb_dev_words::sample::AiPlan`) and the rules of ADR-0029.

use std::collections::{BTreeMap, BTreeSet};

use bb_app::access::{PolicyStore as _, StartingRole};
use bb_app::agent_work::{AgentWorkRecords as _, GrantStore as _};
use bb_app::agents::AgentStore as _;
use bb_app::ai::{AiStore as _, NeverSentStore as _};
use bb_app::auth::OAuthStore as _;
use bb_app::fields::FieldStore as _;
use bb_app::projects::ProjectStore as _;
use bb_app::runs::RunRecordStore as _;
use bb_app::saved_searches::SavedSearchStore as _;
use bb_app::settings::SettingsStore as _;
use bb_core::agent_work::{AgentWorkState, ReviewAnswer};
use bb_core::ai::NeverSentSubject;
use bb_core::event::{Actor, DomainEvent, EntityKind, RecordKind};
use bb_core::fields::Field;
use bb_core::id::{AgentId, EntityId, ProjectId};
use bb_core::mark::WrittenBy;
use bb_core::settings::{AGENTS_RULES_NEVER_RUN_OUT, AI_ENABLED, SettingValue};
use serde::Serialize;

use crate::error::SampleError;
use crate::generate::Sample;

/// More than any sample has.
const LIMIT: u64 = 100_000;

/// An agent of the sample.
#[derive(Debug, Serialize)]
struct DumpAgent {
    name: String,
    /// The name of the person it works for.
    person: String,
    keys: u64,
    state: &'static str,
}

/// The agent work of the sample.
#[derive(Debug, Default, Serialize)]
struct DumpAgentWork {
    by_state: BTreeMap<String, u64>,
    waiting_with_question: u64,
    waiting_with_plan: u64,
    answered_questions: u64,
    sent_back: u64,
    accepted: u64,
    thrown_away: u64,
    results: u64,
    /// Pieces of work on an issue the person of the agent could not change when it was given.
    on_issues_the_person_cannot_change: u64,
}

/// A standing rule.
#[derive(Debug, Serialize)]
struct DumpRule {
    search: String,
    /// How the rule works: `proposals` or `hand-over`.
    way: &'static str,
    /// How many days after the start of the sample it runs out; none for a rule that never does.
    days_ahead: Option<i64>,
    /// The members of an issue its hand-overs may change, by their keys.
    fields: Vec<String>,
    names_a_people_field: bool,
    matches: u64,
}

/// The marks of text an agent wrote.
#[derive(Debug, Default, Serialize)]
struct DumpMarks {
    /// Texts an agent wrote, by the events.
    agent: u64,
    /// Of those, the ones whose mark names the agent of the event.
    matching_events: u64,
}

/// AI and agents.
#[derive(Debug, Default, Serialize)]
pub(crate) struct DumpAi {
    enabled: bool,
    agreement_confirmed: bool,
    projects_on: Vec<String>,
    projects_off: Vec<String>,
    never_sent_projects: Vec<String>,
    never_sent_fields: Vec<String>,
    never_sent_own_fields: Vec<String>,
    agents: Vec<DumpAgent>,
    agents_with_a_role: u64,
    agents_assigned: u64,
    work: DumpAgentWork,
    rules: Vec<DumpRule>,
    /// Whether the organization allows standing rules that never run out.
    rules_never_run_out: bool,
    /// AI apps people allowed, by grants that have not run out.
    apps: u64,
    marks: DumpMarks,
}

impl Sample {
    /// Reads AI and agents from the store and counts the marks in the events.
    pub(crate) async fn ai_of(&self, events: &[DomainEvent]) -> Result<DumpAi, SampleError> {
        let mut ai = DumpAi::default();
        self.ai_setup_of(&mut ai).await?;
        let agents = self.storage.all_agents(None, LIMIT).await?;
        self.agents_of(&mut ai, events, &agents).await?;
        self.rules_of(&mut ai).await?;
        self.apps_of(&mut ai).await?;
        self.marks_of(&mut ai, events).await?;
        Ok(ai)
    }

    /// The settings of AI, the projects that switch it on or off, and what is never sent to it.
    async fn ai_setup_of(&self, ai: &mut DumpAi) -> Result<(), SampleError> {
        let organization = self.storage.setting_layers(None, None).await?;
        ai.enabled = organization.yes(&AI_ENABLED);
        ai.rules_never_run_out = organization.yes(&AGENTS_RULES_NEVER_RUN_OUT);
        ai.agreement_confirmed = self.storage.ai_agreement().await?.is_some();
        let projects: BTreeMap<ProjectId, String> = self
            .storage
            .project_tree()
            .await?
            .into_iter()
            .map(|project| (project.id, project.short_name.to_string()))
            .collect();
        let key = AI_ENABLED.definition().key();
        for (id, short_name) in &projects {
            let layers = self.storage.setting_layers(Some(*id), None).await?;
            let own = layers
                .projects
                .iter()
                .find(|(project, _)| project == id)
                .and_then(|(_, values)| values.get(key));
            match own {
                Some(SettingValue::Bool(true)) => ai.projects_on.push(short_name.clone()),
                Some(SettingValue::Bool(false)) => ai.projects_off.push(short_name.clone()),
                _ => {}
            }
        }
        for mark in self.storage.never_sent_marks().await? {
            if !mark.marked {
                continue;
            }
            match mark.subject {
                NeverSentSubject::Field(field) => {
                    ai.never_sent_fields.push(field.code().to_owned());
                }
                NeverSentSubject::Project(id) => {
                    ai.never_sent_projects.extend(projects.get(&id).cloned());
                }
            }
        }
        ai.projects_on.sort();
        ai.projects_off.sort();
        ai.never_sent_own_fields = self
            .storage
            .field_definitions()
            .await?
            .into_iter()
            .filter(|definition| definition.field.never_to_assistant)
            .map(|definition| definition.field.name)
            .collect();
        ai.never_sent_projects.sort();
        ai.never_sent_fields.sort();
        ai.never_sent_own_fields.sort();
        Ok(())
    }

    /// The agents, what holds them back from roles and *Assigned to*, and their work.
    async fn agents_of(
        &self,
        ai: &mut DumpAi,
        events: &[DomainEvent],
        agents: &[bb_core::agent::Agent],
    ) -> Result<(), SampleError> {
        let names: BTreeMap<_, _> = self
            .people
            .iter()
            .map(|made| (made.id, made.person.name.clone()))
            .collect();
        let mut rows = self.storage.assignments_on(None).await?;
        for project in self.storage.project_tree().await? {
            rows.extend(self.storage.assignments_on(Some(project.id)).await?);
        }
        let assigned = RecordKind::Issue.code();
        for agent in agents {
            let id = agent.id.to_string();
            ai.agents.push(DumpAgent {
                name: agent.name.clone(),
                person: names.get(&agent.person).cloned().unwrap_or_default(),
                keys: self.storage.agent_key_count(agent.id).await?.unsigned_abs(),
                state: agent.state.code(),
            });
            ai.agents_with_a_role += rows
                .iter()
                .filter(|row| {
                    row.person
                        .is_some_and(|p| p.to_bytes() == agent.id.to_bytes())
                })
                .count() as u64;
            ai.agents_assigned += events
                .iter()
                .filter(|event| event.entity.kind.as_str() == assigned)
                .flat_map(|event| &event.changes)
                .filter(|change| {
                    change.field == Field::AssignedTo.code()
                        && change
                            .after
                            .as_ref()
                            .is_some_and(|after| after.to_string().contains(&id))
                })
                .count() as u64;
            self.work_of_agent_into(ai, agent.id).await?;
        }
        ai.agents.sort_by(|one, other| one.name.cmp(&other.name));
        Ok(())
    }

    /// The work of one agent, by state, with its questions, plans, reviews and results.
    async fn work_of_agent_into(&self, ai: &mut DumpAi, agent: AgentId) -> Result<(), SampleError> {
        let work = &mut ai.work;
        for piece in self.storage.work_of_agent(agent, None, LIMIT).await? {
            *work
                .by_state
                .entry(piece.state.code().to_owned())
                .or_default() += 1;
            let questions = self.storage.questions_of(piece.id).await?;
            let attempts = self.storage.attempts_of(piece.id).await?;
            if piece.state == AgentWorkState::WaitingForYou {
                work.waiting_with_question +=
                    u64::from(questions.iter().any(|question| question.answer.is_none()));
                work.waiting_with_plan += u64::from(
                    attempts
                        .last()
                        .is_some_and(|attempt| attempt.plan.is_some() && !attempt.plan_approved),
                );
            }
            work.answered_questions += questions
                .iter()
                .filter(|question| question.answer.is_some())
                .count() as u64;
            for review in self.storage.reviews_of(piece.id).await? {
                match review.answer {
                    ReviewAnswer::Accept => work.accepted += 1,
                    ReviewAnswer::SendBack => work.sent_back += 1,
                    ReviewAnswer::ThrowAway => work.thrown_away += 1,
                }
            }
            for attempt in &attempts {
                work.results += self.storage.results_of(attempt.id).await?.len() as u64;
            }
        }
        work.on_issues_the_person_cannot_change = self.left_behind.agent_work_unchangeable;
        for state in AgentWorkState::ALL {
            ai.work.by_state.entry(state.code().to_owned()).or_default();
        }
        Ok(())
    }

    /// The standing rules that have not ended.
    async fn rules_of(&self, ai: &mut DumpAi) -> Result<(), SampleError> {
        for rule in self.storage.open_rules().await? {
            let search = self
                .storage
                .saved_search(rule.search)
                .await?
                .map(|search| search.name)
                .unwrap_or_default();
            let matches = self.storage.matches_of(rule.id, None, LIMIT).await?;
            let ahead = rule.ends_at.map(|end| {
                (end.as_second() - self.options.start.as_second())
                    .div_euclid(bb_core::time::SECONDS_PER_DAY)
            });
            ai.rules.push(DumpRule {
                search,
                way: rule.way.code(),
                days_ahead: ahead,
                names_a_people_field: rule.fields.iter().any(|field| {
                    Field::from_code(field).is_some_and(|field| field.person_kind().is_some())
                }),
                fields: rule.fields,
                matches: matches.len() as u64,
            });
        }
        Ok(())
    }

    /// The AI apps Contributors allowed.
    async fn apps_of(&self, ai: &mut DumpAi) -> Result<(), SampleError> {
        for made in self
            .people
            .iter()
            .filter(|made| made.person.role == StartingRole::Contributor)
        {
            ai.apps += self
                .storage
                .grants_of(made.id, self.options.start, LIMIT)
                .await?
                .len() as u64;
        }
        Ok(())
    }

    /// Every text an agent wrote has the mark *Written by an agent* naming that agent.
    async fn marks_of(&self, ai: &mut DumpAi, events: &[DomainEvent]) -> Result<(), SampleError> {
        let mut written: BTreeMap<EntityId, AgentId> = BTreeMap::new();
        for event in events {
            if let Actor::Agent { agent, .. } = event.actor
                && event.entity.kind.as_str() == RecordKind::Comment.code()
                && event.changes.iter().any(|change| change.field == "text")
            {
                written.insert(event.entity.id, agent);
            }
        }
        let ids: Vec<EntityId> = written.keys().copied().collect();
        let marks = self
            .storage
            .text_marks(&EntityKind::from(RecordKind::Comment), &ids)
            .await?;
        ai.marks.agent = written.len() as u64;
        let named: BTreeSet<EntityId> = marks
            .iter()
            .filter(|mark| {
                mark.written_by == WrittenBy::Agent
                    && written.get(&mark.entity_id) == mark.agent.as_ref()
            })
            .map(|mark| mark.entity_id)
            .collect();
        ai.marks.matching_events = named.len() as u64;
        Ok(())
    }
}
