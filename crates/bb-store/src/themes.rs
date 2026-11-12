//! The adapter of the own-theme port (`bb_app::themes::ThemeStore`): themes for everyone and
//! personal ones, with their values (THM-04).

use bb_app::themes::ThemeStore;
use bb_core::id::{ThemeId, UserId};
use bb_core::technical::TechnicalName;
use bb_core::theme::{OwnTheme, OwnValues, ThemeKind, ThemeName};
use bb_core::version::Version;
use sea_query::{Cond, Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::locks::{LockClass, lock_key};
use crate::schema::OwnThemeRow;

fn select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            OwnThemeRow::Id,
            OwnThemeRow::Kind,
            OwnThemeRow::Maker,
            OwnThemeRow::Name,
            OwnThemeRow::Extends,
            OwnThemeRow::Values,
            OwnThemeRow::SavedAt,
            OwnThemeRow::CreatedAt,
            OwnThemeRow::UpdatedAt,
            OwnThemeRow::Version,
        ])
        .from(OwnThemeRow::Table);
    select
}

fn read_values(text: &str) -> Result<OwnValues, StoreError> {
    serde_json::from_str(text).map_err(|_| StoreError::Corrupt("the values of a theme"))
}

fn write_values(values: &OwnValues) -> Result<String, StoreError> {
    serde_json::to_string(values).map_err(|_| StoreError::Corrupt("the values of a theme"))
}

fn read_theme(row: &Row) -> Result<OwnTheme, StoreError> {
    Ok(OwnTheme {
        id: row.id::<ThemeId>(OwnThemeRow::Id)?,
        kind: ThemeKind::from_code(&row.get::<String>(OwnThemeRow::Kind)?)
            .ok_or(StoreError::Corrupt("a kind of theme"))?,
        maker: row.id::<UserId>(OwnThemeRow::Maker)?,
        name: ThemeName::new(&row.get::<String>(OwnThemeRow::Name)?)
            .map_err(|_| StoreError::Corrupt("the name of a theme"))?,
        extends: TechnicalName::new(&row.get::<String>(OwnThemeRow::Extends)?)
            .map_err(|_| StoreError::Corrupt("the built-in theme of a theme"))?,
        values: read_values(&row.get::<String>(OwnThemeRow::Values)?)?,
        saved_at: row.optional_time(OwnThemeRow::SavedAt)?,
        created_at: row.time(OwnThemeRow::CreatedAt)?,
        updated_at: row.time(OwnThemeRow::UpdatedAt)?,
        version: row.get(OwnThemeRow::Version)?,
    })
}

fn count_where(kind: ThemeKind, maker: Option<UserId>) -> SelectStatement {
    let mut select = Query::select();
    select
        .expr_as(
            Expr::col(OwnThemeRow::Id).count(),
            crate::schema::Computed::Total,
        )
        .from(OwnThemeRow::Table)
        .and_where(Expr::col(OwnThemeRow::Kind).eq(kind.code()));
    if let Some(maker) = maker {
        select.and_where(Expr::col(OwnThemeRow::Maker).eq(maker.stored()));
    }
    select
}

async fn count(transaction: &mut Transaction, select: &SelectStatement) -> Result<u64, StoreError> {
    let total: i64 = match transaction.fetch_optional(select).await? {
        Some(row) => row.get(crate::schema::Computed::Total)?,
        None => 0,
    };
    u64::try_from(total).map_err(|_| StoreError::Corrupt("a count"))
}

impl ThemeStore for AuthStorage {
    async fn own_theme_in(
        transaction: &mut Transaction,
        id: ThemeId,
    ) -> Result<Option<OwnTheme>, StoreError> {
        let mut select = select();
        select.and_where(Expr::col(OwnThemeRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_theme)
            .transpose()
    }

    async fn count_themes_for_everyone_in(
        transaction: &mut Transaction,
    ) -> Result<u64, StoreError> {
        count(transaction, &count_where(ThemeKind::ForEveryone, None)).await
    }

    async fn count_personal_themes_of_in(
        transaction: &mut Transaction,
        maker: UserId,
    ) -> Result<u64, StoreError> {
        count(transaction, &count_where(ThemeKind::Personal, Some(maker))).await
    }

    async fn themes_listed_for(
        &self,
        person: UserId,
        unsaved_for_everyone: bool,
    ) -> Result<Vec<OwnTheme>, StoreError> {
        let mut for_everyone =
            Cond::all().add(Expr::col(OwnThemeRow::Kind).eq(ThemeKind::ForEveryone.code()));
        if !unsaved_for_everyone {
            for_everyone = for_everyone.add(Expr::col(OwnThemeRow::SavedAt).is_not_null());
        }
        let mut statement = select();
        statement
            .cond_where(
                Cond::any()
                    .add(Expr::col(OwnThemeRow::Maker).eq(person.stored()))
                    .add(for_everyone),
            )
            .order_by(OwnThemeRow::Name, Order::Asc)
            .order_by(OwnThemeRow::Id, Order::Asc);
        self.database()
            .fetch_all(&statement)
            .await?
            .iter()
            .map(read_theme)
            .collect()
    }

    async fn lock_themes_of(
        transaction: &mut Transaction,
        kind: ThemeKind,
        maker: UserId,
    ) -> Result<(), StoreError> {
        // The themes for everyone are one set; the personal ones are a set for each person.
        let key = match kind {
            ThemeKind::ForEveryone => 0,
            ThemeKind::Personal => lock_key(maker.to_bytes()),
        };
        transaction.advisory_lock(LockClass::Themes, key).await
    }

    async fn insert_own_theme(
        transaction: &mut Transaction,
        theme: &OwnTheme,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(OwnThemeRow::Table)
            .columns([
                OwnThemeRow::Id,
                OwnThemeRow::Kind,
                OwnThemeRow::Maker,
                OwnThemeRow::Name,
                OwnThemeRow::Extends,
                OwnThemeRow::Values,
                OwnThemeRow::SavedAt,
                OwnThemeRow::CreatedAt,
                OwnThemeRow::UpdatedAt,
                OwnThemeRow::Version,
            ])
            .values([
                Expr::val(theme.id.stored()),
                Expr::val(theme.kind.code()),
                Expr::val(theme.maker.stored()),
                Expr::val(theme.name.as_str()),
                Expr::val(theme.extends.as_str()),
                Expr::val(write_values(&theme.values)?),
                Expr::val(theme.saved_at.map(to_micros)),
                Expr::val(to_micros(theme.created_at)),
                Expr::val(to_micros(theme.updated_at)),
                Expr::val(theme.version),
            ])
            .map_err(|_| StoreError::Corrupt("a theme"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn save_own_theme(
        transaction: &mut Transaction,
        theme: &OwnTheme,
        version: Version,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(OwnThemeRow::Table)
            .value(OwnThemeRow::Name, theme.name.as_str())
            .value(OwnThemeRow::Values, write_values(&theme.values)?)
            .value(
                OwnThemeRow::SavedAt,
                Expr::val(theme.saved_at.map(to_micros)),
            )
            .value(OwnThemeRow::UpdatedAt, to_micros(theme.updated_at))
            .value(OwnThemeRow::Version, Expr::col(OwnThemeRow::Version).add(1))
            .and_where(Expr::col(OwnThemeRow::Id).eq(theme.id.stored()))
            .and_where(Expr::col(OwnThemeRow::Version).eq(version.get()));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn delete_own_theme(
        transaction: &mut Transaction,
        id: ThemeId,
        version: Version,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(OwnThemeRow::Table)
            .and_where(Expr::col(OwnThemeRow::Id).eq(id.stored()))
            .and_where(Expr::col(OwnThemeRow::Version).eq(version.get()));
        Ok(transaction.execute(&delete).await? > 0)
    }
}

#[cfg(test)]
#[path = "tests/themes.rs"]
mod tests;
