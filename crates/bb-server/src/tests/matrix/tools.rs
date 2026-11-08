//! The tools at `/mcp` in the permission matrix (SEC-MATRIX, AI-21, AI-36, M7-F41): every tool of the
//! one registry, called by every kind of actor, with what the server answered. The entry point is
//! `mcp`; its lines in `tools/checks/sec-matrix-record.txt` start with `tool`.
//!
//! The actors are nobody, a person with each starting role (who calls with a key that may do all
//! they may), an agent that has no work and an agent that was handed one issue. The rows of the
//! routes are written by `routes`/`record`; this test writes and compares its own lines and keeps
//! the others as they are. The check SEC-MATRIX holds the lines against the registry.

use std::fmt::Write as _;

use bb_api_words::sec_matrix as words;
use bb_app::access::StartingRole;
use bb_app::ai::tools::registry;
use bb_core::ErrorCode;
use bb_core::ai::{Tool, ToolEffect};
use bb_core::message::Message;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use crate::sign_in_tests::ai_mcp_rig::{Answer, Mcp, ai_on};
use crate::sign_in_tests::http::Browser;
use crate::sign_in_tests::issues_site::{Site, key_of, ok};
use crate::sign_in_tests::rig::PASSWORD;

/// The root of the repository, from the folder of this crate.
const REPOSITORY_ROOT: &str = "../..";

/// The input of a tool for the issue, good enough to reach the use case.
fn example(tool: Tool, issue: &str, kind: &str) -> Value {
    match tool {
        Tool::SearchIssues => json!({"query": "status group is not Done"}),
        Tool::GetIssue => json!({"issue": issue}),
        Tool::ListProjects | Tool::GetMyWork | Tool::ChangesSince => json!({}),
        Tool::CreateIssue => {
            json!({"project": "ACME", "issue_type": kind, "title": "From the matrix"})
        }
        Tool::UpdateIssue => {
            json!({"issue": issue, "version": 1, "title": "Changed by the matrix"})
        }
        Tool::ChangeStatus => json!({"issue": issue, "version": 1, "status": "done"}),
        Tool::AddComment => json!({"issue": issue, "text": "A comment of the matrix"}),
        Tool::LogTime => json!({"issue": issue, "date": "2027-01-15", "minutes": 5}),
        Tool::StartTimer => json!({"issue": issue}),
        Tool::StopTimer => json!({"version": 1}),
        Tool::Take | Tool::Submit => json!({"issue": issue}),
        Tool::Report => json!({"issue": issue, "minutes": 1, "cost_cents": 0}),
        Tool::Ask => json!({"issue": issue, "question": "Which browser?"}),
        Tool::Attach => {
            json!({"issue": issue, "kind": "link", "label": "Branch", "address": "https://example.test/pull/1"})
        }
        Tool::GiveUp => json!({"issue": issue, "reason": "Cannot reproduce it"}),
        Tool::Propose => json!({
            "rule": bb_core::id::StandingRuleId::generate(&mut bb_app::context::SystemIds).to_string(),
            "issue": issue,
            "field": "labels", "value": ["bug"],
        }),
    }
}

/// What the server answered, in the words of the record. A tool that is let in and then refuses
/// the input or the state of the work answers `allowed`: the permission passed.
fn word_of(answer: &Answer) -> &'static str {
    if answer.status == axum::http::StatusCode::UNAUTHORIZED {
        return words::NOT_SIGNED_IN;
    }
    if answer.json["result"]["isError"] != json!(true) {
        return words::ALLOWED;
    }
    let text = answer.json["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    let said = |code: ErrorCode| Message::from(code.message()).render();
    if text == said(ErrorCode::Forbidden) {
        words::FORBIDDEN
    } else if text == said(ErrorCode::NotFound) {
        words::NOT_FOUND
    } else {
        words::ALLOWED
    }
}

async fn call(
    mcp: &Mcp<'_>,
    key: Option<&str>,
    tool: Tool,
    input: Value,
) -> Result<Answer, Failure> {
    mcp.post(
        key,
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": tool.code(), "arguments": input },
        }),
    )
    .await
}

/// The key of an agent of the person, and whether it was handed `issue`.
async fn agent_key(
    person: &mut Browser,
    name: &str,
    handed: Option<&str>,
) -> Result<String, Failure> {
    let made = person
        .send("POST", "/api/v1/agents", Some(json!({"name": name})))
        .await?;
    let agent = ok(&made)?["id"].as_str().ok_or("no agent ID")?.to_owned();
    let key = person
        .send(
            "POST",
            &format!("/api/v1/agents/{agent}/keys"),
            Some(json!({"password": PASSWORD, "name": "Key"})),
        )
        .await?;
    let secret = ok(&key)?["secret"].as_str().ok_or("no secret")?.to_owned();
    if let Some(issue) = handed {
        let given = person
            .send(
                "POST",
                "/api/v1/hand-overs",
                Some(json!({"agent": agent, "issues": [issue], "note": "Please look at it"})),
            )
            .await?;
        ok(&given)?;
    }
    Ok(secret)
}

/// The lines of the tools: one for every tool of the registry, with the answer of every actor.
async fn lines(db: &bb_testing::TestDatabase) -> Result<Vec<String>, Failure> {
    let (rig, app) = Mcp::start(db).await?;
    let (site, mut ada) = Site::start(&rig, &app).await?;
    ai_on(&rig).await?;
    let issue = key_of(&site.create(&mut ada, "The issue of the matrix").await?)?;
    let mcp = Mcp::new(&app);
    let roles: Vec<&str> = StartingRole::ALL.iter().map(|role| role.code()).collect();
    let mut keys: Vec<(String, Option<String>)> = vec![(words::NOBODY.to_owned(), None)];
    for role in StartingRole::ALL {
        let (mut person, _) = site
            .person(*role, &format!("matrix-{}", role.code()))
            .await?;
        let made = person
            .send(
                "POST",
                "/api/v1/access-keys",
                Some(json!({"password": PASSWORD, "name": "Matrix", "choice": "see-and-change"})),
            )
            .await?;
        let secret = ok(&made)?["secret"].as_str().ok_or("no secret")?.to_owned();
        keys.push((words::role_actor(role.code()), Some(secret)));
    }
    let (mut team, _) = site.person(StartingRole::TeamMember, "matrix-team").await?;
    keys.push((
        words::AGENT_WITHOUT_WORK.to_owned(),
        Some(agent_key(&mut team, "Idle", None).await?),
    ));
    keys.push((
        words::AGENT_WITH_WORK.to_owned(),
        Some(agent_key(&mut team, "Busy", Some(&issue)).await?),
    ));
    let names: Vec<String> = keys.iter().map(|(name, _)| name.clone()).collect();
    assert_eq!(
        names,
        words::mcp_actors(&roles)
            .iter()
            .map(String::clone)
            .collect::<Vec<_>>()
    );
    let mut lines = Vec::new();
    for spec in registry::<bb_store::AuthStorage>() {
        let mut answers = Vec::new();
        for (name, key) in &keys {
            let reply = call(
                &mcp,
                key.as_deref(),
                spec.tool,
                example(spec.tool, &issue, &site.kind),
            )
            .await?;
            let word = word_of(&reply);
            answers.push(format!("{name}={word}"));
            // Whoever is not signed in is told so, and no agent gets what is not for agents, nor
            // does a person get what is for agents only.
            if name == words::NOBODY {
                assert_eq!(word, words::NOT_SIGNED_IN, "{} for nobody", spec.name());
            }
            if spec.tool.is_for_agents() && !name.starts_with("agent") && name != words::NOBODY {
                assert_eq!(word, words::FORBIDDEN, "{} for {name}", spec.name());
            }
            if spec.tool.is_for_people() && name.starts_with("agent") {
                assert_eq!(word, words::FORBIDDEN, "{} for {name}", spec.name());
            }
            if name == words::AGENT_WITHOUT_WORK
                && spec.effect() == ToolEffect::Changes
                && !matches!(spec.tool, Tool::AddComment)
            {
                assert_ne!(
                    word,
                    words::ALLOWED,
                    "{} for an agent with no work: {}",
                    spec.name(),
                    reply.json
                );
            }
        }
        let audience = if spec.tool.is_for_agents() {
            "agents"
        } else if spec.tool.is_for_people() {
            "people"
        } else {
            "everyone"
        };
        lines.push(format!(
            "{}{} {} {} {} : {}",
            words::TOOL_LINE,
            spec.name(),
            words::ENTRY_MCP,
            spec.effect().code(),
            audience,
            answers.join(" ")
        ));
    }
    Ok(lines)
}

/// The tool lines of a text.
fn tool_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with(words::TOOL_LINE))
        .collect()
}

/// Compares the lines with the file of the repository, or writes them when asked to.
fn matches_the_repository(found: &[String]) -> Result<(), Failure> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(REPOSITORY_ROOT)
        .join(words::RECORD_PATH);
    let stored = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read the record: {error}"))?
        .replace("\r\n", "\n");
    if std::env::var(words::WRITE_VARIABLE).as_deref() == Ok(words::WRITE_VALUE) {
        let mut text: String = stored
            .lines()
            .filter(|line| !line.starts_with(words::TOOL_LINE))
            .fold(String::new(), |mut text, line| {
                let _ = writeln!(text, "{line}");
                text
            });
        for line in found {
            let _ = writeln!(text, "{line}");
        }
        std::fs::write(&path, text)?;
        return Ok(());
    }
    let wanted: Vec<&str> = found.iter().map(String::as_str).collect();
    let kept = tool_lines(&stored);
    if kept == wanted {
        return Ok(());
    }
    let place = kept
        .iter()
        .zip(&wanted)
        .enumerate()
        .find(|(_, (before, now))| before != now)
        .map_or_else(
            || "one has more tool lines than the other".to_owned(),
            |(at, (before, now))| {
                format!(
                    "tool line {}: the file has `{before}`, the matrix ran `{now}`",
                    at + 1
                )
            },
        );
    Err(format!(
        "the tool lines of the record differ from what the matrix ran ({place}). Write them again \
         with {}={}.",
        words::WRITE_VARIABLE,
        words::WRITE_VALUE
    )
    .into())
}

#[tokio::test]
async fn sec_01_every_tool_answers_every_actor_as_the_tables_say() -> TestResult {
    each_database("sec_matrix_tools", async |db| {
        let found = lines(db).await?;
        assert_eq!(found.len(), registry::<bb_store::AuthStorage>().len());
        matches_the_repository(&found)
    })
    .await
}
