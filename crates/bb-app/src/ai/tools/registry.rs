//! The one registry of tools (AI-21, ADR-0013): a table. The assistant, AI apps and agents list
//! and call tools only through it, so they cannot differ in what a tool is or may do.
//!
//! Each entry names the tool, whether it reads or changes (which fixes the scope it needs:
//! [`ToolEffect::scope`]), the use case it runs as the actor, and the schema of its input, which is
//! generated from the Rust type of that input. A tool that changes something has a preview
//! (`preview.rs`). What is designed out is not in the table and cannot be added without a decision
//! record: a tool that ranks, evaluates or picks people, reads working hours or another person's
//! time, or reads or changes roles (COMP-10, ADR-0035 decision 9). ARCH-AI-TOOLS holds the table to
//! that.

use bb_core::ai::{Tool, ToolEffect, ToolScope};
use serde_json::Value;
use utoipa::PartialSchema;

use super::inputs::{
    AddCommentInput, ChangeStatusInput, CreateIssueInput, GetIssueInput, GetMyWorkInput,
    ListProjectsInput, LogTimeInput, SearchIssuesInput, StartTimerInput, StopTimerInput,
    UpdateIssueInput,
};

/// One tool of the registry.
#[derive(Clone, Copy, Debug)]
pub struct ToolSpec {
    /// The tool, by the name AI apps call it.
    pub tool: Tool,
    /// Whether it only reads or changes something.
    pub effect: ToolEffect,
    /// The use case of `bb-app` it runs as the actor.
    pub use_case: &'static str,
    /// The schema of its input.
    input: fn() -> Value,
}

impl ToolSpec {
    /// The name AI apps call the tool by.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.tool.code()
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
}

/// The schema of an input type, as JSON.
fn schema_of<T: PartialSchema>() -> Value {
    serde_json::to_value(T::schema()).unwrap_or(Value::Null)
}

const fn reads(tool: Tool, use_case: &'static str, input: fn() -> Value) -> ToolSpec {
    ToolSpec {
        tool,
        effect: ToolEffect::Reads,
        use_case,
        input,
    }
}

const fn changes(tool: Tool, use_case: &'static str, input: fn() -> Value) -> ToolSpec {
    ToolSpec {
        tool,
        effect: ToolEffect::Changes,
        use_case,
        input,
    }
}

/// Every tool, in the order AI apps are shown them.
pub const REGISTRY: [ToolSpec; 11] = [
    reads(Tool::SearchIssues, "search", schema_of::<SearchIssuesInput>),
    reads(Tool::GetIssue, "issue", schema_of::<GetIssueInput>),
    reads(
        Tool::ListProjects,
        "projects",
        schema_of::<ListProjectsInput>,
    ),
    reads(Tool::GetMyWork, "search", schema_of::<GetMyWorkInput>),
    changes(
        Tool::CreateIssue,
        "create_issue",
        schema_of::<CreateIssueInput>,
    ),
    changes(
        Tool::UpdateIssue,
        "change_issue",
        schema_of::<UpdateIssueInput>,
    ),
    changes(
        Tool::ChangeStatus,
        "change_issue",
        schema_of::<ChangeStatusInput>,
    ),
    changes(
        Tool::AddComment,
        "add_comment",
        schema_of::<AddCommentInput>,
    ),
    changes(Tool::LogTime, "add_time", schema_of::<LogTimeInput>),
    changes(
        Tool::StartTimer,
        "start_timer",
        schema_of::<StartTimerInput>,
    ),
    changes(Tool::StopTimer, "stop_timer", schema_of::<StopTimerInput>),
];

/// The use cases no tool runs, besides every use case of `working_hours/` and `roles/`: they read
/// or change what is not for AI (working hours, other people's time, roles and the answers of
/// *Check access*). ARCH-AI-TOOLS reads this list and refuses a tool, or any file outside the
/// registry, that calls one of them.
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

/// The entry of a tool.
#[must_use]
pub fn spec_of(tool: Tool) -> &'static ToolSpec {
    // Every tool has its entry: a test holds the table to `Tool::ALL`.
    REGISTRY
        .iter()
        .find(|spec| spec.tool == tool)
        .unwrap_or(&REGISTRY[0])
}

/// The tool with this name, if there is one.
#[must_use]
pub fn tool_named(name: &str) -> Option<&'static ToolSpec> {
    Tool::from_code(name).map(spec_of)
}
