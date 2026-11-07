//! The marks of texts written with AI (COMP-08, COMP-09): written where the events are written, the
//! one place every change to a record passes, from the actor and the channel of the event and never
//! from what a client sent. A person who edits a marked text keeps the mark, because an event
//! without a mark leaves the row as it is.

use std::collections::BTreeMap;

use bb_core::event::DomainEvent;
use bb_core::id::{AgentId, RunId};
use bb_core::mark::{TextMark, TextPart, WrittenBy, text_part, written_by};
use sea_query::{Expr, ExprTrait as _, OnConflict, Query};

use crate::db::{StoredId, Transaction};
use crate::error::StoreError;
use crate::schema::{ManyChangeRunRow, TextMarkRow};

/// Puts the mark of each text the events write, when the actor and the channel say that AI wrote
/// it.
pub(crate) async fn mark_texts(
    transaction: &mut Transaction,
    events: &[DomainEvent],
) -> Result<(), StoreError> {
    let marked: Vec<(&DomainEvent, WrittenBy)> = events
        .iter()
        .filter_map(|event| Some((event, written_by(event.actor, event.channel)?)))
        .collect();
    if marked.is_empty() {
        return Ok(());
    }
    let apps = apps_of(transaction, &marked).await?;
    for (event, how) in marked {
        let app = event.run.and_then(|run| apps.get(&run).cloned());
        let agent: Option<AgentId> = match event.actor {
            bb_core::event::Actor::Agent { agent, .. } => Some(agent),
            _ => None,
        };
        for change in &event.changes {
            let Some(part) = text_part(&event.entity.kind, change) else {
                continue;
            };
            let mut insert = Query::insert();
            insert
                .into_table(TextMarkRow::Table)
                .columns([
                    TextMarkRow::EntityKind,
                    TextMarkRow::EntityId,
                    TextMarkRow::Part,
                    TextMarkRow::WrittenBy,
                    TextMarkRow::App,
                    TextMarkRow::Agent,
                ])
                .values([
                    Expr::val(event.entity.kind.as_str()),
                    Expr::val(event.entity.id.stored()),
                    Expr::val(part.as_str().to_owned()),
                    Expr::val(how.code()),
                    Expr::val(app.clone()),
                    Expr::val(agent.map(|agent| agent.stored())),
                ])
                .map_err(|_| StoreError::Corrupt("a text mark"))?
                .on_conflict(
                    OnConflict::columns([
                        TextMarkRow::EntityKind,
                        TextMarkRow::EntityId,
                        TextMarkRow::Part,
                    ])
                    .update_columns([TextMarkRow::WrittenBy, TextMarkRow::App, TextMarkRow::Agent])
                    .to_owned(),
                );
            transaction.execute(&insert).await?;
        }
    }
    Ok(())
}

/// The name of the app of each run the marked events were made in.
async fn apps_of(
    transaction: &mut Transaction,
    marked: &[(&DomainEvent, WrittenBy)],
) -> Result<BTreeMap<RunId, String>, StoreError> {
    let mut runs: Vec<RunId> = marked.iter().filter_map(|(event, _)| event.run).collect();
    runs.sort();
    runs.dedup();
    let mut apps = BTreeMap::new();
    if runs.is_empty() {
        return Ok(apps);
    }
    let mut select = Query::select();
    select
        .columns([ManyChangeRunRow::Id, ManyChangeRunRow::App])
        .from(ManyChangeRunRow::Table)
        .and_where(Expr::col(ManyChangeRunRow::Id).is_in(runs.iter().map(|run| run.stored())));
    for row in transaction.fetch_all(&select).await? {
        if let Some(app) = row.get::<Option<String>>(ManyChangeRunRow::App)? {
            apps.insert(row.id::<RunId>(ManyChangeRunRow::Id)?, app);
        }
    }
    Ok(apps)
}

impl crate::db::Database {
    /// The marks of the texts of these records of one kind, once each, in no order; a text without
    /// a mark has no row.
    ///
    /// # Errors
    /// Fails when the database fails or a stored mark is unreadable.
    pub async fn text_marks_of(
        &self,
        kind: &bb_core::event::EntityKind,
        ids: &[bb_core::id::EntityId],
    ) -> Result<Vec<TextMark>, StoreError> {
        let mut marks = Vec::new();
        for chunk in ids.chunks(500) {
            let mut select = Query::select();
            select
                .columns([
                    TextMarkRow::EntityId,
                    TextMarkRow::Part,
                    TextMarkRow::WrittenBy,
                    TextMarkRow::App,
                    TextMarkRow::Agent,
                ])
                .from(TextMarkRow::Table)
                .and_where(Expr::col(TextMarkRow::EntityKind).eq(kind.as_str()))
                .and_where(
                    Expr::col(TextMarkRow::EntityId).is_in(chunk.iter().map(|id| id.stored())),
                );
            for row in self.fetch_all(&select).await? {
                marks.push(TextMark {
                    entity_kind: kind.clone(),
                    entity_id: row.id(TextMarkRow::EntityId)?,
                    part: TextPart::from_stored(row.get(TextMarkRow::Part)?),
                    written_by: WrittenBy::from_code(&row.get::<String>(TextMarkRow::WrittenBy)?)
                        .ok_or(StoreError::Corrupt("a text mark"))?,
                    app: row.get(TextMarkRow::App)?,
                    agent: row.optional_id(TextMarkRow::Agent)?,
                });
            }
        }
        Ok(marks)
    }
}
