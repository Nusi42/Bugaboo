//! The tools at `/mcp` in the permission matrix (SEC-MATRIX, AI-21, AI-36, M7-F41): every tool of the
//! one registry, called by every kind of actor, with what the server answered. The entry point is
//! `mcp`; its lines in `tools/checks/sec-matrix-record.txt` start with `tool`.
//!
//! The actors are nobody, a person with each starting role (who calls with a key that may do all
//! they may), a person signed in through an app, and five agents: one with no work, one handed the
//! issue, one handed another issue, one under a standing rule that found the issue, and one handed an
//! issue of a project where AI is switched off. The rows of the routes are written by
//! `routes`/`record`; this test writes and compares its own lines and keeps the others as they are.
//! The check SEC-MATRIX holds the lines against the registry.
//!
//! An answer is `allowed` only when the tool did its work. A refusal that is not `forbidden` or
//! `not-found` is no answer of the matrix: the test fails with it, so an example that does not reach
//! the use case cannot pass as an allowance.

use std::fmt::Write as _;

use axum::http::StatusCode;
use bb_api_words::sec_matrix as words;
use bb_app::access::StartingRole;
use bb_app::ai::tools::registry;
use bb_core::ErrorCode;
use bb_core::ai::{Tool, ToolEffect};
use bb_core::message::Message;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::examples_time::timer_version;
use crate::sign_in_tests::ai_mcp_rig::{Answer, Mcp, ai_on, an_agent_with_a_key, sign_in_app};
use crate::sign_in_tests::http::Browser;
use crate::sign_in_tests::issues_site::{ISSUES, Site, key_of, ok};
use crate::sign_in_tests::rig::{PASSWORD, Rig};

/// The root of the repository, from the folder of this crate.
const REPOSITORY_ROOT: &str = "../..";

/// What an example of a tool needs to know besides the issue.
struct Ground<'a> {
    issue: &'a str,
    kind: &'a str,
    /// The version of the issue now.
    version: i64,
    /// The version of the timer of the person, when they have one.
    timer: i64,
    /// The standing rule the actor works under.
    rule: &'a str,
}

/// The input of a tool for the issue, good enough to reach the use case.
fn example(tool: Tool, ground: &Ground<'_>) -> Value {
    let issue = ground.issue;
    match tool {
        Tool::SearchIssues => json!({"query": ""}),
        Tool::GetIssue | Tool::StartTimer | Tool::Take | Tool::Submit => json!({"issue": issue}),
        Tool::ListProjects | Tool::GetMyWork | Tool::ChangesSince => json!({}),
        Tool::CreateIssue => {
            json!({"project": "ACME", "issue_type": ground.kind, "title": "From the matrix"})
        }
        Tool::UpdateIssue => {
            json!({"issue": issue, "version": ground.version, "title": "Changed by the matrix"})
        }
        Tool::ChangeStatus => {
            json!({"issue": issue, "version": ground.version, "status": "done"})
        }
        Tool::AddComment => json!({"issue": issue, "text": "A comment of the matrix"}),
        Tool::LogTime => json!({"issue": issue, "date": "2027-01-15", "minutes": 5}),
        Tool::StopTimer => json!({"version": ground.timer}),
        Tool::Report => json!({"issue": issue, "minutes": 1, "cost_cents": 0}),
        Tool::Ask => json!({"issue": issue, "question": "Which browser?"}),
        Tool::Attach => {
            json!({"issue": issue, "kind": "link", "label": "Branch", "address": "https://example.test/pull/1"})
        }
        Tool::GiveUp => json!({"issue": issue, "reason": "Cannot reproduce it"}),
        Tool::Propose => json!({
            "rule": ground.rule, "issue": issue, "field": "labels", "value": ["bug"],
        }),
    }
}

/// What the server answered, in the words of the record: `allowed` when the tool did its work, the
/// refusal of the permission, or no answer of the matrix at all.
fn word_of(tool: Tool, answer: &Answer) -> Result<&'static str, String> {
    if answer.status == StatusCode::UNAUTHORIZED {
        return Ok(words::NOT_SIGNED_IN);
    }
    if answer.json["result"]["isError"] != json!(true) {
        return Ok(words::ALLOWED);
    }
    let text = answer.json["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    let said = |code: ErrorCode| Message::from(code.message()).render();
    // A person who could not start a timer has none to stop: there is nothing of theirs to find.
    if text == said(ErrorCode::NotFound)
        || (tool == Tool::StopTimer && text == said(ErrorCode::TimerNotRunning))
    {
        return Ok(words::NOT_FOUND);
    }
    if text == said(ErrorCode::Forbidden) {
        Ok(words::FORBIDDEN)
    } else {
        Err(format!(
            "a refusal that is no answer of the matrix: {}",
            answer.json
        ))
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

/// A person of the matrix: their name in the record, their key, and the browser they act in.
struct Who {
    name: String,
    key: Option<String>,
    browser: Option<Browser>,
}

/// How many agents the matrix makes for one team member, well below the most one may have.
const AGENTS_OF_A_PERSON: usize = 40;

/// Everything the actors need that is not a person: the issues, the rule, and the agents that are
/// made again for each tool because a tool changes the state the next one needs.
struct Scene<'a> {
    site: &'a Site<'a>,
    ada: Browser,
    team: Browser,
    issue: String,
    off_issue: String,
    rule: String,
    off_agent: String,
    rule_agent: String,
    made: usize,
    /// Team members for the agents beyond the most one person may have.
    more_teams: Vec<Browser>,
}

impl Scene<'_> {
    /// An agent of the team member with a key, handed `issue` when there is one; its name is new.
    async fn agent(&mut self, handed: Option<&str>) -> Result<String, Failure> {
        self.made += 1;
        let name = format!("Matrix agent {}", self.made);
        // One person may have only so many agents: the next team member takes over.
        let wanted = self.made / AGENTS_OF_A_PERSON;
        while self.more_teams.len() < wanted {
            let who = format!("matrix-team-{}", self.more_teams.len() + 2);
            let (person, _) = self.site.person(StartingRole::TeamMember, &who).await?;
            self.more_teams.push(person);
        }
        let team = match wanted {
            0 => &mut self.team,
            _ => &mut self.more_teams[wanted - 1],
        };
        Ok(an_agent_with_a_key(team, &name, handed).await?.1)
    }

    /// An agent that was handed `handed`, and for what works on an attempt one that has taken it.
    async fn agent_with_work(
        &mut self,
        tool: Tool,
        handed: &str,
        taken: bool,
    ) -> Result<String, Failure> {
        let key = self.agent(Some(handed)).await?;
        let works_on_an_attempt = matches!(
            tool,
            Tool::Report | Tool::Ask | Tool::Attach | Tool::Submit | Tool::GiveUp
        );
        if (taken && tool != Tool::Take) || works_on_an_attempt {
            let mcp = Mcp::new(self.site.app);
            let taken = call(&mcp, Some(&key), Tool::Take, json!({"issue": handed})).await?;
            if word_of(Tool::Take, &taken)? != words::ALLOWED {
                return Err("the agent could not take its issue".into());
            }
        }
        Ok(key)
    }

    /// A new issue of ACME.
    async fn new_issue(&mut self, title: &str) -> Result<String, Failure> {
        key_of(&self.site.create(&mut self.ada, title).await?)
    }
}

async fn issue_version(ada: &mut Browser, issue: &str) -> Result<i64, Failure> {
    let read = ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?;
    ok(&read)?["issue"]["version"]
        .as_i64()
        .ok_or_else(|| "no version".into())
}

/// The lines of the tools: one for every tool of the registry, with the answer of every actor.
async fn lines(db: &bb_testing::TestDatabase) -> Result<Vec<String>, Failure> {
    let (rig, app) = Mcp::start(db).await?;
    let (site, ada) = Site::start(&rig, &app).await?;
    ai_on(&rig).await?;
    let mcp = Mcp::new(&app);
    let roles: Vec<&str> = StartingRole::ALL.iter().map(|role| role.code()).collect();
    let (team, _) = site.person(StartingRole::TeamMember, "matrix-team").await?;
    let mut scene = scene(&rig, &site, ada, team).await?;
    let mut people: Vec<Who> = Vec::new();
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
        people.push(Who {
            name: words::role_actor(role.code()),
            key: Some(secret),
            browser: Some(person),
        });
    }
    let signed = sign_in_app(
        &mcp,
        &mut scene.ada,
        "Claude Code",
        "see-and-change",
        "see-and-change",
    )
    .await?;
    let idle = scene.agent(None).await?;
    let names = words::mcp_actors(&roles);
    let mut lines = Vec::new();
    for spec in registry::<bb_store::AuthStorage>() {
        let tool = spec.tool;
        let mut answers = Vec::new();
        for name in &names {
            let issue = scene.issue.clone();
            let mut person = None;
            let (key, on) = match name.as_str() {
                words::NOBODY => (None, issue),
                words::PERSON_VIA_APP => (Some(signed.access.clone()), issue),
                words::AGENT_WITHOUT_WORK => (Some(idle.clone()), issue),
                words::AGENT_WITH_WORK => {
                    let own = scene.new_issue("Handed to the agent of the matrix").await?;
                    (Some(scene.agent_with_work(tool, &own, false).await?), own)
                }
                words::AGENT_WITH_AN_ATTEMPT => {
                    let own = scene.new_issue("Taken by the agent of the matrix").await?;
                    (Some(scene.agent_with_work(tool, &own, true).await?), own)
                }
                words::AGENT_ON_ANOTHER_ISSUE => {
                    let own = scene.new_issue("Handed to another agent").await?;
                    (Some(scene.agent_with_work(tool, &own, false).await?), issue)
                }
                words::AGENT_UNDER_A_RULE => (Some(scene.rule_agent.clone()), issue),
                words::AGENT_IN_AN_AI_OFF_PROJECT => {
                    (Some(scene.off_agent.clone()), scene.off_issue.clone())
                }
                role => {
                    let who = people
                        .iter_mut()
                        .find(|who| who.name == role)
                        .ok_or("an actor without a person")?;
                    person = Some(who);
                    (person.as_ref().and_then(|who| who.key.clone()), issue)
                }
            };
            let version = issue_version(&mut scene.ada, &on).await?;
            let timer = match person.as_mut().and_then(|who| who.browser.as_mut()) {
                Some(browser) => timer_version(browser).await?,
                None => 1,
            };
            let kind = site.kind.clone();
            let ground = Ground {
                issue: &on,
                kind: &kind,
                version,
                timer,
                rule: &scene.rule,
            };
            let reply = call(&mcp, key.as_deref(), tool, example(tool, &ground)).await?;
            let word = word_of(tool, &reply)
                .map_err(|wrong| format!("{} as {name}: {wrong}", spec.name()))?;
            answers.push(format!("{name}={word}"));
            check(tool, spec.effect(), name, word)?;
        }
        lines.push(format!(
            "{}{} {} {} {} : {}",
            words::TOOL_LINE,
            spec.name(),
            words::ENTRY_MCP,
            spec.effect().code(),
            words::audience_of(tool.is_for_agents(), tool.is_for_people()),
            answers.join(" ")
        ));
    }
    Ok(lines)
}

/// What must hold whatever the tables say: nobody is turned away, a tool of agents is not for a
/// person, a tool about a person's own time is not for an agent, and an agent without a way into the
/// issue changes nothing there.
fn check(tool: Tool, effect: ToolEffect, name: &str, word: &str) -> Result<(), Failure> {
    let is_agent = name.starts_with(words::AGENT_PREFIX);
    let wrong = match name {
        words::NOBODY => word != words::NOT_SIGNED_IN,
        _ if tool.is_for_agents() && !is_agent => word != words::FORBIDDEN,
        _ if tool.is_for_people() && is_agent => word != words::FORBIDDEN,
        words::AGENT_WITHOUT_WORK
        | words::AGENT_ON_ANOTHER_ISSUE
        | words::AGENT_IN_AN_AI_OFF_PROJECT => {
            effect == ToolEffect::Changes && word == words::ALLOWED
        }
        // An agent under a rule proposes and changes nothing else.
        words::AGENT_UNDER_A_RULE => {
            effect == ToolEffect::Changes && tool != Tool::Propose && word == words::ALLOWED
        }
        _ => false,
    };
    if wrong {
        return Err(format!("{} as {name}: `{word}` must not be the answer", tool.code()).into());
    }
    Ok(())
}

/// The issues, the rule and the agents that last for the whole run.
async fn scene<'a>(
    rig: &Rig,
    site: &'a Site<'a>,
    mut ada: Browser,
    mut team: Browser,
) -> Result<Scene<'a>, Failure> {
    let issue = key_of(&site.create(&mut ada, "The issue of the matrix").await?)?;
    // A project where AI will be off, with an issue handed over before it is switched off.
    let off_project = ada
        .send(
            "POST",
            "/api/v1/projects",
            Some(json!({"short_name": "OFFAI", "name": "No AI here"})),
        )
        .await?;
    let off_project = ok(&off_project)?["id"]
        .as_str()
        .ok_or("no project")?
        .to_owned();
    let made = ada
        .send(
            "POST",
            &format!("/api/v1/projects/{off_project}/issues"),
            Some(json!({"issue_type": site.kind, "title": "An issue where AI is off"})),
        )
        .await?;
    let off_issue = key_of(&made)?;
    let (_, off_agent) =
        an_agent_with_a_key(&mut team, "Matrix agent off", Some(&off_issue)).await?;
    let switched = ada
        .send(
            "PUT",
            &format!("/api/v1/projects/{off_project}/ai"),
            Some(json!({"enabled": false})),
        )
        .await?;
    ok(&switched)?;
    // An agent under a standing rule over every issue of ACME.
    let (rule_agent_id, rule_agent) =
        an_agent_with_a_key(&mut team, "Matrix agent rule", None).await?;
    let search = team
        .send(
            "POST",
            "/api/v1/saved-searches",
            Some(json!({"name": "Everything", "search_text": ""})),
        )
        .await?;
    let search = ok(&search)?["id"].as_str().ok_or("no search")?.to_owned();
    let rule = team
        .send(
            "POST",
            "/api/v1/standing-rules",
            Some(json!({"agent": rule_agent_id, "search": search, "project": site.acme})),
        )
        .await?;
    let rule = ok(&rule)?["id"].as_str().ok_or("no rule")?.to_owned();
    // The rule finds the issues as the job of the matches runs after a change.
    let version = issue_version(&mut ada, &issue).await?;
    ok(&ada
        .send(
            "PATCH",
            &format!("{ISSUES}/{issue}"),
            Some(
                json!({"version": version, "title": "The issue of the matrix, found by the rule"}),
            ),
        )
        .await?)?;
    rig.run_jobs().await?;
    Ok(Scene {
        site,
        ada,
        team,
        issue,
        off_issue,
        rule,
        off_agent,
        rule_agent,
        made: 0,
        more_teams: Vec::new(),
    })
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
