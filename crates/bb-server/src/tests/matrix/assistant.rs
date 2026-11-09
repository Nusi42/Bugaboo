//! The assistant in the permission matrix (SEC-MATRIX, AI-10, AI-05, AI-03, M7-F52): for every
//! starting role, the person asks the assistant through the fake provider, and the record says what
//! the provider was given and what the assistant could propose and apply. The entry point is
//! `assistant`; its lines in `tools/checks/sec-matrix-record.txt` start with `ask`.
//!
//! The scene: an issue in ACME with a status, a description and an author, where the description is
//! *Never sent to AI*, and an issue in a second project that is *Never sent to AI* as a whole.
//! Each person asks about both issues, and then for a change of the title. The model is the fake
//! provider, scripted to ask for exactly those tools.

use std::fmt::Write as _;

use axum::http::StatusCode;
use bb_api_words::sec_matrix as words;
use bb_app::access::StartingRole;
use bb_app::ai::{AiAnswer, AiPart, AiToolCall};
use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::message::{Message, STATUS_DONE};
use bb_testing::{Failure, TestDatabase};
use serde_json::{Value, json};

use super::delivery::author_in;
use crate::sign_in_tests::assistant_undo_http::local_model_on;
use crate::sign_in_tests::http::{Browser, rig_and_app};
use crate::sign_in_tests::issues_site::{ISSUES, Site, key_of, ok, version_of};
use crate::sign_in_tests::rig::Rig;

/// The root of the repository, from the folder of this crate.
const REPOSITORY_ROOT: &str = "../..";
/// The title the issue starts with.
const TITLE: &str = "Printer jams on Monday";
/// What the description of that issue says; the field is kept from AI, so no provider may get it.
const KEPT_TEXT: &str = "kept-description-of-the-matrix";
/// The title of the issue in the project that is kept from AI.
const KEPT_ISSUE: &str = "Plans of the kept project";
/// How the label of the role of the author reads where names are left out.
const AUTHOR_ROLE: &str = "Administrator";

/// What one person was given and could do through the assistant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Asked {
    pub(super) role: StartingRole,
    /// Whether the issue they may see was in what the tool read: `allowed`, else `not-found`.
    pub(super) read: &'static str,
    /// Whether the status, a field with a level per role, was in what the provider was given.
    pub(super) status_shown: bool,
    /// How the author of the issue was in what the provider was given.
    pub(super) author: PeopleLevel,
    /// Whether anything of what is *Never sent to AI* was in a request to the provider.
    pub(super) leaked: bool,
    /// What the assistant did with a change of the title: a proposal, or the refusal it read.
    pub(super) propose: &'static str,
    /// What *Apply* did with that proposal.
    pub(super) apply: &'static str,
}

impl Asked {
    /// The line of the record.
    pub(super) fn line(&self) -> String {
        format!(
            "{}{} {} : read={} status={} author={} kept={} propose={} apply={}",
            words::ASK_LINE,
            words::ENTRY_ASSISTANT,
            words::role_actor(self.role.code()),
            self.read,
            if self.status_shown {
                words::SHOWN
            } else {
                words::LEFT_OUT
            },
            self.author.code(),
            if self.leaked {
                words::SENT
            } else {
                words::HELD_BACK
            },
            self.propose,
            self.apply
        )
    }
}

/// The model asks for these tools in its next answer, and then says it is done.
fn will_call(rig: &Rig, calls: &[(&str, Value)]) {
    rig.ai.will_answer(AiAnswer {
        tool_calls: calls
            .iter()
            .enumerate()
            .map(|(at, (name, arguments))| AiToolCall {
                id: format!("call-{at}"),
                name: (*name).to_owned(),
                arguments: arguments.clone(),
            })
            .collect(),
        ..AiAnswer::default()
    });
    rig.ai.will_answer(AiAnswer {
        text: "Done.".to_owned(),
        ..AiAnswer::default()
    });
}

/// What the tools answered in the last call to the provider, as one text.
fn tool_results(rig: &Rig) -> String {
    rig.ai
        .calls()
        .last()
        .map(|last| {
            last.request
                .turns
                .iter()
                .flat_map(|turn| turn.parts.iter())
                .filter_map(|part| match part {
                    AiPart::ToolResult { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

/// The refusal a tool that did not run was read as, in the words of the record.
fn refusal_of(text: &str) -> Result<&'static str, Failure> {
    let said = |code: ErrorCode| Message::from(code.message()).render();
    if text.contains(&said(ErrorCode::NotFound)) {
        Ok(words::NOT_FOUND)
    } else if text.contains(&said(ErrorCode::Forbidden)) {
        Ok(words::FORBIDDEN)
    } else {
        Err(format!("a refusal that is no answer of the matrix: {text}").into())
    }
}

/// How an answer of the server reads in the record.
fn status_word(status: StatusCode) -> Result<&'static str, Failure> {
    match status {
        StatusCode::OK => Ok(words::ALLOWED),
        StatusCode::FORBIDDEN => Ok(words::FORBIDDEN),
        StatusCode::NOT_FOUND => Ok(words::NOT_FOUND),
        other => Err(format!("*Apply* answered {other}, which is no answer of the matrix").into()),
    }
}

/// The two issues every person asks about.
struct Scene {
    issue: String,
    kept_issue: String,
}

async fn scene(site: &Site<'_>, ada: &mut Browser) -> Result<Scene, Failure> {
    ok(&ada
        .send("PUT", "/api/v1/ai", Some(local_model_on()))
        .await?)?;
    let made = ada
        .send(
            "POST",
            &format!("/api/v1/projects/{}/issues", site.acme),
            Some(json!({"issue_type": site.kind, "title": TITLE, "description": KEPT_TEXT})),
        )
        .await?;
    let issue = key_of(&made)?;
    let version = version_of(ada, &issue).await?;
    ok(&ada
        .send(
            "PATCH",
            &format!("{ISSUES}/{issue}"),
            Some(json!({"version": version, "status": "done"})),
        )
        .await?)?;
    let kept_project = ok(&ada
        .send(
            "POST",
            "/api/v1/projects",
            Some(json!({"short_name": "KEPT", "name": "Kept from AI"})),
        )
        .await?)?["id"]
        .as_str()
        .ok_or("no project")?
        .to_owned();
    let kept = ada
        .send(
            "POST",
            &format!("/api/v1/projects/{kept_project}/issues"),
            Some(json!({"issue_type": site.kind, "title": KEPT_ISSUE})),
        )
        .await?;
    let kept_issue = key_of(&kept)?;
    for path in ["fields/description", &format!("projects/{kept_project}")] {
        ok(&ada
            .send(
                "PUT",
                &format!("/api/v1/never-sent-to-ai/{path}"),
                Some(json!({"marked": true, "version": 0})),
            )
            .await?)?;
    }
    Ok(Scene { issue, kept_issue })
}

/// One person asks about both issues, and then asks for a change of the title.
async fn ask_as(
    rig: &Rig,
    role: StartingRole,
    person: &mut Browser,
    scene: &Scene,
    version: i64,
) -> Result<Asked, Failure> {
    let calls_before = rig.ai.calls().len();
    will_call(
        rig,
        &[
            ("get_issue", json!({"issue": scene.issue})),
            ("get_issue", json!({"issue": scene.kept_issue})),
        ],
    );
    let asked = person
        .send(
            "POST",
            "/api/v1/assistant/questions",
            Some(json!({"text": "What are these two issues about?"})),
        )
        .await?;
    ok(&asked)?;
    let read = tool_results(rig);
    let requests: Vec<_> = rig.ai.calls()[calls_before..]
        .iter()
        .map(|call| call.request.clone())
        .collect();
    let sent = format!("{requests:?}");
    let title = format!("Asked by {}", role.code());
    will_call(
        rig,
        &[(
            "update_issue",
            json!({"issue": scene.issue, "version": version, "title": title}),
        )],
    );
    let proposed = person
        .send(
            "POST",
            "/api/v1/assistant/questions",
            Some(json!({"text": "Change the title"})),
        )
        .await?;
    ok(&proposed)?;
    let proposal = proposed.body["proposals"][0].clone();
    let (propose, apply) = if proposal.is_object() {
        let applied = person
            .send(
                "POST",
                "/api/v1/assistant/apply",
                Some(json!({
                    "reference": proposal["reference"],
                    "tool": proposal["tool"],
                    "input": proposal["input"],
                })),
            )
            .await?;
        (words::ALLOWED, status_word(applied.status)?)
    } else {
        (refusal_of(&tool_results(rig))?, words::NO_PROPOSAL)
    };
    Ok(Asked {
        role,
        read: if read.contains(&scene.issue) {
            words::ALLOWED
        } else {
            words::NOT_FOUND
        },
        status_shown: read.contains(&Message::from(STATUS_DONE).render()),
        author: author_in(&read, AUTHOR_ROLE),
        leaked: sent.contains(KEPT_TEXT) || sent.contains(KEPT_ISSUE),
        propose,
        apply,
    })
}

/// What every starting role was given and could do through the assistant.
///
/// # Errors
/// The server answers something the scene does not expect.
pub(super) async fn run(db: &TestDatabase) -> Result<Vec<Asked>, Failure> {
    let (rig, app) = rig_and_app(db).await?;
    let (site, mut ada) = Site::start(&rig, &app).await?;
    let scene = scene(&site, &mut ada).await?;
    let mut everyone = Vec::new();
    for role in StartingRole::ALL.iter().copied() {
        let (mut person, _) = site.person(role, &format!("asks-{}", role.code())).await?;
        let version = version_of(&mut ada, &scene.issue).await?;
        everyone.push(ask_as(&rig, role, &mut person, &scene, version).await?);
    }
    Ok(everyone)
}

/// The `ask` lines of a text.
fn ask_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with(words::ASK_LINE))
        .collect()
}

/// Compares the lines with the file of the repository, or writes them when asked to.
///
/// # Errors
/// The file is missing or differs from what ran.
pub(super) fn matches_the_repository(found: &[String]) -> Result<(), Failure> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(REPOSITORY_ROOT)
        .join(words::RECORD_PATH);
    let stored = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read the record: {error}"))?
        .replace("\r\n", "\n");
    if std::env::var(words::WRITE_VARIABLE).as_deref() == Ok(words::WRITE_VALUE) {
        let mut text = String::new();
        for line in stored
            .lines()
            .filter(|line| !line.starts_with(words::ASK_LINE))
        {
            let _ = writeln!(text, "{line}");
        }
        for line in found {
            let _ = writeln!(text, "{line}");
        }
        std::fs::write(&path, text)?;
        return Ok(());
    }
    let wanted: Vec<&str> = found.iter().map(String::as_str).collect();
    let kept = ask_lines(&stored);
    if kept == wanted {
        return Ok(());
    }
    Err(format!(
        "the ask lines of the record differ from what the matrix ran (the file has {kept:?}, the \
         matrix ran {wanted:?}). Write them again with {}={}.",
        words::WRITE_VARIABLE,
        words::WRITE_VALUE
    )
    .into())
}

#[tokio::test]
async fn sec_01_the_assistant_gives_and_changes_no_more_than_the_person_may()
-> bb_testing::TestResult {
    bb_testing::each_database("sec_matrix_assistant", async |db| {
        let asked = run(db).await?;
        assert_eq!(asked.len(), StartingRole::ALL.len());
        let lines: Vec<String> = asked.iter().map(Asked::line).collect();
        eprintln!("{}", lines.join("\n"));
        matches_the_repository(&lines)
    })
    .await
}
