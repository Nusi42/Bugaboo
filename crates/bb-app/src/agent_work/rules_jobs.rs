//! The jobs of standing rules (AI-32, ADR-0029): a rule that ran out, the warning three days before,
//! and the look at what newly comes under the rules of a project.

use bb_core::ErrorCode;
use bb_core::agent_records::{GivingState, StandingRule as RuleRecord};
use bb_core::agent_work::{MOST_RULE_DAYS, RuleWay, rule_end};
use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::id::{IssueId, StandingRuleId};
use serde_json::json;

use super::members::{member, payload};
use super::queue::queue;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, entity, internal, nothing_if_aborted, set_to};
use crate::search::SearchScope;
use bb_core::agent_records::RuleMatch;

/// How many issues one look at a rule reads at once.
const MATCH_PAGE: u32 = 100;

impl<S: AuthStore> Ctx<'_, S> {
    /// The work of the job of a rule that ran out: ends the rule if it is the end the job was
    /// queued for. A rule that was renewed, stopped or is not there leaves nothing to do.
    ///
    /// # Errors
    /// `request.invalid` for a payload that names no rule, an internal error when the store fails.
    pub(crate) async fn run_rule_runs_out(&self, job: &serde_json::Value) -> Result<(), AppError> {
        let Some(rule) = self.rule_of_job(job).await? else {
            return Ok(());
        };
        // Only a rule with an end that has come runs out.
        let Some(ends_at) = rule.ends_at.filter(|end| *end <= self.now()) else {
            return Ok(());
        };
        let event = self.event(
            EventActor::System,
            entity(RecordKind::StandingRule, rule.id),
            vec![
                set_to(member::STATE, json!(GivingState::Ended.code())),
                bb_core::version::counted_up(rule.version),
            ],
        )?;
        let (id, ends_at, version) = (rule.id, Some(ends_at), rule.version);
        let result = self
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    if S::set_rule_end(transaction, id, ends_at, true, version).await? {
                        Ok(())
                    } else {
                        Err(S::aborted())
                    }
                })
            })
            .await;
        nothing_if_aborted::<S>(result)
    }

    /// The work of the job three days before a rule runs out: writes the event the notification
    /// *A standing rule runs out soon* is built from, once, if the rule still ends then.
    ///
    /// # Errors
    /// `request.invalid` for a payload that names no rule, an internal error when the store fails.
    pub(crate) async fn run_rule_about_to_run_out(
        &self,
        job: &serde_json::Value,
    ) -> Result<(), AppError> {
        let Some(rule) = self.rule_of_job(job).await? else {
            return Ok(());
        };
        let event = self.event(
            EventActor::System,
            entity(RecordKind::StandingRule, rule.id),
            vec![
                set_to(member::STATE, json!("about-to-run-out")),
                set_to(member::AGENT, json!(rule.agent.to_string())),
                set_to(
                    member::ENDS_AT,
                    json!(rule.ends_at.map(|end| end.to_string())),
                ),
            ],
        )?;
        self.store()
            .record_change(&event, move |_| Box::pin(async move { Ok(()) }))
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// The rule a job names, when it is still open and still ends when the job was queued for.
    async fn rule_of_job(&self, job: &serde_json::Value) -> Result<Option<RuleRecord>, AppError> {
        let id: StandingRuleId = job
            .get(payload::RULE)
            .and_then(serde_json::Value::as_str)
            .and_then(|text| text.parse().ok())
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        let ends: Option<jiff::Timestamp> = job
            .get(payload::ENDS_AT)
            .and_then(serde_json::Value::as_str)
            .and_then(|text| text.parse().ok());
        Ok(self
            .store()
            .standing_rule(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|rule| {
                rule.state == GivingState::Open && ends.is_some() && ends == rule.ends_at
            }))
    }

    /// The work of the job that writes down what newly came under the rules of a project: it runs
    /// each open rule's saved search as the person who made the rule, and writes down each issue
    /// that is new to the rule, with the moment. An issue the agent may not read is not written
    /// down.
    ///
    /// # Errors
    /// `request.invalid` for a payload that names no project, an internal error when the store
    /// fails.
    pub(crate) async fn run_rule_matches(&self, job: &serde_json::Value) -> Result<(), AppError> {
        let project: bb_core::id::ProjectId = job
            .get(payload::PROJECT)
            .and_then(serde_json::Value::as_str)
            .and_then(|text| text.parse().ok())
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        let rules = self
            .store()
            .open_rules_in(project)
            .await
            .map_err(|error| internal::<S>(&error))?;
        for rule in rules.into_iter().filter(|rule| rule.works_at(self.now())) {
            self.match_rule(&rule).await?;
        }
        Ok(())
    }

    async fn match_rule(&self, rule: &RuleRecord) -> Result<(), AppError> {
        let failed = |error| internal::<S>(&error);
        let Some(person) = self.person_known(rule.person).await? else {
            return Ok(());
        };
        let agent_on = self.store().agent(rule.agent).await.map_err(failed)?;
        if !agent_on
            .as_ref()
            .is_some_and(|agent| agent.works_for(rule.person))
        {
            return Ok(());
        }
        let actor = Actor { person, key: None };
        let opened = self.open_search(&actor, rule.search).await;
        let Ok(opened) = opened else {
            return Ok(());
        };
        let text = opened.stored.search.text.clone();
        let mut prepared = self
            .prepare_search(&actor, &text, SearchScope::Within(rule.project))
            .await?;
        let mut fresh: Vec<IssueId> = Vec::new();
        prepared.request.limit = u64::from(MATCH_PAGE);
        loop {
            let hits = self
                .store()
                .search_page(&prepared.request)
                .await
                .map_err(crate::search::stopped)?;
            let last = hits.last().map(|hit| hit.issue.id);
            // Which of this page the rule knows already: read for the page, never for all.
            let page: Vec<IssueId> = hits.iter().map(|hit| hit.issue.id).collect();
            let known = self
                .store()
                .known_matches(rule.id, page.clone())
                .await
                .map_err(failed)?;
            fresh.extend(page.into_iter().filter(|issue| !known.contains(issue)));
            match last {
                Some(last) if hits.len() as u64 >= prepared.request.limit => {
                    prepared.request.after = Some(last);
                }
                _ => break,
            }
        }
        if fresh.is_empty() {
            return Ok(());
        }
        match rule.way {
            RuleWay::Propose => self.write_down_found(rule, &fresh).await,
            RuleWay::HandOver => {
                let agent = agent_on.ok_or(AppError::Refused(ErrorCode::NotFound))?;
                self.hand_over_found(rule, &actor, &agent, fresh).await
            }
        }
    }

    /// Writes down that these issues came under the rule, with the moment, in one change with the
    /// event that says so.
    pub(super) async fn write_down_found(
        &self,
        rule: &RuleRecord,
        fresh: &[IssueId],
    ) -> Result<(), AppError> {
        let now = self.now();
        let event = self.event(
            EventActor::System,
            entity(RecordKind::StandingRule, rule.id),
            vec![set_to(
                member::ISSUES,
                json!(fresh.iter().map(ToString::to_string).collect::<Vec<_>>()),
            )],
        )?;
        let rule_id = rule.id;
        let fresh = fresh.to_vec();
        self.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    for issue in &fresh {
                        S::insert_match(
                            transaction,
                            &RuleMatch {
                                rule: rule_id,
                                issue: *issue,
                                at: now,
                            },
                        )
                        .await?;
                    }
                    Ok(())
                })
            })
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// The work of the job that follows the switching off of *Allow rules that never run out*:
    /// each open rule with no end gets one, the most days (90) from now, with the jobs of an end,
    /// so the usual warning comes before it runs out. A setting that is on again by now leaves the
    /// rules as they are.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn run_rules_get_an_end(&self) -> Result<(), AppError> {
        if self.rules_never_run_out().await? {
            return Ok(());
        }
        let now = self.now();
        let ends_at = rule_end(now, Some(MOST_RULE_DAYS))
            .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?;
        let open = self
            .store()
            .open_rules()
            .await
            .map_err(|error| internal::<S>(&error))?;
        for rule in open.into_iter().filter(|rule| rule.ends_at.is_none()) {
            let next = RuleRecord {
                ends_at: Some(ends_at),
                version: rule.version + 1,
                ..rule.clone()
            };
            let jobs = self.rule_jobs(&next)?;
            let event = self.event(
                EventActor::System,
                entity(RecordKind::StandingRule, rule.id),
                vec![
                    set_to(member::ENDS_AT, json!(ends_at.to_string())),
                    bb_core::version::counted_up(rule.version),
                ],
            )?;
            let (id, version) = (rule.id, rule.version);
            let result = self
                .store()
                .record_change(&event, move |transaction| {
                    Box::pin(async move {
                        if !S::set_rule_end(transaction, id, Some(ends_at), false, version).await? {
                            return Err(S::aborted());
                        }
                        for job in &jobs {
                            queue::<S>(transaction, job, now).await?;
                        }
                        Ok(())
                    })
                })
                .await;
            nothing_if_aborted::<S>(result)?;
        }
        Ok(())
    }
}
