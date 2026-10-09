//! Issue types (ISS-02): the list, making one, changing one, and the four of the first start.
//!
//! Types are the same in every project. Changing them needs *Manage issue types, kinds of
//! relation and layouts* for the whole organization (M3-Q6). A type is never deleted: it is
//! retired, so no new issues have it while the old ones keep it. A name is used once, without
//! regard to capitals; that is judged inside the change, after the policy lock, so two people who
//! make the same name at the same moment cannot both succeed.

use bb_core::ErrorCode;
use bb_core::event::{Actor, FieldChange, RecordKind};
use bb_core::id::IssueTypeId;
use bb_core::issue::{IssueType, STARTING_ISSUE_TYPES};
use bb_core::message::Message;
use bb_core::paging::window;
use bb_policy::{Authorized, Permission, Place};
use serde_json::json;

use super::types::{ChangeIssueType, IssueTypePage, IssueTypeView, IssueTypesQuery, MakeIssueType};
use crate::auth::{
    AppError, AuthStore, Ctx, Stops, by, change_failed, entity, fixed_entity, internal, set_to,
};
use crate::input::clean_name;
use crate::projects::Why;

/// Whether another type than `except` already has the name, without regard to capitals.
fn name_taken(types: &[IssueType], name: &str, except: Option<IssueTypeId>) -> bool {
    let lower = name.to_lowercase();
    types
        .iter()
        .any(|found| Some(found.id) != except && found.name.to_lowercase() == lower)
}

/// Lists the issue types in the order they are offered in, retired ones too. Any signed-in
/// person may: the names of the types are not hidden from anybody who works here.
///
/// # Errors
/// `request.invalid` for a cursor that is not one.
pub async fn issue_types<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: IssueTypesQuery,
) -> Result<Authorized<IssueTypePage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let types = ctx
        .store()
        .issue_types()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let page = window(types.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(held.authorize(IssueTypePage {
        items: types[page.start..page.end]
            .iter()
            .map(IssueTypeView::from)
            .collect(),
        next_cursor: page.next,
    }))
}

/// Makes an issue type, at the end of the order.
///
/// # Errors
/// `access.forbidden` without *Manage issue types, kinds of relation and layouts* for the whole
/// organization, `request.invalid` for a name that cannot be one, `name.taken` for a name another
/// type has.
pub async fn make_issue_type<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MakeIssueType,
) -> Result<Authorized<IssueTypeView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(&actor, Permission::ManageIssueTypes, Place::Organization)
        .await?;
    let name = clean_name(&input.name)?;
    let types = ctx
        .store()
        .issue_types()
        .await
        .map_err(|error| internal::<S>(&error))?;
    if name_taken(&types, &name, None) {
        return Err(ErrorCode::NameTaken.into());
    }
    let issue_type = IssueType {
        id: IssueTypeId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        name,
        icon: input.icon,
        position: types
            .iter()
            .map(|found| found.position)
            .max()
            .map_or(0, |last| last + 1),
        retired_at: None,
        version: 1,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::IssueType, issue_type.id),
        vec![
            set_to("name", json!(issue_type.name)),
            set_to("icon", json!(issue_type.icon.code())),
        ],
    )?;
    let why = Why::default();
    let (stopped, stored) = (why.clone(), issue_type.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                let types = S::issue_types_in(transaction).await?;
                if name_taken(&types, &stored.name, None) {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                S::insert_issue_type(transaction, &stored).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NameTaken))?;
    Ok(held.authorize(IssueTypeView::from(&issue_type)))
}

/// Changes the name, the icon or the retired state of an issue type: what is `Some` is changed.
///
/// # Errors
/// `access.forbidden` without the permission, `resource.not_found` for a type that is not there,
/// `request.invalid` for a name that cannot be one or when nothing is asked, `name.taken` for a
/// name another type has, `edit.version_changed` when somebody changed the type since it was
/// looked at.
pub async fn change_issue_type<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeIssueType,
) -> Result<Authorized<IssueTypeView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(&actor, Permission::ManageIssueTypes, Place::Organization)
        .await?;
    if input.name.is_none() && input.icon.is_none() && input.retired.is_none() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let name = input.name.as_deref().map(clean_name).transpose()?;
    let types = ctx
        .store()
        .issue_types()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let current = types
        .iter()
        .find(|found| found.id == input.issue_type)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?
        .clone();
    if current.version != input.version {
        return Err(ErrorCode::VersionChanged.into());
    }
    let name = name.unwrap_or_else(|| current.name.clone());
    if name_taken(&types, &name, Some(current.id)) {
        return Err(ErrorCode::NameTaken.into());
    }
    let icon = input.icon.unwrap_or(current.icon);
    let now = ctx.now();
    let retired_at = match input.retired {
        None => current.retired_at,
        Some(true) => current.retired_at.or(Some(now)),
        Some(false) => None,
    };
    let mut differences = Vec::new();
    if name != current.name {
        differences.push(FieldChange {
            field: "name".to_owned(),
            before: Some(json!(current.name)),
            after: Some(json!(name)),
        });
    }
    if icon != current.icon {
        differences.push(FieldChange {
            field: "icon".to_owned(),
            before: Some(json!(current.icon.code())),
            after: Some(json!(icon.code())),
        });
    }
    if retired_at.is_some() != current.retired_at.is_some() {
        differences.push(FieldChange {
            field: "retired".to_owned(),
            before: Some(json!(current.retired_at.is_some())),
            after: Some(json!(retired_at.is_some())),
        });
    }
    let after = IssueType {
        name,
        icon,
        retired_at,
        version: current.version + 1,
        ..current.clone()
    };
    if differences.is_empty() {
        return Ok(held.authorize(IssueTypeView::from(&current)));
    }
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::IssueType, current.id),
        differences,
    )?;
    let why = Why::default();
    let (stopped, written, version) = (why.clone(), after.clone(), current.version);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                let types = S::issue_types_in(transaction).await?;
                if name_taken(&types, &written.name, Some(written.id)) {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                let updated = S::update_issue_type(
                    transaction,
                    written.id,
                    &written.name,
                    written.icon,
                    written.retired_at,
                    version,
                )
                .await?;
                if updated {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::VersionChanged))?;
    Ok(held.authorize(IssueTypeView::from(&after)))
}

/// Makes the issue types of the first start if there are none yet: *Bug, Task, Feature, Goal*,
/// from the one definition in `bb-core`, with an event. It runs at every start and does only what
/// is still to do, so it can be stopped half way and run again.
pub struct MakeStartingIssueTypes;

/// Makes the starting issue types once.
///
/// # Errors
/// Fails when the store fails.
pub async fn make_starting_issue_types<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: MakeStartingIssueTypes,
) -> Result<(), AppError> {
    let existing = ctx
        .store()
        .issue_types()
        .await
        .map_err(|error| internal::<S>(&error))?;
    if !existing.is_empty() {
        return Ok(());
    }
    let mut types = Vec::with_capacity(STARTING_ISSUE_TYPES.len());
    for (position, starting) in (0_i64..).zip(STARTING_ISSUE_TYPES) {
        types.push(IssueType {
            id: IssueTypeId::generate(&mut ctx.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
            name: Message::from(starting.name).render(),
            icon: starting.icon,
            position,
            retired_at: None,
            version: 1,
        });
    }
    let names: Vec<serde_json::Value> = types.iter().map(|found| json!(found.name)).collect();
    let event = ctx.event(
        Actor::System,
        fixed_entity(RecordKind::IssueType, "starting")?,
        vec![set_to("issue_types", serde_json::Value::Array(names))],
    )?;
    let result = ctx
        .store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // Two servers that start together both saw "none" above; the one that gets the
                // lock second finds them made and stops, which is not a failure.
                if !S::issue_types_in(transaction).await?.is_empty() {
                    return Err(S::aborted());
                }
                for issue_type in &types {
                    S::insert_issue_type(transaction, issue_type).await?;
                }
                Ok(())
            })
        })
        .await;
    match result {
        Err(error) if S::is_aborted(&error) => Ok(()),
        Err(error) => Err(change_failed::<S>(&error, Stops::NONE)),
        Ok(()) => Ok(()),
    }
}
