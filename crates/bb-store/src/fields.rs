//! The adapter of the own-field port (`bb_app::fields::FieldStore`): the definition of a field,
//! where it is used, its choices and the values items hold (CF-01, CF-03, CF-04, ADR-0006).
//!
//! Like the other adapters, reads run in one snapshot and writes are associated functions that run
//! inside the transaction of a change. A value is one row per value in `field_value`, with one
//! typed column per kind; the cells of `bb_core::own_fields_value` are those rows.

use bb_core::version::Version;
use std::collections::BTreeMap;

use bb_app::fields::{Definition, FieldStore, StoredValues};
use bb_core::event::{EntityKind, RecordKind};
use bb_core::id::{ChoiceId, EntityId, FieldId, IssueTypeId, ProjectId};
use bb_core::issue::IssueIcon;
use bb_core::own_fields::{
    ChoiceColor, FieldChoice, FieldFor, FieldType, Look, OwnField, Rules, Scope,
};
use bb_core::own_fields_value::Cell;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{Reader, Row, STATEMENT_ITEMS, StoredId as _, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{
    FieldChoiceRow, FieldIssueTypeRow, FieldProjectRow, FieldRow, FieldValueRow, IssueRow,
};

/// The IDs of every own field, so that each has a level in an evaluation.
pub(crate) async fn field_ids_in(reader: &mut impl Reader) -> Result<Vec<FieldId>, StoreError> {
    let mut select = Query::select();
    select
        .column(FieldRow::Id)
        .from(FieldRow::Table)
        .order_by(FieldRow::Id, Order::Asc);
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| row.id::<FieldId>(FieldRow::Id))
        .collect()
}

fn read_field(row: &Row) -> Result<OwnField, StoreError> {
    Ok(OwnField {
        id: row.id::<FieldId>(FieldRow::Id)?,
        name: row.get(FieldRow::Name)?,
        help: row.get(FieldRow::Help)?,
        kind: FieldType::from_code(&row.get::<String>(FieldRow::Kind)?)
            .ok_or(StoreError::Corrupt("a field type"))?,
        applies_to: FieldFor::from_code(&row.get::<String>(FieldRow::AppliesTo)?)
            .ok_or(StoreError::Corrupt("what a field is for"))?,
        rules: serde_json::from_str::<Rules>(&row.get::<String>(FieldRow::Rules)?)
            .map_err(|_| StoreError::Corrupt("the rules of a field"))?,
        default_value: row.get(FieldRow::DefaultValue)?,
        look: Look::from_code(&row.get::<String>(FieldRow::Look)?)
            .ok_or(StoreError::Corrupt("a look"))?,
        never_to_assistant: row.get(FieldRow::NeverToAssistant)?,
        position: row.get(FieldRow::Position)?,
        retired_at: row.optional_time(FieldRow::RetiredAt)?,
        version: row.get(FieldRow::Version)?,
    })
}

fn read_choice(row: &Row) -> Result<FieldChoice, StoreError> {
    Ok(FieldChoice {
        id: row.id::<ChoiceId>(FieldChoiceRow::Id)?,
        field: row.id::<FieldId>(FieldChoiceRow::Field)?,
        name: row.get(FieldChoiceRow::Name)?,
        color: ChoiceColor::from_code(&row.get::<String>(FieldChoiceRow::Color)?)
            .ok_or(StoreError::Corrupt("a choice color"))?,
        icon: row
            .get::<Option<String>>(FieldChoiceRow::Icon)?
            .map(|code| IssueIcon::from_code(&code).ok_or(StoreError::Corrupt("an icon")))
            .transpose()?,
        position: row.get(FieldChoiceRow::Position)?,
        retired_at: row.optional_time(FieldChoiceRow::RetiredAt)?,
    })
}

async fn definitions_in(reader: &mut impl Reader) -> Result<Vec<Definition>, StoreError> {
    let mut select = Query::select();
    select
        .columns([
            FieldRow::Id,
            FieldRow::Name,
            FieldRow::Help,
            FieldRow::Kind,
            FieldRow::AppliesTo,
            FieldRow::Rules,
            FieldRow::DefaultValue,
            FieldRow::Look,
            FieldRow::NeverToAssistant,
            FieldRow::Position,
            FieldRow::RetiredAt,
            FieldRow::Version,
        ])
        .from(FieldRow::Table)
        .order_by(FieldRow::Position, Order::Asc)
        .order_by(FieldRow::Id, Order::Asc);
    let mut found: Vec<Definition> = Vec::new();
    for row in reader.fetch_all(&select).await? {
        found.push(Definition {
            field: read_field(&row)?,
            scope: Scope::default(),
            choices: Vec::new(),
        });
    }
    let position: BTreeMap<FieldId, usize> = found
        .iter()
        .enumerate()
        .map(|(at, definition)| (definition.field.id, at))
        .collect();

    let mut select = Query::select();
    select
        .columns([FieldProjectRow::Field, FieldProjectRow::Project])
        .from(FieldProjectRow::Table)
        .order_by(FieldProjectRow::Project, Order::Asc);
    for row in reader.fetch_all(&select).await? {
        let field = row.id::<FieldId>(FieldProjectRow::Field)?;
        if let Some(at) = position.get(&field) {
            found[*at]
                .scope
                .projects
                .push(row.id::<ProjectId>(FieldProjectRow::Project)?);
        }
    }

    let mut select = Query::select();
    select
        .columns([FieldIssueTypeRow::Field, FieldIssueTypeRow::IssueType])
        .from(FieldIssueTypeRow::Table)
        .order_by(FieldIssueTypeRow::IssueType, Order::Asc);
    for row in reader.fetch_all(&select).await? {
        let field = row.id::<FieldId>(FieldIssueTypeRow::Field)?;
        if let Some(at) = position.get(&field) {
            found[*at]
                .scope
                .issue_types
                .push(row.id::<IssueTypeId>(FieldIssueTypeRow::IssueType)?);
        }
    }

    let mut select = Query::select();
    select
        .columns([
            FieldChoiceRow::Id,
            FieldChoiceRow::Field,
            FieldChoiceRow::Name,
            FieldChoiceRow::Color,
            FieldChoiceRow::Icon,
            FieldChoiceRow::Position,
            FieldChoiceRow::RetiredAt,
        ])
        .from(FieldChoiceRow::Table)
        .order_by(FieldChoiceRow::Position, Order::Asc)
        .order_by(FieldChoiceRow::Id, Order::Asc);
    for row in reader.fetch_all(&select).await? {
        let choice = read_choice(&row)?;
        if let Some(at) = position.get(&choice.field) {
            found[*at].choices.push(choice);
        }
    }
    Ok(found)
}

fn read_cell(row: &Row) -> Result<Cell, StoreError> {
    Ok(Cell {
        text: row.get(FieldValueRow::ValueText)?,
        number: row.get(FieldValueRow::ValueNumber)?,
        date: row.get(FieldValueRow::ValueDate)?,
        reference: row
            .get::<Option<Vec<u8>>>(FieldValueRow::ValueRef)?
            .map(|bytes| {
                <[u8; 16]>::try_from(bytes).map_err(|_| StoreError::Corrupt("a value reference"))
            })
            .transpose()?,
    })
}

pub(crate) fn kind_text(kind: &EntityKind) -> String {
    kind.as_str().to_owned()
}

fn kind_from(text: &str) -> Result<EntityKind, StoreError> {
    EntityKind::new(text).map_err(|_| StoreError::Corrupt("a kind of item"))
}

async fn values_in(
    reader: &mut impl Reader,
    kind: &EntityKind,
    entities: &[EntityId],
) -> Result<Vec<StoredValues>, StoreError> {
    let mut found: Vec<StoredValues> = Vec::new();
    for chunk in entities.chunks(STATEMENT_ITEMS) {
        found.extend(values_of_chunk(reader, kind, chunk).await?);
    }
    Ok(found)
}

/// The values of at most [`STATEMENT_ITEMS`] items of one kind, in one statement.
async fn values_of_chunk(
    reader: &mut impl Reader,
    kind: &EntityKind,
    entities: &[EntityId],
) -> Result<Vec<StoredValues>, StoreError> {
    let mut select = Query::select();
    select
        .columns([
            FieldValueRow::EntityId,
            FieldValueRow::Field,
            FieldValueRow::ValueText,
            FieldValueRow::ValueNumber,
            FieldValueRow::ValueDate,
            FieldValueRow::ValueRef,
        ])
        .from(FieldValueRow::Table)
        .and_where(Expr::col(FieldValueRow::EntityKind).eq(kind_text(kind)))
        .and_where(
            Expr::col(FieldValueRow::EntityId)
                .is_in(entities.iter().map(|id| id.stored()).collect::<Vec<_>>()),
        )
        .order_by(FieldValueRow::EntityId, Order::Asc)
        .order_by(FieldValueRow::Field, Order::Asc)
        .order_by(FieldValueRow::Position, Order::Asc);
    let mut found: Vec<StoredValues> = Vec::new();
    for row in reader.fetch_all(&select).await? {
        let entity = row.id::<EntityId>(FieldValueRow::EntityId)?;
        let field = row.id::<FieldId>(FieldValueRow::Field)?;
        let cell = read_cell(&row)?;
        match found.last_mut() {
            Some(last) if last.entity == entity && last.field == field => last.cells.push(cell),
            _ => found.push(StoredValues {
                entity,
                field,
                cells: vec![cell],
            }),
        }
    }
    Ok(found)
}

async fn items_with_choice_in(
    reader: &mut impl Reader,
    field: FieldId,
    choice: ChoiceId,
) -> Result<Vec<(EntityKind, EntityId)>, StoreError> {
    let mut select = Query::select();
    select
        .distinct()
        .columns([FieldValueRow::EntityKind, FieldValueRow::EntityId])
        .from(FieldValueRow::Table)
        .and_where(Expr::col(FieldValueRow::Field).eq(field.stored()))
        .and_where(Expr::col(FieldValueRow::ValueRef).eq(choice.stored()))
        .order_by(FieldValueRow::EntityId, Order::Asc);
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            Ok((
                kind_from(&row.get::<String>(FieldValueRow::EntityKind)?)?,
                row.id::<EntityId>(FieldValueRow::EntityId)?,
            ))
        })
        .collect()
}

async fn write_scope(
    transaction: &mut Transaction,
    field: FieldId,
    scope: &Scope,
) -> Result<(), StoreError> {
    let mut delete = Query::delete();
    delete
        .from_table(FieldProjectRow::Table)
        .and_where(Expr::col(FieldProjectRow::Field).eq(field.stored()));
    transaction.execute(&delete).await?;
    let mut delete = Query::delete();
    delete
        .from_table(FieldIssueTypeRow::Table)
        .and_where(Expr::col(FieldIssueTypeRow::Field).eq(field.stored()));
    transaction.execute(&delete).await?;
    for project in &scope.projects {
        let mut insert = Query::insert();
        insert
            .into_table(FieldProjectRow::Table)
            .columns([FieldProjectRow::Field, FieldProjectRow::Project])
            .values([Expr::val(field.stored()), Expr::val(project.stored())])
            .map_err(|_| StoreError::Corrupt("a project of a field"))?;
        transaction.execute(&insert).await?;
    }
    for issue_type in &scope.issue_types {
        let mut insert = Query::insert();
        insert
            .into_table(FieldIssueTypeRow::Table)
            .columns([FieldIssueTypeRow::Field, FieldIssueTypeRow::IssueType])
            .values([Expr::val(field.stored()), Expr::val(issue_type.stored())])
            .map_err(|_| StoreError::Corrupt("an issue type of a field"))?;
        transaction.execute(&insert).await?;
    }
    Ok(())
}

async fn insert_choice(
    transaction: &mut Transaction,
    choice: &FieldChoice,
) -> Result<(), StoreError> {
    let mut insert = Query::insert();
    insert
        .into_table(FieldChoiceRow::Table)
        .columns([
            FieldChoiceRow::Id,
            FieldChoiceRow::Field,
            FieldChoiceRow::Name,
            FieldChoiceRow::Color,
            FieldChoiceRow::Icon,
            FieldChoiceRow::Position,
            FieldChoiceRow::RetiredAt,
        ])
        .values([
            Expr::val(choice.id.stored()),
            Expr::val(choice.field.stored()),
            Expr::val(choice.name.as_str()),
            Expr::val(choice.color.code()),
            Expr::val(choice.icon.map(IssueIcon::code)),
            Expr::val(choice.position),
            Expr::val(choice.retired_at.map(to_micros)),
        ])
        .map_err(|_| StoreError::Corrupt("a choice"))?;
    transaction.execute(&insert).await.map(|_| ())
}

fn rules_text(rules: &Rules) -> Result<String, StoreError> {
    serde_json::to_string(rules).map_err(|_| StoreError::Corrupt("the rules of a field"))
}

impl FieldStore for AuthStorage {
    async fn field_definitions_in(
        transaction: &mut Transaction,
    ) -> Result<Vec<Definition>, StoreError> {
        definitions_in(transaction).await
    }

    async fn field_values_in(
        transaction: &mut Transaction,
        kind: EntityKind,
        entities: Vec<EntityId>,
    ) -> Result<Vec<StoredValues>, StoreError> {
        values_in(transaction, &kind, &entities).await
    }

    async fn items_with_choice_in(
        transaction: &mut Transaction,
        field: FieldId,
        choice: ChoiceId,
    ) -> Result<Vec<(EntityKind, EntityId)>, StoreError> {
        items_with_choice_in(transaction, field, choice).await
    }

    async fn touch_items(
        transaction: &mut Transaction,
        items: &[(EntityKind, EntityId)],
        at: Timestamp,
    ) -> Result<Vec<Option<i64>>, StoreError> {
        let issue = EntityKind::from(RecordKind::Issue);
        let mut versions: Vec<Option<i64>> = vec![None; items.len()];
        // Issues, a chunk of them in one statement; any other kind of item one at a time.
        let mut issues: Vec<(usize, EntityId)> = Vec::new();
        for (place, (kind, id)) in items.iter().enumerate() {
            if *kind == issue {
                issues.push((place, *id));
            } else if let Some(slot) = versions.get_mut(place) {
                *slot = <Self as FieldStore>::touch_item(transaction, kind, *id, at).await?;
            }
        }
        for chunk in issues.chunks(STATEMENT_ITEMS) {
            let mut update = Query::update();
            update.table(IssueRow::Table);
            crate::issues::counting_up(&mut update, at)
                .and_where(
                    Expr::col(IssueRow::Id)
                        .is_in(chunk.iter().map(|(_, id)| id.stored()).collect::<Vec<_>>()),
                )
                .returning(Query::returning().columns([IssueRow::Id, IssueRow::Version]));
            for row in transaction.fetch_all(&update).await? {
                let id = row.id::<EntityId>(IssueRow::Id)?;
                let now: i64 = row.get(IssueRow::Version)?;
                let place = chunk
                    .iter()
                    .find(|(_, each)| *each == id)
                    .map(|(place, _)| *place);
                // The version it had before the statement counted it up.
                if let Some(slot) = place.and_then(|place| versions.get_mut(place)) {
                    *slot = Some(now - 1);
                }
            }
        }
        Ok(versions)
    }

    async fn projects_with_values(&self, field: FieldId) -> Result<Vec<ProjectId>, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        let mut projects: Vec<ProjectId> = Vec::new();
        // The projects of the issues that hold a value.
        let mut select = Query::select();
        select
            .distinct()
            .column((IssueRow::Table, IssueRow::Project))
            .from(FieldValueRow::Table)
            .inner_join(
                IssueRow::Table,
                Expr::col((IssueRow::Table, IssueRow::Id))
                    .equals((FieldValueRow::Table, FieldValueRow::EntityId)),
            )
            .and_where(Expr::col((FieldValueRow::Table, FieldValueRow::Field)).eq(field.stored()))
            .and_where(
                Expr::col((FieldValueRow::Table, FieldValueRow::EntityKind))
                    .eq(RecordKind::Issue.code()),
            );
        for row in transaction.fetch_all(&select).await? {
            projects.push(row.id::<ProjectId>(IssueRow::Project)?);
        }
        // The projects that hold a value themselves.
        let mut select = Query::select();
        select
            .distinct()
            .column(FieldValueRow::EntityId)
            .from(FieldValueRow::Table)
            .and_where(Expr::col(FieldValueRow::Field).eq(field.stored()))
            .and_where(Expr::col(FieldValueRow::EntityKind).eq(RecordKind::Project.code()));
        for row in transaction.fetch_all(&select).await? {
            let id = row.id::<EntityId>(FieldValueRow::EntityId)?;
            projects.push(
                ProjectId::from_bytes(id.to_bytes())
                    .map_err(|_| StoreError::Corrupt("a project ID"))?,
            );
        }
        projects.sort();
        projects.dedup();
        Ok(projects)
    }

    async fn insert_field(
        transaction: &mut Transaction,
        definition: &Definition,
    ) -> Result<(), StoreError> {
        let field = &definition.field;
        let mut insert = Query::insert();
        insert
            .into_table(FieldRow::Table)
            .columns([
                FieldRow::Id,
                FieldRow::Name,
                FieldRow::Help,
                FieldRow::Kind,
                FieldRow::AppliesTo,
                FieldRow::Rules,
                FieldRow::DefaultValue,
                FieldRow::Look,
                FieldRow::NeverToAssistant,
                FieldRow::Position,
                FieldRow::RetiredAt,
                FieldRow::Version,
            ])
            .values([
                Expr::val(field.id.stored()),
                Expr::val(field.name.as_str()),
                Expr::val(field.help.as_str()),
                Expr::val(field.kind.code()),
                Expr::val(field.applies_to.code()),
                Expr::val(rules_text(&field.rules)?),
                Expr::val(field.default_value.as_str()),
                Expr::val(field.look.code()),
                Expr::val(field.never_to_assistant),
                Expr::val(field.position),
                Expr::val(field.retired_at.map(to_micros)),
                Expr::val(field.version),
            ])
            .map_err(|_| StoreError::Corrupt("a field"))?;
        transaction.execute(&insert).await?;
        write_scope(transaction, field.id, &definition.scope).await?;
        for choice in &definition.choices {
            insert_choice(transaction, choice).await?;
        }
        Ok(())
    }

    async fn update_field(
        transaction: &mut Transaction,
        field: &OwnField,
        scope: &Scope,
        version: Version,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(FieldRow::Table)
            .value(FieldRow::Name, field.name.as_str())
            .value(FieldRow::Help, field.help.as_str())
            .value(FieldRow::Rules, rules_text(&field.rules)?)
            .value(FieldRow::DefaultValue, field.default_value.as_str())
            .value(FieldRow::Look, field.look.code())
            .value(FieldRow::NeverToAssistant, field.never_to_assistant)
            .value(FieldRow::Position, field.position)
            .value(FieldRow::RetiredAt, field.retired_at.map(to_micros))
            .value(FieldRow::Version, Expr::col(FieldRow::Version).add(1))
            .and_where(Expr::col(FieldRow::Id).eq(field.id.stored()))
            .and_where(Expr::col(FieldRow::Version).eq(version.get()));
        if transaction.execute(&update).await? == 0 {
            return Ok(false);
        }
        write_scope(transaction, field.id, scope).await?;
        Ok(true)
    }

    async fn bump_field(
        transaction: &mut Transaction,
        field: FieldId,
        version: Version,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(FieldRow::Table)
            .value(FieldRow::Version, Expr::col(FieldRow::Version).add(1))
            .and_where(Expr::col(FieldRow::Id).eq(field.stored()))
            .and_where(Expr::col(FieldRow::Version).eq(version.get()));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_choice(
        transaction: &mut Transaction,
        choice: &FieldChoice,
    ) -> Result<(), StoreError> {
        insert_choice(transaction, choice).await
    }

    async fn update_choice(
        transaction: &mut Transaction,
        choice: &FieldChoice,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(FieldChoiceRow::Table)
            .value(FieldChoiceRow::Name, choice.name.as_str())
            .value(FieldChoiceRow::Color, choice.color.code())
            .value(FieldChoiceRow::Icon, choice.icon.map(IssueIcon::code))
            .value(FieldChoiceRow::RetiredAt, choice.retired_at.map(to_micros))
            .and_where(Expr::col(FieldChoiceRow::Id).eq(choice.id.stored()));
        transaction.execute(&update).await.map(|_| ())
    }

    async fn order_choices(
        transaction: &mut Transaction,
        field: FieldId,
        order: &[ChoiceId],
    ) -> Result<(), StoreError> {
        for (position, choice) in (0_i64..).zip(order) {
            let mut update = Query::update();
            update
                .table(FieldChoiceRow::Table)
                .value(FieldChoiceRow::Position, position)
                .and_where(Expr::col(FieldChoiceRow::Id).eq(choice.stored()))
                .and_where(Expr::col(FieldChoiceRow::Field).eq(field.stored()));
            transaction.execute(&update).await?;
        }
        Ok(())
    }

    async fn put_field_value(
        transaction: &mut Transaction,
        kind: EntityKind,
        entity: EntityId,
        field: FieldId,
        cells: &[Cell],
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(FieldValueRow::Table)
            .and_where(Expr::col(FieldValueRow::EntityKind).eq(kind_text(&kind)))
            .and_where(Expr::col(FieldValueRow::EntityId).eq(entity.stored()))
            .and_where(Expr::col(FieldValueRow::Field).eq(field.stored()));
        transaction.execute(&delete).await?;
        for (position, cell) in (0_i64..).zip(cells) {
            let mut insert = Query::insert();
            insert
                .into_table(FieldValueRow::Table)
                .columns([
                    FieldValueRow::EntityKind,
                    FieldValueRow::EntityId,
                    FieldValueRow::Field,
                    FieldValueRow::ValueText,
                    FieldValueRow::ValueNumber,
                    FieldValueRow::ValueDate,
                    FieldValueRow::ValueRef,
                    FieldValueRow::Position,
                ])
                .values([
                    Expr::val(kind_text(&kind)),
                    Expr::val(entity.stored()),
                    Expr::val(field.stored()),
                    Expr::val(cell.text.clone()),
                    Expr::val(cell.number),
                    Expr::val(cell.date),
                    Expr::val(cell.reference.map(|bytes| bytes.to_vec())),
                    Expr::val(position),
                ])
                .map_err(|_| StoreError::Corrupt("a field value"))?;
            transaction.execute(&insert).await?;
        }
        Ok(())
    }

    async fn combine_choice_values(
        transaction: &mut Transaction,
        field: FieldId,
        from: ChoiceId,
        into: ChoiceId,
    ) -> Result<Vec<(EntityKind, EntityId)>, StoreError> {
        let holders = items_with_choice_in(transaction, field, from).await?;
        if holders.is_empty() {
            return Ok(holders);
        }
        // An item that holds both choices keeps the one row it has for the other: the rows of
        // the first choice go, for every such item at once.
        let mut into_holders = Query::select();
        into_holders
            .columns([FieldValueRow::EntityKind, FieldValueRow::EntityId])
            .from(FieldValueRow::Table)
            .and_where(Expr::col(FieldValueRow::Field).eq(field.stored()))
            .and_where(Expr::col(FieldValueRow::ValueRef).eq(into.stored()));
        let mut delete = Query::delete();
        delete
            .from_table(FieldValueRow::Table)
            .and_where(Expr::col(FieldValueRow::Field).eq(field.stored()))
            .and_where(Expr::col(FieldValueRow::ValueRef).eq(from.stored()))
            .and_where(
                Expr::tuple([
                    Expr::col(FieldValueRow::EntityKind),
                    Expr::col(FieldValueRow::EntityId),
                ])
                .in_subquery(into_holders),
            );
        transaction.execute(&delete).await?;
        // The others move to the other choice, all in one statement.
        let mut update = Query::update();
        update
            .table(FieldValueRow::Table)
            .value(FieldValueRow::ValueRef, into.stored())
            .and_where(Expr::col(FieldValueRow::Field).eq(field.stored()))
            .and_where(Expr::col(FieldValueRow::ValueRef).eq(from.stored()));
        transaction.execute(&update).await?;
        Ok(holders)
    }
}

#[cfg(test)]
#[path = "tests/fields.rs"]
mod tests;
