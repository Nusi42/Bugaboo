//! The one registry of tools (AI-21, ADR-0013): a table, and the dispatch. The assistant, AI apps and
//! agents list and call tools only through it, so they cannot differ in what a tool is or may do.
//!
//! Each entry names the tool, whether it reads or changes (which fixes the scope it needs:
//! [`ToolEffect::scope`]), the use case it runs as the actor, the schema of its input, which is
//! generated from the Rust type of that input, and the function that runs it: for a tool that reads,
//! what it does; for a tool that changes, its preview (`preview.rs`). The call goes through that
//! function pointer and nowhere else, so what the table says is what runs. What is designed out is
//! not in the table and cannot be added without a decision record: a tool that ranks, evaluates or
//! picks people, reads working hours, other people's time or roles (COMP-10). ARCH-AI-TOOLS holds
//! the table to that.

use std::future::Future;
use std::pin::Pin;

use bb_core::ErrorCode;
use bb_core::ai::{Tool, ToolCaller, ToolEffect, ToolScope};
use serde::de::DeserializeOwned;
use serde_json::Value;
use utoipa::PartialSchema;

use super::inputs::{
    AddCommentInput, AskInput, AttachInput, ChangeStatusInput, ChangesSinceInput, CreateIssueInput,
    GetIssueInput, GetMyWorkInput, GiveUpInput, ListProjectsInput, LogTimeInput, ProposeInput,
    ReportInput, SearchIssuesInput, StartTimerInput, StopTimerInput, SubmitInput, TakeInput,
    UpdateIssueInput,
};
use super::{agents, apply, preview, read};
use crate::access::Actor;
use crate::ai::Withheld;
use crate::ai::context::Part;
use crate::auth::{AppError, AuthStore, Ctx};

/// A future the registry hands back.
pub(super) type Boxed<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// What a tool is run with: who asks, the input, and what is kept from AI.
#[derive(Clone, Copy)]
pub(super) struct Run<'a> {
    pub(super) actor: &'a Actor,
    pub(super) token: &'a str,
    pub(super) input: &'a Value,
    pub(super) withheld: &'a Withheld,
    pub(super) caller: ToolCaller,
}

impl Run<'_> {
    /// The input as the type of the tool; an input that is not one is `request.invalid`.
    pub(super) fn input<T: DeserializeOwned>(&self) -> Result<T, AppError> {
        serde_json::from_value(self.input.clone())
            .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))
    }
}

/// The function that runs a tool, or previews it.
pub(super) type Runner<S> =
    for<'a, 'c> fn(&'a Ctx<'c, S>, Run<'a>) -> Boxed<'a, Result<Vec<Part>, AppError>>;

/// One tool of the registry.
pub struct ToolSpec<S: AuthStore> {
    /// The tool, by the name AI apps call it.
    pub tool: Tool,
    /// The use case of `bb-app` it runs as the actor.
    pub use_case: &'static str,
    input: fn() -> Value,
    effect: ToolEffect,
    run: Runner<S>,
    /// For a tool that changes something: *Apply*, which saves what the preview showed.
    apply: Option<Runner<S>>,
}

impl<S: AuthStore> ToolSpec<S> {
    /// The name AI apps call the tool by.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.tool.code()
    }

    /// Whether the tool only reads or changes something.
    #[must_use]
    pub const fn effect(&self) -> ToolEffect {
        self.effect
    }

    /// The scope a caller needs to use the tool.
    #[must_use]
    pub const fn scope(&self) -> ToolScope {
        self.effect.scope()
    }

    /// The JSON schema of the input, generated from the Rust type.
    #[must_use]
    pub fn input_schema(&self) -> Value {
        (self.input)()
    }

    /// *Apply* of a tool that changes something: the same input, saved as the person. A tool that
    /// only reads has none.
    pub(super) fn apply<'a>(
        &self,
        ctx: &'a Ctx<'_, S>,
        run: Run<'a>,
    ) -> Option<Boxed<'a, Result<Vec<Part>, AppError>>> {
        self.apply.map(|apply| apply(ctx, run))
    }

    /// What the tool does: for a tool that reads, its answer; for a tool that changes, the
    /// preview of what it would record.
    pub(super) fn run<'a>(
        &self,
        ctx: &'a Ctx<'_, S>,
        run: Run<'a>,
    ) -> Boxed<'a, Result<Vec<Part>, AppError>> {
        (self.run)(ctx, run)
    }
}

/// The schema of an input type, as JSON.
fn schema_of<T: PartialSchema>() -> Value {
    serde_json::to_value(T::schema()).unwrap_or(Value::Null)
}

fn reads<S: AuthStore>(
    tool: Tool,
    use_case: &'static str,
    input: fn() -> Value,
    run: Runner<S>,
) -> ToolSpec<S> {
    ToolSpec {
        tool,
        use_case,
        input,
        effect: ToolEffect::Reads,
        run,
        apply: None,
    }
}

fn changes<S: AuthStore>(
    tool: Tool,
    use_case: &'static str,
    input: fn() -> Value,
    run: Runner<S>,
    apply: Runner<S>,
) -> ToolSpec<S> {
    ToolSpec {
        tool,
        use_case,
        input,
        effect: ToolEffect::Changes,
        run,
        apply: Some(apply),
    }
}

/// Every tool, in the order AI apps are shown them.
#[must_use]
pub fn registry<S: AuthStore>() -> Vec<ToolSpec<S>> {
    let mut all = Vec::from(tools_of_people::<S>());
    all.extend(tools_of_agents::<S>());
    all
}

/// The tools every caller may be offered.
fn tools_of_people<S: AuthStore>() -> [ToolSpec<S>; 11] {
    [
        reads(
            Tool::SearchIssues,
            "search_page_within",
            schema_of::<SearchIssuesInput>,
            read::search_issues::<S>,
        ),
        reads(
            Tool::GetIssue,
            "issue",
            schema_of::<GetIssueInput>,
            read::get_issue::<S>,
        ),
        reads(
            Tool::ListProjects,
            "projects",
            schema_of::<ListProjectsInput>,
            read::list_projects::<S>,
        ),
        reads(
            Tool::GetMyWork,
            "search_page_within",
            schema_of::<GetMyWorkInput>,
            read::get_my_work::<S>,
        ),
        changes(
            Tool::CreateIssue,
            "create_issue",
            schema_of::<CreateIssueInput>,
            preview::create_issue::<S>,
            apply::create_issue::<S>,
        ),
        changes(
            Tool::UpdateIssue,
            "change_issue",
            schema_of::<UpdateIssueInput>,
            preview::update_issue::<S>,
            apply::update_issue::<S>,
        ),
        changes(
            Tool::ChangeStatus,
            "change_issue",
            schema_of::<ChangeStatusInput>,
            preview::change_status::<S>,
            apply::change_status::<S>,
        ),
        changes(
            Tool::AddComment,
            "add_comment",
            schema_of::<AddCommentInput>,
            preview::add_comment::<S>,
            apply::add_comment::<S>,
        ),
        changes(
            Tool::LogTime,
            "add_time",
            schema_of::<LogTimeInput>,
            preview::log_time::<S>,
            apply::log_time::<S>,
        ),
        changes(
            Tool::StartTimer,
            "start_timer",
            schema_of::<StartTimerInput>,
            preview::start_timer::<S>,
            apply::start_timer::<S>,
        ),
        changes(
            Tool::StopTimer,
            "stop_timer",
            schema_of::<StopTimerInput>,
            preview::stop_timer::<S>,
            apply::stop_timer::<S>,
        ),
    ]
}

/// The tools of agents (AI-36): the work they were handed, what changed, and the proposals of a
/// standing rule.
fn tools_of_agents<S: AuthStore>() -> [ToolSpec<S>; 8] {
    [
        changes(
            Tool::Take,
            "take_work",
            schema_of::<TakeInput>,
            agents::preview_take::<S>,
            agents::apply_take::<S>,
        ),
        changes(
            Tool::Report,
            "report_attempt",
            schema_of::<ReportInput>,
            agents::preview_report::<S>,
            agents::apply_report::<S>,
        ),
        changes(
            Tool::Ask,
            "ask_question",
            schema_of::<AskInput>,
            agents::preview_ask::<S>,
            agents::apply_ask::<S>,
        ),
        changes(
            Tool::Attach,
            "attach_result",
            schema_of::<AttachInput>,
            agents::preview_attach::<S>,
            agents::apply_attach::<S>,
        ),
        changes(
            Tool::Submit,
            "end_attempt",
            schema_of::<SubmitInput>,
            agents::preview_submit::<S>,
            agents::apply_submit::<S>,
        ),
        changes(
            Tool::GiveUp,
            "end_attempt",
            schema_of::<GiveUpInput>,
            agents::preview_give_up::<S>,
            agents::apply_give_up::<S>,
        ),
        reads(
            Tool::ChangesSince,
            "changes_since",
            schema_of::<ChangesSinceInput>,
            agents::changes::<S>,
        ),
        changes(
            Tool::Propose,
            "propose_change",
            schema_of::<ProposeInput>,
            agents::preview_propose::<S>,
            agents::apply_propose::<S>,
        ),
    ]
}

/// The use cases no tool runs, besides every use case of `working_hours/`, `work_times/` and
/// `roles/`: they read or change what is not for AI (working hours, work times, other people's time,
/// roles and the answers of *Check access*). ARCH-AI-TOOLS reads this list and refuses a tool, or
/// any file outside the registry, that calls one of them.
pub const DESIGNED_OUT: &[&str] = &[
    "check_access",
    "people_to_check",
    "who_may_change",
    "own_loss",
    "people_visibility",
    "field_permissions",
    "issue_time_entries",
    "issue_time",
    "project_time",
];
