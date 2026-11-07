//! Whose runs an actor reads and undoes, and the two words every part of runs shares.

use bb_core::ErrorCode;
use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::{IssueId, RunId};
use bb_core::many_changes::ManyChangeRun;

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// The run, when it is the actor's own: a person's own runs (the attempts of their agents too),
    /// and for an agent only its own attempts, never the runs of the person behind it (AI-30).
    /// For anybody else it is not there.
    ///
    /// # Errors
    /// `resource.not-found` for a run that is not there or is not the actor's.
    pub(crate) async fn own_run(
        &self,
        actor: &Actor,
        run: RunId,
    ) -> Result<ManyChangeRun, AppError> {
        self.store()
            .many_changes_run(run)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|found| match actor.agent() {
                Some(agent) => found.agent == Some(agent),
                None => found.person == actor.person.id,
            })
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }
}

/// The issue a record is, when it is one.
pub(crate) fn issue_of(entity: &EntityRef) -> Option<IssueId> {
    (entity.kind.as_str() == RecordKind::Issue.code())
        .then(|| IssueId::from_bytes(entity.id.to_bytes()).ok())
        .flatten()
}
