//! The adapter of the port of the sign-in of AI apps (`bb_app::auth::OAuthStore`): apps, grants,
//! codes, tokens and the web addresses administrators allowed (AI-22, ADR-0042).
//!
//! A code or a token is found by the hash of its secret (the primary key), so an unknown secret and
//! a wrong one are the same lookup that finds nothing. Ending a grant takes away its codes and
//! tokens in the same change.

use bb_app::auth::{GrantRecord, GrantWithApp, OAuthCleaned, OAuthStore};
use bb_core::access_key::KeyChoice;
use bb_core::id::{GrantId, OAuthClientId, ProjectId, UserId};
use bb_core::oauth::{
    GrantEnd, OAuthClient, OAuthCode, OAuthGrant, OAuthToken, OAuthWebAddress, TokenKind,
};
use bb_core::security_log::Digest;
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, Func, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{
    Computed, OAuthClientRow, OAuthCodeRow, OAuthGrantProjectRow, OAuthGrantRow, OAuthTokenRow,
    OAuthWebAddressRow,
};

/// The addresses of an app are one text, one address per line: an address holds no line break.
const LINE: &str = "\n";

fn digest(row: &Row, column: impl sea_query::Iden + Copy + 'static) -> Result<Digest, StoreError> {
    Digest::from_slice(&row.get::<Vec<u8>>(column)?).ok_or(StoreError::Corrupt("a hash"))
}

fn read_client(row: &Row) -> Result<OAuthClient, StoreError> {
    Ok(OAuthClient {
        id: row.id::<OAuthClientId>(OAuthClientRow::Id)?,
        name: row.get(OAuthClientRow::Name)?,
        return_addresses: row
            .get::<String>(OAuthClientRow::ReturnAddresses)?
            .split(LINE)
            .map(str::to_owned)
            .collect(),
        registered_at: row.time(OAuthClientRow::RegisteredAt)?,
    })
}

const GRANT_COLUMNS: [OAuthGrantRow; 9] = [
    OAuthGrantRow::Id,
    OAuthGrantRow::Client,
    OAuthGrantRow::Person,
    OAuthGrantRow::Scope,
    OAuthGrantRow::AllowedAt,
    OAuthGrantRow::LastUsedAt,
    OAuthGrantRow::EndsAt,
    OAuthGrantRow::EndedAt,
    OAuthGrantRow::Ended,
];

fn read_grant(row: &Row) -> Result<OAuthGrant, StoreError> {
    Ok(OAuthGrant {
        id: row.id::<GrantId>(OAuthGrantRow::Id)?,
        client: row.id::<OAuthClientId>(OAuthGrantRow::Client)?,
        person: row.id::<UserId>(OAuthGrantRow::Person)?,
        scope: KeyChoice::from_code(&row.get::<String>(OAuthGrantRow::Scope)?)
            .ok_or(StoreError::Corrupt("a grant scope"))?,
        allowed_at: row.time(OAuthGrantRow::AllowedAt)?,
        last_used_at: row.optional_time(OAuthGrantRow::LastUsedAt)?,
        ends_at: row.time(OAuthGrantRow::EndsAt)?,
        ended_at: row.optional_time(OAuthGrantRow::EndedAt)?,
        ended: row
            .get::<Option<String>>(OAuthGrantRow::Ended)?
            .map(|code| GrantEnd::from_code(&code).ok_or(StoreError::Corrupt("a grant end")))
            .transpose()?,
    })
}

/// The grants with their projects, in the order given.
async fn with_projects(
    database: &crate::db::Database,
    grants: Vec<OAuthGrant>,
) -> Result<Vec<GrantRecord>, StoreError> {
    if grants.is_empty() {
        return Ok(Vec::new());
    }
    let mut select = Query::select();
    select
        .columns([OAuthGrantProjectRow::Grant, OAuthGrantProjectRow::Project])
        .from(OAuthGrantProjectRow::Table)
        .and_where(
            Expr::col(OAuthGrantProjectRow::Grant).is_in(
                grants
                    .iter()
                    .map(|grant| grant.id.stored())
                    .collect::<Vec<_>>(),
            ),
        )
        .order_by(OAuthGrantProjectRow::Project, Order::Asc);
    let mut projects: std::collections::BTreeMap<GrantId, Vec<ProjectId>> =
        std::collections::BTreeMap::new();
    for row in database.fetch_all(&select).await? {
        projects
            .entry(row.id::<GrantId>(OAuthGrantProjectRow::Grant)?)
            .or_default()
            .push(row.id::<ProjectId>(OAuthGrantProjectRow::Project)?);
    }
    Ok(grants
        .into_iter()
        .map(|grant| GrantRecord {
            projects: projects.remove(&grant.id).unwrap_or_default(),
            grant,
        })
        .collect())
}

/// Takes away the codes and tokens of grants, by the ID of the grant.
async fn delete_secrets_of(
    transaction: &mut Transaction,
    grants: Vec<Vec<u8>>,
) -> Result<(), StoreError> {
    if grants.is_empty() {
        return Ok(());
    }
    let mut codes = Query::delete();
    codes
        .from_table(OAuthCodeRow::Table)
        .and_where(Expr::col(OAuthCodeRow::Grant).is_in(grants.clone()));
    transaction.execute(&codes).await?;
    let mut tokens = Query::delete();
    tokens
        .from_table(OAuthTokenRow::Table)
        .and_where(Expr::col(OAuthTokenRow::Grant).is_in(grants));
    transaction.execute(&tokens).await?;
    Ok(())
}

impl OAuthStore for AuthStorage {
    async fn oauth_client(&self, id: OAuthClientId) -> Result<Option<OAuthClient>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                OAuthClientRow::Id,
                OAuthClientRow::Name,
                OAuthClientRow::ReturnAddresses,
                OAuthClientRow::RegisteredAt,
            ])
            .from(OAuthClientRow::Table)
            .and_where(Expr::col(OAuthClientRow::Id).eq(id.stored()));
        self.database()
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_client)
            .transpose()
    }

    async fn oauth_grant(&self, id: GrantId) -> Result<Option<GrantRecord>, StoreError> {
        let mut select = Query::select();
        select
            .columns(GRANT_COLUMNS)
            .from(OAuthGrantRow::Table)
            .and_where(Expr::col(OAuthGrantRow::Id).eq(id.stored()));
        let Some(row) = self.database().fetch_optional(&select).await? else {
            return Ok(None);
        };
        let mut found = with_projects(self.database(), vec![read_grant(&row)?]).await?;
        Ok(found.pop())
    }

    async fn grants_of(
        &self,
        person: UserId,
        now: Timestamp,
        limit: u64,
    ) -> Result<Vec<GrantWithApp>, StoreError> {
        let mut select = Query::select();
        select
            .columns(GRANT_COLUMNS)
            .from(OAuthGrantRow::Table)
            .and_where(Expr::col(OAuthGrantRow::Person).eq(person.stored()))
            .and_where(Expr::col(OAuthGrantRow::EndedAt).is_null())
            .and_where(Expr::col(OAuthGrantRow::EndsAt).gt(to_micros(now)))
            .order_by(OAuthGrantRow::AllowedAt, Order::Desc)
            .order_by(OAuthGrantRow::Id, Order::Asc)
            .limit(limit);
        let grants = self
            .database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_grant)
            .collect::<Result<Vec<_>, _>>()?;
        let records = with_projects(self.database(), grants).await?;
        // The apps of all the grants in one read, not one read for each grant.
        let mut apps = Query::select();
        apps.columns([
            OAuthClientRow::Id,
            OAuthClientRow::Name,
            OAuthClientRow::ReturnAddresses,
            OAuthClientRow::RegisteredAt,
        ])
        .from(OAuthClientRow::Table)
        .and_where(
            Expr::col(OAuthClientRow::Id).is_in(
                records
                    .iter()
                    .map(|found| found.grant.client.stored())
                    .collect::<Vec<_>>(),
            ),
        );
        let mut clients = std::collections::BTreeMap::new();
        if !records.is_empty() {
            for row in self.database().fetch_all(&apps).await? {
                let client = read_client(&row)?;
                clients.insert(client.id, client);
            }
        }
        Ok(records
            .into_iter()
            .filter_map(|record| {
                clients
                    .get(&record.grant.client)
                    .cloned()
                    .map(|client| GrantWithApp { record, client })
            })
            .collect())
    }

    async fn grant_count_of(&self, person: UserId, now: Timestamp) -> Result<i64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(Expr::col(OAuthGrantRow::Id).count(), Computed::Total)
            .from(OAuthGrantRow::Table)
            .and_where(Expr::col(OAuthGrantRow::Person).eq(person.stored()))
            .and_where(Expr::col(OAuthGrantRow::EndedAt).is_null())
            .and_where(Expr::col(OAuthGrantRow::EndsAt).gt(to_micros(now)));
        match self.database().fetch_optional(&select).await? {
            Some(row) => row.get(Computed::Total),
            None => Ok(0),
        }
    }

    async fn oauth_token(&self, hash: &Digest) -> Result<Option<OAuthToken>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                OAuthTokenRow::SecretHash,
                OAuthTokenRow::Grant,
                OAuthTokenRow::Kind,
                OAuthTokenRow::CreatedAt,
                OAuthTokenRow::ExpiresAt,
                OAuthTokenRow::UsedAt,
                OAuthTokenRow::PausedAt,
            ])
            .from(OAuthTokenRow::Table)
            .and_where(Expr::col(OAuthTokenRow::SecretHash).eq(hash.as_bytes().to_vec()));
        let Some(row) = self.database().fetch_optional(&select).await? else {
            return Ok(None);
        };
        Ok(Some(OAuthToken {
            secret_hash: digest(&row, OAuthTokenRow::SecretHash)?,
            grant: row.id::<GrantId>(OAuthTokenRow::Grant)?,
            kind: TokenKind::from_code(&row.get::<String>(OAuthTokenRow::Kind)?)
                .ok_or(StoreError::Corrupt("a token kind"))?,
            created_at: row.time(OAuthTokenRow::CreatedAt)?,
            expires_at: row.time(OAuthTokenRow::ExpiresAt)?,
            used_at: row.optional_time(OAuthTokenRow::UsedAt)?,
            paused_at: row.optional_time(OAuthTokenRow::PausedAt)?,
        }))
    }

    async fn oauth_code(&self, hash: &Digest) -> Result<Option<OAuthCode>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                OAuthCodeRow::SecretHash,
                OAuthCodeRow::Grant,
                OAuthCodeRow::ReturnAddress,
                OAuthCodeRow::Challenge,
                OAuthCodeRow::ExpiresAt,
                OAuthCodeRow::UsedAt,
            ])
            .from(OAuthCodeRow::Table)
            .and_where(Expr::col(OAuthCodeRow::SecretHash).eq(hash.as_bytes().to_vec()));
        let Some(row) = self.database().fetch_optional(&select).await? else {
            return Ok(None);
        };
        Ok(Some(OAuthCode {
            secret_hash: digest(&row, OAuthCodeRow::SecretHash)?,
            grant: row.id::<GrantId>(OAuthCodeRow::Grant)?,
            return_address: row.get(OAuthCodeRow::ReturnAddress)?,
            challenge: row.get(OAuthCodeRow::Challenge)?,
            expires_at: row.time(OAuthCodeRow::ExpiresAt)?,
            used_at: row.optional_time(OAuthCodeRow::UsedAt)?,
        }))
    }

    async fn oauth_web_addresses(&self) -> Result<Vec<OAuthWebAddress>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                OAuthWebAddressRow::Address,
                OAuthWebAddressRow::AddedBy,
                OAuthWebAddressRow::AddedAt,
            ])
            .from(OAuthWebAddressRow::Table)
            .order_by(OAuthWebAddressRow::Address, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok(OAuthWebAddress {
                    address: row.get(OAuthWebAddressRow::Address)?,
                    added_by: row.id::<UserId>(OAuthWebAddressRow::AddedBy)?,
                    added_at: row.time(OAuthWebAddressRow::AddedAt)?,
                })
            })
            .collect()
    }

    async fn touch_grant(&self, id: GrantId, at: Timestamp) -> Result<(), StoreError> {
        let micros = to_micros(at);
        let mut update = Query::update();
        update
            .table(OAuthGrantRow::Table)
            .value(OAuthGrantRow::LastUsedAt, micros)
            .and_where(Expr::col(OAuthGrantRow::Id).eq(id.stored()))
            .cond_where(
                Cond::any()
                    .add(Expr::col(OAuthGrantRow::LastUsedAt).is_null())
                    .add(Expr::col(OAuthGrantRow::LastUsedAt).lt(micros)),
            );
        self.database().execute(&update).await.map(|_| ())
    }

    async fn insert_oauth_client(
        transaction: &mut Transaction,
        client: &OAuthClient,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(OAuthClientRow::Table)
            .columns([
                OAuthClientRow::Id,
                OAuthClientRow::Name,
                OAuthClientRow::ReturnAddresses,
                OAuthClientRow::RegisteredAt,
            ])
            .values([
                Expr::val(client.id.stored()),
                Expr::val(client.name.as_str()),
                Expr::val(client.return_addresses.join(LINE)),
                Expr::val(to_micros(client.registered_at)),
            ])
            .map_err(|_| StoreError::Corrupt("an app"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn insert_grant(
        transaction: &mut Transaction,
        record: &GrantRecord,
    ) -> Result<(), StoreError> {
        let grant = &record.grant;
        let mut insert = Query::insert();
        insert
            .into_table(OAuthGrantRow::Table)
            .columns(GRANT_COLUMNS)
            .values([
                Expr::val(grant.id.stored()),
                Expr::val(grant.client.stored()),
                Expr::val(grant.person.stored()),
                Expr::val(grant.scope.code()),
                Expr::val(to_micros(grant.allowed_at)),
                Expr::val(grant.last_used_at.map(to_micros)),
                Expr::val(to_micros(grant.ends_at)),
                Expr::val(grant.ended_at.map(to_micros)),
                Expr::val(grant.ended.map(GrantEnd::code)),
            ])
            .map_err(|_| StoreError::Corrupt("a grant"))?;
        transaction.execute(&insert).await?;
        for project in &record.projects {
            let mut insert = Query::insert();
            insert
                .into_table(OAuthGrantProjectRow::Table)
                .columns([OAuthGrantProjectRow::Grant, OAuthGrantProjectRow::Project])
                .values([Expr::val(grant.id.stored()), Expr::val(project.stored())])
                .map_err(|_| StoreError::Corrupt("a project of a grant"))?;
            transaction.execute(&insert).await?;
        }
        Ok(())
    }

    async fn insert_oauth_code(
        transaction: &mut Transaction,
        code: &OAuthCode,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(OAuthCodeRow::Table)
            .columns([
                OAuthCodeRow::SecretHash,
                OAuthCodeRow::Grant,
                OAuthCodeRow::ReturnAddress,
                OAuthCodeRow::Challenge,
                OAuthCodeRow::ExpiresAt,
                OAuthCodeRow::UsedAt,
            ])
            .values([
                Expr::val(code.secret_hash.as_bytes().to_vec()),
                Expr::val(code.grant.stored()),
                Expr::val(code.return_address.as_str()),
                Expr::val(code.challenge.as_str()),
                Expr::val(to_micros(code.expires_at)),
                Expr::val(code.used_at.map(to_micros)),
            ])
            .map_err(|_| StoreError::Corrupt("a code"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn use_oauth_code(
        transaction: &mut Transaction,
        hash: &Digest,
        at: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(OAuthCodeRow::Table)
            .value(OAuthCodeRow::UsedAt, to_micros(at))
            .and_where(Expr::col(OAuthCodeRow::SecretHash).eq(hash.as_bytes().to_vec()))
            .and_where(Expr::col(OAuthCodeRow::UsedAt).is_null());
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_oauth_token(
        transaction: &mut Transaction,
        token: &OAuthToken,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(OAuthTokenRow::Table)
            .columns([
                OAuthTokenRow::SecretHash,
                OAuthTokenRow::Grant,
                OAuthTokenRow::Kind,
                OAuthTokenRow::CreatedAt,
                OAuthTokenRow::ExpiresAt,
                OAuthTokenRow::UsedAt,
                OAuthTokenRow::PausedAt,
            ])
            .values([
                Expr::val(token.secret_hash.as_bytes().to_vec()),
                Expr::val(token.grant.stored()),
                Expr::val(token.kind.code()),
                Expr::val(to_micros(token.created_at)),
                Expr::val(to_micros(token.expires_at)),
                Expr::val(token.used_at.map(to_micros)),
                Expr::val(token.paused_at.map(to_micros)),
            ])
            .map_err(|_| StoreError::Corrupt("a token"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn use_oauth_token(
        transaction: &mut Transaction,
        hash: &Digest,
        at: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(OAuthTokenRow::Table)
            .value(OAuthTokenRow::UsedAt, to_micros(at))
            .and_where(Expr::col(OAuthTokenRow::SecretHash).eq(hash.as_bytes().to_vec()))
            .and_where(Expr::col(OAuthTokenRow::UsedAt).is_null());
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn end_grant(
        transaction: &mut Transaction,
        id: GrantId,
        how: GrantEnd,
        at: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(OAuthGrantRow::Table)
            .value(OAuthGrantRow::EndedAt, to_micros(at))
            .value(OAuthGrantRow::Ended, how.code())
            .and_where(Expr::col(OAuthGrantRow::Id).eq(id.stored()))
            .and_where(Expr::col(OAuthGrantRow::EndedAt).is_null());
        let ended = transaction.execute(&update).await? > 0;
        if ended {
            delete_secrets_of(transaction, vec![id.stored()]).await?;
        }
        Ok(ended)
    }

    async fn end_grants_of(
        transaction: &mut Transaction,
        person: UserId,
        how: GrantEnd,
        at: Timestamp,
    ) -> Result<Vec<GrantId>, StoreError> {
        let mut select = Query::select();
        select
            .column(OAuthGrantRow::Id)
            .from(OAuthGrantRow::Table)
            .and_where(Expr::col(OAuthGrantRow::Person).eq(person.stored()))
            .and_where(Expr::col(OAuthGrantRow::EndedAt).is_null())
            .order_by(OAuthGrantRow::Id, Order::Asc);
        let mut ids = Vec::new();
        for row in transaction.fetch_all(&select).await? {
            ids.push(row.id::<GrantId>(OAuthGrantRow::Id)?);
        }
        if !ids.is_empty() {
            let stored: Vec<Vec<u8>> = ids.iter().map(|id| id.stored()).collect();
            let mut update = Query::update();
            update
                .table(OAuthGrantRow::Table)
                .value(OAuthGrantRow::EndedAt, to_micros(at))
                .value(OAuthGrantRow::Ended, how.code())
                .and_where(Expr::col(OAuthGrantRow::Id).is_in(stored.clone()));
            transaction.execute(&update).await?;
            delete_secrets_of(transaction, stored).await?;
        }
        Ok(ids)
    }

    async fn insert_oauth_web_address(
        transaction: &mut Transaction,
        address: &OAuthWebAddress,
    ) -> Result<bool, StoreError> {
        let mut held = Query::select();
        held.column(OAuthWebAddressRow::Address)
            .from(OAuthWebAddressRow::Table)
            .and_where(Expr::col(OAuthWebAddressRow::Address).eq(address.address.as_str()));
        if transaction.fetch_optional(&held).await?.is_some() {
            return Ok(false);
        }
        let mut insert = Query::insert();
        insert
            .into_table(OAuthWebAddressRow::Table)
            .columns([
                OAuthWebAddressRow::Address,
                OAuthWebAddressRow::AddedBy,
                OAuthWebAddressRow::AddedAt,
            ])
            .values([
                Expr::val(address.address.as_str()),
                Expr::val(address.added_by.stored()),
                Expr::val(to_micros(address.added_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a web address"))?;
        transaction.execute(&insert).await?;
        Ok(true)
    }

    async fn delete_oauth_web_address(
        transaction: &mut Transaction,
        address: &str,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(OAuthWebAddressRow::Table)
            .and_where(Expr::col(OAuthWebAddressRow::Address).eq(address));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn pause_unused_tokens(
        transaction: &mut Transaction,
        now: Timestamp,
    ) -> Result<u64, StoreError> {
        // The end moves by the time since the last pause, so a clean-up that runs twice in a day, or
        // at every start of the server, adds only the time that passed. The first pause of a token
        // only marks the moment.
        let now_micros = to_micros(now);
        let since: Expr =
            Func::coalesce([Expr::col(OAuthTokenRow::PausedAt), Expr::val(now_micros)]).into();
        let waited = Expr::val(now_micros).sub(since);
        let mut update = Query::update();
        update
            .table(OAuthTokenRow::Table)
            .value(
                OAuthTokenRow::ExpiresAt,
                Expr::col(OAuthTokenRow::ExpiresAt).add(waited),
            )
            .value(OAuthTokenRow::PausedAt, now_micros)
            .and_where(Expr::col(OAuthTokenRow::Kind).eq(TokenKind::Refresh.code()))
            .and_where(Expr::col(OAuthTokenRow::UsedAt).is_null())
            .and_where(Expr::col(OAuthTokenRow::ExpiresAt).gt(to_micros(now)));
        transaction.execute(&update).await
    }

    async fn clean_oauth(
        transaction: &mut Transaction,
        now: Timestamp,
        ended_before: Timestamp,
        registered_before: Timestamp,
    ) -> Result<OAuthCleaned, StoreError> {
        // Apps are on: what waited has stopped waiting.
        let mut resume = Query::update();
        resume
            .table(OAuthTokenRow::Table)
            .value(OAuthTokenRow::PausedAt, Option::<i64>::None)
            .and_where(Expr::col(OAuthTokenRow::PausedAt).is_not_null());
        transaction.execute(&resume).await?;
        let mut codes = Query::delete();
        codes
            .from_table(OAuthCodeRow::Table)
            .and_where(Expr::col(OAuthCodeRow::ExpiresAt).lte(to_micros(now)));
        let mut tokens = Query::delete();
        tokens
            .from_table(OAuthTokenRow::Table)
            .and_where(Expr::col(OAuthTokenRow::ExpiresAt).lte(to_micros(now)));
        let secrets = transaction.execute(&codes).await? + transaction.execute(&tokens).await?;
        // The projects, codes and tokens of a grant that is deleted go first.
        let mut old = Query::select();
        old.column(OAuthGrantRow::Id)
            .from(OAuthGrantRow::Table)
            .cond_where(
                Cond::any()
                    .add(Expr::col(OAuthGrantRow::EndedAt).lte(to_micros(ended_before)))
                    .add(Expr::col(OAuthGrantRow::EndsAt).lte(to_micros(ended_before))),
            );
        let mut gone = Vec::new();
        for row in transaction.fetch_all(&old).await? {
            gone.push(row.id::<GrantId>(OAuthGrantRow::Id)?.stored());
        }
        let mut grants = 0;
        if !gone.is_empty() {
            delete_secrets_of(transaction, gone.clone()).await?;
            let mut projects = Query::delete();
            projects
                .from_table(OAuthGrantProjectRow::Table)
                .and_where(Expr::col(OAuthGrantProjectRow::Grant).is_in(gone.clone()));
            transaction.execute(&projects).await?;
            let mut delete = Query::delete();
            delete
                .from_table(OAuthGrantRow::Table)
                .and_where(Expr::col(OAuthGrantRow::Id).is_in(gone));
            grants = transaction.execute(&delete).await?;
        }
        let mut held = Query::select();
        held.column(OAuthGrantRow::Client)
            .from(OAuthGrantRow::Table);
        let mut unused = Query::delete();
        unused
            .from_table(OAuthClientRow::Table)
            .and_where(Expr::col(OAuthClientRow::RegisteredAt).lte(to_micros(registered_before)))
            .and_where(Expr::col(OAuthClientRow::Id).not_in_subquery(held));
        let clients = transaction.execute(&unused).await?;
        Ok(OAuthCleaned {
            secrets,
            grants,
            clients,
        })
    }
}
