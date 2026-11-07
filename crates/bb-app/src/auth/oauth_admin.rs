//! What an administrator and the server itself ask of the sign-in of AI apps (AI-22, ADR-0042): the
//! web addresses apps may come back to, the daily clean-up, who a call to the MCP server is from, and
//! where the sign-in is.

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::oauth::{MCP_PATH, MOST_WEB_ADDRESSES, OAuthWebAddress, ReturnAddress, ReturnKind};
use bb_core::sign_in::SecurityAction;
use bb_core::time::days;
use bb_policy::{Authorized, Permission, Place};
use serde_json::json;

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed, internal};
use super::events::{by, changed, set_to};
use super::oauth_types::{
    AddWebAddress, McpIdentity, McpIdentityQuery, OAuthMetadata, OAuthMetadataQuery,
    RemoveWebAddress, WebAddressView, WebAddresses, WebAddressesQuery,
};
use super::store::AuthStore;
use super::types::Done;
use crate::access::CredentialSource;

/// The web addresses administrators allowed, for *Apps that may sign in*.
///
/// # Errors
/// `access.forbidden` for someone who may not change the settings of the organization.
pub async fn web_addresses<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: WebAddressesQuery,
) -> Result<Authorized<WebAddresses>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let items = ctx
        .store()
        .oauth_web_addresses()
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .map(|each| WebAddressView {
            address: each.address,
            added_at: each.added_at,
        })
        .collect();
    Ok(access.authorize(WebAddresses { items }))
}

/// Allows a web address for AI apps.
///
/// # Errors
/// `access.forbidden` for someone who may not change the settings of the organization,
/// `request.invalid` for an address that is not a web address or too many addresses.
pub async fn add_web_address<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AddWebAddress,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(
        &actor,
        Permission::ChangeOrganizationSettings,
        Place::Organization,
    )
    .await?;
    let address = input.address.trim().to_owned();
    if ReturnAddress::read(&address).map(|read| read.kind()) != Some(ReturnKind::Web) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let listed = ctx
        .store()
        .oauth_web_addresses()
        .await
        .map_err(|error| internal::<S>(&error))?;
    if listed.len() >= MOST_WEB_ADDRESSES {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let row = OAuthWebAddress {
        address,
        added_by: actor.person.id,
        added_at: ctx.now(),
    };
    let event = ctx.event(
        by(actor.person.id),
        crate::auth::events::fixed_entity(RecordKind::Setting, "oauth-web-addresses")?,
        vec![set_to("added", json!(row.address))],
    )?;
    let stored = row.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::insert_oauth_web_address(transaction, &stored)
                    .await
                    .map(|_| ())
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::OAuthAddressChanged,
        Some(actor.person.id),
        None,
    )
    .await;
    Ok(Done)
}

/// Takes a web address away. Apps that registered with it are refused at their next sign-in.
///
/// # Errors
/// `access.forbidden` for someone who may not change the settings of the organization,
/// `resource.not-found` for an address that was not allowed.
pub async fn remove_web_address<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RemoveWebAddress,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(
        &actor,
        Permission::ChangeOrganizationSettings,
        Place::Organization,
    )
    .await?;
    let address = input.address;
    let event = ctx.event(
        by(actor.person.id),
        crate::auth::events::fixed_entity(RecordKind::Setting, "oauth-web-addresses")?,
        vec![changed("removed")],
    )?;
    let wanted = address.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::delete_oauth_web_address(transaction, &wanted).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| {
            change_failed::<S>(&error, Stops::new(ErrorCode::NotFound, ErrorCode::NotFound))
        })?;
    ctx.note(
        SecurityAction::OAuthAddressChanged,
        Some(actor.person.id),
        None,
    )
    .await;
    Ok(Done)
}

/// The daily clean-up (`oauth.clean-old-grants`): codes and tokens that ran out, grants that ended
/// or ran out long ago, and apps that registered and were never allowed. While AI apps are off
/// nothing is deleted and the days without use do not count, as ADR-0042 says.
pub async fn clean_old_grants<S: AuthStore>(
    ctx: &Ctx<'_, S>,
) -> Result<super::oauth_store::OAuthCleaned, AppError> {
    let now = ctx.now();
    // While AI apps are off the days without use do not count: the tokens that were not used are
    // moved by a day, and nothing is deleted (ADR-0042), so switching on again brings the apps back.
    let apps_on = ctx.apps_may_sign_in().await?;
    let back = |count: i64| {
        now.checked_sub(days(count))
            .map_err(|error| AppError::Internal(error.to_string()))
    };
    let (ended_before, registered_before) = (
        back(bb_core::oauth::ENDED_GRANT_KEEP_DAYS)?,
        back(bb_core::oauth::UNUSED_CLIENT_DAYS)?,
    );
    let event = ctx.event(
        bb_core::event::Actor::System,
        crate::auth::events::fixed_entity(RecordKind::ConnectedApp, "clean")?,
        vec![changed("cleaned")],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if apps_on {
                    S::clean_oauth(transaction, now, ended_before, registered_before).await
                } else {
                    S::pause_unused_tokens(transaction, now, 24 * 60 * 60)
                        .await
                        .map(|_| super::oauth_store::OAuthCleaned::default())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))
}

/// Who a call to the MCP server is from: the person, the app by name, and the agent when an agent
/// key signed in. The server opens the run of a session of an app with it (ADR-0042, decision 5).
///
/// # Errors
/// `auth.not-signed-in` for a token or key that does not work.
pub async fn mcp_identity<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: McpIdentityQuery,
) -> Result<McpIdentity, AppError> {
    // AI apps off (or AI off for the organization) turns away every way in to MCP, whatever the
    // credential: an app, an access key or an agent key. It is no guess of a network.
    if !ctx.apps_may_sign_in().await? {
        return Err(AppError::Refused(ErrorCode::NotSignedIn));
    }
    let actor = ctx.actor(&input.token).await?;
    // Only a credential for programs opens MCP: the secret of a session of the browser does not.
    if actor.key.is_none() {
        return Err(AppError::Refused(ErrorCode::NotSignedIn));
    }
    // Every message to the MCP server counts against the limits of its grant (or key) and person.
    if let Err(error) = ctx.count_call(&actor) {
        ctx.note(SecurityAction::AiCallLimited, Some(actor.person.id), None)
            .await;
        return Err(error);
    }
    let app = match actor.key.as_ref().map(|key| &key.source) {
        Some(CredentialSource::App { name, .. }) => Some(name.clone()),
        _ => None,
    };
    let credential = match actor.key.as_ref().map(|key| &key.source) {
        Some(CredentialSource::App { grant, .. }) => grant.to_string(),
        Some(CredentialSource::Key(key)) => key.to_string(),
        None => actor.person.id.to_string(),
    };
    Ok(McpIdentity {
        person: actor.person.id,
        credential,
        app,
        agent: actor.agent(),
    })
}

/// Where the sign-in of AI apps is: the public address of the server when it knows it, else the
/// address the request came to.
///
/// # Errors
/// Never fails today; the type is the one of a use case.
pub async fn oauth_metadata<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: OAuthMetadataQuery,
) -> Result<OAuthMetadata, AppError> {
    Ok(OAuthMetadata {
        origin: ctx.mcp_origin(),
        resource: ctx.mcp_resource(),
    })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The address of this server as it is told to apps: the public address when the server knows
    /// it, else the address the request came to, never with a slash at the end.
    pub(crate) fn mcp_origin(&self) -> String {
        let known = self.services.public_url.trim_end_matches('/');
        if known.is_empty() {
            self.request.origin_seen.trim_end_matches('/').to_owned()
        } else {
            known.to_owned()
        }
    }

    /// The resource a token is for: the MCP server of this server. The one place that builds it.
    pub(crate) fn mcp_resource(&self) -> String {
        format!("{}{MCP_PATH}", self.mcp_origin())
    }
}
