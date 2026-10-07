//! The people routes below `/api/v1/people` (SEC-02, PRIV-12, M2-Q3): the list of people and the
//! page of one person, for those who hold *See everybody's e-mail address and correct their
//! names*. A person is shown by name; the groups and roles that go with them are shown as far as
//! the viewer may see them.

use axum::extract::State;
use axum::response::Response;
use bb_app::people::{PeopleQuery, PersonLine, PersonQuery};
use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::{GroupRef, RoleAtRef, cursor_error};
use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, NotFoundAnswer, NotSignedInAnswer, Session, answer,
    answer_authorized,
};
use crate::errors::{ApiError, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of a person in a list that can be asked for, and which of them a list holds when
/// the request names none: what the list needs.
const PEOPLE_FIELDS: [FieldSpec; 4] = [
    FieldSpec::by_default("person"),
    FieldSpec::by_default("email"),
    FieldSpec::on_request("groups"),
    FieldSpec::on_request("roles"),
];
/// The name of the list, so its cursors are its own.
const PEOPLE_LIST: &str = "people";

/// A person in a list, or on the page of one: the fields that can be asked for. An answer holds
/// only those that were asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct PersonItem {
    /// The person, by name: whoever holds *Manage people* sees names.
    #[serde(skip_serializing_if = "Option::is_none")]
    person: Option<PersonRef>,
    /// The e-mail address. Only people who manage people see it.
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    /// The groups the person is in. Left out when the viewer may not see groups.
    #[serde(skip_serializing_if = "Option::is_none")]
    groups: Option<Vec<GroupRef>>,
    /// The roles the person has through rows of their own, in the places where the viewer
    /// decides who has which role.
    #[serde(skip_serializing_if = "Option::is_none")]
    roles: Option<Vec<RoleAtRef>>,
}

impl Masked for PersonItem {}

impl From<&PersonLine> for PersonItem {
    fn from(line: &PersonLine) -> Self {
        Self {
            person: Some(line.person.clone()),
            email: Some(line.email.clone()),
            groups: line
                .groups
                .as_ref()
                .map(|groups| groups.iter().map(GroupRef::from).collect()),
            roles: Some(line.roles.iter().map(RoleAtRef::from).collect()),
        }
    }
}

/// What narrows the list of people.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct PeopleFilter {
    /// Words that every found person holds in their name or e-mail address, without regard to
    /// capitals.
    search: Option<String>,
}

/// One person with their groups and roles.
#[derive(Serialize, ToSchema)]
pub(crate) struct PersonDetailBody {
    /// The person, with their e-mail address, groups and roles.
    person: PersonItem,
    /// When the account was made.
    created_at: String,
}

impl Masked for PersonDetailBody {}

/// The people, by name, a page at a time. Needs the permission *See everybody's e-mail address
/// and correct their names*.
#[utoipa::path(
    get,
    path = "/api/v1/people",
    operation_id = "listPeople",
    tag = "people",
    extensions(("x-never-stored" = json!(true))),
    params(ListParams, PeopleFilter),
    responses((status = 200, description = "The people, by name.", body = Page<PersonItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_people(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
    ApiQuery(filter): ApiQuery<PeopleFilter>,
) -> Result<Response, ApiError> {
    // Who asks is judged first (401 before 400): a cursor that does not open is passed on as one
    // the use case cannot read, so its answer comes after the check of the person.
    let position = params.position(PEOPLE_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let page = auth
        .api()?
        .people(
            info,
            PeopleQuery {
                session_token,
                search: filter.search,
                cursor,
                limit: params.limit(),
                // The groups and the roles of a person are read only when the request asks.
                groups: params.asks_for(&PEOPLE_FIELDS, "groups"),
                roles: params.asks_for(&PEOPLE_FIELDS, "roles"),
            },
        )
        .await
        .map_err(cursor_error)?;
    position?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(PersonItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&PEOPLE_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(PEOPLE_LIST, &list, &fields)?))
}

/// One person with their e-mail address, groups and roles. Needs the permission *See everybody's
/// e-mail address and correct their names*.
#[utoipa::path(
    get,
    path = "/api/v1/people/{person_id}",
    operation_id = "getPerson",
    tag = "people",
    extensions(("x-never-stored" = json!(true))),
    params(("person_id" = String, Path, description = "The ID of the person.")),
    responses((status = 200, description = "The person.", body = PersonDetailBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_person(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(person_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let person = parse::<UserId>(&person_id)?;
    let found = auth
        .api()?
        .person(
            info,
            PersonQuery {
                session_token,
                person,
            },
        )
        .await?;
    answer_authorized(&found.map(|detail| PersonDetailBody {
        person: PersonItem::from(&detail.line),
        created_at: detail.created_at.to_string(),
    }))
}

/// The ID in an address of the API; a text that is no ID names nothing.
pub(crate) fn parse<Id: std::str::FromStr>(text: &str) -> Result<Id, ApiError> {
    text.parse().map_err(|_| ApiError::new(ErrorCode::NotFound))
}

/// The routes of the people.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_people))
        .routes(routes!(get_person))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_a_person_in_a_list() {
        let item = PersonItem {
            person: Some(PersonRef::Someone),
            email: Some("e".to_owned()),
            groups: Some(Vec::new()),
            roles: Some(Vec::new()),
        };
        crate::lean::assert_fields_are_the_keys_of(&PEOPLE_FIELDS, &item);
    }
}
