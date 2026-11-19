//! The text of `/.well-known/security.txt` (COMP-03, ADR-0047 decision 4): asked for by anybody,
//! signed in or not, so it reads only the organization's contact for security reports
//! (`security.contact`) and gives nothing else of the organization away.

use bb_core::release::{RELEASE_DATE, security_txt as text_of};
use bb_core::settings::security::SECURITY_CONTACT;

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Who asks for `security.txt`: anybody, before anybody signs in.
pub struct SecurityTxtQuery;

/// The text of `security.txt`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecurityTxt {
    /// The fields, one a line, as RFC 9116 writes them.
    pub text: String,
}

/// The text of `security.txt` for this organization and this release.
///
/// # Errors
/// `internal` when the settings cannot be read.
pub async fn security_txt<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: SecurityTxtQuery,
) -> Result<SecurityTxt, AppError> {
    let layers = ctx
        .store()
        .setting_layers(None, None)
        .await
        .map_err(|error| internal::<S>(&error))?;
    Ok(SecurityTxt {
        text: text_of(&layers.text(&SECURITY_CONTACT), RELEASE_DATE),
    })
}
