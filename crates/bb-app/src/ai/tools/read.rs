//! The read tools (AI-21): each runs an existing use case as the actor and gives back the parts the
//! AI reads. What the person may not read is not in the use case's answer; what is kept from AI is
//! taken out here, once, for every read tool.

use bb_core::id::{ProjectId, UserId};
use serde_json::Value;

use super::inputs::{GetIssueInput, GetMyWorkInput, ListProjectsInput, SearchIssuesInput};
use crate::access::Actor;
use crate::ai::Withheld;
use crate::ai::context::{AuthorKind, Part, drop_withheld, people_in};
use crate::auth::{AppError, AuthStore, Ctx};
use crate::history::{HistoryQuery, history};
use crate::issues::{IssueQuery, IssueView, issue};
use crate::projects::{ProjectsQuery, projects};
use crate::search::{SearchQuery, SearchScope, search_page_within};
use bb_core::ErrorCode;
use bb_core::history::HistoryTab;

/// How many issues a search tool answers with when it is not told.
const ISSUES_DEFAULT: u32 = 20;
/// How many comments `get_issue` reads.
const COMMENTS_MAX: u32 = 20;
/// The search of `get_my_work`, in the search language.
const MY_WORK: &str = "assigned to is me and status group is not Done";

/// The members left out of what the AI reads: HTML made from a text that is in the answer already
/// as Markdown, and the buttons of the screen.
const LEFT_OUT: &[&str] = &["html", "description_html", "status_changes"];

fn drop_html(value: &mut Value) {
    match value {
        Value::Object(object) => {
            object.retain(|name, _| !LEFT_OUT.contains(&name.as_str()));
            object.values_mut().for_each(drop_html);
        }
        Value::Array(items) => items.iter_mut().for_each(drop_html),
        _ => {}
    }
}

fn masked<T: bb_policy::Masked + serde::Serialize>(
    answer: &bb_policy::Authorized<T>,
) -> Result<Value, AppError> {
    answer
        .masked_value()
        .map_err(|error| AppError::Internal(error.to_string()))
}

/// The ID of the project in the masked value of a view.
fn project_of(value: &Value) -> Option<ProjectId> {
    value.get("project")?.get("id")?.as_str()?.parse().ok()
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The kind of author the person in `member` of `value` is in the project, or *someone outside*
    /// when the value names nobody by ID (a person shown by role or as *someone* is not known).
    async fn author_in(
        &self,
        value: &Value,
        member: &str,
        project: Option<ProjectId>,
    ) -> Result<AuthorKind, AppError> {
        let person: Option<UserId> = value
            .get(member)
            .and_then(|each| people_in(each).into_iter().next());
        match (person, project) {
            (Some(person), Some(project)) => self.author_kind(person, project).await,
            _ => Ok(AuthorKind::Outsider),
        }
    }

    /// One issue of a search page or of `get_issue` as a part: masked, without what is kept from AI.
    async fn issue_part(
        &self,
        mut value: Value,
        withheld: &Withheld,
    ) -> Result<Option<Part>, AppError> {
        let project = project_of(&value);
        // An issue in a project kept from AI is not there for the AI.
        if project.is_some_and(|project| withheld.hides_project(project)) {
            return Ok(None);
        }
        // The author is told before *Reported by* may be taken out.
        let author = self.author_in(&value, "reported_by", project).await?;
        drop_withheld::<IssueView>(&mut value, withheld);
        drop_html(&mut value);
        Ok(Some(Part {
            name: "issue",
            value,
            author,
            project,
        }))
    }

    async fn search_parts(
        &self,
        actor: &Actor,
        session_token: &str,
        text: &str,
        limit: Option<u32>,
        withheld: &Withheld,
    ) -> Result<Vec<Part>, AppError> {
        let query = SearchQuery {
            session_token: session_token.to_owned(),
            text: text.to_owned(),
            scope: SearchScope::Everywhere,
            cursor: None,
            limit: limit.unwrap_or(ISSUES_DEFAULT),
            labels: false,
            time: false,
        };
        let page = search_page_within(self, actor, &query, true, Some(withheld)).await?;
        let mut parts = Vec::new();
        if let Some(Value::Array(items)) = masked(&page)?.get_mut("items").map(Value::take) {
            for item in items {
                if let Some(part) = self.issue_part(item, withheld).await? {
                    parts.push(part);
                }
            }
        }
        Ok(parts)
    }

    /// `search_issues`.
    pub(super) async fn tool_search_issues(
        &self,
        actor: &Actor,
        session_token: &str,
        input: &SearchIssuesInput,
        withheld: &Withheld,
    ) -> Result<Vec<Part>, AppError> {
        self.search_parts(actor, session_token, &input.query, input.limit, withheld)
            .await
    }

    /// `get_my_work`.
    pub(super) async fn tool_get_my_work(
        &self,
        actor: &Actor,
        session_token: &str,
        input: &GetMyWorkInput,
        withheld: &Withheld,
    ) -> Result<Vec<Part>, AppError> {
        self.search_parts(actor, session_token, MY_WORK, input.limit, withheld)
            .await
    }

    /// `get_issue`: the issue and its comments.
    pub(super) async fn tool_get_issue(
        &self,
        session_token: &str,
        input: &GetIssueInput,
        withheld: &Withheld,
    ) -> Result<Vec<Part>, AppError> {
        let detail = issue(
            self,
            IssueQuery {
                session_token: session_token.to_owned(),
                issue: input.issue.clone(),
            },
        )
        .await?;
        let mut whole = masked(&detail)?;
        let issue_value = whole
            .get_mut("issue")
            .map(Value::take)
            .ok_or_else(|| AppError::Internal("an issue answer without its issue".into()))?;
        let project = project_of(&issue_value);
        let Some(first) = self.issue_part(issue_value, withheld).await? else {
            // The same answer as for an issue the person cannot read.
            return Err(AppError::Refused(ErrorCode::NotFound));
        };
        let mut parts = vec![first];
        let comments = history(
            self,
            HistoryQuery {
                session_token: session_token.to_owned(),
                issue: input.issue.clone(),
                tab: HistoryTab::Comments,
                cursor: None,
                limit: COMMENTS_MAX,
            },
        )
        .await?;
        let page = masked(&comments)?;
        if let Some(Value::Array(items)) = page.get("items") {
            for item in items {
                let author = self.author_in(item, "by", project).await?;
                let mut value = item.clone();
                drop_html(&mut value);
                parts.push(Part {
                    name: "comment",
                    value,
                    author,
                    project,
                });
            }
        }
        Ok(parts)
    }

    /// `list_projects`: without the projects kept from AI.
    pub(super) async fn tool_list_projects(
        &self,
        session_token: &str,
        input: &ListProjectsInput,
        withheld: &Withheld,
    ) -> Result<Vec<Part>, AppError> {
        let page = projects(
            self,
            ProjectsQuery {
                session_token: session_token.to_owned(),
                inside: None,
                search: Some(input.search.clone().unwrap_or_default()),
                outside: None,
                movable: None,
                cursor: None,
                limit: input.limit.unwrap_or(ISSUES_DEFAULT),
            },
        )
        .await?;
        let value = masked(&page)?;
        let mut parts = Vec::new();
        if let Some(Value::Array(items)) = value.get("items") {
            for item in items {
                let id: Option<ProjectId> = item
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|text| text.parse().ok());
                if id.is_some_and(|id| withheld.hides_project(id)) {
                    continue;
                }
                parts.push(Part {
                    name: "project",
                    value: item.clone(),
                    author: AuthorKind::Member,
                    project: id,
                });
            }
        }
        Ok(parts)
    }
}
