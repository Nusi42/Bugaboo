//! People, their accounts and their passwords.

use bb_app::auth::{Account, AccountStore, PeoplePosition, SearchIn};
use bb_core::id::UserId;
use bb_core::names::squeezed;
use bb_core::person::Person;
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, Func, Order, Query};

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
        // Without regard to capitals, in the database itself, so the order and the position agree
        // with each other whatever the database does with capitals and accents.
        let name = || Func::lower(Expr::col(PersonRow::DisplayName));
        let mut select = Query::select();
        select
            .columns(person_columns())
            .from(PersonRow::Table)
            .order_by_expr(name().into(), Order::Asc)
            .order_by(PersonRow::Id, Order::Asc)
            .limit(limit);
        if look_at == SearchIn::NameStarts {
            // The start as typed, its spaces squeezed; the database folds the capitals.
            let start = squeezed(search.unwrap_or_default());
            select.and_where(crate::like::starting_with_any_case(
                Expr::col(PersonRow::DisplayName),
                &start,
            ));
        } else {
            // Every word of the search is held by the name (or, if asked for, by the address).
            for word in search.unwrap_or_default().split_whitespace() {
                let mut any = Cond::any().add(crate::like::containing_any_case(
                    Expr::col(PersonRow::DisplayName),
                    word,
                ));
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
                || Expr::expr(name()),
                || Expr::expr(Func::lower(Expr::val(position.name.clone()))),
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
            .columns(person_columns())
            .values([
                Expr::val(person.id.stored()),
                Expr::val(person.display_name.as_str()),
                Expr::val(person.email.as_str()),
                Expr::val(to_micros(person.created_at)),
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
