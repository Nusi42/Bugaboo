//! Lean lists: `?fields=`, the cursor and the page size, written once (PERF-02, architecture §7
//! "Performance").
//!
//! A list route takes [`ListParams`], chooses its fields with [`ListParams::fields`], lets
//! [`Fields::project`] drop everything else from each item, and answers one page with a
//! `next_cursor` made by [`crate::paging::seal`]. The route only says which fields exist, which
//! are the default and where a page ends; this file and `paging.rs` do the rest. ARCH-LEAN checks
//! the API description of every list.
//!
//! A person may only ask for fields they may read. A field that is hidden from them is refused
//! exactly like one that does not exist (`request.unknown_field`, SEC-06), so the answer never
//! tells which fields exist behind a mask. The policy engine (`Authorized<T>`, M2 WP-05) plugs in
//! by passing the names of the fields the person may read as `allowed`.

use bb_core::ErrorCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::IntoParams;

use crate::errors::ApiError;
use crate::paging;

/// What every list takes in its query: the fields, the cursor and the page size.
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ListParams {
    /// The fields each item holds, separated by commas. The answer schema names every field
    /// that can be asked for; when this is left out the route's own small default is used.
    fields: Option<String>,
    /// Where the page starts: the `next_cursor` of the page before.
    cursor: Option<String>,
    /// How many items a page holds at most (default 50, at most 100).
    #[param(minimum = 1, maximum = 100)]
    limit: Option<u32>,
}

impl ListParams {
    /// The fields to answer with.
    ///
    /// # Errors
    /// `request.unknown_field` for a name that is not in `allowed`.
    pub(crate) fn fields(
        &self,
        allowed: &[&'static str],
        default: &[&'static str],
    ) -> Result<Fields, ApiError> {
        Fields::choose(self.fields.as_deref(), allowed, default)
    }

    /// Where the page starts, if the request named a cursor.
    ///
    /// # Errors
    /// `request.bad_cursor` for a cursor that was changed or is not one of ours.
    pub(crate) fn position(&self) -> Result<Option<String>, ApiError> {
        self.cursor.as_deref().map(paging::open).transpose()
    }

    /// How many items the page holds at most.
    pub(crate) fn limit(&self) -> u32 {
        paging::limit(self.limit)
    }
}

/// The fields a list answers with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fields {
    names: Vec<&'static str>,
}

impl Fields {
    /// The fields asked for (`asked`, names separated by commas), or `default` when the request
    /// said nothing. `allowed` is every field the person may read; the default is cut to it.
    ///
    /// # Errors
    /// `request.unknown_field` for a name that is not in `allowed`, whether it does not exist
    /// or the person may not read it.
    pub(crate) fn choose(
        asked: Option<&str>,
        allowed: &[&'static str],
        default: &[&'static str],
    ) -> Result<Self, ApiError> {
        let Some(asked) = asked else {
            let names = default
                .iter()
                .copied()
                .filter(|name| allowed.contains(name))
                .collect();
            return Ok(Self { names });
        };
        let mut names = Vec::new();
        for name in asked.split(',').map(str::trim) {
            let known = allowed
                .iter()
                .copied()
                .find(|candidate| *candidate == name)
                .ok_or_else(|| ApiError::new(ErrorCode::UnknownField))?;
            if !names.contains(&known) {
                names.push(known);
            }
        }
        Ok(Self { names })
    }

    /// The item with only the chosen fields. Works on any view that can be serialized to an
    /// object.
    ///
    /// # Errors
    /// An internal error when the view does not serialize to an object (a coding mistake).
    pub(crate) fn project<T: Serialize>(&self, item: &T) -> Result<Value, ApiError> {
        match serde_json::to_value(item) {
            Ok(Value::Object(mut fields)) => {
                fields.retain(|name, _| self.names.contains(&name.as_str()));
                Ok(Value::Object(fields))
            }
            Ok(_) => Err(ApiError::internal(&"a list item is not an object")),
            Err(error) => Err(ApiError::internal(&error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::response::{IntoResponse as _, Response};
    use axum::routing::get;
    use serde_json::json;
    use tower::ServiceExt as _;

    use super::*;
    use crate::errors::ApiQuery;

    const ALLOWED: [&str; 4] = ["id", "title", "status", "cost_center"];
    const DEFAULT: [&str; 2] = ["id", "title"];

    /// One row of the pretend list.
    #[derive(Serialize)]
    struct Row {
        id: String,
        title: String,
        status: String,
        cost_center: String,
    }

    fn rows() -> Vec<Row> {
        (0..5)
            .map(|n| Row {
                id: format!("r{n}"),
                title: format!("Title {n}"),
                status: "open".to_owned(),
                cost_center: "4711".to_owned(),
            })
            .collect()
    }

    /// A list route made of the helpers only. `hidden` plays a field the person may not read.
    fn list(hidden: Option<&'static str>, params: &ListParams) -> Result<Response, ApiError> {
        let allowed: Vec<&'static str> = ALLOWED
            .iter()
            .copied()
            .filter(|name| Some(*name) != hidden)
            .collect();
        let fields = params.fields(&allowed, &DEFAULT)?;
        let after = params.position()?;
        let limit = params.limit() as usize;
        let start = after.map_or(0, |position| position.parse::<usize>().unwrap_or(0) + 1);
        let page: Vec<Row> = rows().into_iter().skip(start).take(limit + 1).collect();
        let more = page.len() > limit;
        let items = page
            .iter()
            .take(limit)
            .map(|row| fields.project(row))
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = more.then(|| paging::seal(&(start + limit - 1).to_string()));
        Ok(axum::Json(json!({"items": items, "next_cursor": next_cursor})).into_response())
    }

    fn app(hidden: Option<&'static str>) -> Router {
        Router::new().route(
            "/list",
            get(move |ApiQuery(params): ApiQuery<ListParams>| async move { list(hidden, &params) }),
        )
    }

    async fn ask(app: Router, query: &str) -> (StatusCode, Value) {
        let request = Request::get(format!("/list?{query}"))
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn perf_02_a_list_answers_only_the_fields_asked_for() {
        let (status, body) = ask(app(None), "fields=id,cost_center&limit=2").await;
        assert_eq!(status, StatusCode::OK);
        for item in body["items"].as_array().unwrap() {
            let mut keys: Vec<&String> = item.as_object().unwrap().keys().collect();
            keys.sort();
            assert_eq!(keys, ["cost_center", "id"]);
        }
        // Left out: the small default of the route, never everything.
        let (_, body) = ask(app(None), "limit=1").await;
        assert_eq!(body["items"][0], json!({"id": "r0", "title": "Title 0"}));
    }

    #[tokio::test]
    async fn perf_02_a_list_refuses_an_unknown_field_like_a_hidden_one() {
        // `salary` does not exist; `cost_center` exists but is hidden from this person.
        let (unknown_status, unknown) = ask(app(Some("cost_center")), "fields=id,salary").await;
        let (hidden_status, hidden) = ask(app(Some("cost_center")), "fields=id,cost_center").await;
        assert_eq!(unknown_status, StatusCode::BAD_REQUEST);
        assert_eq!(hidden_status, unknown_status);
        assert_eq!(unknown["code"], "request.unknown_field");
        // Same code, same message, same everything: nothing tells the two apart.
        assert_eq!(hidden, unknown);
        // Left out, a hidden default is simply not there.
        let (_, body) = ask(app(Some("title")), "limit=1").await;
        assert_eq!(body["items"][0], json!({"id": "r0"}));
    }

    #[tokio::test]
    async fn perf_02_empty_and_misspelled_field_names_are_refused() {
        for query in [
            "fields=",
            "fields=id,,title",
            "fields=ID",
            "fields=id;title",
        ] {
            let (status, body) = ask(app(None), query).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
            assert_eq!(body["code"], "request.unknown_field", "{query}");
        }
    }

    #[tokio::test]
    async fn perf_02_a_changed_cursor_answers_bad_cursor_and_a_good_one_continues() {
        let (_, first) = ask(app(None), "fields=id&limit=2").await;
        let cursor = first["next_cursor"].as_str().unwrap().to_owned();
        let (status, second) = ask(app(None), &format!("fields=id&limit=2&cursor={cursor}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second["items"], json!([{"id": "r2"}, {"id": "r3"}]));
        let (status, body) = ask(app(None), "cursor=nonsense").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "request.bad_cursor");
    }

    #[test]
    fn fields_are_chosen_once_and_in_the_order_of_the_request() {
        let chosen = Fields::choose(Some("title, id,title"), &ALLOWED, &DEFAULT).unwrap();
        assert_eq!(chosen.names, ["title", "id"]);
    }

    #[test]
    fn a_view_that_is_not_an_object_cannot_be_projected() {
        let chosen = Fields::choose(None, &ALLOWED, &DEFAULT).unwrap();
        let error = chosen.project(&["not", "an", "object"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::Internal);
    }

    #[test]
    fn the_documented_maximum_is_the_one_the_server_enforces() {
        // `#[param(maximum = 100)]` needs a literal, so the number is written twice and kept
        // together here.
        let parameters =
            ListParams::into_params(|| Some(utoipa::openapi::path::ParameterIn::Query));
        let limit = parameters
            .iter()
            .find(|parameter| parameter.name == "limit")
            .unwrap();
        let schema = serde_json::to_value(limit.schema.as_ref().unwrap()).unwrap();
        assert_eq!(schema["maximum"], json!(paging::MAX_LIMIT));
        assert_eq!(schema["minimum"], json!(1));
    }
}
