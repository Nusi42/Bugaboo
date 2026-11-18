//! The job `privacy.export` that writes the file of *Download my data* (ADR-0044, decisions 2 and
//! 3), and the daily round that deletes the files and records past their keep period.
//!
//! The job reads the person's rows through the walk ([`PersonData`]) and nothing else. Other people
//! are shown as the person sees them now, at the strictest level of *Who sees people* the person
//! has for any kind of people information, so the file never shows more of somebody than any
//! screen would. It logs IDs only (DATA-LOGS).

use std::collections::BTreeMap;
use std::io::Write as _;

use bb_core::data_export::{DataExport, EXPORT_KEPT, ExportState};
use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::files::LinkTarget;
use bb_core::id::{DataExportId, UserId};
use bb_core::message::Message;
use bb_core::people_kinds::PeopleKind;
use bb_core::personal::Link;
use bb_core::security_log::Digest;
use bb_core::settings::MAIL_ENABLED;
use bb_core::wording::Region;
use bb_policy::{Evaluation, PeopleContext, PersonRef, Place};
use serde_json::{Value, json};

use super::ask::EXPORT_KEY;
use super::issue_fields::SeenInProjects;
use super::shape::{ShapedKind, people_in, shape_row};
use super::writer::ExportWriter;
use super::{ExportSink, recipients};
use crate::auth::{AppError, AuthStore, Ctx, entity, internal, set_to};
use crate::member::data_export as member;
use crate::privacy::walk::FoundRow;

/// How many old exports a round of the daily job deletes at most; the next round deletes the rest.
const OLD_EXPORTS_AT_A_TIME: u64 = 100;

/// The field of an attachment that holds the hash of its file, and the one that holds its ID.
const ATTACHMENT_FILE: &str = "file";
const ATTACHMENT_ID: &str = "id";

/// A file the person uploaded: the ID of its attachment, which names it in the file, and the hash
/// of its bytes.
struct Upload {
    id: String,
    hash: Digest,
}

/// The file the person uploaded that `row` of an attachment names, when the person added it.
fn upload_of(row: &FoundRow) -> Option<Upload> {
    if !row.links.contains(&Link::Author) {
        return None;
    }
    let text = |field: &str| {
        row.cells.iter().find_map(|cell| match &cell.value {
            crate::privacy::walk::Value::Id(text) | crate::privacy::walk::Value::Bytes(text)
                if cell.field == field =>
            {
                Some(text.clone())
            }
            _ => None,
        })
    };
    Some(Upload {
        id: text(ATTACHMENT_ID)?,
        hash: Digest::new(bb_core::hex::decode::<32>(&text(ATTACHMENT_FILE)?)?),
    })
}

/// The kind of people information the person sees others at most openly by: the strictest of
/// their levels. Pictures are left out, as they never stand for a person on their own.
fn strictest_kind(viewer: &Evaluation) -> PeopleKind {
    PeopleKind::ALL
        .iter()
        .copied()
        .filter(|kind| *kind != PeopleKind::Pictures)
        .min_by_key(|kind| viewer.people_level(*kind, PeopleContext::Ordinary))
        .unwrap_or(PeopleKind::ProjectMembers)
}

fn failed(error: &std::io::Error) -> AppError {
    AppError::Internal(error.to_string())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The export a job names.
    async fn export_of_job(&self, job: &Value) -> Result<Option<DataExport>, AppError> {
        let id = job
            .get(EXPORT_KEY)
            .and_then(Value::as_str)
            .and_then(|text| text.parse::<DataExportId>().ok())
            .ok_or(AppError::Refused(bb_core::ErrorCode::InvalidRequest))?;
        self.store()
            .data_export(id)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// How the person sees each person `rows` name, looked up once each.
    async fn seen_by(
        &self,
        viewer: &Evaluation,
        kind: PeopleKind,
        rows: &[FoundRow],
        seen: &mut BTreeMap<UserId, PersonRef>,
    ) -> Result<(), AppError> {
        for person in rows.iter().flat_map(people_in) {
            if let std::collections::btree_map::Entry::Vacant(entry) = seen.entry(person) {
                entry.insert(
                    self.person_ref(viewer, kind, PeopleContext::Ordinary, person)
                        .await?,
                );
            }
        }
        Ok(())
    }

    /// Who receives the data, in words: the people of the organization, the mail server when
    /// e-mail is on, and the AI provider with its country when AI counts.
    async fn recipients_now(&self) -> Result<Vec<String>, AppError> {
        let layers = self.setting_layers_of(None).await?;
        let ai = self.ai_state_at(None).await?;
        let provider = if ai.counts() {
            ai.destination().ok().map(|destination| {
                let row = destination.row();
                (
                    Message::from(row.name).render(),
                    Message::from(row.country).render(),
                )
            })
        } else {
            None
        };
        Ok(recipients(
            layers.yes(&MAIL_ENABLED),
            provider
                .as_ref()
                .map(|(name, country)| (name.as_str(), country.as_str())),
        ))
    }

    /// Writes the bytes of the files the person uploaded into `files/`.
    async fn write_uploads(
        &self,
        writer: &mut ExportWriter<Box<dyn ExportSink>>,
        uploads: Vec<Upload>,
    ) -> Result<(), AppError> {
        for upload in uploads {
            let Some(mut body) = self
                .files()
                .open(upload.hash, LinkTarget::File)
                .await
                .map_err(|error| failed(&error))?
            else {
                tracing::warn!(
                    attachment = upload.id.as_str(),
                    "a file of an export is missing"
                );
                continue;
            };
            writer
                .start_upload(&upload.id)
                .map_err(|error| failed(&error))?;
            while let Some(chunk) = body
                .reader
                .next_chunk()
                .await
                .map_err(|error| failed(&error))?
            {
                writer.write_bytes(&chunk).map_err(|error| failed(&error))?;
            }
        }
        Ok(())
    }

    /// Writes the file of `export`: every kind of record the walk finds, the files the person
    /// uploaded and the README. Answers the size of the file.
    async fn write_export(&self, export: &DataExport) -> Result<u64, AppError> {
        let person = export.person;
        let viewer = self.evaluate(person, Place::Organization).await?;
        let kind = strictest_kind(&viewer);
        let region = Region::of(&self.setting_layers_of(Some(person)).await?);
        let sink = self
            .exports()
            .create(export.id)
            .map_err(|error| failed(&error))?;
        let mut writer = ExportWriter::new(sink, region);
        let mut seen = BTreeMap::new();
        let mut uploads = Vec::new();
        let mut in_projects = SeenInProjects::default();
        let kinds = self
            .store()
            .kinds_of(person)
            .await
            .map_err(|error| internal::<S>(&error))?;
        for found in kinds {
            let mut rows = Vec::new();
            let mut from = Some(0);
            while let Some(start) = from {
                let page = self
                    .store()
                    .person_rows(person, &found.kind, start)
                    .await
                    .map_err(|error| internal::<S>(&error))?;
                self.seen_by(&viewer, kind, &page.rows, &mut seen).await?;
                if found.kind == RecordKind::Attachment.code() {
                    uploads.extend(page.rows.iter().filter_map(upload_of));
                }
                for row in page.rows {
                    let row = if found.kind == RecordKind::Issue.code() {
                        self.issue_as_seen(person, row, &mut in_projects).await?
                    } else {
                        row
                    };
                    rows.push(shape_row(row, &found.marks, person, &seen));
                }
                from = page.next;
            }
            writer
                .kind(&ShapedKind {
                    kind: found.kind,
                    marks: found.marks,
                    rows,
                })
                .map_err(|error| failed(&error))?;
        }
        self.write_uploads(&mut writer, uploads).await?;
        let mut sink = writer
            .finish(&self.recipients_now().await?)
            .map_err(|error| failed(&error))?;
        sink.flush().map_err(|error| failed(&error))?;
        sink.keep().map_err(|error| failed(&error))
    }

    /// Ends a waiting export in `state`, with its event, from which the person is told.
    async fn finish_export(
        &self,
        export: &DataExport,
        state: ExportState,
        bytes: Option<u64>,
    ) -> Result<(), AppError> {
        let now = self.now();
        let ready_at = (state == ExportState::Ready).then_some(now);
        let bytes = bytes.map(|bytes| i64::try_from(bytes).unwrap_or(i64::MAX));
        let event = self.event(
            EventActor::System,
            entity(RecordKind::DataExport, export.id),
            vec![set_to(member::STATE, json!(state.code()))],
        )?;
        let id = export.id;
        self.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    S::finish_data_export(transaction, id, state, ready_at, bytes)
                        .await
                        .map(|_| ())
                })
            })
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// The work of a job of `privacy.export`: writes the file of the export the payload names, if
    /// it still waits, and marks it ready. A try that fails leaves the export waiting for the
    /// next; the half-written file is replaced then.
    ///
    /// # Errors
    /// `request.invalid` for a payload that names no export; the error of the store or of the
    /// file system.
    pub(crate) async fn run_export(&self, job: &Value) -> Result<(), AppError> {
        let Some(export) = self.export_of_job(job).await? else {
            return Ok(());
        };
        if export.state != ExportState::Waiting {
            return Ok(());
        }
        let bytes = self.write_export(&export).await?;
        self.finish_export(&export, ExportState::Ready, Some(bytes))
            .await?;
        tracing::info!(export = %export.id, bytes, "a file of a person's data was written");
        Ok(())
    }

    /// The job of an export gave up: the export fails, so the person may ask again, and what was
    /// written of its file is deleted.
    ///
    /// # Errors
    /// The error of the store or of the file system.
    pub(crate) async fn export_gave_up(&self, job: &Value) -> Result<(), AppError> {
        let Some(export) = self.export_of_job(job).await? else {
            return Ok(());
        };
        if export.state != ExportState::Waiting {
            return Ok(());
        }
        self.exports()
            .delete(export.id)
            .await
            .map_err(|error| failed(&error))?;
        self.finish_export(&export, ExportState::Failed, None).await
    }

    /// Deletes the exports asked for more than [`EXPORT_KEPT`] ago, the file first, then the record
    /// with its event. Safe to run again: a file that is gone is no error. Answers how many went.
    ///
    /// # Errors
    /// The error of the store or of the file system.
    pub(crate) async fn delete_old_exports(&self) -> Result<u64, AppError> {
        let before = self
            .now()
            .checked_sub(EXPORT_KEPT)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let old = self
            .store()
            .exports_asked_before(before, OLD_EXPORTS_AT_A_TIME)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut deleted = 0;
        for export in old {
            self.exports()
                .delete(export.id)
                .await
                .map_err(|error| failed(&error))?;
            let event = self.event(
                EventActor::System,
                entity(RecordKind::DataExport, export.id),
                vec![set_to(member::STATE, json!(null))],
            )?;
            let id = export.id;
            let gone = self
                .store()
                .record_change(&event, move |transaction| {
                    Box::pin(async move { S::delete_data_export(transaction, id).await })
                })
                .await
                .map_err(|error| internal::<S>(&error))?;
            deleted += u64::from(gone);
        }
        Ok(deleted)
    }
}
