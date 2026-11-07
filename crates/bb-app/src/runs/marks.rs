//! The mark of a text written with AI as the views carry it (COMP-08, COMP-09): read once for the
//! texts of a page and given as a member of each text. The mark is stored where events are written
//! (`bb-store`, from the actor and the channel) and is never taken from the client.

use std::collections::{HashMap, HashSet};

use bb_core::event::{EntityKind, RecordKind};
use bb_core::id::EntityId;
use bb_core::mark::{TextMark, WrittenBy};
use serde::Serialize;

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// How a text was written, when AI wrote it: shown beside the text and given in the API.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WrittenByView {
    /// *Written with the assistant*, *Written by an AI app* or *Written by an agent*.
    pub kind: WrittenBy,
    /// The name of the AI app, for an app.
    pub app: Option<String>,
    /// The name of the agent, for an agent.
    pub agent: Option<String>,
}

/// The marks a request has read: which records it asked about, and what it found.
#[derive(Default)]
pub(crate) struct Marks {
    asked: HashSet<(String, EntityId)>,
    found: HashMap<(String, EntityId, String), TextMark>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Reads the marks of the texts of these records at once, the ones not read yet, so a page of
    /// texts costs one read (PERF-N1).
    pub(crate) async fn load_marks(
        &self,
        kind: RecordKind,
        ids: &[EntityId],
    ) -> Result<(), AppError> {
        let kind_name = kind.code().to_owned();
        let wanted: Vec<EntityId> = {
            let marks = self
                .marks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            ids.iter()
                .copied()
                .filter(|id| !marks.asked.contains(&(kind_name.clone(), *id)))
                .collect()
        };
        if wanted.is_empty() {
            return Ok(());
        }
        let found = self
            .store()
            .text_marks(&EntityKind::from(kind), &wanted)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut marks = self
            .marks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for id in wanted {
            marks.asked.insert((kind_name.clone(), id));
        }
        for mark in found {
            marks.found.insert(
                (
                    kind_name.clone(),
                    mark.entity_id,
                    mark.part.as_str().to_owned(),
                ),
                mark,
            );
        }
        Ok(())
    }

    /// Reads the marks of the texts of a page of comments at once (PERF-N1).
    pub(crate) async fn load_comment_marks(
        &self,
        comments: &[bb_core::id::CommentId],
    ) -> Result<(), AppError> {
        let ids: Vec<EntityId> = comments.iter().map(|id| (*id).into()).collect();
        self.load_marks(RecordKind::Comment, &ids).await
    }

    /// How the text `part` of a record was written, when AI wrote it.
    pub(crate) async fn written_by(
        &self,
        kind: RecordKind,
        id: impl Into<EntityId>,
        part: &str,
    ) -> Result<Option<WrittenByView>, AppError> {
        let id = id.into();
        self.load_marks(kind, &[id]).await?;
        let mark = {
            let marks = self
                .marks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            marks
                .found
                .get(&(kind.code().to_owned(), id, part.to_owned()))
                .cloned()
        };
        let Some(mark) = mark else {
            return Ok(None);
        };
        let agent = match mark.agent {
            Some(agent) => self
                .store()
                .agent(agent)
                .await
                .map_err(|error| internal::<S>(&error))?
                .map(|found| found.name),
            None => None,
        };
        Ok(Some(WrittenByView {
            kind: mark.written_by,
            app: mark.app,
            agent,
        }))
    }
}
