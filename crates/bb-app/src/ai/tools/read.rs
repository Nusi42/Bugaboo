//! The read tools (AI-21): each runs an existing use case as the actor and gives back the parts the
//! AI reads. What the person may not read is not in the use case's answer; what is kept from AI is
//! taken out here, once, for every read tool.

use bb_core::id::{ProjectId, UserId};
use serde_json::Value;

use super::inputs::{
    GetIssueInput, GetMyWorkInput, ListProjectsInput, SearchIssuesInput, limit_within,
};
use super::preview::proposer;
use super::registry::{Boxed, Run};
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
/// The searches of `get_my_work`, in the search language, from the one that says the most to the one
/// that says the least. The search language refuses a field the reader may not see, so a reader who
/// may not see the status gets the issues assigned to them without it, and one who may not see who an
/// issue is assigned to has no work to list (the answer is empty, not a mistake).
const UNKNOWN_FIELD: &str = "search.mistake.unknown-field";

/// The searches of `get_my_work` (see above).
const MY_WORK: [&str; 2] = [
    "assigned to is me and status group is not Done",
    "assigned to is me",
];

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

/// The masked value of an answer, or the error of masking it: the one way the tools read one.
pub(crate) fn masked<T: bb_policy::Masked + serde::Serialize>(
    answer: &bb_policy::Authorized<T>,
) -> Result<Value, AppError> {
    answer
        .masked_value()
        .map_err(|error| AppError::Internal(error.to_string()))
}

/// The ID of the project in the masked value of a view.
pub(super) fn project_of(value: &Value) -> Option<ProjectId> {
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
        limit: Option<&serde_json::Number>,
        withheld: &Withheld,
    ) -> Result<Vec<Part>, AppError> {
        let limit = limit_within(limit)?;
        let query = SearchQuery {
            session_token: session_token.to_owned(),
            text: text.to_owned(),
            scope: SearchScope::Everywhere,
            cursor: None,
            limit: limit.unwrap_or(ISSUES_DEFAULT),
            labels: false,
            agent: crate::issues::WithAgent::No,
            time: false,
        };
        let page = search_page_within(self, actor, &query, true, withheld).await?;
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

    /// `search_issues`: a text that is no search is read as plain words to find in the texts of the
    /// issues (`bb_searchlang::words_search`), and the answer says how it was read.
    pub(super) async fn tool_search_issues(
        &self,
        run: &Run<'_>,
        input: &SearchIssuesInput,
    ) -> Result<Vec<Part>, AppError> {
        // `words_search` alone decides what plain words are. A text that parses is never plain
        // words, so it is tried first only to spare a search the read of the fields the reader may
        // search by, which the decision needs.
        let words = if bb_searchlang::parse(&input.query).is_err() {
            let fields = self
                .readable_fields_within(run.actor, SearchScope::Everywhere, run.withheld)
                .await?;
            bb_searchlang::words_search(&input.query, &fields)
        } else {
            None
        };
        let Some(words) = words else {
            return self
                .search_parts(
                    run.actor,
                    run.token,
                    &input.query,
                    input.limit.as_ref(),
                    run.withheld,
                )
                .await;
        };
        // The search is the caller's words, and fenced as theirs.
        let mut parts = vec![Part {
            name: "searched_as",
            value: Value::String(words.clone()),
            author: proposer(run.caller),
            project: None,
        }];
        parts.extend(
            self.search_parts(
                run.actor,
                run.token,
                &words,
                input.limit.as_ref(),
                run.withheld,
            )
            .await?,
        );
        Ok(parts)
    }

    /// `get_my_work`.
    pub(super) async fn tool_get_my_work(
        &self,
        actor: &Actor,
        session_token: &str,
        input: &GetMyWorkInput,
        withheld: &Withheld,
    ) -> Result<Vec<Part>, AppError> {
        for text in MY_WORK {
            match self
                .search_parts(actor, session_token, text, input.limit.as_ref(), withheld)
                .await
            {
                // Only a field the reader may not see is a reason to ask for less; any other
                // mistake is an error of the program and surfaces.
                Err(AppError::Mistakes(mistakes))
                    if mistakes
                        .iter()
                        .all(|mistake| mistake.key.starts_with(UNKNOWN_FIELD)) => {}
                answered => return answered,
            }
        }
        Ok(Vec::new())
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
                limit: limit_within(input.limit.as_ref())?.unwrap_or(ISSUES_DEFAULT),
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

/// What the AI is told about a search that cannot be read: the first mistake, as the person would
/// read it (with what may have been meant), and how a search is written. The one sentence of the
/// tools over MCP and of the assistant.
#[must_use]
pub fn search_refusal(mistakes: &[bb_searchlang::Mistake]) -> String {
    let mistake = mistakes
        .first()
        .and_then(|first| {
            let words: Vec<(&str, &str)> = first
                .args
                .iter()
                .map(|(name, word)| (name.as_str(), word.as_str()))
                .collect();
            bb_core::message::render_named(&first.key, &words)
        })
        .unwrap_or_default();
    bb_core::message::ai_search_mistake(mistake)
        .render()
        .trim_start()
        .to_owned()
}

/// `search_issues`.
pub(super) fn search_issues<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: SearchIssuesInput = run.input()?;
        ctx.tool_search_issues(&run, &input).await
    })
}

/// `get_my_work`.
pub(super) fn get_my_work<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: GetMyWorkInput = run.input()?;
        ctx.tool_get_my_work(run.actor, run.token, &input, run.withheld)
            .await
    })
}

/// `get_issue`.
pub(super) fn get_issue<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: GetIssueInput = run.input()?;
        ctx.tool_get_issue(run.token, &input, run.withheld).await
    })
}

/// `list_projects`.
pub(super) fn list_projects<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: ListProjectsInput = run.input()?;
        ctx.tool_list_projects(run.token, &input, run.withheld)
            .await
    })
}
