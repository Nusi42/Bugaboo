//! The list of a person's e-mailed searches (QRY-07), under *My settings › Notification settings*.

use bb_core::search_mail::{SEARCH_MAIL_ISSUES_MAX, SEARCH_MAILS_MAX};
use bb_core::settings::MAIL_SHORT;
use bb_policy::{Authorized, Place};

use super::sending;
use super::types::{SearchMailList, SearchMailsQuery};
use super::write::view_of;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The e-mailed searches of the person, oldest first, each with the name of its saved search while
/// they may open it, and whether the organization lets them send and uses *Short e-mails*.
///
/// # Errors
/// `auth.not-signed-in` without a session.
pub async fn search_mails<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SearchMailsQuery,
) -> Result<Authorized<SearchMailList>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let mails = ctx
        .store()
        .search_mails_of(actor.person.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let reaching = ctx.reaching(&actor).await?;
    let mut judge = ctx.judge(&actor, &reaching).await?;
    let mut items = Vec::with_capacity(mails.len());
    for mail in &mails {
        // The name is the person's to read only while the search opens for them: a search they
        // lost tells them nothing new, not even its new name.
        let name = match ctx.stored_search(mail.search).await? {
            Some(found) if ctx.may_open(&mut judge, &reaching, &found).await? => {
                Some(found.search.name)
            }
            _ => None,
        };
        items.push(view_of(mail, name));
    }
    drop(judge);
    let layers = ctx.setting_layers_of(None).await?;
    let organization = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(organization.authorize(SearchMailList {
        items,
        most: u32::try_from(SEARCH_MAILS_MAX).unwrap_or(u32::MAX),
        most_listed: u32::try_from(SEARCH_MAIL_ISSUES_MAX).unwrap_or(u32::MAX),
        sending: sending(&layers),
        short_e_mails: layers.yes(&MAIL_SHORT),
    }))
}
