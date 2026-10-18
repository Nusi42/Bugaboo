//! Migration 19: the fields that were taken off a layout (CFG-01, ADR-0027, M3-F107).
//!
//! A field that is taken off the form keeps the roles it was shown to, so that putting it back
//! shows it to the same roles. The roles stay in `layout_field_role` as they are for a field on the
//! form; this table only says that the field is off the form, and which layout it is off.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::schema::{LayoutOffFieldRow, LayoutRow};

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Table(
        Table::create()
            .table(LayoutOffFieldRow::Table)
            .col(id_column(LayoutOffFieldRow::Layout))
            .col(ColumnDef::new(LayoutOffFieldRow::Field).text().not_null())
            .primary_key(
                Index::create()
                    .col(LayoutOffFieldRow::Layout)
                    .col(LayoutOffFieldRow::Field),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(LayoutOffFieldRow::Table, LayoutOffFieldRow::Layout)
                    .to(LayoutRow::Table, LayoutRow::Id),
            )
            .to_owned(),
    )]
}
