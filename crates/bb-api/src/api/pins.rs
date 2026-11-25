//! The pins of a person (CFG-05, ADR-0050, decision 12): `/api/v1/me/pins`.
//!
//! A pin is the person's own: only a session reaches these routes, never an access key. The list
//! holds only the pins the person may open now; one that they may not is left out with no word and
//! no count.

use axum::extract::State;
use axum::response::Response;
use bb_app::pins::{NamedPin, OrderPins, PinList, PinRequest, PinsQuery};
use bb_core::pin::PinKind;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized, done,
};
use crate::errors::{ApiError, ApiJson, ApiQuery};

/// One pin of the person.
#[derive(Serialize, ToSchema)]
pub(crate) struct PinItem {
    /// A project or a saved search.
    kind: PinKind,
    /// The ID of the project or the saved search.
    id: String,
    /// Its name.
    name: String,
    /// The short name of a project, which is in its address; none for a saved search.
    #[serde(skip_serializing_if = "Option::is_none")]
    short_name: Option<String>,
}

/// The pins of the person that they may open, in their order.
#[derive(Serialize, ToSchema)]
pub(crate) struct PinListBody {
    /// The pins.
    items: Vec<PinItem>,
}

impl Masked for PinListBody {}

impl From<PinList> for PinListBody {
    fn from(list: PinList) -> Self {
        Self {
            items: list
                .items
                .into_iter()
                .map(|view| PinItem {
                    kind: view.kind,
                    id: view.id.to_string(),
                    name: view.name,
                    short_name: view.short_name,
                })
                .collect(),
        }
    }
}

/// What a pin points to.
#[derive(Deserialize, ToSchema)]
pub(crate) struct PinBody {
    /// A project or a saved search.
    kind: PinKind,
    /// The ID of the project or the saved search.
    target: String,
}

/// What a pin to take away points to.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct PinParams {
    /// A project or a saved search.
    kind: PinKind,
    /// The ID of the project or the saved search.
    target: String,
}

/// The pins in the order the person wants them.
#[derive(Deserialize, ToSchema)]
pub(crate) struct OrderBody {
    /// The pins the person sees, in the new order.
    pins: Vec<PinBody>,
}

fn request(session_token: String, kind: PinKind, target: String) -> PinRequest {
    PinRequest {
        session_token,
        kind,
        target,
    }
}

/// The pins of the person that they may open, in their order.
#[utoipa::path(
    get,
    path = "/api/v1/me/pins",
    operation_id = "listPins",
    tag = "pins",
    responses((status = 200, description = "The pins.", body = PinListBody), NotSignedInAnswer)
)]
async fn list_pins(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let list = auth
        .api()?
        .my_pins(info, PinsQuery { session_token })
        .await?;
    answer_authorized(&list.map(PinListBody::from))
}

/// Pins a project or a saved search that the person may open to the end of their pins. Pinning
/// what is pinned changes nothing.
#[utoipa::path(
    post,
    path = "/api/v1/me/pins",
    operation_id = "createPin",
    tag = "pins",
    request_body = PinBody,
    responses(
        (status = 204, description = "The pin is there."),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn create_pin(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<PinBody>, ApiError>,
) -> Result<Response, ApiError> {
    let ApiJson(body) = auth.after_who_asks(&info, &session_token, body).await?;
    auth.api()?
        .pin_to_sidebar(info, request(session_token, body.kind, body.target))
        .await?;
    Ok(done())
}

/// Takes a pin away. Taking away what is not pinned changes nothing.
#[utoipa::path(
    delete,
    path = "/api/v1/me/pins",
    operation_id = "deletePin",
    tag = "pins",
    params(PinParams),
    responses((status = 204, description = "The pin is not there."), InvalidAnswer, NotSignedInAnswer)
)]
async fn delete_pin(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    params: Result<ApiQuery<PinParams>, ApiError>,
) -> Result<Response, ApiError> {
    let ApiQuery(params) = auth.after_who_asks(&info, &session_token, params).await?;
    auth.api()?
        .unpin(info, request(session_token, params.kind, params.target))
        .await?;
    Ok(done())
}

/// Puts the pins named in the order given. The pins the person does not see keep their places.
#[utoipa::path(
    put,
    path = "/api/v1/me/pins/order",
    operation_id = "orderPins",
    tag = "pins",
    request_body = OrderBody,
    responses((status = 204, description = "The pins are in the new order."), InvalidAnswer, NotSignedInAnswer)
)]
async fn order_pins(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<OrderBody>, ApiError>,
) -> Result<Response, ApiError> {
    let ApiJson(body) = auth.after_who_asks(&info, &session_token, body).await?;
    let pins = body
        .pins
        .into_iter()
        .map(|pin| NamedPin {
            kind: pin.kind,
            target: pin.target,
        })
        .collect();
    auth.api()?
        .order_pins(
            info,
            OrderPins {
                session_token,
                pins,
            },
        )
        .await?;
    Ok(done())
}

/// The routes of pins.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_pins, create_pin, delete_pin))
        .routes(routes!(order_pins))
}
