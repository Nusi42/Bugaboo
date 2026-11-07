//! The adapters of the ports of the tools (`bb_app::ai::NeverSentStore`, `bb_app::ai::tools::ToolCallStore`):
//! the marks *Never sent to AI* of built-in fields and projects, and the log of tool calls.

use bb_app::ai::NeverSentStore;
use bb_app::ai::tools::ToolCallStore;
use bb_core::ai::{NeverSentMark, NeverSentSubject, ToolCall, ToolCaller, ToolEffect, ToolOutcome};
use bb_core::fields::Field;
use bb_core::id::ProjectId;
use jiff::Timestamp;
use sea_query::{Condition, Expr, ExprTrait as _, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId as _, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{NeverSentRow, ToolCallRow};

const MARK_COLUMNS: [NeverSentRow; 4] = [
    NeverSentRow::Field,
    NeverSentRow::Project,
    NeverSentRow::Marked,
    NeverSentRow::Version,
];

fn read_mark(row: &Row) -> Result<NeverSentMark, StoreError> {
    let field = row.get::<Option<String>>(NeverSentRow::Field)?;
    let project = row.optional_id::<ProjectId>(NeverSentRow::Project)?;
    let subject = match (field, project) {
        (Some(word), None) => NeverSentSubject::Field(
            Field::from_code(&word).ok_or(StoreError::Corrupt("a built-in field"))?,
        ),
        (None, Some(project)) => NeverSentSubject::Project(project),
        _ => return Err(StoreError::Corrupt("a mark of Never sent to AI")),
    };
    Ok(NeverSentMark {
        subject,
        marked: row.get(NeverSentRow::Marked)?,
        version: row.get(NeverSentRow::Version)?,
    })
}

/// The condition that picks the row of a subject.
fn of(subject: NeverSentSubject) -> Condition {
    match subject {
        NeverSentSubject::Field(field) => {
            Condition::all().add(Expr::col(NeverSentRow::Field).eq(field.code()))
        }
        NeverSentSubject::Project(project) => {
            Condition::all().add(Expr::col(NeverSentRow::Project).eq(project.stored()))
        }
    }
}

impl NeverSentStore for AuthStorage {
    async fn never_sent_marks_in(
        transaction: &mut Transaction,
    ) -> Result<Vec<NeverSentMark>, StoreError> {
        let mut select = Query::select();
        select.columns(MARK_COLUMNS).from(NeverSentRow::Table);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_mark)
            .collect()
    }

    async fn put_never_sent_mark(
        transaction: &mut Transaction,
        mark: &NeverSentMark,
    ) -> Result<bool, StoreError> {
        // The row is written when it is still at the version before the new one; a subject
        // that has no row is at version 0.
        if mark.version == 1 {
            let (field, project) = match mark.subject {
                NeverSentSubject::Field(field) => (Some(field.code()), None),
                NeverSentSubject::Project(project) => (None, Some(project.stored())),
            };
            let mut insert = Query::insert();
            insert
                .into_table(NeverSentRow::Table)
                .columns(MARK_COLUMNS)
                .values([
                    Expr::val(field),
                    Expr::val(project),
                    Expr::val(mark.marked),
                    Expr::val(mark.version),
                ])
                .map_err(|_| StoreError::Corrupt("a mark of Never sent to AI"))?
                .on_conflict(sea_query::OnConflict::new().do_nothing().to_owned());
            return Ok(transaction.execute(&insert).await? > 0);
        }
        let mut update = Query::update();
        update
            .table(NeverSentRow::Table)
            .value(NeverSentRow::Marked, mark.marked)
            .value(NeverSentRow::Version, mark.version)
            .cond_where(of(mark.subject))
            .and_where(Expr::col(NeverSentRow::Version).eq(mark.version - 1));
        Ok(transaction.execute(&update).await? > 0)
    }
}

const CALL_COLUMNS: [ToolCallRow; 10] = [
    ToolCallRow::Id,
    ToolCallRow::Person,
    ToolCallRow::Caller,
    ToolCallRow::App,
    ToolCallRow::Tool,
    ToolCallRow::Effect,
    ToolCallRow::BytesIn,
    ToolCallRow::BytesOut,
    ToolCallRow::At,
    ToolCallRow::Outcome,
];

fn read_call(row: &Row) -> Result<ToolCall, StoreError> {
    let corrupt = StoreError::Corrupt("a tool call");
    Ok(ToolCall {
        id: row.id(ToolCallRow::Id)?,
        person: row.id(ToolCallRow::Person)?,
        caller: ToolCaller::from_code(&row.get::<String>(ToolCallRow::Caller)?)
            .ok_or(StoreError::Corrupt("a tool caller"))?,
        app: row.get(ToolCallRow::App)?,
        tool: row
            .get::<Option<String>>(ToolCallRow::Tool)?
            .map(|code| bb_core::ai::Tool::from_code(&code).ok_or(StoreError::Corrupt("a tool")))
            .transpose()?,
        effect: row
            .get::<Option<String>>(ToolCallRow::Effect)?
            .map(|code| ToolEffect::from_code(&code).ok_or(StoreError::Corrupt("a tool effect")))
            .transpose()?,
        bytes_in: row.get(ToolCallRow::BytesIn)?,
        bytes_out: row.get(ToolCallRow::BytesOut)?,
        at: row.time(ToolCallRow::At)?,
        outcome: ToolOutcome::from_code(&row.get::<String>(ToolCallRow::Outcome)?)
            .ok_or(corrupt)?,
    })
}

impl ToolCallStore for AuthStorage {
    async fn append_tool_call(&self, call: &ToolCall) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(ToolCallRow::Table)
            .columns(CALL_COLUMNS)
            .values([
                Expr::val(call.id.stored()),
                Expr::val(call.person.stored()),
                Expr::val(call.caller.code()),
                Expr::val(call.app.clone()),
                Expr::val(call.tool.map(bb_core::ai::Tool::code)),
                Expr::val(call.effect.map(ToolEffect::code)),
                Expr::val(call.bytes_in),
                Expr::val(call.bytes_out),
                Expr::val(to_micros(call.at)),
                Expr::val(call.outcome.code()),
            ])
            .map_err(|_| StoreError::Corrupt("a tool call"))?;
        self.database().execute(&insert).await.map(|_| ())
    }

    async fn prune_tool_calls(&self, before: Timestamp) -> Result<u64, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(ToolCallRow::Table)
            .and_where(Expr::col(ToolCallRow::At).lt(to_micros(before)));
        self.database().execute(&delete).await
    }

    async fn tool_calls(&self, limit: u64) -> Result<Vec<ToolCall>, StoreError> {
        let mut select = Query::select();
        select
            .columns(CALL_COLUMNS)
            .from(ToolCallRow::Table)
            .order_by(ToolCallRow::At, Order::Desc)
            .order_by(ToolCallRow::Id, Order::Desc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_call)
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/ai_tools.rs"]
mod tests;
