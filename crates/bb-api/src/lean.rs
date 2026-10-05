//! Lean lists: `?fields=`, the cursor and the page size, written once (PERF-02, architecture §7
//! "Performance").
//!
//! A list route takes [`ListParams`], chooses its fields with [`ListParams::fields`], answers
//! one [`Page`] made by [`Page::of_authorized`], and names itself (the *list*) so its cursors are its own.
//! The route only says which fields exist, which are the default and where a page ends; this file,
//! `paging.rs` and `seal.rs` do the rest. ARCH-LEAN checks the API description of every list.
//!
//! A person may only ask for fields they may read. A field that is hidden from them is refused
//! exactly like one that does not exist (`request.unknown_field`, SEC-06), so the answer never
//! tells which fields exist behind a mask. The policy engine (`Authorized<T>`, M2 WP-05) plugs in
//! by passing only the fields the person may read.

use bb_core::ErrorCode;
use bb_core::paging::{PAGE_SIZE_DEFAULT, PAGE_SIZE_MAX};
use bb_policy::{Authorized, Masked, mask_member};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use utoipa::openapi::{Object, ObjectBuilder, RefOr, Schema, Type};
use utoipa::{IntoParams, ToSchema};

use crate::errors::ApiError;
use crate::paging;

/// One field of the items of a list: its name and whether the list holds it when the request
/// does not name fields. A route keeps one such table, so its fields and its default are said
/// once.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FieldSpec {
    pub(crate) name: &'static str,
    pub(crate) default: bool,
}

impl FieldSpec {
    /// A field the list holds by default.
    pub(crate) const fn by_default(name: &'static str) -> Self {
        Self {
            name,
            default: true,
        }
    }

    /// A field that is only there when asked for.
    pub(crate) const fn on_request(name: &'static str) -> Self {
        Self {
            name,
            default: false,
        }
    }
}

/// What every list takes in its query: the fields, the cursor and the page size.
#[derive(Deserialize)]
pub(crate) struct ListParams {
    fields: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
}

/// The parameters as the API description says them. The numbers come from `bb_core::paging`, so
/// the text can never say something else than the server does.
impl IntoParams for ListParams {
    fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
        let place = parameter_in_provider().unwrap_or(ParameterIn::Query);
        let text = ObjectBuilder::new().schema_type(Type::String).build();
        let count = ObjectBuilder::new()
            .schema_type(Type::Integer)
            .minimum(Some(1))
            .maximum(Some(PAGE_SIZE_MAX))
            .build();
        let parameter = |name: &str, description: String, schema: Object| {
            ParameterBuilder::new()
                .name(name)
                .parameter_in(place.clone())
                .description(Some(description))
                .schema(Some(RefOr::T(Schema::Object(schema))))
                .build()
        };
        vec![
            parameter(
                "fields",
                "The fields each item holds, separated by commas. The answer schema names every field that can be asked for; when this is left out the route's own small default is used.".to_owned(),
                text.clone(),
            ),
            parameter(
                "cursor",
                "Where the page starts: the `next_cursor` of the page before.".to_owned(),
                text,
            ),
            parameter(
                "limit",
                format!(
                    "How many items a page holds at most (default {PAGE_SIZE_DEFAULT}, at most {PAGE_SIZE_MAX})."
                ),
                count,
            ),
        ]
    }
}

impl ListParams {
    /// The fields to answer with, out of the fields the person may ask for: the names in `hidden`
    /// (fields the mask of the person hides) are left out of the table first, so asking for one
    /// fails exactly like asking for a field that does not exist (SEC-06).
    ///
    /// # Errors
    /// `request.unknown_field` for a name that is not in `fields`, or is hidden.
    pub(crate) fn fields_readable(
        &self,
        fields: &[FieldSpec],
        hidden: &[&str],
    ) -> Result<Fields, ApiError> {
        let readable: Vec<FieldSpec> = fields
            .iter()
            .filter(|field| !hidden.contains(&field.name))
            .copied()
            .collect();
        Fields::choose(self.fields.as_deref(), &readable)
    }

    /// Where the page starts, if the request named a cursor. `list` names the list, so a cursor
    /// of another list is refused.
    ///
    /// # Errors
    /// `request.bad_cursor` for a cursor that was changed or is not one of this list.
    pub(crate) fn position(&self, list: &str) -> Result<Option<String>, ApiError> {
        self.cursor
            .as_deref()
            .map(|cursor| paging::open(list, cursor))
            .transpose()
    }

    /// How many items the page holds at most.
    pub(crate) fn limit(&self) -> u32 {
        bb_core::paging::page_size(self.limit)
    }
}

/// The fields a list answers with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fields {
    names: Vec<&'static str>,
}

impl Fields {
    /// The fields asked for (`asked`, names separated by commas), or the ones marked as default
    /// when the request said nothing.
    ///
    /// # Errors
    /// `request.unknown_field` for a name that is not in `fields`, whether it does not exist or
    /// the person may not read it.
    pub(crate) fn choose(asked: Option<&str>, fields: &[FieldSpec]) -> Result<Self, ApiError> {
        let Some(asked) = asked else {
            let names = fields
                .iter()
                .filter(|field| field.default)
                .map(|field| field.name)
                .collect();
            return Ok(Self { names });
        };
        let mut names = Vec::new();
        for name in asked.split(',').map(str::trim) {
            let known = fields
                .iter()
                .map(|field| field.name)
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

/// One page of a list. `T` is the view of an item for the API description; an answer holds the
/// items cut to the chosen fields ([`Page::of_authorized`]).
#[derive(Serialize, ToSchema)]
pub(crate) struct Page<T> {
    /// The items of this page.
    items: Vec<T>,
    /// The cursor of the next page; `null` on the last page.
    #[schema(required = true)]
    next_cursor: Option<String>,
}

impl Page<Value> {
    /// The answer for views that came out of a use case as `Authorized`: the hidden fields are left
    /// out first, then the items are cut to the chosen fields.
    ///
    /// # Errors
    /// An internal error when the views do not serialize to a list of objects or the cursor cannot
    /// be sealed.
    pub(crate) fn of_authorized<V: Masked + Serialize>(
        list: &str,
        views: &Authorized<Listed<V>>,
        fields: &Fields,
    ) -> Result<Self, ApiError> {
        let masked = views
            .masked_value()
            .map_err(|error| ApiError::internal(&error))?;
        let Some(Value::Array(items)) = masked.get("items") else {
            return Err(ApiError::internal(&"the views are not a list"));
        };
        let next = masked.get("next").and_then(Value::as_str);
        Ok(Self {
            items: items
                .iter()
                .map(|item| fields.project(item))
                .collect::<Result<_, _>>()?,
            next_cursor: next
                .map(|position| paging::seal(list, position))
                .transpose()?,
        })
    }
}

/// The items of a page as a use case gave them, before they are cut to the fields asked for and the
/// position is sealed: what `Authorized` wraps, so the mask reaches the items.
#[derive(Serialize)]
pub(crate) struct Listed<V> {
    pub(crate) items: Vec<V>,
    pub(crate) next: Option<String>,
}

impl<V: Masked> Masked for Listed<V> {
    fn mask(value: &mut Value, mask: &bb_policy::FieldMask) {
        mask_member::<Vec<V>>(value, "items", mask);
    }

    fn hidden_properties(mask: &bb_policy::FieldMask) -> Vec<&'static str> {
        V::hidden_properties(mask)
    }
}

/// For tests of a route: every name in its field table is a key of the view it serializes, so a
/// field cannot be asked for and then be missing, or exist without being askable.
#[cfg(test)]
pub(crate) fn assert_fields_are_the_keys_of<T: Serialize>(fields: &[FieldSpec], view: &T) {
    use std::collections::BTreeSet;

    let value = serde_json::to_value(view).unwrap();
    let keys: BTreeSet<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let names: BTreeSet<&str> = fields.iter().map(|field| field.name).collect();
    assert_eq!(names, keys, "the field table and the view disagree");
    assert!(
        fields.iter().any(|field| field.default),
        "a list needs at least one default field"
    );
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::response::{IntoResponse as _, Response};
    use axum::routing::get;
    use serde_json::json;
    use tower::ServiceExt as _;

    use bb_core::access::{AppliesTo, FieldLevel};
    use bb_core::fields::Field;
    use bb_core::id::{AssignmentId, ProjectId, RoleId, SequentialIds, UserId};
    use bb_policy::{
        Evaluation, FieldRule, Inputs, Place, RoleData, Row as PolicyRow, Who, evaluate,
    };

    use super::*;
    use crate::errors::ApiQuery;

    const LIST: &str = "rows";
    const FIELDS: [FieldSpec; 4] = [
        FieldSpec::by_default("id"),
        FieldSpec::by_default("title"),
        FieldSpec::on_request("status"),
        FieldSpec::on_request("cost_center"),
    ];

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

    impl Masked for Row {
        const FIELDS: &'static [(&'static str, Field)] = &[
            ("title", Field::Title),
            ("status", Field::Status),
            ("cost_center", Field::CostCenter),
        ];
    }

    /// What the person who asks may do: a role that changes every field of the catalogue except
    /// the one that is hidden from them. The mask is the engine's, not a list made up here.
    fn viewer(hidden: Option<Field>) -> Evaluation {
        let mut ids = SequentialIds::default();
        let person = UserId::generate(&mut ids);
        let role = RoleId::generate(&mut ids).unwrap();
        let project = ProjectId::generate(&mut ids).unwrap();
        evaluate(&Inputs {
            person,
            place: Place::Project(project),
            chain: vec![project],
            rows: vec![PolicyRow {
                id: AssignmentId::generate(&mut ids).unwrap(),
                who: Who::Person,
                role: Some(role),
                project: Some(project),
                applies_to: AppliesTo::ProjectAndSubProjects,
            }],
            roles: vec![RoleData {
                id: role,
                name: "Role".to_owned(),
                position: 0,
                permissions: std::collections::BTreeSet::new(),
                two_step: false,
            }],
            field_rules: Field::ALL
                .iter()
                .map(|field| FieldRule {
                    project: None,
                    role,
                    field: *field,
                    level: if Some(*field) == hidden {
                        FieldLevel::Hidden
                    } else {
                        FieldLevel::CanChange
                    },
                })
                .collect(),
            people_rules: Vec::new(),
        })
    }

    /// A list route made of the helpers only. `hidden` is a field the mask of the person hides.
    fn list(hidden: Option<Field>, params: &ListParams) -> Result<Response, ApiError> {
        let after = params.position(LIST)?;
        let limit = params.limit() as usize;
        let start = after.map_or(0, |position| position.parse::<usize>().unwrap_or(0) + 1);
        let mut page: Vec<Row> = rows().into_iter().skip(start).take(limit + 1).collect();
        let more = page.len() > limit;
        let end = (start + limit - 1).to_string();
        page.truncate(limit);
        let list = viewer(hidden).authorize(Listed {
            items: page,
            next: more.then_some(end),
        });
        let fields = params.fields_readable(&FIELDS, &list.hidden_properties())?;
        let page = Page::of_authorized(LIST, &list, &fields)?;
        Ok(axum::Json(page).into_response())
    }

    fn app(hidden: Option<Field>) -> Router {
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
        let (unknown_status, unknown) = ask(app(Some(Field::CostCenter)), "fields=id,salary").await;
        let (hidden_status, hidden) =
            ask(app(Some(Field::CostCenter)), "fields=id,cost_center").await;
        assert_eq!(unknown_status, StatusCode::BAD_REQUEST);
        assert_eq!(hidden_status, unknown_status);
        assert_eq!(unknown["code"], "request.unknown_field");
        // Same code, same message, same everything: nothing tells the two apart.
        assert_eq!(hidden, unknown);
        // Left out, a hidden default is simply not there.
        let (_, body) = ask(app(Some(Field::Title)), "limit=1").await;
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
    async fn perf_02_every_page_answers_next_cursor_and_the_last_one_answers_null() {
        let (_, first) = ask(app(None), "fields=id&limit=2").await;
        let cursor = first["next_cursor"].as_str().unwrap().to_owned();
        let (status, second) = ask(app(None), &format!("fields=id&limit=2&cursor={cursor}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second["items"], json!([{"id": "r2"}, {"id": "r3"}]));
        let (_, last) = ask(app(None), "fields=id&limit=100").await;
        assert_eq!(last["items"].as_array().map(Vec::len), Some(5));
        assert!(last.as_object().unwrap().contains_key("next_cursor"));
        assert_eq!(last["next_cursor"], Value::Null);
    }

    #[tokio::test]
    async fn perf_02_a_changed_made_up_or_foreign_cursor_answers_bad_cursor() {
        let (_, first) = ask(app(None), "limit=2").await;
        let mut changed = first["next_cursor"].as_str().unwrap().to_owned();
        changed.pop();
        changed.push('A');
        let foreign = paging::seal("another-list", "1").unwrap();
        for cursor in ["nonsense".to_owned(), changed, foreign] {
            let (status, body) = ask(app(None), &format!("cursor={cursor}")).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{cursor}");
            assert_eq!(body["code"], "request.bad_cursor", "{cursor}");
        }
    }

    #[test]
    fn fields_are_chosen_once_and_in_the_order_of_the_request() {
        let chosen = Fields::choose(Some("title, id,title"), &FIELDS).unwrap();
        assert_eq!(chosen.names, ["title", "id"]);
    }

    #[test]
    fn a_view_that_is_not_an_object_cannot_be_projected() {
        let chosen = Fields::choose(None, &FIELDS).unwrap();
        let error = chosen.project(&["not", "an", "object"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::Internal);
    }

    #[test]
    fn perf_02_the_field_table_and_the_view_name_the_same_fields() {
        let row = rows().remove(0);
        assert_fields_are_the_keys_of(&FIELDS, &row);
    }

    #[test]
    #[should_panic(expected = "the field table and the view disagree")]
    fn perf_02_a_field_that_is_not_in_the_view_is_found() {
        let row = rows().remove(0);
        assert_fields_are_the_keys_of(&FIELDS[..3], &row);
    }

    #[test]
    fn perf_02_the_description_of_the_parameters_says_what_the_server_enforces() {
        let parameters = ListParams::into_params(|| Some(ParameterIn::Query));
        let found: BTreeMap<String, Value> = parameters
            .iter()
            .map(|parameter| {
                (
                    parameter.name.clone(),
                    serde_json::to_value(parameter).unwrap(),
                )
            })
            .collect();
        assert_eq!(
            found.keys().map(String::as_str).collect::<Vec<_>>(),
            ["cursor", "fields", "limit"]
        );
        let limit = &found["limit"];
        assert_eq!(limit["schema"]["maximum"], json!(PAGE_SIZE_MAX));
        assert_eq!(limit["schema"]["minimum"], json!(1));
        let text = limit["description"].as_str().unwrap();
        assert!(text.contains(&PAGE_SIZE_DEFAULT.to_string()), "{text}");
        assert!(text.contains(&PAGE_SIZE_MAX.to_string()), "{text}");
        for parameter in found.values() {
            assert_eq!(parameter["in"], "query");
        }
    }
}
