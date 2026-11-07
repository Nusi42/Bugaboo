//! The example requests of the routes of hand-overs, agent work and standing rules (M7 WP-08).
//!
//! Every record the requests name is nobody's, so a person who is let in is told
//! `resource.not-found`, and the actors that are not let in are refused as for the routes of agents.

use serde_json::json;

use super::examples::NOBODYS_RANDOM_ID;
use super::examples::Request;

/// The request of a route of agent work, or `None` for a route of another area.
pub(super) fn agent_work_request(operation: &str) -> Option<Request> {
    let nobody = NOBODYS_RANDOM_ID;
    Some(match operation {
        "handOver" => (
            "POST",
            "/api/v1/hand-overs".into(),
            Some(json!({"agent": nobody, "issues": [nobody]})),
        ),
        "getIssueAgentWork" => ("GET", format!("/api/v1/issues/{nobody}/agent-work"), None),
        "answerAgentQuestion" => (
            "POST",
            format!("/api/v1/agent-work/{nobody}/answer"),
            Some(json!({"question": nobody, "answer": "Firefox", "version": 1})),
        ),
        "approveAgentPlan" => (
            "POST",
            format!("/api/v1/agent-work/{nobody}/approve-plan"),
            Some(json!({"version": 1})),
        ),
        "reviewAgentWork" => (
            "POST",
            format!("/api/v1/agent-work/{nobody}/review"),
            Some(json!({"answer": "accept", "version": 1})),
        ),
        "lookAtAgentWork" => (
            "POST",
            format!("/api/v1/agent-work/{nobody}/looked-at"),
            None,
        ),
        "createStandingRule" => (
            "POST",
            "/api/v1/standing-rules".into(),
            Some(json!({"agent": nobody, "search": nobody, "project": nobody})),
        ),
        "changeStandingRule" => (
            "PATCH",
            format!("/api/v1/standing-rules/{nobody}"),
            Some(json!({"action": "stop", "version": 1})),
        ),
        "listStandingRules" => (
            "GET",
            format!("/api/v1/agents/{nobody}/standing-rules"),
            None,
        ),
        "decideProposal" => (
            "POST",
            format!("/api/v1/proposals/{nobody}/decision"),
            Some(json!({"apply": false})),
        ),
        _ => return None,
    })
}
