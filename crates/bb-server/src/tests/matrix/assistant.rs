//! The assistant in the permission matrix (SEC-MATRIX, AI-10, AI-05, AI-03, M7-F52): for every
//! starting role, the person asks the assistant through the fake provider, and the record says what
//! the provider was given and what the assistant could propose and apply. The entry point is
//! `assistant`; its lines in `tools/checks/sec-matrix-record.txt` start with `ask`.
//!
//! The scene: an issue in ACME with a status, an estimate, a description and a person it is assigned to, where the
//! description is *Never sent to AI*, and an issue in a second project that is *Never sent to AI* as
//! a whole. Each person asks about both issues, then for a change of the title, and then tries
//! *Apply* with a reference nobody made. The model is the fake provider, scripted to ask for exactly
//! those tools. A Guest reports the issue they read, as in the scene of the deliveries, so the
//! fields and the people that are masked for a Guest are really in play.

use axum::http::StatusCode;
use bb_api_words::sec_matrix as words;
use bb_app::access::{Permission, StartingRole};
use bb_app::ai::AiPart;
use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_testing::{Failure, TestDatabase};
use serde_json::json;

use super::delivery::{author_in, follows_issues};
use super::record::{matches_the_repository_for, refusal_word};
use crate::sign_in_tests::assistant_undo_http::{apply, ask, local_model_on};
use crate::sign_in_tests::http::{Browser, Reply, rig_and_app};
use crate::sign_in_tests::issues_site::{ISSUES, Site, key_of, ok, version_of};
use crate::sign_in_tests::rig::Rig;

/// What the description of the issue says; the field is kept from AI, so no provider may get it.
const KEPT_TEXT: &str = "kept-description-of-the-matrix";
/// The title of the issue in the project that is kept from AI.
const KEPT_ISSUE: &str = "Plans of the kept project";
/// The name of the field *Estimated time* in what a tool reads; some roles may not see it.
const ESTIMATE_KEY: &str = "estimated_time";
/// The name of the field *Status* in what a tool reads.
const STATUS_KEY: &str = "status";
/// The person the issue is assigned to: an administrator, so a role that sees names only as roles
/// reads "Administrator".
const ASSIGNED_TO_NAME: &str = "Alan Assignee";
/// How the role of that person is named where names are left out.
const ASSIGNED_TO_ROLE: &str = "Administrator";
/// A reference that this server never made for anybody.
const MADE_UP_REFERENCE: &str = "AAAA";

/// What one person was given and could do through the assistant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Asked {
    pub(super) role: StartingRole,
    /// Whether the issue of the administrator was in what the tool read: `allowed`, else `not-found`.
    pub(super) read: &'static str,
    /// Whether the issue the person reads to show what is masked for them (their own, for a Guest)
    /// was in what the tool read.
    pub(super) mine: &'static str,
    /// Whether the field *Status* was in what the provider was given of that issue.
    pub(super) status: &'static str,
    /// Whether the field *Estimated time*, hidden for some roles, was in it.
    pub(super) estimate: &'static str,
    /// How the person the issue is assigned to was in it (the kind *Assigned to*).
    pub(super) assigned_to: PeopleLevel,
    /// Whether anything that is *Never sent to AI* was in a request to the provider, or the issue
    /// of the project that is kept from AI was not refused as unknown.
    pub(super) leaked: bool,
    /// What *Apply* answered to a reference that was never made for the person.
    pub(super) forged: &'static str,
    /// What the assistant did with a change of the title: a proposal, or the refusal it read.
    pub(super) propose: &'static str,
    /// What *Apply* did with that proposal.
    pub(super) apply: &'static str,
}

impl Asked {
    /// The line of the record.
    pub(super) fn line(&self) -> String {
        format!(
            "{}{} {} : read={} mine={} status={} estimate={} assigned-to={} kept={} forged={} propose={} \
             apply={}",
            words::ASK_LINE,
            words::ENTRY_ASSISTANT,
            words::role_actor(self.role.code()),
            self.read,
            self.mine,
            self.status,
            self.estimate,
            self.assigned_to.code(),
            if self.leaked {
                words::SENT
            } else {
                words::HELD_BACK
            },
            self.forged,
            self.propose,
            self.apply
        )
    }
}

/// What the tools answered in the last call to the provider, one text for each.
fn tool_results(rig: &Rig) -> Vec<String> {
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
                .collect()
        })
        .unwrap_or_default()
}

/// How an answer of *Apply* reads in the record.
fn apply_word(reply: &Reply) -> Result<&'static str, Failure> {
    match reply.status {
        StatusCode::OK => Ok(words::ALLOWED),
        StatusCode::FORBIDDEN => Ok(words::FORBIDDEN),
        StatusCode::NOT_FOUND => Ok(words::NOT_FOUND),
        StatusCode::CONFLICT if reply.code() == ErrorCode::AiProposalInvalid.code() => {
            Ok(words::INVALID_PROPOSAL)
        }
        other => Err(format!(
            "*Apply* answered {other} {}, which is no answer of the matrix",
            reply.body
        )
        .into()),
    }
}

/// Whether a field is in what the tool read.
fn shown(read: &str, key: &str) -> &'static str {
    if read.contains(&format!("{key}:")) {
        words::SHOWN
    } else {
        words::LEFT_OUT
    }
}

/// The key of the issue of the project that is kept from AI, and the person issues are assigned to.
struct Scene {
    shared: String,
    kept_issue: String,
    assigned_to: String,
}

async fn scene(site: &Site<'_>, ada: &mut Browser) -> Result<Scene, Failure> {
    ok(&ada
        .send("PUT", "/api/v1/ai", Some(local_model_on()))
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
    let (_, assigned_to) = site
        .person_called(
            StartingRole::Administrator,
            "assigned-to-person",
            ASSIGNED_TO_NAME,
        )
        .await?;
    let assigned_to = assigned_to.to_string();
    let shared = issue_by(
        site,
        ada,
        None,
        "Printer of the administrator",
        &assigned_to,
    )
    .await?;
    Ok(Scene {
        shared,
        kept_issue,
        assigned_to,
    })
}

/// An issue in ACME made by `maker` with a description, which the administrator then gives a
/// status, an estimate and the person it is assigned to.
async fn issue_by(
    site: &Site<'_>,
    ada: &mut Browser,
    maker: Option<&mut Browser>,
    title: &str,
    assigned_to: &str,
) -> Result<String, Failure> {
    let mut body = site.new_issue(title);
    body["description"] = json!(KEPT_TEXT);
    let path = format!("/api/v1/projects/{}/issues", site.acme);
    let made = match maker {
        Some(person) => person.send("POST", &path, Some(body)).await?,
        None => ada.send("POST", &path, Some(body)).await?,
    };
    let issue = key_of(&made)?;
    let version = version_of(ada, &issue).await?;
    ok(&ada
        .send(
            "PATCH",
            &format!("{ISSUES}/{issue}"),
            Some(json!({
                "version": version, "status": "done", "estimated_time": 240,
                "assigned_to": [assigned_to],
            })),
        )
        .await?)?;
    Ok(issue)
}

/// The issue of the administrator that every person asks about, and the issue they read to show
/// what is masked for them: the same one, except for a Guest, who sees only what they reported and
/// so reports one of their own.
async fn issues_for(
    site: &Site<'_>,
    ada: &mut Browser,
    role: StartingRole,
    person: &mut Browser,
    scene: &Scene,
) -> Result<String, Failure> {
    if follows_issues(role) && !role.permissions().contains(&Permission::SeeIssues) {
        let title = format!("Printer of the {}", role.code());
        issue_by(site, ada, Some(person), &title, &scene.assigned_to).await
    } else {
        Ok(scene.shared.clone())
    }
}

/// One person asks about both issues, then asks for a change of the title, and then tries a
/// reference nobody made.
async fn ask_as(
    rig: &Rig,
    role: StartingRole,
    person: &mut Browser,
    ada: &mut Browser,
    (issue, mine, kept_issue): (&str, &str, &str),
) -> Result<Asked, Failure> {
    let calls_before = rig.ai.calls().len();
    ask(
        rig,
        person,
        "What are these two issues about?",
        &[
            ("get_issue", json!({"issue": issue})),
            ("get_issue", json!({"issue": kept_issue})),
            ("get_issue", json!({"issue": mine})),
        ],
    )
    .await?;
    let results = tool_results(rig);
    let read = results.first().cloned().unwrap_or_default();
    let own = results.get(2).cloned().unwrap_or_default();
    // The issue of the project that is kept from AI is not there for the assistant, whatever the
    // person may read.
    let kept_read = results.get(1).map(String::as_str).and_then(refusal_word);
    let update = json!({
        "issue": issue,
        "version": version_of(ada, issue).await?,
        "title": format!("Asked by {}", role.code()),
    });
    let proposed = ask(
        rig,
        person,
        "Change the title",
        &[("update_issue", update.clone())],
    )
    .await?;
    let requests: Vec<_> = rig.ai.calls()[calls_before..]
        .iter()
        .map(|call| call.request.clone())
        .collect();
    let sent = format!("{requests:?}");
    let proposal = proposed["proposals"][0].clone();
    let (propose, applied) = if proposal.is_object() {
        let applied = apply(person, &proposal).await?;
        (words::ALLOWED, apply_word(&applied)?)
    } else {
        let refused = tool_results(rig)
            .first()
            .and_then(|text| refusal_word(text));
        (
            refused.ok_or("the assistant neither proposed nor read a refusal")?,
            words::NO_PROPOSAL,
        )
    };
    let forged = apply(
        person,
        &json!({"reference": MADE_UP_REFERENCE, "tool": "update_issue", "input": update}),
    )
    .await?;
    Ok(Asked {
        role,
        read: if read.contains(issue) {
            words::ALLOWED
        } else {
            words::NOT_FOUND
        },
        mine: if own.contains(mine) {
            words::ALLOWED
        } else {
            words::NOT_FOUND
        },
        status: shown(&own, STATUS_KEY),
        estimate: shown(&own, ESTIMATE_KEY),
        assigned_to: author_in(&own, ASSIGNED_TO_NAME, ASSIGNED_TO_ROLE),
        leaked: sent.contains(KEPT_TEXT)
            || sent.contains(KEPT_ISSUE)
            || kept_read != Some(words::NOT_FOUND),
        forged: apply_word(&forged)?,
        propose,
        apply: applied,
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
        let name = format!("Asker {}", role.code());
        let (mut person, _) = site
            .person_called(role, &format!("asks-{}", role.code()), &name)
            .await?;
        let mine = issues_for(&site, &mut ada, role, &mut person, &scene).await?;
        everyone.push(
            ask_as(
                &rig,
                role,
                &mut person,
                &mut ada,
                (&scene.shared, &mine, &scene.kept_issue),
            )
            .await?,
        );
    }
    Ok(everyone)
}

#[tokio::test]
async fn sec_01_the_assistant_gives_and_changes_no_more_than_the_person_may()
-> bb_testing::TestResult {
    bb_testing::each_database("sec_matrix_assistant", async |db| {
        let asked = run(db).await?;
        assert_eq!(asked.len(), StartingRole::ALL.len());
        // Whatever the tables say, nothing that is kept from AI may reach the provider, *Apply* runs
        // only for a proposal, and a reference nobody made is refused.
        for each in &asked {
            assert!(!each.leaked, "{}", each.line());
            assert_eq!(
                each.apply == words::ALLOWED,
                each.propose == words::ALLOWED,
                "{}",
                each.line()
            );
            assert!(
                matches!(
                    each.forged,
                    words::FORBIDDEN | words::NOT_FOUND | words::INVALID_PROPOSAL
                ),
                "{}",
                each.line()
            );
        }
        let lines: Vec<String> = asked.iter().map(Asked::line).collect();
        matches_the_repository_for(words::ASK_LINE, &lines)
    })
    .await
}
