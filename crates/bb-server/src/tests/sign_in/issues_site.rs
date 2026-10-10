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
    /// The ID of the issue type *Bug*.
    pub(super) bug: String,
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
        let bug = types.body["items"][0]["id"]
            .as_str()
            .ok_or("no issue type on a started server")?
            .to_owned();
        Ok((
            Self {
                org,
                app,
                roles,
                acme,
                bug,
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
        let email = format!("{name}@example.test");
        let started = self.org.rig.person(&self.org.admin.session, &email).await?;
        let member = super::policy_rig::Member {
            id: started.person.id,
            session: started.session_token,
        };
        self.org
            .give_everywhere(&member, self.roles[role.code()])
            .await?;
        Ok((Browser::signed_in_as(self.app, &email).await?, member.id))
    }

    /// The body that makes an issue with this title.
    pub(super) fn new_issue(&self, title: &str) -> Value {
        json!({"issue_type": self.bug, "title": title})
    }

    /// Makes an issue in ACME as `browser`; the answer.
    pub(super) async fn make(&self, browser: &mut Browser, title: &str) -> Result<Reply, Failure> {
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
