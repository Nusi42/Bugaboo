//! Migration 62: what a person lets their agent do beyond its work (ADR-0029, amendment M9).
//!
//! Two switches of an agent, *May hand over to my other agents* and *May undo its own runs*, both
//! off for every agent there is and every agent made later. Beside them, on a hand-over, the agent
//! that made it (none for a person or a standing rule) and its place in a chain of hand-overs (0 for
//! every hand-over there is, which a person or a rule made).
//!
//! The event an undo writes on its run is marked by its reason, `run-undone`
//! (`bb_core::run::RUN_UNDONE`), so the page of the run and the history of an agent find it by that
//! alone; the events of undos made before get the mark. The history of an agent reads the
//! hand-overs it made, the runs of the agent and the attempt of each run through three indexes. The words are written here as they were
//! when the migration was made, as a migration never changes.

use sea_query::{ColumnDef, Expr, ExprTrait as _, Index, Query, Table};

use super::Step;
use crate::schema::{AgentRow, AttemptRow, Event, HandOverRow, ManyChangeRunRow};

fn off(name: impl sea_query::Iden + 'static) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.boolean().not_null().default(false);
    column
}

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Alter(
            Table::alter()
                .table(AgentRow::Table)
                .add_column(off(AgentRow::MayHandOver))
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(AgentRow::Table)
                .add_column(off(AgentRow::MayUndo))
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(HandOverRow::Table)
                .add_column(ColumnDef::new(HandOverRow::ByAgent).blob())
                .to_owned(),
        ),
        Step::Alter(
            Table::alter()
                .table(HandOverRow::Table)
                .add_column(
                    ColumnDef::new(HandOverRow::Depth)
                        .big_integer()
                        .not_null()
                        .default(0),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("hand_over_by_making_agent")
                .table(HandOverRow::Table)
                .col(HandOverRow::ByAgent)
                .col(HandOverRow::Person)
                .col(HandOverRow::MadeAt)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("change_run_by_agent")
                .table(ManyChangeRunRow::Table)
                .col(ManyChangeRunRow::Agent)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("agent_attempt_by_run")
                .table(AttemptRow::Table)
                .col(AttemptRow::Run)
                .to_owned(),
        ),
        Step::Update(
            Query::update()
                .table(Event::Table)
                .value(Event::Reason, "run-undone")
                .and_where(Expr::col(Event::EntityKind).eq("change_run"))
                .and_where(Expr::col(Event::Changes).like(r#"%"field":"undone"%"#))
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use bb_app::ports::ChangeStore as _;
    use bb_core::event::{
        Actor, Channel, DomainEvent, EntityKind, EntityRef, FieldChange, RecordKind,
    };
    use bb_core::id::{EntityId, EventId, RunId, UserId};
    use bb_testing::{TestResult, each_database};
    use serde_json::json;

    use super::*;
    use crate::db::StoredId as _;
    use crate::test_support::{at, fixture, ids};

    /// An event of a run, with one change of `field` and no reason, as the server wrote it before.
    fn of_the_run(
        source: &mut impl bb_core::id::IdSource,
        run: RunId,
        field: &str,
    ) -> Result<DomainEvent, bb_core::id::IdError> {
        Ok(DomainEvent {
            id: EventId::generate(source)?,
            time: at(5),
            actor: Actor::Person(UserId::generate(source)),
            channel: Channel::Api,
            entity: EntityRef {
                kind: EntityKind::from(RecordKind::ChangeRun),
                id: EntityId::from(run),
            },
            changes: vec![FieldChange {
                field: field.to_owned(),
                before: None,
                after: Some(json!(1)),
            }],
            reason: None,
            run: None,
        })
    }

    #[tokio::test]
    async fn ai_38_the_undos_made_before_are_marked_as_undos_of_their_run() -> TestResult {
        each_database("ai_38_undo_fact_mark", async |db| {
            let fixture = fixture(db).await?;
            let storage = &fixture.storage;
            let mut source = ids();
            let run = RunId::generate(&mut source)?;
            let undone = of_the_run(&mut source, run, "undone")?;
            let opened = of_the_run(&mut source, run, "status")?;
            for event in [&undone, &opened] {
                storage
                    .record_change(event, |_| Box::pin(async { Ok(()) }))
                    .await?;
            }
            for step in steps() {
                if let Step::Update(update) = step {
                    storage.database().execute(&update).await?;
                }
            }
            let reason_of =
                async |event: &DomainEvent| -> Result<Option<String>, crate::StoreError> {
                    let mut select = Query::select();
                    select
                        .column(Event::Reason)
                        .from(Event::Table)
                        .and_where(Expr::col(Event::Id).eq(event.id.stored()));
                    storage.database().fetch_all(&select).await?[0].get(Event::Reason)
                };
            assert_eq!(
                reason_of(&undone).await?.as_deref(),
                Some(bb_core::run::RUN_UNDONE)
            );
            assert_eq!(reason_of(&opened).await?, None);
            Ok(())
        })
        .await
    }
}
