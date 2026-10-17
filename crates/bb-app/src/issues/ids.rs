//! Issue IDs, counting or random (ISS-01a, ADR-0021): which mode a project is in, how the code of
//! a new issue is given out, and how every issue gets a new code when the mode changes.
//!
//! The mode is the setting `issues.id-mode`: the organization's value is the default, a project
//! can choose its own, and a sub-project follows its parent until it chooses (the layers of
//! ADR-0018). A code is given out inside the change that makes the issue, so two writers never
//! get the same one: a counting code comes from the counter of the project (the store raises it
//! in the transaction), a random one is drawn from the secure source and drawn again when the
//! project has it (or had it) already. When the mode of a project changes, every issue of it
//! gets a code of the new mode in the same change, and the old code stays with the issue.

use bb_core::ErrorCode;
use bb_core::event::{Actor, FieldChange, RecordKind};
use bb_core::id::{IdSource, JobId, ProjectId, UserId};
use bb_core::issue::IssueAlias;
use bb_core::many_changes::{MANY_CHANGES_JOB_ABOVE, MANY_CHANGES_STEP};
use bb_core::project::ShortName;
use bb_core::readable_id::{IdMode, IssueCode, ReadableId, code_length, longest_code};
use bb_core::settings::{
    ISSUES_ID_LENGTH, ISSUES_ID_MODE, Layers, Scope, SettingDefinition, SettingValue,
};
use bb_core::text::CodeDraws;
use bb_core::version::{FIRST_VERSION, counted_up};
use bb_policy::Tree;
use jiff::Timestamp;

use serde_json::Value;

use super::fields::member;
use crate::auth::{AppError, AuthStore, Ctx, EventMaker, entity, internal};
use crate::jobs::{kind, kind_name};

/// How many numbers a counting project tries before it gives up (which would mean the codes of the
/// project are not what the rules make them).
const COUNTING_TRIES: usize = 1000;

/// How the next codes of a project are given out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CodePlan {
    /// Counting or random.
    pub(crate) mode: IdMode,
    /// The length a random code starts with.
    pub(crate) length: usize,
}

impl CodePlan {
    /// The plan that counts for the project whose layers these are. A value that is not a mode
    /// counts as the default mode (the registry accepts nothing else, so this is a stored value
    /// that was made by hand).
    pub(crate) fn of(layers: &Layers) -> Self {
        let parse = |text: String| IdMode::from_code(&text);
        let mode = parse(layers.text(&ISSUES_ID_MODE))
            .or_else(|| ISSUES_ID_MODE.default_value().ok().and_then(parse))
            // The default of the registry is one of the modes: this is never reached.
            .unwrap_or(IdMode::ALL[0]);
        let length = layers.number(&ISSUES_ID_LENGTH);
        Self {
            mode,
            length: code_length(length),
        }
    }
}

/// The new code of an issue of a project, given out inside the change that needs it.
///
/// # Errors
/// The error of the store, or its "aborted" error when even the longest random codes were all
/// taken (which would mean the random source is broken).
pub(crate) async fn new_code_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    project: ProjectId,
    plan: CodePlan,
    source: &mut impl IdSource,
) -> Result<IssueCode, S::Error> {
    match plan.mode {
        IdMode::Counting => {
            // The next number, unless the code is taken (it cannot be: a random code holds a
            // letter and a number is given out once; a code that is taken is skipped all the same).
            for _ in 0..COUNTING_TRIES {
                let code =
                    IssueCode::counting(S::next_counting_number(transaction, project).await?);
                if !S::issue_code_taken_in(transaction, project, code.clone()).await? {
                    return Ok(code);
                }
            }
            Err(S::aborted())
        }
        IdMode::Random => {
            for symbols in CodeDraws::new(source, plan.length, longest_code()) {
                let Some(code) = IssueCode::random(&symbols) else {
                    continue;
                };
                if !S::issue_code_taken_in(transaction, project, code.clone()).await? {
                    return Ok(code);
                }
            }
            Err(S::aborted())
        }
    }
}

/// A project whose issues get a new code of a new mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Renewal {
    pub(crate) project: ProjectId,
    /// Its short name now, which starts the old and the new ID in the event.
    pub(crate) short_name: ShortName,
    pub(crate) plan: CodePlan,
}

/// What happens to the value of the mode on a project.
#[derive(Clone, Debug)]
pub(crate) enum ModeChange {
    /// The project chooses this value.
    Set(SettingValue),
    /// The project goes back to the value of the layer above (*Reset to default*).
    Reset,
}

/// The layers of `project` as they would be after `change` of the mode on `on`.
fn after(
    layers: &Layers,
    on: ProjectId,
    definition: &SettingDefinition,
    change: &ModeChange,
) -> Layers {
    match change {
        ModeChange::Reset => {
            let mut lower = layers.clone();
            for (id, values) in &mut lower.projects {
                if *id == on {
                    values.remove(definition.key());
                }
            }
            lower
        }
        ModeChange::Set(value) => {
            let mut set = layers.clone();
            for (id, values) in &mut set.projects {
                if *id == on {
                    values.insert(definition.key().to_owned(), value.clone());
                }
            }
            set
        }
    }
}

/// How the codes of `project` are given out, read inside the change that gives them out, after
/// the lock on the codes of the project.
///
/// # Errors
/// The error of the store.
pub(crate) async fn code_plan_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    project: ProjectId,
) -> Result<CodePlan, S::Error> {
    let layers = S::setting_layers_in(transaction, Some(project), None).await?;
    Ok(CodePlan::of(&layers))
}

/// The projects whose issues get new codes when `change` is made to the setting with this `key` on
/// `scope`, judged inside the change: only the mode of a project changes any, and then the
/// project itself and the projects inside it that follow it do, each with a code of its new mode.
/// The codes of all of them are locked first (in the order of their IDs), so a change that makes
/// an issue meanwhile sees the old mode or the new one, never a mix, and two changes of the mode
/// do not renew twice.
///
/// # Errors
/// The error of the store.
pub(crate) async fn plan_renewals_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    scope: Scope,
    key: &str,
    change: &ModeChange,
) -> Result<Vec<Renewal>, S::Error> {
    let Scope::Project(on) = scope else {
        return Ok(Vec::new());
    };
    if key != ISSUES_ID_MODE.definition().key() {
        return Ok(Vec::new());
    }
    let projects = S::project_tree_in(transaction).await?;
    let mut affected = Tree::of(&projects).subtree(on);
    affected.sort();
    for project in &affected {
        S::lock_issue_codes(transaction, *project).await?;
    }
    let mut renewals = Vec::new();
    for project in affected {
        let layers = S::setting_layers_in(transaction, Some(project), None).await?;
        let (before, to) = (
            CodePlan::of(&layers),
            CodePlan::of(&after(&layers, on, ISSUES_ID_MODE.definition(), change)),
        );
        if let Some(found) = projects.iter().find(|found| found.id == project)
            && before.mode != to.mode
        {
            renewals.push(Renewal {
                project,
                short_name: found.short_name.clone(),
                plan: to,
            });
        }
    }
    Ok(renewals)
}

/// Gives every issue of the project, also those in the trash, a code of the new mode, keeps the
/// old code with the issue (`issue_alias`) and writes an event for each issue (its ID changed, by
/// the person who changed the mode), inside the change that changed the mode.
///
/// # Errors
/// The error of the store.
pub(crate) async fn renew_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    renewal: &Renewal,
    source: &mut impl IdSource,
    events: &EventMaker,
    at: Timestamp,
) -> Result<(), S::Error> {
    renew_step_in::<S>(transaction, renewal, source, events, at, usize::MAX)
        .await
        .map(drop)
}

/// The same for at most `limit` issues: the ones that do not have a code of the new mode yet, in
/// the order of their IDs. Says how many it renewed, so a big project is renewed a step at a time
/// (`renewal_step`) and a step that finds nothing to do is the last.
///
/// # Errors
/// The error of the store.
pub(crate) async fn renew_step_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    renewal: &Renewal,
    source: &mut impl IdSource,
    events: &EventMaker,
    at: Timestamp,
    limit: usize,
) -> Result<usize, S::Error> {
    let mut renewed = 0;
    for (issue, old) in S::issue_codes_in(transaction, renewal.project).await? {
        if renewed >= limit {
            break;
        }
        if old.mode() == renewal.plan.mode {
            continue;
        }
        let code = new_code_in::<S>(transaction, renewal.project, renewal.plan, source).await?;
        S::set_issue_code(transaction, issue, &code).await?;
        S::insert_issue_alias(
            transaction,
            &IssueAlias {
                project: renewal.project,
                code: old.clone(),
                issue,
                retired_at: at,
            },
        )
        .await?;
        let version = S::issue_in(transaction, issue)
            .await?
            .map_or(FIRST_VERSION, |found| found.version);
        let key = |code: IssueCode| {
            serde_json::json!(ReadableId::new(renewal.short_name.clone(), code).to_string())
        };
        let event = events
            .create(
                entity(RecordKind::Issue, issue),
                vec![
                    FieldChange {
                        field: member::KEY.to_owned(),
                        before: Some(key(old)),
                        after: Some(key(code)),
                    },
                    counted_up(version),
                ],
            )
            .map_err(|_| S::aborted())?;
        S::append_follow_up_event(transaction, &event).await?;
        renewed += 1;
    }
    Ok(renewed)
}

/// How many issues of the project, also those in the trash, do not have a code of the plan's mode
/// yet, read inside a change.
///
/// # Errors
/// The error of the store.
pub(crate) async fn still_to_renew_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    renewal: &Renewal,
) -> Result<usize, S::Error> {
    Ok(S::issue_codes_in(transaction, renewal.project)
        .await?
        .iter()
        .filter(|(_, code)| code.mode() != renewal.plan.mode)
        .count())
}

/// The member of the payload of the job that renews the IDs of a project: the project.
const PROJECT_KEY: &str = "project";

/// The member of the payload of that job: the person who changed the mode, whose name is on the
/// events of the renewed IDs.
const PERSON_KEY: &str = "by";

/// Renews the IDs of the issues of a project after its mode changed, inside the change that changed
/// it: all of them when the project has no more issues than a change to many issues waits for
/// (`MANY_CHANGES_JOB_ABOVE`), and otherwise a first step, with a job that goes on with the rest a step at
/// a time (M3-F27). Issues made meanwhile already get codes of the new mode.
///
/// # Errors
/// The error of the store.
pub(crate) async fn renew_or_queue_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    renewal: &Renewal,
    source: &mut (impl IdSource + Send),
    events: &EventMaker,
    person: UserId,
    at: Timestamp,
) -> Result<(), S::Error> {
    if still_to_renew_in::<S>(transaction, renewal).await? <= MANY_CHANGES_JOB_ABOVE {
        return renew_in::<S>(transaction, renewal, source, events, at).await;
    }
    renew_step_in::<S>(transaction, renewal, source, events, at, MANY_CHANGES_STEP).await?;
    let job = JobId::generate(source).map_err(|_| S::aborted())?;
    let name = kind_name(kind::RENEW_IDS).map_err(|_| S::aborted())?;
    let payload = serde_json::json!({
        PROJECT_KEY: renewal.project.to_string(),
        PERSON_KEY: person.to_string(),
    });
    S::enqueue_job(transaction, job, &name, &payload, at, at).await
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The renewal of the IDs of a project gave up (its job is on the dead-letter list): the issues
    /// keep the IDs they have, and the person who changed the mode is told by a notification
    /// (M3-F150). Written as an event of the project, which the notifications reader turns into the
    /// note for that person.
    ///
    /// # Errors
    /// The error of the store.
    pub(crate) async fn renewal_gave_up(&self, payload: &Value) -> Result<(), AppError> {
        let named = |key: &str| payload.get(key).and_then(Value::as_str);
        let (Some(project), Some(person)) = (
            named(PROJECT_KEY).and_then(|text| text.parse::<ProjectId>().ok()),
            named(PERSON_KEY).and_then(|text| text.parse::<UserId>().ok()),
        ) else {
            return Ok(());
        };
        let event = self.event(
            Actor::System,
            entity(RecordKind::Project, project),
            vec![FieldChange {
                field: member::ISSUE_IDS_FAILED.to_owned(),
                before: None,
                after: Some(serde_json::json!(person.to_string())),
            }],
        )?;
        self.store()
            .record_change(&event, |_| Box::pin(async { Ok(()) }))
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Does the next step of the renewal of the IDs of a project, as the job of the queue says: the
    /// issues that do not have a code of the mode the project has now, a hundred at a time, and
    /// when some are left the next step is queued. A project whose mode changed again meanwhile is
    /// renewed to the mode it has now.
    ///
    /// # Errors
    /// `request.invalid` for a payload that names no project or person, and the error of the store.
    pub(crate) async fn renew_ids_step(&self, payload: &Value) -> Result<(), AppError> {
        let named = |key: &str| payload.get(key).and_then(Value::as_str);
        let project: ProjectId = named(PROJECT_KEY)
            .and_then(|text| text.parse().ok())
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        let person: UserId = named(PERSON_KEY)
            .and_then(|text| text.parse().ok())
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        let Some(found) = self
            .store()
            .project(project)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(());
        };
        let layers = self
            .store()
            .setting_layers(Some(project), None)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let renewal = Renewal {
            project,
            short_name: found.short_name,
            plan: CodePlan::of(&layers),
        };
        let counted = renewal.clone();
        let left = self
            .store()
            .read_snapshot(move |transaction| {
                Box::pin(async move { still_to_renew_in::<S>(transaction, &counted).await })
            })
            .await
            .map_err(|error| internal::<S>(&error))?;
        if left == 0 {
            return Ok(());
        }
        // The project says once that its IDs were renewed, in the step that renews the last; the
        // steps before write the events of the issues they renew and nothing else.
        let finishing = left <= MANY_CHANGES_STEP;
        let said = if finishing {
            vec![self.event(
                Actor::Person(person),
                entity(RecordKind::Project, project),
                vec![FieldChange {
                    field: member::ISSUE_IDS.to_owned(),
                    before: None,
                    after: Some(serde_json::json!(renewal.plan.mode.code())),
                }],
            )?]
        } else {
            Vec::new()
        };
        let (mut source, events, now) = (
            self.owned_ids(),
            self.event_maker(Actor::Person(person)),
            self.now(),
        );
        self.store()
            .record_changes(&said, move |transaction| {
                Box::pin(async move {
                    S::lock_issue_codes(transaction, renewal.project).await?;
                    renew_step_in::<S>(
                        transaction,
                        &renewal,
                        &mut source,
                        &events,
                        now,
                        MANY_CHANGES_STEP,
                    )
                    .await
                    .map(drop)
                })
            })
            .await
            .map_err(|error| internal::<S>(&error))?;
        if !finishing {
            let job = JobId::generate(&mut self.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?;
            self.store()
                .queue_job(job, &kind_name(kind::RENEW_IDS)?, payload, now, now)
                .await
                .map_err(|error| internal::<S>(&error))?;
        }
        Ok(())
    }
}
