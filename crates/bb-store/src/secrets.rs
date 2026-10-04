//! Stored secrets (SEC-16): mail password, AI keys, OIDC client secrets, webhook signing keys.
//!
//! **Envelope encryption.** Every secret gets a key of its own (32 random bytes). The value is
//! encrypted with that key and the key is encrypted ("wrapped") with the newest master key, both
//! with XChaCha20-Poly1305 and a fresh random 24-byte nonce. The ID of the master key is stored
//! with the secret. The name of the secret (and the key ID) are part of what the encryption
//! authenticates, so a row copied to another name or tampered with fails to decrypt.
//!
//! **Rotation** ([`Database::rotate_master_key`]) adds a master key and wraps every secret's key
//! again with it; the values themselves are not touched. Old master keys leave the key file only
//! when nothing uses them. Every step leaves a state that still decrypts.

use bb_core::secret::StoredSecret;
use bb_core::technical::TechnicalName;
use chacha20poly1305::aead::{Aead as _, Payload};
use chacha20poly1305::{Key, KeyInit as _, XChaCha20Poly1305, XNonce};
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};
use zeroize::Zeroizing;

use crate::db::{Database, Row, to_micros};
use crate::error::{SecretProblem, StoreError};
use crate::keys::{KEY_LENGTH, KeyRing, random_bytes};
use crate::schema::Secret as Columns;
use crate::security_log::resign_checkpoint;

const VALUE_LABEL: &[u8] = b"bugaboo.secret.value.v1";
const KEY_LABEL: &[u8] = b"bugaboo.secret.key.v1";
const NONCE_LENGTH: usize = 24;

/// What the encryption authenticates besides the data: which secret, and for wrapping which key.
fn context(label: &[u8], name: &TechnicalName, key_id: Option<&TechnicalName>) -> Vec<u8> {
    let mut aad = label.to_vec();
    aad.push(0);
    aad.extend_from_slice(name.as_str().as_bytes());
    if let Some(key_id) = key_id {
        aad.push(0);
        aad.extend_from_slice(key_id.as_str().as_bytes());
    }
    aad
}

fn tampered() -> StoreError {
    StoreError::Secret(SecretProblem::Tampered)
}

fn seal(
    key: &[u8; KEY_LENGTH],
    nonce: &[u8; NONCE_LENGTH],
    aad: &[u8],
    plain: &[u8],
) -> Result<Vec<u8>, StoreError> {
    XChaCha20Poly1305::new(&Key::from(*key))
        .encrypt(&XNonce::from(*nonce), Payload { msg: plain, aad })
        .map_err(|_| tampered())
}

fn open(
    key: &[u8; KEY_LENGTH],
    nonce: &[u8],
    aad: &[u8],
    cipher: &[u8],
) -> Result<Zeroizing<Vec<u8>>, StoreError> {
    let nonce = <[u8; NONCE_LENGTH]>::try_from(nonce).map_err(|_| tampered())?;
    XChaCha20Poly1305::new(&Key::from(*key))
        .decrypt(&XNonce::from(nonce), Payload { msg: cipher, aad })
        .map(Zeroizing::new)
        .map_err(|_| tampered())
}

/// The key of a secret, decrypted with the master key that wrapped it.
fn unwrap_key(
    keys: &KeyRing,
    secret: &StoredSecret,
) -> Result<Zeroizing<[u8; KEY_LENGTH]>, StoreError> {
    let master = keys
        .get(&secret.key_id)
        .ok_or(StoreError::Secret(SecretProblem::UnknownKey))?;
    let plain = open(
        master.bytes(),
        &secret.wrapped_nonce,
        &context(KEY_LABEL, &secret.name, Some(&secret.key_id)),
        &secret.wrapped_key,
    )?;
    <[u8; KEY_LENGTH]>::try_from(plain.as_slice())
        .map(Zeroizing::new)
        .map_err(|_| tampered())
}

impl Database {
    /// Stores a secret, replacing the one with the same name.
    ///
    /// # Errors
    /// Fails when the system gives no secure random bytes, or when the database fails.
    pub async fn put_secret(
        &self,
        keys: &KeyRing,
        name: &TechnicalName,
        value: &[u8],
        now: Timestamp,
    ) -> Result<(), StoreError> {
        let master = keys.newest()?;
        let secret_key = Zeroizing::new(random_bytes::<KEY_LENGTH>()?);
        let nonce = random_bytes::<NONCE_LENGTH>()?;
        let wrapped_nonce = random_bytes::<NONCE_LENGTH>()?;
        let secret = StoredSecret {
            name: name.clone(),
            key_id: master.id().clone(),
            wrapped_key: seal(
                master.bytes(),
                &wrapped_nonce,
                &context(KEY_LABEL, name, Some(master.id())),
                secret_key.as_slice(),
            )?,
            wrapped_nonce: wrapped_nonce.to_vec(),
            nonce: nonce.to_vec(),
            ciphertext: seal(
                &secret_key,
                &nonce,
                &context(VALUE_LABEL, name, None),
                value,
            )?,
            updated_at: now,
        };
        let mut transaction = self.begin().await?;
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Name).eq(name.as_str()));
        transaction.execute(&delete).await?;
        let mut insert = Query::insert();
        insert
            .into_table(Columns::Table)
            .columns(all_columns())
            .values([
                Expr::val(secret.name.as_str()),
                Expr::val(secret.key_id.as_str()),
                Expr::val(secret.wrapped_key.clone()),
                Expr::val(secret.wrapped_nonce.clone()),
                Expr::val(secret.nonce.clone()),
                Expr::val(secret.ciphertext.clone()),
                Expr::val(to_micros(secret.updated_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a secret"))?;
        transaction.execute(&insert).await?;
        transaction.commit().await
    }

    /// The value of a secret. The bytes are wiped from memory when the result is dropped.
    ///
    /// # Errors
    /// Fails with [`SecretProblem::Missing`] for an unknown name, with
    /// [`SecretProblem::UnknownKey`] when the master key that wrapped it is gone, with
    /// [`SecretProblem::Tampered`] when the stored data was changed, and when the database fails.
    pub async fn get_secret(
        &self,
        keys: &KeyRing,
        name: &TechnicalName,
    ) -> Result<Zeroizing<Vec<u8>>, StoreError> {
        let mut select = Query::select();
        select
            .columns(all_columns())
            .from(Columns::Table)
            .and_where(Expr::col(Columns::Name).eq(name.as_str()));
        let row = self
            .fetch_optional(&select)
            .await?
            .ok_or(StoreError::Secret(SecretProblem::Missing))?;
        let secret = read_secret(&row)?;
        let secret_key = unwrap_key(keys, &secret)?;
        open(
            &secret_key,
            &secret.nonce,
            &context(VALUE_LABEL, &secret.name, None),
            &secret.ciphertext,
        )
    }

    /// Deletes a secret. Returns `false` when there was none.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn delete_secret(&self, name: &TechnicalName) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Name).eq(name.as_str()));
        Ok(self.execute(&delete).await? > 0)
    }

    /// The names of all secrets, in order.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn secret_names(&self) -> Result<Vec<TechnicalName>, StoreError> {
        let mut select = Query::select();
        select
            .column(Columns::Name)
            .from(Columns::Table)
            .order_by(Columns::Name, Order::Asc);
        self.fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                TechnicalName::new(&row.get::<String>(Columns::Name)?)
                    .map_err(|_| StoreError::Corrupt("a secret name"))
            })
            .collect()
    }

    /// Rotates the master key: adds a new one to the key file, wraps the key of every secret
    /// with it (the values are not touched), signs the security log's checkpoint with it, and
    /// then drops the old master keys nothing uses. Returns how many secrets were re-wrapped.
    ///
    /// If the program stops half way, the key file has both keys and everything still
    /// decrypts; the next rotation finishes the job.
    ///
    /// # Errors
    /// Fails when a secret cannot be unwrapped (the old keys are kept then), when the key file
    /// cannot be written, and when the database fails.
    pub async fn rotate_master_key(
        &self,
        keys: &mut KeyRing,
        now: Timestamp,
    ) -> Result<u64, StoreError> {
        keys.add_key()?;
        let newest = keys.newest()?.clone();
        let mut transaction = self.begin().await?;
        let mut select = Query::select();
        select.columns(all_columns()).from(Columns::Table);
        let secrets: Vec<StoredSecret> = transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_secret)
            .collect::<Result<_, _>>()?;
        let mut rewrapped = 0;
        for secret in &secrets {
            let secret_key = unwrap_key(keys, secret)?;
            let nonce = random_bytes::<NONCE_LENGTH>()?;
            let wrapped = seal(
                newest.bytes(),
                &nonce,
                &context(KEY_LABEL, &secret.name, Some(newest.id())),
                secret_key.as_slice(),
            )?;
            let mut update = Query::update();
            update
                .table(Columns::Table)
                .value(Columns::KeyId, newest.id().as_str())
                .value(Columns::WrappedKey, wrapped)
                .value(Columns::WrappedNonce, nonce.to_vec())
                .value(Columns::UpdatedAt, to_micros(now))
                .and_where(Expr::col(Columns::Name).eq(secret.name.as_str()));
            transaction.execute(&update).await?;
            rewrapped += 1;
        }
        resign_checkpoint(&mut transaction, keys).await?;
        transaction.commit().await?;
        let in_use = self.key_ids_in_use().await?;
        keys.retire_unused(&in_use)?;
        Ok(rewrapped)
    }

    /// The IDs of the master keys that wrapped a secret.
    async fn key_ids_in_use(&self) -> Result<Vec<TechnicalName>, StoreError> {
        let mut select = Query::select();
        select
            .distinct()
            .column(Columns::KeyId)
            .from(Columns::Table);
        self.fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                TechnicalName::new(&row.get::<String>(Columns::KeyId)?)
                    .map_err(|_| StoreError::Corrupt("a key ID"))
            })
            .collect()
    }
}

fn all_columns() -> [Columns; 7] {
    [
        Columns::Name,
        Columns::KeyId,
        Columns::WrappedKey,
        Columns::WrappedNonce,
        Columns::Nonce,
        Columns::Ciphertext,
        Columns::UpdatedAt,
    ]
}

fn read_secret(row: &Row) -> Result<StoredSecret, StoreError> {
    let name = |column| {
        TechnicalName::new(&row.get::<String>(column)?)
            .map_err(|_| StoreError::Corrupt("a secret name"))
    };
    Ok(StoredSecret {
        name: name(Columns::Name)?,
        key_id: name(Columns::KeyId)?,
        wrapped_key: row.get(Columns::WrappedKey)?,
        wrapped_nonce: row.get(Columns::WrappedNonce)?,
        nonce: row.get(Columns::Nonce)?,
        ciphertext: row.get(Columns::Ciphertext)?,
        updated_at: row.time(Columns::UpdatedAt)?,
    })
}

#[cfg(test)]
mod tests {
    use bb_testing::{TestResult, each_database};

    use super::*;
    use crate::chain::Verification;
    use crate::security_log::NewSecurityLogEntry;
    use crate::test_support::{at, open};

    fn name(text: &str) -> TechnicalName {
        TechnicalName::new(text).unwrap()
    }

    fn keys() -> (tempfile::TempDir, KeyRing) {
        let folder = tempfile::tempdir().unwrap();
        let ring = KeyRing::open_or_create(folder.path()).unwrap();
        (folder, ring)
    }

    async fn stored(database: &Database, secret: &str) -> StoredSecret {
        let mut select = Query::select();
        select
            .columns(all_columns())
            .from(Columns::Table)
            .and_where(Expr::col(Columns::Name).eq(secret));
        read_secret(&database.fetch_optional(&select).await.unwrap().unwrap()).unwrap()
    }

    #[tokio::test]
    async fn sec_16_a_secret_comes_back_and_is_not_stored_in_the_clear() -> TestResult {
        each_database("sec_16_secret_roundtrip", async |db| {
            let database = open(db).await?;
            let (_folder, ring) = keys();
            let mail = name("mail.password");
            database
                .put_secret(&ring, &mail, b"correct horse battery staple", at(1))
                .await?;
            let value = database.get_secret(&ring, &mail).await?;
            assert_eq!(value.as_slice(), b"correct horse battery staple");

            let row = stored(&database, "mail.password").await;
            assert_eq!(row.key_id, name("k1"));
            let all = [
                &row.wrapped_key,
                &row.wrapped_nonce,
                &row.nonce,
                &row.ciphertext,
            ];
            for part in all {
                assert!(
                    !part.windows(7).any(|window| window == b"correct"),
                    "the value shows in the stored data"
                );
            }
            assert_eq!(row.nonce.len(), 24);

            database.put_secret(&ring, &mail, b"new", at(2)).await?;
            assert_eq!(database.get_secret(&ring, &mail).await?.as_slice(), b"new");
            assert_eq!(database.secret_names().await?, std::slice::from_ref(&mail));
            assert!(database.delete_secret(&mail).await?);
            assert!(matches!(
                database.get_secret(&ring, &mail).await,
                Err(StoreError::Secret(SecretProblem::Missing))
            ));
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn bad_example_changed_or_moved_data_does_not_decrypt() -> TestResult {
        each_database("sec_16_tampered_secret", async |db| {
            let database = open(db).await?;
            let (_folder, ring) = keys();
            database
                .put_secret(&ring, &name("ai.key"), b"alpha", at(1))
                .await?;
            database
                .put_secret(&ring, &name("webhook.key"), b"beta", at(1))
                .await?;
            let other = stored(&database, "webhook.key").await;

            // A flipped bit in the value.
            let mut flipped = stored(&database, "ai.key").await.ciphertext;
            flipped[0] ^= 1;
            let mut update = Query::update();
            update
                .table(Columns::Table)
                .value(Columns::Ciphertext, flipped)
                .and_where(Expr::col(Columns::Name).eq("ai.key"));
            database.execute(&update).await?;
            assert!(matches!(
                database.get_secret(&ring, &name("ai.key")).await,
                Err(StoreError::Secret(SecretProblem::Tampered))
            ));

            // The whole row of another secret copied under this name.
            let mut update = Query::update();
            update
                .table(Columns::Table)
                .value(Columns::WrappedKey, other.wrapped_key)
                .value(Columns::WrappedNonce, other.wrapped_nonce)
                .value(Columns::Nonce, other.nonce)
                .value(Columns::Ciphertext, other.ciphertext)
                .and_where(Expr::col(Columns::Name).eq("ai.key"));
            database.execute(&update).await?;
            assert!(matches!(
                database.get_secret(&ring, &name("ai.key")).await,
                Err(StoreError::Secret(SecretProblem::Tampered))
            ));

            // The key file of another installation.
            let (_other_folder, stranger) = keys();
            assert!(
                database
                    .get_secret(&stranger, &name("webhook.key"))
                    .await
                    .is_err()
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn sec_16_rotation_wraps_every_key_again_and_drops_the_old_master_key() -> TestResult {
        each_database("sec_16_rotation", async |db| {
            let database = open(db).await?;
            let (folder, mut ring) = keys();
            let old_ring = KeyRing::open_or_create(folder.path())?;
            for (secret, value) in [("a.one", "1"), ("b.two", "22"), ("c.three", "333")] {
                database
                    .put_secret(&ring, &name(secret), value.as_bytes(), at(1))
                    .await?;
            }
            let before = stored(&database, "b.two").await;

            // A checkpoint signed by the old key must survive the rotation.
            for number in 0..4 {
                database
                    .append_security_log(&NewSecurityLogEntry {
                        time: at(number),
                        person: None,
                        action: name("sign_in.ok"),
                        subject: None,
                        network: None,
                    })
                    .await?;
            }
            database.prune_in_blocks(&ring, at(100), at(100), 3).await?;

            assert_eq!(database.rotate_master_key(&mut ring, at(5)).await?, 3);
            assert_eq!(ring.ids(), [name("k2")], "k1 is not used any more");
            assert_eq!(
                KeyRing::open_or_create(folder.path())?.ids(),
                [name("k2")],
                "the key file has only the new key"
            );
            let after = stored(&database, "b.two").await;
            assert_eq!(after.key_id, name("k2"));
            assert_ne!(after.wrapped_key, before.wrapped_key);
            assert_eq!(
                after.ciphertext, before.ciphertext,
                "the value is not touched"
            );
            for (secret, value) in [("a.one", "1"), ("b.two", "22"), ("c.three", "333")] {
                let found = database.get_secret(&ring, &name(secret)).await?;
                assert_eq!(found.as_slice(), value.as_bytes());
            }
            // The old key file no longer opens the secrets.
            assert!(
                database
                    .get_secret(&old_ring, &name("a.one"))
                    .await
                    .is_err()
            );
            // New values use the new key, and the log still verifies with it.
            database
                .put_secret(&ring, &name("d.four"), b"4", at(6))
                .await?;
            assert_eq!(stored(&database, "d.four").await.key_id, name("k2"));
            assert!(matches!(
                database.verify_security_log(&ring).await?,
                Verification::Valid { .. }
            ));
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn a_rotation_that_stops_half_way_keeps_everything_readable() -> TestResult {
        each_database("sec_16_half_rotation", async |db| {
            let database = open(db).await?;
            let (folder, mut ring) = keys();
            database
                .put_secret(&ring, &name("a.one"), b"1", at(1))
                .await?;
            // Step one of a rotation only: the new key is in the file, nothing re-wrapped yet.
            ring.add_key()?;
            let reopened = KeyRing::open_or_create(folder.path())?;
            assert_eq!(reopened.ids(), [name("k1"), name("k2")]);
            assert_eq!(
                database
                    .get_secret(&reopened, &name("a.one"))
                    .await?
                    .as_slice(),
                b"1"
            );
            // The next rotation finishes the job.
            let mut ring = reopened;
            assert_eq!(database.rotate_master_key(&mut ring, at(2)).await?, 1);
            assert_eq!(ring.ids(), [name("k3")]);
            assert_eq!(
                database.get_secret(&ring, &name("a.one")).await?.as_slice(),
                b"1"
            );
            Ok(())
        })
        .await
    }
}
