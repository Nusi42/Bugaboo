//! The adapter of the sign-in port (`bb_app::auth::AuthStore`): people, passwords, sessions,
//! second steps, passkeys, invitations and the setup code.
//!
//! Writes are associated functions that run inside the transaction of a change
//! ([`Database::record_change`]), so a change and its event are committed together. The few
//! reads and writes that are not domain changes use the pool directly: marking a session as used,
//! the encrypted code app secret and the security log.

use std::sync::Arc;

use bb_app::auth::{
    AccessKeyRecord, Account, AuthSettings, AuthStore, PeoplePosition, ProjectChoice,
    SecurityEvent, SessionRecord,
};
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::event::DomainEvent;
use bb_core::id::{AccessKeyId, InvitationId, PasskeyId, RoleId, SessionId, UserId};
use bb_core::network::NetworkPrefix;
use bb_core::person::Person;
use bb_core::security_log::Digest;
use bb_core::settings::{
    AUTH_ACCESS_KEY_DEFAULT_DAYS, AUTH_ACCESS_KEY_LONGEST_DAYS, AUTH_SIGN_UP_DOMAINS,
    AUTH_SIGN_UP_ENABLED, SECURITY_SESSION_ABSOLUTE_HOURS, SECURITY_SESSION_IDLE_MINUTES,
    SettingValue,
};
use bb_core::sign_in::{
    Invitation, SessionStage, SetupCode, SignInSession, StoredPasskey, TwoStep,
};
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{BinOper, Cond, Expr, ExprTrait as _, Func, Order, Query};
use zeroize::Zeroizing;

use crate::anchor::LogSeal;
use crate::db::{Database, Row, StoredId, Transaction, to_micros};
use crate::error::{SecretProblem, StoreError};
use crate::keys::KeyRing;
use crate::schema::{
    BackupCode, Computed, Invitation as InvitationColumns, PasskeyRow, PasswordLogin, PersonRow,
    SetupCodeRow, SignInSession as SessionColumns, TwoStep as TwoStepColumns,
};
use crate::security_log::NewSecurityLogEntry;
use crate::settings::{Scopes, put_organization_setting, resolved};

/// The prefix of the name of a code app secret in the secrets table.
const TOTP_SECRET_PREFIX: &str = "totp.";

/// The sign-in storage: the database, the keys that protect secrets in it, and the seal of the
/// security log.
#[derive(Clone, Debug)]
pub struct AuthStorage {
    database: Database,
    keys: Arc<KeyRing>,
    seal: Arc<LogSeal>,
}

impl AuthStorage {
    /// The storage on `database`.
    #[must_use]
    pub fn new(database: Database, keys: Arc<KeyRing>, seal: LogSeal) -> Self {
        Self {
            database,
            keys,
            seal: Arc::new(seal),
        }
    }

    /// The database underneath.
    #[must_use]
    pub const fn database(&self) -> &Database {
        &self.database
    }
}

impl ChangeStore for AuthStorage {
    type Transaction = Transaction;
    type Error = StoreError;

    fn record_change<'a, T: Send + 'a>(
        &'a self,
        event: &'a DomainEvent,
        change: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>>
        + Send
        + 'a,
    ) -> impl Future<Output = Result<T, StoreError>> + Send + 'a {
        ChangeStore::record_change(&self.database, event, change)
    }

    fn read_snapshot<'a, T: Send + 'a>(
        &'a self,
        read: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>>
        + Send
        + 'a,
    ) -> impl Future<Output = Result<T, StoreError>> + Send + 'a {
        ChangeStore::read_snapshot(&self.database, read)
    }
}

fn digest(bytes: &[u8]) -> Result<Digest, StoreError> {
    Digest::from_slice(bytes).ok_or(StoreError::Corrupt("a hash"))
}

fn read_person(row: &Row) -> Result<Person, StoreError> {
    Ok(Person {
        id: row.id::<UserId>(PersonRow::Id)?,
        display_name: row.get(PersonRow::DisplayName)?,
        email: row.get(PersonRow::Email)?,
        created_at: row.time(PersonRow::CreatedAt)?,
    })
}

fn person_columns() -> [PersonRow; 4] {
    [
        PersonRow::Id,
        PersonRow::DisplayName,
        PersonRow::Email,
        PersonRow::CreatedAt,
    ]
}

fn session_columns() -> [SessionColumns; 8] {
    [
        SessionColumns::Id,
        SessionColumns::Person,
        SessionColumns::TokenHash,
        SessionColumns::Stage,
        SessionColumns::CreatedAt,
        SessionColumns::LastSeenAt,
        SessionColumns::Network,
        SessionColumns::Device,
    ]
}

fn read_session(row: &Row) -> Result<SignInSession, StoreError> {
    Ok(SignInSession {
        id: row.id::<SessionId>(SessionColumns::Id)?,
        person: row.id::<UserId>(SessionColumns::Person)?,
        token_hash: digest(&row.get::<Vec<u8>>(SessionColumns::TokenHash)?)?,
        stage: SessionStage::from_code(&row.get::<String>(SessionColumns::Stage)?)
            .ok_or(StoreError::Corrupt("a session stage"))?,
        created_at: row.time(SessionColumns::CreatedAt)?,
        last_seen_at: row.time(SessionColumns::LastSeenAt)?,
        network: row
            .get::<Option<String>>(SessionColumns::Network)?
            .map(|text| NetworkPrefix::from_stored(&text).ok_or(StoreError::Corrupt("a network")))
            .transpose()?,
        device: row.get(SessionColumns::Device)?,
    })
}

fn passkey_columns() -> [PasskeyRow; 7] {
    [
        PasskeyRow::Id,
        PasskeyRow::Person,
        PasskeyRow::CredentialId,
        PasskeyRow::Data,
        PasskeyRow::Label,
        PasskeyRow::CreatedAt,
        PasskeyRow::LastUsedAt,
    ]
}

fn read_passkey(row: &Row) -> Result<StoredPasskey, StoreError> {
    Ok(StoredPasskey {
        id: row.id::<PasskeyId>(PasskeyRow::Id)?,
        person: row.id::<UserId>(PasskeyRow::Person)?,
        credential_id: row.get(PasskeyRow::CredentialId)?,
        data: row.get(PasskeyRow::Data)?,
        label: row.get(PasskeyRow::Label)?,
        created_at: row.time(PasskeyRow::CreatedAt)?,
        last_used_at: row.optional_time(PasskeyRow::LastUsedAt)?,
    })
}

fn invitation_columns() -> [InvitationColumns; 7] {
    [
        InvitationColumns::Id,
        InvitationColumns::TokenHash,
        InvitationColumns::Email,
        InvitationColumns::InvitedBy,
        InvitationColumns::CreatedAt,
        InvitationColumns::ExpiresAt,
        InvitationColumns::UsedAt,
    ]
}

fn read_invitation(row: &Row) -> Result<Invitation, StoreError> {
    Ok(Invitation {
        id: row.id::<InvitationId>(InvitationColumns::Id)?,
        token_hash: digest(&row.get::<Vec<u8>>(InvitationColumns::TokenHash)?)?,
        email: row.get(InvitationColumns::Email)?,
        invited_by: row.id::<UserId>(InvitationColumns::InvitedBy)?,
        created_at: row.time(InvitationColumns::CreatedAt)?,
        expires_at: row.time(InvitationColumns::ExpiresAt)?,
        used_at: row.optional_time(InvitationColumns::UsedAt)?,
    })
}

/// The words of a search without the characters that `LIKE` takes for wildcards, so that a search
/// for `50%` finds names with `50` in them and never every name.
fn wildcards_removed(words: &str) -> String {
    words
        .trim()
        .chars()
        .filter(|character| !matches!(character, '%' | '_' | '\\'))
        .collect()
}

fn totp_name(person: UserId) -> Result<TechnicalName, StoreError> {
    TechnicalName::new(&format!("{TOTP_SECRET_PREFIX}{person}"))
        .map_err(|_| StoreError::Corrupt("a secret name"))
}

impl AuthStorage {
    async fn person_by_id(&self, id: UserId) -> Result<Option<Person>, StoreError> {
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

    async fn count(&self, select: &sea_query::SelectStatement) -> Result<i64, StoreError> {
        match self.database.fetch_optional(select).await? {
            Some(row) => row.get(Computed::Total),
            None => Ok(0),
        }
    }
}

impl AuthStore for AuthStorage {
    fn is_duplicate(error: &StoreError) -> bool {
        error.is_duplicate()
    }

    fn aborted() -> StoreError {
        StoreError::Aborted
    }

    fn is_aborted(error: &StoreError) -> bool {
        matches!(error, StoreError::Aborted)
    }

    fn refused(refusal: bb_policy::guards::Refusal) -> StoreError {
        StoreError::Refused(refusal)
    }

    fn as_refusal(error: &StoreError) -> Option<bb_policy::guards::Refusal> {
        match error {
            StoreError::Refused(refusal) => Some(*refusal),
            _ => None,
        }
    }

    fn describe(error: &StoreError) -> String {
        error.to_string()
    }

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

    async fn people_page(
        &self,
        search: Option<&str>,
        by_email: bool,
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
        // Every word of the search is held by the name (or, if asked for, by the address).
        let words = search.map(wildcards_removed).unwrap_or_default();
        for word in words.split_whitespace() {
            let pattern = format!("%{word}%");
            let mut any = Cond::any().add(
                Expr::expr(name()).binary(BinOper::Like, Func::lower(Expr::val(pattern.clone()))),
            );
            if by_email {
                any = any.add(
                    Expr::expr(Func::lower(Expr::col(PersonRow::Email)))
                        .binary(BinOper::Like, Func::lower(Expr::val(pattern))),
                );
            }
            select.cond_where(any);
        }
        if let Some(position) = after {
            let last = || Func::lower(Expr::val(position.name.clone()));
            select.cond_where(
                Cond::any().add(Expr::expr(name()).gt(last())).add(
                    Cond::all()
                        .add(Expr::expr(name()).eq(last()))
                        .add(Expr::col(PersonRow::Id).gt(position.id.stored())),
                ),
            );
        }
        self.database
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_person)
            .collect()
    }

    async fn session_by_hash(&self, hash: &Digest) -> Result<Option<SessionRecord>, StoreError> {
        let mut select = Query::select();
        select
            .columns(session_columns())
            .from(SessionColumns::Table)
            .and_where(Expr::col(SessionColumns::TokenHash).eq(hash.as_bytes().to_vec()));
        let Some(row) = self.database.fetch_optional(&select).await? else {
            return Ok(None);
        };
        let session = read_session(&row)?;
        let Some(person) = self.person_by_id(session.person).await? else {
            return Ok(None);
        };
        Ok(Some(SessionRecord { session, person }))
    }

    async fn sessions_of(&self, person: UserId) -> Result<Vec<SignInSession>, StoreError> {
        let mut select = Query::select();
        select
            .columns(session_columns())
            .from(SessionColumns::Table)
            .and_where(Expr::col(SessionColumns::Person).eq(person.stored()))
            .order_by(SessionColumns::CreatedAt, Order::Desc)
            .order_by(SessionColumns::Id, Order::Asc);
        self.database
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_session)
            .collect()
    }

    async fn setup_code(&self) -> Result<Option<SetupCode>, StoreError> {
        let mut select = Query::select();
        select
            .columns([SetupCodeRow::CodeHash, SetupCodeRow::CreatedAt])
            .from(SetupCodeRow::Table)
            .limit(1);
        let Some(row) = self.database.fetch_optional(&select).await? else {
            return Ok(None);
        };
        Ok(Some(SetupCode {
            code_hash: digest(&row.get::<Vec<u8>>(SetupCodeRow::CodeHash)?)?,
            created_at: row.time(SetupCodeRow::CreatedAt)?,
        }))
    }

    async fn two_step(&self, person: UserId) -> Result<Option<TwoStep>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                TwoStepColumns::Person,
                TwoStepColumns::Confirmed,
                TwoStepColumns::LastStep,
                TwoStepColumns::CreatedAt,
            ])
            .from(TwoStepColumns::Table)
            .and_where(Expr::col(TwoStepColumns::Person).eq(person.stored()));
        let Some(row) = self.database.fetch_optional(&select).await? else {
            return Ok(None);
        };
        Ok(Some(TwoStep {
            person,
            confirmed: row.get(TwoStepColumns::Confirmed)?,
            last_step: row.get(TwoStepColumns::LastStep)?,
            created_at: row.time(TwoStepColumns::CreatedAt)?,
        }))
    }

    async fn totp_secret(&self, person: UserId) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        match self
            .database
            .get_secret(&self.keys, &totp_name(person)?)
            .await
        {
            Ok(secret) => Ok(Some(secret)),
            Err(StoreError::Secret(SecretProblem::Missing)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn backup_codes_left(&self, person: UserId) -> Result<i64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(Expr::col(BackupCode::CodeHash).count(), Computed::Total)
            .from(BackupCode::Table)
            .and_where(Expr::col(BackupCode::Person).eq(person.stored()))
            .and_where(Expr::col(BackupCode::UsedAt).is_null());
        self.count(&select).await
    }

    async fn passkeys_of(&self, person: UserId) -> Result<Vec<StoredPasskey>, StoreError> {
        let mut select = Query::select();
        select
            .columns(passkey_columns())
            .from(PasskeyRow::Table)
            .and_where(Expr::col(PasskeyRow::Person).eq(person.stored()))
            .order_by(PasskeyRow::CreatedAt, Order::Asc)
            .order_by(PasskeyRow::Id, Order::Asc);
        self.database
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_passkey)
            .collect()
    }

    async fn invitation_by_hash(&self, hash: &Digest) -> Result<Option<Invitation>, StoreError> {
        let mut select = Query::select();
        select
            .columns(invitation_columns())
            .from(InvitationColumns::Table)
            .and_where(Expr::col(InvitationColumns::TokenHash).eq(hash.as_bytes().to_vec()));
        self.database
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_invitation)
            .transpose()
    }

    async fn open_invitations(
        &self,
        now: Timestamp,
        after: Option<(Timestamp, InvitationId)>,
        limit: u64,
    ) -> Result<Vec<Invitation>, StoreError> {
        let mut select = Query::select();
        select
            .columns(invitation_columns())
            .from(InvitationColumns::Table)
            .and_where(Expr::col(InvitationColumns::UsedAt).is_null())
            .and_where(Expr::col(InvitationColumns::ExpiresAt).gt(to_micros(now)))
            .order_by(InvitationColumns::CreatedAt, Order::Desc)
            .order_by(InvitationColumns::Id, Order::Asc)
            .limit(limit);
        if let Some((created, id)) = after {
            // Keyset paging: what comes after the last one in this order.
            let micros = to_micros(created);
            select.cond_where(
                Cond::any()
                    .add(Expr::col(InvitationColumns::CreatedAt).lt(micros))
                    .add(
                        Cond::all()
                            .add(Expr::col(InvitationColumns::CreatedAt).eq(micros))
                            .add(Expr::col(InvitationColumns::Id).gt(id.stored())),
                    ),
            );
        }
        self.database
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_invitation)
            .collect()
    }

    async fn auth_settings(&self) -> Result<AuthSettings, StoreError> {
        // One snapshot of the organization's values: every request that needs a session asks.
        let layers = self.database.setting_layers(&Scopes::default()).await?;
        Ok(AuthSettings {
            // A value that was stored before the bounds, or by hand, is cut to them.
            session_idle_minutes: SECURITY_SESSION_IDLE_MINUTES
                .definition()
                .clamp(resolved(&SECURITY_SESSION_IDLE_MINUTES, &layers)?),
            session_absolute_hours: SECURITY_SESSION_ABSOLUTE_HOURS
                .definition()
                .clamp(resolved(&SECURITY_SESSION_ABSOLUTE_HOURS, &layers)?),
            sign_up_enabled: resolved(&AUTH_SIGN_UP_ENABLED, &layers)?,
            sign_up_domains: AUTH_SIGN_UP_DOMAINS
                .definition()
                .cap_list(&resolved(&AUTH_SIGN_UP_DOMAINS, &layers)?),
            access_key_default_days: AUTH_ACCESS_KEY_DEFAULT_DAYS
                .definition()
                .clamp(resolved(&AUTH_ACCESS_KEY_DEFAULT_DAYS, &layers)?),
            access_key_longest_days: AUTH_ACCESS_KEY_LONGEST_DAYS
                .definition()
                .clamp(resolved(&AUTH_ACCESS_KEY_LONGEST_DAYS, &layers)?),
        })
    }

    async fn access_key_by_hash(
        &self,
        hash: &Digest,
    ) -> Result<Option<AccessKeyRecord>, StoreError> {
        self.database.access_key_by_hash(hash).await
    }

    async fn access_keys_of(
        &self,
        owner: UserId,
        after: Option<(Timestamp, AccessKeyId)>,
        limit: u64,
    ) -> Result<Vec<AccessKeyRecord>, StoreError> {
        self.database.access_keys_of(owner, after, limit).await
    }

    async fn access_key_count(&self, owner: UserId) -> Result<i64, StoreError> {
        self.database.access_key_count(owner).await
    }

    async fn projects_after(
        &self,
        after: Option<&str>,
        skip: u64,
        limit: u64,
    ) -> Result<Vec<ProjectChoice>, StoreError> {
        self.database.projects_after(after, skip, limit).await
    }

    async fn second_step_required(&self, person: UserId) -> Result<bool, StoreError> {
        self.database.second_step_required(person).await
    }

    async fn has_second_step(&self, person: UserId) -> Result<bool, StoreError> {
        self.database.has_second_step(person).await
    }

    async fn people_without_second_step(&self, role: RoleId) -> Result<Vec<UserId>, StoreError> {
        self.database.people_without_second_step(role).await
    }

    async fn touch_access_key(&self, id: AccessKeyId, at: Timestamp) -> Result<(), StoreError> {
        self.database.touch_access_key(id, at).await
    }

    async fn touch_session(&self, id: SessionId, at: Timestamp) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(SessionColumns::Table)
            .value(SessionColumns::LastSeenAt, to_micros(at))
            .and_where(Expr::col(SessionColumns::Id).eq(id.stored()))
            .and_where(Expr::col(SessionColumns::LastSeenAt).lt(to_micros(at)));
        self.database.execute(&update).await.map(|_| ())
    }

    async fn put_totp_secret(
        &self,
        person: UserId,
        secret: &[u8],
        at: Timestamp,
    ) -> Result<(), StoreError> {
        self.database
            .put_secret(&self.keys, &totp_name(person)?, secret, at)
            .await
    }

    async fn delete_totp_secret(&self, person: UserId) -> Result<(), StoreError> {
        self.database
            .delete_secret(&totp_name(person)?)
            .await
            .map(|_| ())
    }

    async fn append_security_log(&self, event: &SecurityEvent) -> Result<(), StoreError> {
        let entry = NewSecurityLogEntry {
            time: event.time,
            person: event.person,
            action: TechnicalName::new(event.action.code())
                .map_err(|_| StoreError::Corrupt("a security action"))?,
            subject: event.subject.clone(),
            network: event.network.clone(),
        };
        self.database
            .append_security_log(&self.seal, &entry)
            .await
            .map(|_| ())
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

    async fn insert_session(
        transaction: &mut Transaction,
        session: &SignInSession,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(SessionColumns::Table)
            .columns(session_columns())
            .values([
                Expr::val(session.id.stored()),
                Expr::val(session.person.stored()),
                Expr::val(session.token_hash.as_bytes().to_vec()),
                Expr::val(session.stage.code()),
                Expr::val(to_micros(session.created_at)),
                Expr::val(to_micros(session.last_seen_at)),
                Expr::val(session.network.as_ref().map(NetworkPrefix::as_str)),
                Expr::val(session.device.as_str()),
            ])
            .map_err(|_| StoreError::Corrupt("a session"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_session(
        transaction: &mut Transaction,
        id: SessionId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(SessionColumns::Table)
            .and_where(Expr::col(SessionColumns::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn delete_sessions_of(
        transaction: &mut Transaction,
        person: UserId,
        except: Option<SessionId>,
    ) -> Result<u64, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(SessionColumns::Table)
            .and_where(Expr::col(SessionColumns::Person).eq(person.stored()));
        if let Some(except) = except {
            delete.and_where(Expr::col(SessionColumns::Id).ne(except.stored()));
        }
        transaction.execute(&delete).await
    }

    async fn purge_sessions(
        transaction: &mut Transaction,
        idle_before: Timestamp,
        absolute_before: Timestamp,
        pending_before: Timestamp,
    ) -> Result<u64, StoreError> {
        let active = Expr::col(SessionColumns::Stage)
            .eq(SessionStage::Active.code())
            .and(
                Expr::col(SessionColumns::LastSeenAt)
                    .lt(to_micros(idle_before))
                    .or(Expr::col(SessionColumns::CreatedAt).lt(to_micros(absolute_before))),
            );
        let pending = Expr::col(SessionColumns::Stage)
            .eq(SessionStage::SecondStepPending.code())
            .and(Expr::col(SessionColumns::CreatedAt).lt(to_micros(pending_before)));
        let mut delete = Query::delete();
        delete
            .from_table(SessionColumns::Table)
            .and_where(active.or(pending));
        transaction.execute(&delete).await
    }

    async fn upsert_two_step(
        transaction: &mut Transaction,
        step: &TwoStep,
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(TwoStepColumns::Table)
            .and_where(Expr::col(TwoStepColumns::Person).eq(step.person.stored()));
        transaction.execute(&delete).await?;
        let mut insert = Query::insert();
        insert
            .into_table(TwoStepColumns::Table)
            .columns([
                TwoStepColumns::Person,
                TwoStepColumns::Confirmed,
                TwoStepColumns::LastStep,
                TwoStepColumns::CreatedAt,
            ])
            .values([
                Expr::val(step.person.stored()),
                Expr::val(step.confirmed),
                Expr::val(step.last_step),
                Expr::val(to_micros(step.created_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a second step"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn confirm_two_step(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(TwoStepColumns::Table)
            .value(TwoStepColumns::Confirmed, true)
            .and_where(Expr::col(TwoStepColumns::Person).eq(person.stored()));
        transaction.execute(&update).await.map(|_| ())
    }

    async fn delete_two_step(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<(), StoreError> {
        let mut codes = Query::delete();
        codes
            .from_table(BackupCode::Table)
            .and_where(Expr::col(BackupCode::Person).eq(person.stored()));
        transaction.execute(&codes).await?;
        let mut delete = Query::delete();
        delete
            .from_table(TwoStepColumns::Table)
            .and_where(Expr::col(TwoStepColumns::Person).eq(person.stored()));
        transaction.execute(&delete).await.map(|_| ())
    }

    async fn accept_totp_step(
        transaction: &mut Transaction,
        person: UserId,
        step: i64,
    ) -> Result<bool, StoreError> {
        // One statement checks and writes, so two requests with the same code cannot both pass.
        let mut update = Query::update();
        update
            .table(TwoStepColumns::Table)
            .value(TwoStepColumns::LastStep, step)
            .and_where(Expr::col(TwoStepColumns::Person).eq(person.stored()))
            .and_where(Expr::col(TwoStepColumns::LastStep).lt(step));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn replace_backup_codes(
        transaction: &mut Transaction,
        person: UserId,
        hashes: &[Digest],
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(BackupCode::Table)
            .and_where(Expr::col(BackupCode::Person).eq(person.stored()));
        transaction.execute(&delete).await?;
        for hash in hashes {
            let mut insert = Query::insert();
            insert
                .into_table(BackupCode::Table)
                .columns([BackupCode::Person, BackupCode::CodeHash, BackupCode::UsedAt])
                .values([
                    Expr::val(person.stored()),
                    Expr::val(hash.as_bytes().to_vec()),
                    Expr::val(Option::<i64>::None),
                ])
                .map_err(|_| StoreError::Corrupt("a backup code"))?;
            transaction.execute(&insert).await?;
        }
        Ok(())
    }

    async fn use_backup_code(
        transaction: &mut Transaction,
        person: UserId,
        hash: &Digest,
        at: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(BackupCode::Table)
            .value(BackupCode::UsedAt, to_micros(at))
            .and_where(Expr::col(BackupCode::Person).eq(person.stored()))
            .and_where(Expr::col(BackupCode::CodeHash).eq(hash.as_bytes().to_vec()))
            .and_where(Expr::col(BackupCode::UsedAt).is_null());
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_passkey(
        transaction: &mut Transaction,
        passkey: &StoredPasskey,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(PasskeyRow::Table)
            .columns(passkey_columns())
            .values([
                Expr::val(passkey.id.stored()),
                Expr::val(passkey.person.stored()),
                Expr::val(passkey.credential_id.clone()),
                Expr::val(passkey.data.as_str()),
                Expr::val(passkey.label.as_str()),
                Expr::val(to_micros(passkey.created_at)),
                Expr::val(passkey.last_used_at.map(to_micros)),
            ])
            .map_err(|_| StoreError::Corrupt("a passkey"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn use_passkey(
        transaction: &mut Transaction,
        id: PasskeyId,
        data: Option<&str>,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(PasskeyRow::Table)
            .value(PasskeyRow::LastUsedAt, to_micros(at))
            .and_where(Expr::col(PasskeyRow::Id).eq(id.stored()));
        if let Some(data) = data {
            update.value(PasskeyRow::Data, data);
        }
        transaction.execute(&update).await.map(|_| ())
    }

    async fn delete_passkey(
        transaction: &mut Transaction,
        person: UserId,
        id: PasskeyId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(PasskeyRow::Table)
            .and_where(Expr::col(PasskeyRow::Id).eq(id.stored()))
            .and_where(Expr::col(PasskeyRow::Person).eq(person.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn insert_access_key(
        transaction: &mut Transaction,
        record: &AccessKeyRecord,
    ) -> Result<(), StoreError> {
        crate::access_keys::insert_access_key(transaction, record).await
    }

    async fn delete_access_key(
        transaction: &mut Transaction,
        owner: UserId,
        id: AccessKeyId,
    ) -> Result<bool, StoreError> {
        crate::access_keys::delete_access_key(transaction, owner, id).await
    }

    async fn delete_access_keys_of(
        transaction: &mut Transaction,
        owner: UserId,
    ) -> Result<Vec<AccessKeyId>, StoreError> {
        crate::access_keys::delete_access_keys_of(transaction, owner).await
    }

    async fn insert_invitation(
        transaction: &mut Transaction,
        invitation: &Invitation,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(InvitationColumns::Table)
            .columns(invitation_columns())
            .values([
                Expr::val(invitation.id.stored()),
                Expr::val(invitation.token_hash.as_bytes().to_vec()),
                Expr::val(invitation.email.as_str()),
                Expr::val(invitation.invited_by.stored()),
                Expr::val(to_micros(invitation.created_at)),
                Expr::val(to_micros(invitation.expires_at)),
                Expr::val(invitation.used_at.map(to_micros)),
            ])
            .map_err(|_| StoreError::Corrupt("an invitation"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_invitation(
        transaction: &mut Transaction,
        id: InvitationId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(InvitationColumns::Table)
            .and_where(Expr::col(InvitationColumns::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn use_invitation(
        transaction: &mut Transaction,
        id: InvitationId,
        now: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(InvitationColumns::Table)
            .value(InvitationColumns::UsedAt, to_micros(now))
            .and_where(Expr::col(InvitationColumns::Id).eq(id.stored()))
            .and_where(Expr::col(InvitationColumns::UsedAt).is_null())
            .and_where(Expr::col(InvitationColumns::ExpiresAt).gt(to_micros(now)));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn put_setup_code(
        transaction: &mut Transaction,
        code: &SetupCode,
    ) -> Result<(), StoreError> {
        let delete = Query::delete().from_table(SetupCodeRow::Table).to_owned();
        transaction.execute(&delete).await?;
        let mut insert = Query::insert();
        insert
            .into_table(SetupCodeRow::Table)
            .columns([SetupCodeRow::CodeHash, SetupCodeRow::CreatedAt])
            .values([
                Expr::val(code.code_hash.as_bytes().to_vec()),
                Expr::val(to_micros(code.created_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a setup code"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn take_setup_code(
        transaction: &mut Transaction,
        hash: &Digest,
    ) -> Result<bool, StoreError> {
        // The hash is matched here, inside the transaction, so a code that was replaced while the
        // request was running is not used up by a request that typed the old one.
        let mut delete = Query::delete();
        delete
            .from_table(SetupCodeRow::Table)
            .and_where(Expr::col(SetupCodeRow::CodeHash).eq(hash.as_bytes().to_vec()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn clear_setup_code(transaction: &mut Transaction) -> Result<(), StoreError> {
        let delete = Query::delete().from_table(SetupCodeRow::Table).to_owned();
        transaction.execute(&delete).await.map(|_| ())
    }

    async fn put_setting(
        transaction: &mut Transaction,
        key: &str,
        value: &SettingValue,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        put_organization_setting(transaction, key, value, at).await
    }
}

#[cfg(test)]
#[path = "tests/auth.rs"]
mod tests;
