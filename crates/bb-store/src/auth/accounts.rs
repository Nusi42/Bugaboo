//! People, their accounts and their passwords.

use bb_app::auth::{Account, AccountStore, PeoplePosition, SearchIn};
use bb_core::id::UserId;
use bb_core::names::name_key;
use bb_core::person::Person;
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, Order, Query};

use crate::db::{StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::paging::after_last;
use crate::schema::{PasswordLogin, PersonRow};

use super::AuthStorage;
use crate::db::Row;

pub(crate) fn person_columns() -> [PersonRow; 4] {
    [
        PersonRow::Id,
        PersonRow::DisplayName,
        PersonRow::Email,
        PersonRow::CreatedAt,
    ]
}

pub(crate) fn read_person(row: &Row) -> Result<Person, StoreError> {
    Ok(Person {
        id: row.id::<UserId>(PersonRow::Id)?,
        display_name: row.get(PersonRow::DisplayName)?,
        email: row.get(PersonRow::Email)?,
        created_at: row.time(PersonRow::CreatedAt)?,
    })
}

impl AuthStorage {
    pub(super) async fn person_by_id(&self, id: UserId) -> Result<Option<Person>, StoreError> {
        let mut select = Query::select();
        select
            .columns(person_columns())
            .from(PersonRow::Table)
            .and_where(Expr::col(PersonRow::Id).eq(id.stored()));
        self.database
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_person)
            .transpose()
    }
}

impl AccountStore for AuthStorage {
    async fn account_by_email(&self, email: &str) -> Result<Option<Account>, StoreError> {
        let mut select = Query::select();
        select
            .columns(person_columns().map(|column| (PersonRow::Table, column)))
            .column((PasswordLogin::Table, PasswordLogin::Hash))
            .from(PersonRow::Table)
            .left_join(
                PasswordLogin::Table,
                Expr::col((PersonRow::Table, PersonRow::Id))
                    .equals((PasswordLogin::Table, PasswordLogin::Person)),
            )
            .and_where(Expr::col((PersonRow::Table, PersonRow::Email)).eq(email));
        let Some(row) = self.database.fetch_optional(&select).await? else {
            return Ok(None);
        };
        Ok(Some(Account {
            person: read_person(&row)?,
            password_hash: row.get::<Option<String>>(PasswordLogin::Hash)?,
        }))
    }

    async fn person(&self, id: UserId) -> Result<Option<Person>, StoreError> {
        self.person_by_id(id).await
    }

    async fn people_of(&self, ids: &[UserId]) -> Result<Vec<Person>, StoreError> {
        let mut found = Vec::new();
        for chunk in ids.chunks(crate::db::BIND_LIMIT) {
            let mut select = Query::select();
            select
                .columns(person_columns())
                .from(PersonRow::Table)
                .and_where(Expr::col(PersonRow::Id).is_in(chunk.iter().map(|id| id.stored())));
            for row in &self.database.fetch_all(&select).await? {
                found.push(read_person(row)?);
            }
        }
        Ok(found)
    }

    async fn people_page(
        &self,
        search: Option<&str>,
        look_at: SearchIn,
        after: Option<&PeoplePosition>,
        limit: u64,
    ) -> Result<Vec<Person>, StoreError> {
        // By the stored name key (capitals and repeated spaces do not count, accents do), so the
        // search, the order and the position agree with each other on both databases.
        let name = || Expr::col(PersonRow::NameKey);
        let mut select = Query::select();
        select
            .columns(person_columns())
            .from(PersonRow::Table)
            .order_by(PersonRow::NameKey, Order::Asc)
            .order_by(PersonRow::Id, Order::Asc)
            .limit(limit);
        if look_at == SearchIn::NameStarts {
            // The start as typed, by the one rule that made the keys.
            let start = name_key(search.unwrap_or_default());
            select
                .and_where(Expr::col(PersonRow::NameKey).like(crate::like::starting_with(&start)));
        } else {
            // Every word of the search is held by the name (or, if asked for, by the address).
            for word in search.unwrap_or_default().split_whitespace() {
                let mut any = Cond::any().add(
                    Expr::col(PersonRow::NameKey).like(crate::like::containing(&name_key(word))),
                );
                if look_at == SearchIn::NamesAndAddresses {
                    any = any.add(crate::like::containing_any_case(
                        Expr::col(PersonRow::Email),
                        word,
                    ));
                }
                select.cond_where(any);
            }
        }
        if let Some(position) = after {
            select.cond_where(after_last(
                name,
                || Expr::val(name_key(&position.name)),
                &Order::Asc,
                PersonRow::Id,
                position.id.stored(),
            ));
        }
        self.database
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_person)
            .collect()
    }

    async fn insert_person(
        transaction: &mut Transaction,
        person: &Person,
        password_hash: &str,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(PersonRow::Table)
            .columns(person_columns().into_iter().chain([PersonRow::NameKey]))
            .values([
                Expr::val(person.id.stored()),
                Expr::val(person.display_name.as_str()),
                Expr::val(person.email.as_str()),
                Expr::val(to_micros(person.created_at)),
                Expr::val(name_key(&person.display_name)),
            ])
            .map_err(|_| StoreError::Corrupt("a person"))?;
        transaction.execute(&insert).await?;
        let mut login = Query::insert();
        login
            .into_table(PasswordLogin::Table)
            .columns([
                PasswordLogin::Person,
                PasswordLogin::Hash,
                PasswordLogin::ChangedAt,
            ])
            .values([
                Expr::val(person.id.stored()),
                Expr::val(password_hash),
                Expr::val(to_micros(at)),
            ])
            .map_err(|_| StoreError::Corrupt("a password"))?;
        transaction.execute(&login).await.map(|_| ())
    }

    async fn set_password(
        transaction: &mut Transaction,
        person: UserId,
        password_hash: &str,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(PasswordLogin::Table)
            .value(PasswordLogin::Hash, password_hash)
            .value(PasswordLogin::ChangedAt, to_micros(at))
            .and_where(Expr::col(PasswordLogin::Person).eq(person.stored()));
        if transaction.execute(&update).await? > 0 {
            return Ok(());
        }
        // A person who signed in without a password so far (a passkey, company sign-in) gets one.
        let mut login = Query::insert();
        login
            .into_table(PasswordLogin::Table)
            .columns([
                PasswordLogin::Person,
                PasswordLogin::Hash,
                PasswordLogin::ChangedAt,
            ])
            .values([
                Expr::val(person.stored()),
                Expr::val(password_hash),
                Expr::val(to_micros(at)),
            ])
            .map_err(|_| StoreError::Corrupt("a password"))?;
        transaction.execute(&login).await.map(|_| ())
    }
}

/// People per transaction when the name keys are filled.
const KEYS_PER_STEP: u64 = 500;

impl crate::db::Database {
    /// Gives the people whose name key is still empty (written before migration 26) the key of
    /// their name, a page and a transaction at a time. Returns how many it filled. It changes
    /// nothing once every person has a key, and two processes that fill at once write the same.
    ///
    /// # Errors
    /// When the database fails.
    pub(crate) async fn fill_name_keys(&self) -> Result<u64, StoreError> {
        let mut filled = 0_u64;
        let mut after: Option<Vec<u8>> = None;
        loop {
            let mut transaction = self.begin().await?;
            let mut select = Query::select();
            select
                .columns([PersonRow::Id, PersonRow::DisplayName])
                .from(PersonRow::Table)
                .and_where(Expr::col(PersonRow::NameKey).eq(""))
                .order_by(PersonRow::Id, Order::Asc)
                .limit(KEYS_PER_STEP);
            if let Some(after) = &after {
                select.and_where(Expr::col(PersonRow::Id).gt(after.clone()));
            }
            let rows = transaction.fetch_all(&select).await?;
            for row in &rows {
                let id: Vec<u8> = row.get(PersonRow::Id)?;
                let name: String = row.get(PersonRow::DisplayName)?;
                let mut update = Query::update();
                update
                    .table(PersonRow::Table)
                    .value(PersonRow::NameKey, name_key(&name))
                    .and_where(Expr::col(PersonRow::Id).eq(id.clone()));
                transaction.execute(&update).await?;
                after = Some(id);
            }
            transaction.commit().await?;
            filled = filled.saturating_add(u64::try_from(rows.len()).unwrap_or(u64::MAX));
            if rows.len() < usize::try_from(KEYS_PER_STEP).unwrap_or(usize::MAX) {
                return Ok(filled);
            }
        }
    }
}
