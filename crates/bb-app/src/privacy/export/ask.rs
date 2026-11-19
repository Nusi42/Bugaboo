//! Asking for the file of *Download my data*, seeing where it stands, and downloading it through a
//! signed link (PRIV-03, ADR-0044, decisions 2 and 4). A person asks for their own; an administrator
//! with *Download someone's data* asks for somebody else's, always with a reason, and the security
//! log records it. One export waits or runs per person at a time.

use bb_core::ErrorCode;
use bb_core::data_export::{
    DataExport, EXPORT_MEDIA_TYPE, EXPORT_REASON_MOST, ExportLink, ExportState,
};
use bb_core::event::RecordKind;
use bb_core::id::{DataExportId, JobId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Evaluation, PeopleContext, Permission, Place};
use serde_json::json;

use super::types::{
    AskMyData, AskSomeonesData, DataExportsQuery, DownloadExportQuery, ExportLinkQuery,
    ExportLinkView, ExportPage, ExportView, MyDataExportQuery,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::files::FileDownload;
use crate::jobs::JobKind;
use crate::member::data_export as member;
use crate::projects::Why;

/// The key of the export in the payload of its job.
pub(super) const EXPORT_KEY: &str = "export";

/// The most exports a page of the list holds.
pub const EXPORTS_PAGE_MOST: u64 = 100;

/// A waiting export to add: whose data, who asks, and why.
struct NewExport {
    person: UserId,
    asked_by: UserId,
    reason: Option<String>,
}

impl NewExport {
    /// Adds the export with its event and its job, in one change that holds the person's exports
    /// and refuses a second one while one waits. The one write of a new export, for the person's
    /// own and an administrator's (ARCH-EVENTS reads the call by its name).
    async fn record_change<S: AuthStore>(self, ctx: &Ctx<'_, S>) -> Result<DataExport, AppError> {
        let now = ctx.now();
        let export = DataExport {
            id: DataExportId::generate(&mut ctx.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
            person: self.person,
            asked_by: self.asked_by,
            reason: self.reason,
            state: ExportState::Waiting,
            asked_at: now,
            ready_at: None,
            bytes: None,
        };
        let mut changes = vec![set_to(member::STATE, json!(export.state.code()))];
        if let Some(reason) = &export.reason {
            // Whose data and why stay in the history after the export and its record are deleted,
            // so the entry of the security log that points to it can still be answered.
            changes.push(set_to(member::PERSON, json!(export.person)));
            changes.push(set_to(member::REASON, json!(reason)));
        }
        let event = ctx.event(
            by(export.asked_by),
            entity(RecordKind::DataExport, export.id),
            changes,
        )?;
        let job = JobId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let kind = JobKind::DataExport.name()?;
        let payload = json!({ EXPORT_KEY: export.id.to_string() });
        let why = Why::default();
        let (stopped, adding) = (why.clone(), export.clone());
        ctx.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    S::lock_exports_of(transaction, adding.person).await?;
                    if S::waiting_export_of_in(transaction, adding.person)
                        .await?
                        .is_some()
                    {
                        return Err(stopped.stop::<S>(ErrorCode::ExportWaiting));
                    }
                    S::insert_data_export(transaction, &adding).await?;
                    S::enqueue_job(transaction, job, &kind, &payload, now, now).await
                })
            })
            .await
            .map_err(|error| why.said_or_failed::<S>(&error))?;
        Ok(export)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The export as `viewer` sees it: its people resolved for them.
    async fn export_view(
        &self,
        viewer: &Evaluation,
        context: PeopleContext,
        export: DataExport,
    ) -> Result<ExportView, AppError> {
        let person = self
            .person_ref(viewer, PeopleKind::Authors, context, export.person)
            .await?;
        let asked_by = self
            .person_ref(viewer, PeopleKind::Authors, context, export.asked_by)
            .await?;
        Ok(ExportView {
            export,
            person,
            asked_by,
        })
    }

    /// The export that `actor` asked for, when it is ready: the only one they may download. One
    /// they asked for somebody else is theirs only while they still hold *Download someone's data*.
    async fn ready_export_of(
        &self,
        actor: &Actor,
        export: DataExportId,
    ) -> Result<DataExport, AppError> {
        let found = self
            .store()
            .data_export(export)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|export| {
                export.state == ExportState::Ready && export.asked_by == actor.person.id
            })
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        if found.by_somebody_else()
            && !self
                .evaluation_of(actor, Place::Organization)
                .await?
                .allows(Permission::DownloadSomeonesData)
        {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        Ok(found)
    }
}

/// Asks for the file of the person's own data. Only a session asks: an access key cannot.
///
/// # Errors
/// `data-export.waiting` while a file of the person's data is being made.
pub async fn ask_for_my_data<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AskMyData,
) -> Result<Authorized<ExportView>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let export = NewExport {
        person: actor.person.id,
        asked_by: actor.person.id,
        reason: None,
    }
    .record_change(ctx)
    .await?;
    let view = ctx
        .export_view(&held, PeopleContext::Ordinary, export)
        .await?;
    Ok(held.authorize(view))
}

/// Asks, with *Download someone's data*, for the file of somebody else's data, with a reason. The
/// file holds what the person's own would hold; the person is told, and the security log records
/// who asked for whom (the export it names holds why).
///
/// # Errors
/// `access.forbidden` without the permission, `resource.not-found` for a person who is not there,
/// `data-export.reason-needed` for an empty reason, `request.invalid` for one that is too long,
/// `data-export.waiting` while a file of the person's data is being made.
pub async fn ask_for_someones_data<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AskSomeonesData,
) -> Result<Authorized<ExportView>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let held = ctx
        .require(
            &actor,
            Permission::DownloadSomeonesData,
            Place::Organization,
        )
        .await?;
    let reason = input
        .reason
        .as_deref()
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?
        .trim();
    if reason.is_empty() {
        return Err(ErrorCode::ExportReasonNeeded.into());
    }
    if reason.chars().count() > EXPORT_REASON_MOST {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let person = input.person.ok_or(AppError::Refused(ErrorCode::NotFound))?;
    if ctx.person_known(person).await?.is_none() {
        return Err(ErrorCode::NotFound.into());
    }
    let export = NewExport {
        person,
        asked_by: actor.person.id,
        reason: Some(reason.to_owned()),
    }
    .record_change(ctx)
    .await?;
    ctx.note(
        SecurityAction::DataExportForSomebodyElse,
        Some(actor.person.id),
        Some(entity(RecordKind::DataExport, export.id)),
    )
    .await;
    let view = ctx
        .export_view(&held, PeopleContext::Management, export)
        .await?;
    Ok(held.authorize(view))
}

/// Where the newest file of the person's own data stands; none before the first, and none once it
/// is deleted.
///
/// # Errors
/// `auth.not-signed-in` without a session.
pub async fn my_data_export<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MyDataExportQuery,
) -> Result<Authorized<Option<ExportView>>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let found = ctx
        .store()
        .latest_export_of(actor.person.id, actor.person.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let view = match found {
        Some(export) => Some(
            ctx.export_view(&held, PeopleContext::Ordinary, export)
                .await?,
        ),
        None => None,
    };
    Ok(held.authorize(view))
}

/// A page of every export, newest first, for a holder of *Download someone's data*.
///
/// # Errors
/// `access.forbidden` without the permission.
pub async fn data_exports<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DataExportsQuery,
) -> Result<Authorized<ExportPage>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let held = ctx
        .require(
            &actor,
            Permission::DownloadSomeonesData,
            Place::Organization,
        )
        .await?;
    let limit = input.limit.clamp(1, EXPORTS_PAGE_MOST);
    let mut found = ctx
        .store()
        .data_exports_page(input.skip, limit + 1)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let more = u64::try_from(found.len()).unwrap_or(u64::MAX) > limit;
    found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    let mut exports = Vec::with_capacity(found.len());
    for export in found {
        exports.push(
            ctx.export_view(&held, PeopleContext::Management, export)
                .await?,
        );
    }
    Ok(held.authorize(ExportPage { exports, more }))
}

/// The signed link of the download of a ready export, for the person who asked for it, good for
/// five minutes.
///
/// # Errors
/// `resource.not-found` for an export that is not there, not ready, or that somebody else asked for.
pub async fn export_link<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ExportLinkQuery,
) -> Result<Authorized<ExportLinkView>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let export = input.export.ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let export = ctx.ready_export_of(&actor, export).await?;
    let token = ExportLink::new(export.id, actor.person.id, ctx.now())
        .and_then(|link| ctx.services.links.sign(&link.to_bytes()))
        .ok_or_else(|| AppError::Internal("a link could not be signed".to_owned()))?;
    Ok(held.authorize(ExportLinkView { token }))
}

/// Opens the file of an export for the person who holds its link, and writes the download into the
/// security log.
///
/// # Errors
/// `resource.not-found` for a link that is not ours, ran out or is for another person, and for an
/// export that is gone.
pub async fn download_export<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DownloadExportQuery,
) -> Result<FileDownload, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let link = ctx
        .services
        .links
        .open(&input.link)
        .and_then(|claim| ExportLink::from_bytes(&claim))
        .filter(|link| link.works_at(ctx.now()) && link.person == actor.person.id)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let export = ctx.ready_export_of(&actor, link.export).await?;
    let body = ctx
        .exports()
        .open(export.id)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    ctx.note(
        SecurityAction::DataExportDownloaded,
        Some(actor.person.id),
        Some(entity(RecordKind::DataExport, export.id)),
    )
    .await;
    Ok(FileDownload {
        name: export.file_name(),
        media_type: EXPORT_MEDIA_TYPE.to_owned(),
        body,
    })
}
