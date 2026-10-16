//! The hosts that images may come from.

use super::{
    ApiError, AuthHandle, ImageHostsQuery, Info, Masked, NotSignedInAnswer, Response, Serialize,
    Session, State, ToSchema, answer_authorized,
};

/// The hosts that images in text may come from, already split into one host for each item.
#[derive(Serialize, ToSchema)]
pub(crate) struct ImageHostsBody {
    /// The hosts, such as `images.example.com`. Empty means no image from outside.
    hosts: Vec<String>,
}

impl Masked for ImageHostsBody {}

/// The hosts that images in text may come from (the setting `content.image-hosts`), for the preview
/// of the editor. Every signed-in person may read them: they are in the content security policy
/// that every page carries. The list is split here once; the browser never splits it again.
#[utoipa::path(
    get,
    path = "/api/v1/content/image-hosts",
    operation_id = "getImageHosts",
    tag = "settings",
    responses((status = 200, description = "The hosts.", body = ImageHostsBody), NotSignedInAnswer)
)]
pub(super) async fn get_image_hosts(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let hosts = auth
        .api()?
        .image_hosts(info, ImageHostsQuery { session_token })
        .await?;
    answer_authorized(&hosts.map(|found| ImageHostsBody { hosts: found.hosts }))
}
