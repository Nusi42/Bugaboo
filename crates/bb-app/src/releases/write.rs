//! The write of a release inside its change (ISS-08, ADR-0049): judged again after the lock of the
//! releases, on what is stored now, so two people at the same moment cannot both take a name, and a
//! release that an issue names is never deleted.

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{ReleaseId, UserId};
use bb_core::names::same_name;
use bb_core::project_release::Release;
use bb_core::version::Version;
use bb_policy::Place;

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity};
use crate::projects::Why;

/// Whether another release of the same project than `except`, not in the trash, has the name.
pub(super) fn name_taken(releases: &[Release], name: &str, except: Option<ReleaseId>) -> bool {
    releases
        .iter()
        .any(|found| Some(found.id) != except && same_name(&found.name, name))
}

/// A release as a change writes it, judged again inside the change after the lock of the
/// releases: the name, and for a delete whether an issue names it.
#[derive(Clone)]
pub(super) struct ReleaseWrite {
    /// The release as it will be.
    written: Release,
    /// The version the change is made on.
    version: i64,
    /// Whether the change is refused while an issue names the release (a delete).
    unused_only: bool,
}

impl ReleaseWrite {
    /// The write of `written` on `version`.
    pub(super) fn of(written: &Release, version: i64, unused_only: bool) -> Self {
        Self {
            written: written.clone(),
            version,
            unused_only,
        }
    }

    async fn write_in<S: AuthStore>(
        &self,
        transaction: &mut S::Transaction,
        stopped: &Why,
    ) -> Result<(), S::Error> {
        let written = &self.written;
        S::lock_releases(transaction).await?;
        if written.deleted_at.is_none() {
            let others = S::releases_of_projects_in(transaction, vec![written.project]).await?;
            if name_taken(&others, &written.name, Some(written.id)) {
                return Err(stopped.stop::<S>(ErrorCode::ReleaseNameTaken));
            }
        }
        if self.unused_only && S::release_named_in(transaction, written.id).await? {
            return Err(stopped.stop::<S>(ErrorCode::ReleaseInUse));
        }
        if S::update_release(transaction, written, Version::new(self.version)).await? {
            Ok(())
        } else {
            Err(stopped.stop::<S>(ErrorCode::VersionChanged))
        }
    }

    /// The change that writes it, for `record_change`.
    pub(super) fn change<S: AuthStore>(
        &self,
        stopped: &Why,
    ) -> impl for<'t> FnOnce(
        &'t mut S::Transaction,
    ) -> crate::ports::BoxFuture<'t, Result<(), S::Error>>
    + Send
    + 'static {
        let (writing, stopped) = (self.clone(), stopped.clone());
        move |transaction| {
            Box::pin(async move { writing.write_in::<S>(transaction, &stopped).await })
        }
    }

    /// What the person is told when the write was refused or failed.
    pub(super) fn refused<S: AuthStore>(
        &self,
        why: &Why,
        error: &S::Error,
        editor: UserId,
    ) -> AppError {
        why.answer::<S>(error, ErrorCode::VersionChanged).about(
            &entity(RecordKind::Release, self.written.id),
            Place::Project(self.written.project),
            editor,
        )
    }

    /// The event of the change, by `actor`.
    pub(super) fn event<S: AuthStore>(
        &self,
        ctx: &Ctx<'_, S>,
        actor: &Actor,
        changes: Vec<FieldChange>,
    ) -> Result<bb_core::event::DomainEvent, AppError> {
        ctx.event(
            by(actor.person.id),
            entity(RecordKind::Release, self.written.id),
            changes,
        )
    }
}
