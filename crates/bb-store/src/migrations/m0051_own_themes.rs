//! Migration 51: own themes (THM-04, ADR-0026 update M8).
//!
//! One row for each theme of an organization or of a person: its name, whether it is for everyone
//! or personal, the person who made it, the built-in theme it extends and the values it changes (as
//! JSON text); the unsaved values of the editor are its editing draft (ADR-0019). A theme that has
//! never been saved has no `saved_at` and cannot be chosen. Nothing is written into the table. The maker has no reference to the
//! person (a removed person leaves a tombstone, PRIV-04).

use sea_query::{ColumnDef, Index, Table};

use super::{Step, id_column};
use crate::schema::OwnThemeRow;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(OwnThemeRow::Table)
                .col(
                    ColumnDef::new(OwnThemeRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(OwnThemeRow::Kind).text().not_null())
                .col(id_column(OwnThemeRow::Maker))
                .col(ColumnDef::new(OwnThemeRow::Name).text().not_null())
                .col(ColumnDef::new(OwnThemeRow::Extends).text().not_null())
                .col(ColumnDef::new(OwnThemeRow::Values).text().not_null())
                .col(ColumnDef::new(OwnThemeRow::SavedAt).big_integer())
                .col(
                    ColumnDef::new(OwnThemeRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(OwnThemeRow::UpdatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(OwnThemeRow::Version)
                        .big_integer()
                        .not_null(),
                )
                .to_owned(),
        ),
        // The themes a person may use are read by kind and maker.
        Step::Index(
            Index::create()
                .name("own_theme_by_maker")
                .table(OwnThemeRow::Table)
                .col(OwnThemeRow::Kind)
                .col(OwnThemeRow::Maker)
                .to_owned(),
        ),
    ]
}
