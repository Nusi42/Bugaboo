//! What the sample leaves behind for the demo: a lock held and a draft waiting.

use bb_app::context::SystemIds;
use bb_app::editing::{DraftText, SaveDraftQuery, TakeLockQuery, save_draft, take_lock};
use bb_core::edit::{DraftTarget, EditWhat};
use bb_core::event::{EntityRef, RecordKind};
use bb_core::fields::Field;
use bb_core::id::EditorId;
use bb_core::technical::TechnicalName;

use crate::error::{SampleError, Step};
use crate::generate::LeftBehind;
use crate::issues::Maker;

impl Maker<'_, '_> {
    /// The description of an issue, as the thing that is edited under a lock.
    fn description_of(&self, at: usize) -> Result<EditWhat, SampleError> {
        Ok(EditWhat {
            record: EntityRef {
                kind: RecordKind::Issue.into(),
                id: self.issue(at)?.id.into(),
            },
            part: TechnicalName::new(Field::Description.code())
                .map_err(|error| SampleError::step(Step::Editing, &error))?,
        })
    }

    /// The states the sample leaves behind for the demo: one person is editing the description of
    /// an issue (the lock holds for its lease, a minute, and then runs out like any lock), and
    /// another has a draft of a description waiting.
    pub(crate) async fn leave_states_behind(&mut self) -> Result<LeftBehind, SampleError> {
        let end = self.world.start.as_second() - 120;
        self.world
            .clock
            .set(jiff::Timestamp::from_second(end).map_err(|_| SampleError::Start(end))?);
        let count = self.plan.issues.len();
        let (locked, drafted) = (count / 2, count / 3);
        let (holder, writer) = (self.hands.workers[0], self.hands.workers[1]);
        let (lock_on, draft_on) = (self.description_of(locked)?, self.description_of(drafted)?);
        let left = LeftBehind {
            issues: u32::try_from(count).unwrap_or(0),
            locks: vec![lock_on.clone()],
            drafts: self
                .world
                .people
                .get(writer)
                .map(|made| (made.id, DraftTarget::Editing(draft_on.clone())))
                .into_iter()
                .collect(),
            ..LeftBehind::default()
        };
        let ctx = self.world.services.ctx(self.world.request());
        let token = self.hands.token(holder).await?;
        take_lock(
            &ctx,
            TakeLockQuery {
                session_token: token,
                what: lock_on,
                editor: EditorId::generate(&mut SystemIds),
            },
        )
        .await
        .map_err(|error| SampleError::step(Step::Editing, &error))?;
        let token = self.hands.token(writer).await?;
        let base = self.issue(drafted)?.version;
        let text = format!("{}\n\n", self.plan.issues[drafted].description);
        save_draft(
            &ctx,
            SaveDraftQuery {
                session_token: token,
                target: DraftTarget::Editing(draft_on),
                draft: DraftText {
                    text,
                    base_version: Some(base),
                },
            },
        )
        .await
        .map_err(|error| SampleError::step(Step::Editing, &error))?;
        Ok(left)
    }
}
