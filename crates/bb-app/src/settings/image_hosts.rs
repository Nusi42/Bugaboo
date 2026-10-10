//! The hosts that images in text may come from (SEC-13, the setting `content.image-hosts`): read
//! once here, already split with the registry's own reading of a list, so no screen and no
//! renderer splits it again (M3-F10, M3-F11).

use bb_core::settings::{CONTENT_IMAGE_HOSTS, list_items};
use bb_policy::{Authorized, Masked, Place};
use serde::Serialize;

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Who asks for the image hosts, signed in: the editor's preview needs them.
pub struct ImageHostsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// The image hosts of the page itself, asked for before anybody signs in: the browser needs them
/// in the content security policy of the document (`img-src`).
pub struct DocumentImagesQuery;

/// The hosts that images in text may come from, one host for each item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ImageHosts {
    /// The hosts, such as `images.example.com`. Empty means no image from outside.
    pub hosts: Vec<String>,
}

impl Masked for ImageHosts {}

impl<S: AuthStore> Ctx<'_, S> {
    /// The hosts images in text may come from: the organization's setting `content.image-hosts`.
    pub(crate) async fn image_hosts(&self) -> Result<Vec<String>, AppError> {
        let layers = self
            .store()
            .setting_layers(None, None)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let value = layers.resolve(CONTENT_IMAGE_HOSTS.definition()).value;
        let text = CONTENT_IMAGE_HOSTS
            .resolve(Some(&value))
            .unwrap_or_default();
        Ok(list_items(&text).map(str::to_owned).collect())
    }
}

/// The image hosts for a signed-in person (any person may read them: they are in the content
/// security policy of every page).
///
/// # Errors
/// `auth.not_signed_in` without a session.
pub async fn image_hosts<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ImageHostsQuery,
) -> Result<Authorized<ImageHosts>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let hosts = ctx.image_hosts().await?;
    let access = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(access.authorize(ImageHosts { hosts }))
}

/// The image hosts for the document, which is sent before anybody has signed in. They are no
/// secret: the policy that carries them is sent to everyone.
///
/// # Errors
/// Fails when the store fails.
pub async fn document_image_hosts<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: DocumentImagesQuery,
) -> Result<ImageHosts, AppError> {
    Ok(ImageHosts {
        hosts: ctx.image_hosts().await?,
    })
}
