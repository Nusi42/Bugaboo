//! The adapter of the layouts port (`bb_app::layouts::LayoutStore`): where each layout is set, its
//! sections, the fields in them and the roles each field is shown to (CFG-01, ADR-0027).
//!
//! A layout is saved as a whole: its sections, fields and roles are deleted and written again in
//! the one transaction that checks the version, so a reader never sees half a layout.

use bb_app::layouts::LayoutStore;
use bb_core::fields::FieldKey;
use bb_core::id::{LayoutId, RoleId};
use bb_core::layout::{
    Layout, LayoutContent, LayoutField, LayoutFieldRole, LayoutKind, LayoutOffField, LayoutRows,
    LayoutSection, StoredLayout,
};
use bb_core::version::Version;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction};
use crate::error::StoreError;
use crate::schema::{
    LayoutFieldRoleRow, LayoutFieldRow, LayoutOffFieldRow, LayoutRow, LayoutSectionRow,
};

fn read_kind(code: &str) -> Result<LayoutKind, StoreError> {
    LayoutKind::from_code(code).ok_or(StoreError::Corrupt("a kind of layout"))
}

fn read_field(code: &str) -> Result<FieldKey, StoreError> {
    FieldKey::from_code(code).ok_or(StoreError::Corrupt("a field of a layout"))
}

fn header_select() -> sea_query::SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            LayoutRow::Id,
            LayoutRow::Project,
            LayoutRow::IssueType,
            LayoutRow::Kind,
            LayoutRow::Version,
        ])
        .from(LayoutRow::Table);
    select
}

fn read_header(row: &Row) -> Result<Layout, StoreError> {
    Ok(Layout {
        id: row.id::<LayoutId>(LayoutRow::Id)?,
        project: row.optional_id(LayoutRow::Project)?,
        issue_type: row.optional_id(LayoutRow::IssueType)?,
        kind: read_kind(&row.get::<String>(LayoutRow::Kind)?)?,
        version: row.get(LayoutRow::Version)?,
    })
}

/// Writes the sections, fields and roles of a layout.
async fn insert_content(
    transaction: &mut Transaction,
    id: LayoutId,
    content: &LayoutContent,
) -> Result<(), StoreError> {
    let LayoutRows {
        sections,
        fields,
        off,
        roles,
    } = content.rows(id);
    for section in &sections {
        let mut insert = Query::insert();
        insert
            .into_table(LayoutSectionRow::Table)
            .columns([
                LayoutSectionRow::Layout,
                LayoutSectionRow::Position,
                LayoutSectionRow::Tab,
                LayoutSectionRow::Title,
            ])
            .values([
                Expr::val(section.layout.stored()),
                Expr::val(section.position),
                Expr::val(section.tab.as_str()),
                Expr::val(section.title.as_str()),
            ])
            .map_err(|_| StoreError::Corrupt("a section of a layout"))?;
        transaction.execute(&insert).await?;
    }
    for field in &fields {
        let mut insert = Query::insert();
        insert
            .into_table(LayoutFieldRow::Table)
            .columns([
                LayoutFieldRow::Layout,
                LayoutFieldRow::Field,
                LayoutFieldRow::Section,
                LayoutFieldRow::Position,
            ])
            .values([
                Expr::val(field.layout.stored()),
                Expr::val(field.field.code().into_owned()),
                Expr::val(field.section),
                Expr::val(field.position),
            ])
            .map_err(|_| StoreError::Corrupt("a field of a layout"))?;
        transaction.execute(&insert).await?;
    }
    for field in &off {
        let mut insert = Query::insert();
        insert
            .into_table(LayoutOffFieldRow::Table)
            .columns([LayoutOffFieldRow::Layout, LayoutOffFieldRow::Field])
            .values([
                Expr::val(field.layout.stored()),
                Expr::val(field.field.code().into_owned()),
            ])
            .map_err(|_| StoreError::Corrupt("a field taken off a layout"))?;
        transaction.execute(&insert).await?;
    }
    for role in &roles {
        let mut insert = Query::insert();
        insert
            .into_table(LayoutFieldRoleRow::Table)
            .columns([
                LayoutFieldRoleRow::Layout,
                LayoutFieldRoleRow::Field,
                LayoutFieldRoleRow::Role,
            ])
            .values([
                Expr::val(role.layout.stored()),
                Expr::val(role.field.code().into_owned()),
                Expr::val(role.role.stored()),
            ])
            .map_err(|_| StoreError::Corrupt("a role of a field of a layout"))?;
        transaction.execute(&insert).await?;
    }
    Ok(())
}

/// Takes the sections, fields and roles of a layout away.
async fn delete_content(transaction: &mut Transaction, id: LayoutId) -> Result<(), StoreError> {
    let mut roles = Query::delete();
    roles
        .from_table(LayoutFieldRoleRow::Table)
        .and_where(Expr::col(LayoutFieldRoleRow::Layout).eq(id.stored()));
    transaction.execute(&roles).await?;
    let mut off = Query::delete();
    off.from_table(LayoutOffFieldRow::Table)
        .and_where(Expr::col(LayoutOffFieldRow::Layout).eq(id.stored()));
    transaction.execute(&off).await?;
    let mut fields = Query::delete();
    fields
        .from_table(LayoutFieldRow::Table)
        .and_where(Expr::col(LayoutFieldRow::Layout).eq(id.stored()));
    transaction.execute(&fields).await?;
    let mut sections = Query::delete();
    sections
        .from_table(LayoutSectionRow::Table)
        .and_where(Expr::col(LayoutSectionRow::Layout).eq(id.stored()));
    transaction.execute(&sections).await?;
    Ok(())
}

async fn read_content(
    transaction: &mut Transaction,
    id: LayoutId,
) -> Result<LayoutContent, StoreError> {
    let mut sections = Query::select();
    sections
        .columns([
            LayoutSectionRow::Layout,
            LayoutSectionRow::Position,
            LayoutSectionRow::Tab,
            LayoutSectionRow::Title,
        ])
        .from(LayoutSectionRow::Table)
        .and_where(Expr::col(LayoutSectionRow::Layout).eq(id.stored()))
        .order_by(LayoutSectionRow::Position, Order::Asc);
    let sections = transaction
        .fetch_all(&sections)
        .await?
        .iter()
        .map(|row| {
            Ok(LayoutSection {
                layout: row.id(LayoutSectionRow::Layout)?,
                position: row.get(LayoutSectionRow::Position)?,
                tab: row.get(LayoutSectionRow::Tab)?,
                title: row.get(LayoutSectionRow::Title)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let mut fields = Query::select();
    fields
        .columns([
            LayoutFieldRow::Layout,
            LayoutFieldRow::Field,
            LayoutFieldRow::Section,
            LayoutFieldRow::Position,
        ])
        .from(LayoutFieldRow::Table)
        .and_where(Expr::col(LayoutFieldRow::Layout).eq(id.stored()))
        .order_by(LayoutFieldRow::Section, Order::Asc)
        .order_by(LayoutFieldRow::Position, Order::Asc);
    let fields = transaction
        .fetch_all(&fields)
        .await?
        .iter()
        .map(|row| {
            Ok(LayoutField {
                layout: row.id(LayoutFieldRow::Layout)?,
                field: read_field(&row.get::<String>(LayoutFieldRow::Field)?)?,
                section: row.get(LayoutFieldRow::Section)?,
                position: row.get(LayoutFieldRow::Position)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let mut off = Query::select();
    off.columns([LayoutOffFieldRow::Layout, LayoutOffFieldRow::Field])
        .from(LayoutOffFieldRow::Table)
        .and_where(Expr::col(LayoutOffFieldRow::Layout).eq(id.stored()))
        .order_by(LayoutOffFieldRow::Field, Order::Asc);
    let off = transaction
        .fetch_all(&off)
        .await?
        .iter()
        .map(|row| {
            Ok(LayoutOffField {
                layout: row.id(LayoutOffFieldRow::Layout)?,
                field: read_field(&row.get::<String>(LayoutOffFieldRow::Field)?)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let mut roles = Query::select();
    roles
        .columns([
            LayoutFieldRoleRow::Layout,
            LayoutFieldRoleRow::Field,
            LayoutFieldRoleRow::Role,
        ])
        .from(LayoutFieldRoleRow::Table)
        .and_where(Expr::col(LayoutFieldRoleRow::Layout).eq(id.stored()))
        .order_by(LayoutFieldRoleRow::Role, Order::Asc);
    let roles = transaction
        .fetch_all(&roles)
        .await?
        .iter()
        .map(|row| {
            Ok(LayoutFieldRole {
                layout: row.id(LayoutFieldRoleRow::Layout)?,
                field: read_field(&row.get::<String>(LayoutFieldRoleRow::Field)?)?,
                role: row.id::<RoleId>(LayoutFieldRoleRow::Role)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(LayoutContent::from_rows(&LayoutRows {
        sections,
        fields,
        off,
        roles,
    }))
}

impl LayoutStore for AuthStorage {
    async fn layout_headers_in(
        transaction: &mut Transaction,
        kind: LayoutKind,
    ) -> Result<Vec<Layout>, StoreError> {
        let mut select = header_select();
        select
            .and_where(Expr::col(LayoutRow::Kind).eq(kind.code()))
            .order_by(LayoutRow::Id, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_header)
            .collect()
    }

    async fn stored_layout_in(
        transaction: &mut Transaction,
        id: LayoutId,
    ) -> Result<Option<StoredLayout>, StoreError> {
        let mut select = header_select();
        select.and_where(Expr::col(LayoutRow::Id).eq(id.stored()));
        let Some(layout) = transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_header)
            .transpose()?
        else {
            return Ok(None);
        };
        let content = read_content(transaction, id).await?;
        Ok(Some(StoredLayout { layout, content }))
    }

    async fn insert_layout(
        transaction: &mut Transaction,
        layout: &Layout,
        content: &LayoutContent,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(LayoutRow::Table)
            .columns([
                LayoutRow::Id,
                LayoutRow::Project,
                LayoutRow::IssueType,
                LayoutRow::Kind,
                LayoutRow::Version,
            ])
            .values([
                Expr::val(layout.id.stored()),
                Expr::val(layout.project.map(StoredId::stored)),
                Expr::val(layout.issue_type.map(StoredId::stored)),
                Expr::val(layout.kind.code()),
                Expr::val(layout.version),
            ])
            .map_err(|_| StoreError::Corrupt("a layout"))?;
        transaction.execute(&insert).await?;
        insert_content(transaction, layout.id, content).await
    }

    async fn replace_layout(
        transaction: &mut Transaction,
        id: LayoutId,
        content: &LayoutContent,
        version: Version,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(LayoutRow::Table)
            .value(LayoutRow::Version, Expr::col(LayoutRow::Version).add(1))
            .and_where(Expr::col(LayoutRow::Id).eq(id.stored()))
            .and_where(Expr::col(LayoutRow::Version).eq(version.get()));
        if transaction.execute(&update).await? == 0 {
            return Ok(false);
        }
        delete_content(transaction, id).await?;
        insert_content(transaction, id, content).await?;
        Ok(true)
    }

    async fn delete_layout(
        transaction: &mut Transaction,
        id: LayoutId,
        version: Version,
    ) -> Result<bool, StoreError> {
        let mut select = header_select();
        select
            .and_where(Expr::col(LayoutRow::Id).eq(id.stored()))
            .and_where(Expr::col(LayoutRow::Version).eq(version.get()));
        if transaction.fetch_optional(&select).await?.is_none() {
            return Ok(false);
        }
        delete_content(transaction, id).await?;
        let mut delete = Query::delete();
        delete
            .from_table(LayoutRow::Table)
            .and_where(Expr::col(LayoutRow::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }
}

#[cfg(test)]
#[path = "tests/layouts.rs"]
mod tests;
