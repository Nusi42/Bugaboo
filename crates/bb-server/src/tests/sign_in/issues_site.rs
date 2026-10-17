//! A small organization for the issue tests: Ada with the project ACME, the issue types of the
//! first start, and people with a starting role who work over HTTP like a browser does.

use std::collections::BTreeMap;

use axum::Router;
use axum::http::StatusCode;
use bb_app::access::StartingRole;
use bb_core::id::{RoleId, UserId};
use bb_testing::Failure;
use serde_json::{Value, json};

use super::http::{ADA_EMAIL, Browser, Reply};
use super::policy_rig::Org;
use super::rig::Rig;

pub(super) const PROJECTS: &str = "/api/v1/projects";
pub(super) const ISSUES: &str = "/api/v1/issues";

/// The organization of the tests: the administrator, the project ACME and the roles of the first
/// start.
pub(super) struct Site<'a> {
    pub(super) org: Org<'a>,
    pub(super) app: &'a Router,
    roles: BTreeMap<String, RoleId>,
    /// The ID of the project ACME.
    pub(super) acme: String,
    /// The ID of the issue type the tests create issues of: *Task*, which uses the starter workflow
    /// *Simple*, where any status can follow any other.
    pub(super) kind: String,
}

impl<'a> Site<'a> {
    /// The organization, and Ada signed in.
    pub(super) async fn start(rig: &'a Rig, app: &'a Router) -> Result<(Self, Browser), Failure> {
        let org = Org::start(rig).await?;
        let roles = org.starting_roles().await?;
        let mut ada = Browser::signed_in_as(app, ADA_EMAIL).await?;
        let acme = ada
            .send(
                "POST",
                PROJECTS,
                Some(json!({"short_name": "ACME", "name": "Acme"})),
            )
            .await?
            .body["id"]
            .as_str()
            .ok_or("no project ID")?
            .to_owned();
        let types = ada.send("GET", "/api/v1/issue-types", None).await?;
        let kind = types.body["items"]
            .as_array()
            .and_then(|items| items.iter().find(|each| each["name"] == "Task"))
            .and_then(|found| found["id"].as_str())
            .ok_or("no issue type Task on a started server")?
            .to_owned();
        Ok((
            Self {
                org,
                app,
                roles,
                acme,
                kind,
            },
            ada,
        ))
    }

    /// A person with the starting role for the whole organization, signed in.
    pub(super) async fn person(
        &self,
        role: StartingRole,
        name: &str,
    ) -> Result<(Browser, UserId), Failure> {
        self.person_called(role, name, "Bob Builder").await
    }

    /// The same, for a person who goes by this name.
    pub(super) async fn person_called(
        &self,
        role: StartingRole,
        name: &str,
        display_name: &str,
    ) -> Result<(Browser, UserId), Failure> {
        let email = format!("{name}@example.test");
        let started = self
            .org
            .rig
            .person_called(&self.org.admin.session, &email, display_name)
            .await?;
        let member = super::policy_rig::Member {
            id: started.person.id,
            session: started.session_token,
        };
        self.org
            .give_everywhere(&member, self.roles[role.code()])
            .await?;
        Ok((Browser::signed_in_as(self.app, &email).await?, member.id))
    }

    /// A person with the starting role in ACME only, signed in: every other project is not there
    /// for them.
    pub(super) async fn person_in_acme(
        &self,
        role: StartingRole,
        name: &str,
    ) -> Result<(Browser, UserId), Failure> {
        let email = format!("{name}@example.test");
        let started = self
            .org
            .rig
            .person_called(&self.org.admin.session, &email, "Bob Builder")
            .await?;
        let member = super::policy_rig::Member {
            id: started.person.id,
            session: started.session_token,
        };
        let acme = self.acme.parse().map_err(|_| "no project ID")?;
        self.org
            .give(&member, self.roles[role.code()], acme)
            .await?;
        Ok((Browser::signed_in_as(self.app, &email).await?, member.id))
    }

    /// A person with no role anywhere: they exist, and they are no member of any project.
    pub(super) async fn outsider(&self, name: &str) -> Result<UserId, Failure> {
        let email = format!("{name}@example.test");
        let started = self.org.rig.person(&self.org.admin.session, &email).await?;
        Ok(started.person.id)
    }

    /// Chooses how a starting role sees the people of a kind, everywhere.
    pub(super) async fn show_people(
        &self,
        admin: &mut Browser,
        role: StartingRole,
        kind: &str,
        level: &str,
    ) -> Result<(), Failure> {
        let set = admin
            .send(
                "PUT",
                &format!(
                    "/api/v1/people-visibility/{}/{kind}",
                    self.roles[role.code()]
                ),
                Some(json!({"level": level})),
            )
            .await?;
        assert_eq!(set.status, StatusCode::NO_CONTENT, "{}", set.body);
        Ok(())
    }

    /// The body that makes an issue with this title.
    pub(super) fn new_issue(&self, title: &str) -> Value {
        json!({"issue_type": self.kind, "title": title})
    }

    /// Creates an issue in ACME as `browser`; the answer.
    pub(super) async fn create(
        &self,
        browser: &mut Browser,
        title: &str,
    ) -> Result<Reply, Failure> {
        browser
            .send(
                "POST",
                &format!("{PROJECTS}/{}/issues", self.acme),
                Some(self.new_issue(title)),
            )
            .await
    }
}

/// The answer, which has to be a success.
pub(super) fn ok(reply: &Reply) -> Result<&Value, Failure> {
    if reply.status == StatusCode::OK {
        Ok(&reply.body)
    } else {
        Err(format!("{}: {}", reply.status, reply.body).into())
    }
}

/// The text of a member of an answer.
pub(super) fn text_in(answer: &Value, member: &str) -> Result<String, Failure> {
    answer[member]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("the answer holds no {member}: {answer}").into())
}

/// The readable ID (`ACME-142`) in an answer that is about an issue.
pub(super) fn key_in(answer: &Value) -> Result<String, Failure> {
    text_in(answer, "key")
}

/// The ID in an answer.
pub(super) fn id_in(answer: &Value) -> Result<String, Failure> {
    text_in(answer, "id")
}

/// The readable ID in the answer of a call that has to succeed.
pub(super) fn key_of(reply: &Reply) -> Result<String, Failure> {
    key_in(ok(reply)?)
}

/// The ID in the answer of a call that has to succeed.
pub(super) fn id_of(reply: &Reply) -> Result<String, Failure> {
    id_in(ok(reply)?)
}

/// The version an issue has now, read as `browser` sees it.
pub(super) async fn version_of(browser: &mut Browser, issue: &str) -> Result<i64, Failure> {
    let read = browser
        .send("GET", &format!("{ISSUES}/{issue}"), None)
        .await?;
    Ok(ok(&read)?["issue"]["version"]
        .as_i64()
        .ok_or("no version")?)
}
