//! Access keys, and the projects a person may choose for one.

use bb_app::auth::{AccessKeyRecord, AccessKeyStore, ProjectChoice};
use bb_core::id::{AccessKeyId, UserId};
use bb_core::security_log::Digest;
use jiff::Timestamp;

use crate::db::Transaction;
use crate::error::StoreError;

use super::AuthStorage;

impl AccessKeyStore for AuthStorage {
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

    async fn touch_access_key(&self, id: AccessKeyId, at: Timestamp) -> Result<(), StoreError> {
        self.database.touch_access_key(id, at).await
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
}
