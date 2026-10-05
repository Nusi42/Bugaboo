//! Migration 4: settings in layers (CFG-04, CFG-06, THM-02, ADR-0018).
//!
//! The values people chose for themselves get their own table, `person_setting`, so the person
//! they belong to is a column with a personal-data mark (it sat in `scope` before, which also holds
//! project IDs and is marked as not personal). The one setting `appearance.theme` is replaced by
//! the mode and the light and the dark theme: its stored values are carried over, for people and
//! for the organization (with the lock), so nobody's choice changes.
//!
//! | `appearance.theme` was | `appearance.mode` | `appearance.light-theme` | `appearance.dark-theme` |
//! |------------------------|-------------------|--------------------------|-------------------------|
//! | `device`               | `device`          | –                        | –                       |
//! | `daylight`, `paper`    | `light`           | the same                 | –                       |
//! | `dusk`, `midnight`     | `dark`            | –                        | the same                |

use sea_query::{ColumnDef, Expr, ExprTrait as _, ForeignKey, Query, SelectStatement, Table};

use super::Step;
use crate::schema::{PersonRow, PersonSettingRow, SettingRow};

/// The setting that the three below replace.
const OLD_THEME: &str = "appearance.theme";
const MODE: &str = "appearance.mode";
const LIGHT_THEME: &str = "appearance.light-theme";
const DARK_THEME: &str = "appearance.dark-theme";

/// The levels as they are stored in `setting.level`.
const ORGANIZATION: &str = "organization";
const PERSON: &str = "person";

/// What `appearance.theme` held, as the JSON text that is stored, with what it becomes.
const LIGHT_THEMES: [&str; 2] = ["\"daylight\"", "\"paper\""];
const DARK_THEMES: [&str; 2] = ["\"dusk\"", "\"midnight\""];
const FOLLOW_DEVICE: &str = "\"device\"";
const MODE_LIGHT: &str = "\"light\"";
const MODE_DARK: &str = "\"dark\"";

pub(super) fn steps() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut steps = vec![person_setting_table()];
    steps.extend(carry_over_people()?);
    steps.extend(carry_over_organization()?);
    steps.extend(forget_the_old_rows());
    Ok(steps)
}

fn person_setting_table() -> Step {
    Step::Table(
        Table::create()
            .table(PersonSettingRow::Table)
            .col(ColumnDef::new(PersonSettingRow::Person).blob().not_null())
            .col(ColumnDef::new(PersonSettingRow::Key).text().not_null())
            .col(ColumnDef::new(PersonSettingRow::Value).text().not_null())
            .col(
                ColumnDef::new(PersonSettingRow::UpdatedAt)
                    .big_integer()
                    .not_null(),
            )
            .primary_key(
                sea_query::Index::create()
                    .col(PersonSettingRow::Person)
                    .col(PersonSettingRow::Key),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(PersonSettingRow::Table, PersonSettingRow::Person)
                    .to(PersonRow::Table, PersonRow::Id),
            )
            .to_owned(),
    )
}

/// The old person-level rows of one level, from people who exist.
fn old_person_rows(values: &[&str]) -> SelectStatement {
    let mut people = Query::select();
    people.column(PersonRow::Id).from(PersonRow::Table);
    let mut select = Query::select();
    select
        .from(SettingRow::Table)
        .and_where(Expr::col(SettingRow::Level).eq(PERSON))
        .and_where(Expr::col(SettingRow::Scope).in_subquery(people));
    if !values.is_empty() {
        select.and_where(Expr::col(SettingRow::Key).eq(OLD_THEME));
        select.and_where(Expr::col(SettingRow::Value).is_in(values.iter().copied()));
    }
    select
}

/// A person's row `key = value`, made from the old rows in `select`.
fn person_row(
    mut select: SelectStatement,
    key: &str,
    value: &str,
) -> Result<Step, sea_query::error::Error> {
    select
        .column(SettingRow::Scope)
        .expr(Expr::val(key))
        .expr(Expr::val(value))
        .column(SettingRow::UpdatedAt);
    let mut insert = Query::insert();
    insert
        .into_table(PersonSettingRow::Table)
        .columns([
            PersonSettingRow::Person,
            PersonSettingRow::Key,
            PersonSettingRow::Value,
            PersonSettingRow::UpdatedAt,
        ])
        .select_from(select)?;
    Ok(Step::Seed(insert))
}

fn carry_over_people() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut steps = vec![
        person_row(old_person_rows(&[FOLLOW_DEVICE]), MODE, FOLLOW_DEVICE)?,
        person_row(old_person_rows(&LIGHT_THEMES), MODE, MODE_LIGHT)?,
        person_row(old_person_rows(&DARK_THEMES), MODE, MODE_DARK)?,
    ];
    // The light and the dark theme keep the value they had (it is the column `value` itself).
    for (themes, key) in [(LIGHT_THEMES, LIGHT_THEME), (DARK_THEMES, DARK_THEME)] {
        let mut select = old_person_rows(&themes);
        select
            .column(SettingRow::Scope)
            .expr(Expr::val(key))
            .column(SettingRow::Value)
            .column(SettingRow::UpdatedAt);
        let mut insert = Query::insert();
        insert
            .into_table(PersonSettingRow::Table)
            .columns([
                PersonSettingRow::Person,
                PersonSettingRow::Key,
                PersonSettingRow::Value,
                PersonSettingRow::UpdatedAt,
            ])
            .select_from(select)?;
        steps.push(Step::Seed(insert));
    }
    // Every other value a person set stays as it is.
    let mut others = old_person_rows(&[]);
    others
        .and_where(Expr::col(SettingRow::Key).ne(OLD_THEME))
        .column(SettingRow::Scope)
        .column(SettingRow::Key)
        .column(SettingRow::Value)
        .column(SettingRow::UpdatedAt);
    let mut insert = Query::insert();
    insert
        .into_table(PersonSettingRow::Table)
        .columns([
            PersonSettingRow::Person,
            PersonSettingRow::Key,
            PersonSettingRow::Value,
            PersonSettingRow::UpdatedAt,
        ])
        .select_from(others)?;
    steps.push(Step::Seed(insert));
    Ok(steps)
}

/// The organization's rows of the old setting, with the lock they had.
fn old_organization_rows(values: &[&str]) -> SelectStatement {
    let mut select = Query::select();
    select
        .from(SettingRow::Table)
        .and_where(Expr::col(SettingRow::Key).eq(OLD_THEME))
        .and_where(Expr::col(SettingRow::Level).eq(ORGANIZATION))
        .and_where(Expr::col(SettingRow::Value).is_in(values.iter().copied()));
    select
}

fn organization_row(
    mut select: SelectStatement,
    key: &str,
    value: Option<&str>,
) -> Result<Step, sea_query::error::Error> {
    select
        .expr(Expr::val(key))
        .column(SettingRow::Level)
        .column(SettingRow::Scope);
    match value {
        Some(value) => select.expr(Expr::val(value)),
        None => select.column(SettingRow::Value),
    };
    select
        .column(SettingRow::Locked)
        .column(SettingRow::UpdatedAt);
    let mut insert = Query::insert();
    insert
        .into_table(SettingRow::Table)
        .columns([
            SettingRow::Key,
            SettingRow::Level,
            SettingRow::Scope,
            SettingRow::Value,
            SettingRow::Locked,
            SettingRow::UpdatedAt,
        ])
        .select_from(select)?;
    Ok(Step::Seed(insert))
}

fn carry_over_organization() -> Result<Vec<Step>, sea_query::error::Error> {
    Ok(vec![
        organization_row(
            old_organization_rows(&[FOLLOW_DEVICE]),
            MODE,
            Some(FOLLOW_DEVICE),
        )?,
        organization_row(old_organization_rows(&LIGHT_THEMES), MODE, Some(MODE_LIGHT))?,
        organization_row(old_organization_rows(&DARK_THEMES), MODE, Some(MODE_DARK))?,
        organization_row(old_organization_rows(&LIGHT_THEMES), LIGHT_THEME, None)?,
        organization_row(old_organization_rows(&DARK_THEMES), DARK_THEME, None)?,
    ])
}

/// What was carried over (and whatever could not be: a person who is gone) is deleted from the
/// old places: the table of the organization keeps only organization and project rows.
fn forget_the_old_rows() -> Vec<Step> {
    let mut people = Query::delete();
    people
        .from_table(SettingRow::Table)
        .and_where(Expr::col(SettingRow::Level).eq(PERSON));
    let mut old_theme = Query::delete();
    old_theme
        .from_table(SettingRow::Table)
        .and_where(Expr::col(SettingRow::Key).eq(OLD_THEME));
    vec![Step::Delete(people), Step::Delete(old_theme)]
}
