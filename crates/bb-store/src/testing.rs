//! Test hooks that must not be in the program: a database as M1 left it. Only the `testing`
//! feature compiles this, and only `[dev-dependencies]` switch the feature on (M1-F56).

use bb_core::id::UserId;
use jiff::Timestamp;
use sea_query::{Expr, IntoIden as _, Query};

use crate::db::{Database, StoredId, to_micros};
use crate::error::StoreError;
use crate::schema::{PasswordLogin, PersonRow, Retired};

/// A person as the schema of M1 stored them: with the administrator flag.
pub struct M1Person<'a> {
    /// The ID of the person.
    pub id: UserId,
    /// The name.
    pub display_name: &'a str,
    /// The lower-case e-mail address.
    pub email: &'a str,
    /// The password hash they sign in with.
    pub password_hash: &'a str,
    /// Whether M1 made them an administrator.
    pub administrator: bool,
}

impl Database {
    /// Stores people as migration 2 (M1) knew them. The database must be at migration 2 exactly
    /// (`migrate_up_to`): later it has no administrator flag to set.
    ///
    /// # Errors
    /// Fails when the database fails, for example because migration 3 was applied already.
    pub async fn seed_m1_people(
        &self,
        people: &[M1Person<'_>],
        now: Timestamp,
    ) -> Result<(), StoreError> {
        for person in people {
            let mut insert = Query::insert();
            insert
                .into_table(PersonRow::Table)
                .columns([
                    PersonRow::Id.into_iden(),
                    PersonRow::DisplayName.into_iden(),
                    PersonRow::Email.into_iden(),
                    PersonRow::CreatedAt.into_iden(),
                    Retired::Administrator.into_iden(),
                ])
                .values([
                    Expr::val(person.id.stored()),
                    Expr::val(person.display_name),
                    Expr::val(person.email),
                    Expr::val(to_micros(now)),
                    Expr::val(person.administrator),
                ])
                .map_err(|_| StoreError::Corrupt("a person"))?;
            self.execute(&insert).await?;
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
                    Expr::val(person.password_hash),
                    Expr::val(to_micros(now)),
                ])
                .map_err(|_| StoreError::Corrupt("a password"))?;
            self.execute(&login).await?;
        }
        Ok(())
    }
}
