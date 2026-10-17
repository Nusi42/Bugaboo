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

use crate::db::Transaction;
use crate::db::{Database, Row, to_micros};
use crate::error::{SecretProblem, StoreError};
use crate::keys::{KEY_LENGTH, KeyRing, MasterKey, OpenMode, random_bytes};
use crate::schema::{Secret as Columns, SecurityLog, SecurityLogCheckpoint};

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
        .get(&secret.key_id)?
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
    /// Opens the key files in `folder` (the `keys/` folder of the data folder). The files are
    /// made only on the very first start, when the database holds nothing that needs them. If the
    /// database has secrets or a security log and a key file is missing, this refuses, because
    /// new keys could never read what the old ones protected (SEC-16).
    ///
    /// # Errors
    /// Fails with [`StoreError::KeysMissing`] as described, and when a file cannot be read or is
    /// damaged.
    pub async fn open_keys(&self, folder: &std::path::Path) -> Result<KeyRing, StoreError> {
        let has_data = self.has_key_bound_data().await?;
        let mode = if has_data {
            OpenMode::ExistingOnly
        } else {
            OpenMode::CreateMissing
        };
        KeyRing::open(folder, mode)
    }

    /// Whether the database holds anything that only the key files can make sense of.
    async fn has_key_bound_data(&self) -> Result<bool, StoreError> {
        let mut secrets = Query::select();
        secrets.column(Columns::Name).from(Columns::Table).limit(1);
        let mut entries = Query::select();
        entries
            .column(SecurityLog::Seq)
            .from(SecurityLog::Table)
            .limit(1);
        let mut checkpoints = Query::select();
        checkpoints
            .column(SecurityLogCheckpoint::Seq)
            .from(SecurityLogCheckpoint::Table)
            .limit(1);
        for select in [&secrets, &entries, &checkpoints] {
            if self.fetch_optional(select).await?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Stores a secret, replacing the one with the same name. The master key is taken from the
    /// key file as it is now; if a rotation retired that key while the secret was written, the
    /// write is done again with the new key.
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
        for _ in 0..3 {
            let master = keys.newest()?;
            self.store_secret(&master, name, value, now).await?;
            if keys.get(master.id())?.is_some() {
                return Ok(());
            }
        }
        Err(StoreError::Contended)
    }

    async fn store_secret(
        &self,
        master: &MasterKey,
        name: &TechnicalName,
        value: &[u8],
        now: Timestamp,
    ) -> Result<(), StoreError> {
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

    /// Deletes a secret inside a change, so it goes or stays with the rest of the change.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn delete_secret_in(
        transaction: &mut Transaction,
        name: &TechnicalName,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Name).eq(name.as_str()));
        Ok(transaction.execute(&delete).await? > 0)
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
    /// with it (the values are not touched), and then drops the old master keys nothing uses.
    /// Returns how many secrets were re-wrapped. The log key does not change, so the security
    /// log is not touched.
    ///
    /// A secret that is replaced while the rotation runs is not overwritten with an old copy:
    /// each row is changed only if it is still the one that was read (so PostgreSQL, where two
    /// transactions can run side by side, cannot lose the newer value). If the row changed, it is
    /// read again.
    ///
    /// If the program stops half way, the key file has both keys and everything still decrypts;
    /// the next rotation finishes the job. Other processes (a running server) read the key file
    /// again when it changes, so they stop using the retired key; run a rotation while no secret
    /// is being changed (it is a rare administrator action).
    ///
    /// # Errors
    /// Fails when a secret cannot be unwrapped (the old keys are kept then), when a secret keeps
    /// changing under the rotation ([`StoreError::Contended`]), when the key file cannot be
    /// written, and when the database fails.
    pub async fn rotate_master_key(
        &self,
        keys: &KeyRing,
        now: Timestamp,
    ) -> Result<u64, StoreError> {
        keys.add_key()?;
        let newest = keys.newest()?;
        let mut transaction = self.begin().await?;
        let mut names = Query::select();
        names.column(Columns::Name).from(Columns::Table);
        let names: Vec<String> = transaction
            .fetch_all(&names)
            .await?
            .iter()
            .map(|row| row.get(Columns::Name))
            .collect::<Result<_, _>>()?;
        let mut rewrapped = 0;
        for name in &names {
            let mut done = false;
            for _ in 0..3 {
                let mut select = Query::select();
                select
                    .columns(all_columns())
                    .from(Columns::Table)
                    .and_where(Expr::col(Columns::Name).eq(name.as_str()));
                let Some(row) = transaction.fetch_optional(&select).await? else {
                    done = true;
                    break;
                };
                let secret = read_secret(&row)?;
                if secret.key_id == *newest.id() {
                    done = true;
                    break;
                }
                if rewrap_one(&mut transaction, keys, &newest, &secret, now).await? {
                    rewrapped += 1;
                    done = true;
                    break;
                }
            }
            if !done {
                return Err(StoreError::Contended);
            }
        }
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

/// Wraps the key of `secret` with `newest`, but only if the row is still exactly the one that was
/// read. Returns `false` when it changed in the meantime (a newer value must not be overwritten).
async fn rewrap_one(
    transaction: &mut Transaction,
    keys: &KeyRing,
    newest: &MasterKey,
    secret: &StoredSecret,
    now: Timestamp,
) -> Result<bool, StoreError> {
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
        .and_where(Expr::col(Columns::Name).eq(secret.name.as_str()))
        .and_where(Expr::col(Columns::KeyId).eq(secret.key_id.as_str()))
        .and_where(Expr::col(Columns::WrappedKey).eq(secret.wrapped_key.clone()));
    Ok(transaction.execute(&update).await? == 1)
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
    use crate::anchor::LogSeal;
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
            let (folder, ring) = keys();
            // A copy of the key files from before the rotation, like an old backup.
            let backup = tempfile::tempdir()?;
            for file in ["master.key", "log.key"] {
                std::fs::copy(folder.path().join(file), backup.path().join(file))?;
            }
            let old_ring = KeyRing::open(backup.path(), OpenMode::ExistingOnly)?;
            for (secret, value) in [("a.one", "1"), ("b.two", "22"), ("c.three", "333")] {
                database
                    .put_secret(&ring, &name(secret), value.as_bytes(), at(1))
                    .await?;
            }
            let before = stored(&database, "b.two").await;

            // The security log is signed with the log key, which a rotation does not touch.
            let seal = LogSeal::new(&ring, folder.path().join("head.log"));
            for number in 0..4 {
                database
                    .append_security_log(
                        &seal,
                        &NewSecurityLogEntry {
                            time: at(number),
                            person: None,
                            action: name("sign_in.ok"),
                            subject: None,
                            network: None,
                        },
                    )
                    .await?;
            }
            database.prune_in_blocks(&seal, at(100), at(100), 3).await?;

            assert_eq!(database.rotate_master_key(&ring, at(5)).await?, 3);
            assert_eq!(ring.ids()?, [name("k2")], "k1 is not used any more");
            assert_eq!(
                KeyRing::open(folder.path(), OpenMode::ExistingOnly)?.ids()?,
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
            // The old key files no longer open the secrets.
            assert!(
                database
                    .get_secret(&old_ring, &name("a.one"))
                    .await
                    .is_err()
            );
            // New values use the new key, and the log still verifies.
            database
                .put_secret(&ring, &name("d.four"), b"4", at(6))
                .await?;
            assert_eq!(stored(&database, "d.four").await.key_id, name("k2"));
            assert!(matches!(
                database.verify_security_log(&seal).await?,
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
            let (folder, ring) = keys();
            database
                .put_secret(&ring, &name("a.one"), b"1", at(1))
                .await?;
            // Step one of a rotation only: the new key is in the file, nothing re-wrapped yet.
            ring.add_key()?;
            let reopened = KeyRing::open(folder.path(), OpenMode::ExistingOnly)?;
            assert_eq!(reopened.ids()?, [name("k1"), name("k2")]);
            assert_eq!(
                database
                    .get_secret(&reopened, &name("a.one"))
                    .await?
                    .as_slice(),
                b"1"
            );
            // The next rotation finishes the job.
            assert_eq!(database.rotate_master_key(&reopened, at(2)).await?, 1);
            assert_eq!(reopened.ids()?, [name("k3")]);
            assert_eq!(
                database
                    .get_secret(&reopened, &name("a.one"))
                    .await?
                    .as_slice(),
                b"1"
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn a_running_server_never_encrypts_with_a_key_the_command_line_retired() -> TestResult {
        each_database("sec_16_retired_key", async |db| {
            let database = open(db).await?;
            let (folder, server) = keys();
            let command_line = KeyRing::open(folder.path(), OpenMode::ExistingOnly)?;
            database
                .put_secret(&server, &name("a.one"), b"1", at(1))
                .await?;
            assert_eq!(database.rotate_master_key(&command_line, at(2)).await?, 1);
            // The server's ring has not been restarted, but it follows the key file.
            database
                .put_secret(&server, &name("b.two"), b"2", at(3))
                .await?;
            assert_eq!(stored(&database, "b.two").await.key_id, name("k2"));
            assert_eq!(
                database
                    .get_secret(&server, &name("a.one"))
                    .await?
                    .as_slice(),
                b"1"
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn bad_example_a_rotation_does_not_overwrite_a_secret_that_changed_meanwhile()
    -> TestResult {
        each_database("sec_16_rotation_lost_update", async |db| {
            let database = open(db).await?;
            let (_folder, ring) = keys();
            database
                .put_secret(&ring, &name("a.one"), b"old", at(1))
                .await?;
            let read_by_rotation = stored(&database, "a.one").await;
            ring.add_key()?;
            // Somebody replaces the secret after the rotation read it.
            database
                .put_secret(&ring, &name("a.one"), b"new", at(2))
                .await?;
            let newest = ring.newest()?;
            let mut transaction = database.begin().await?;
            let changed =
                rewrap_one(&mut transaction, &ring, &newest, &read_by_rotation, at(3)).await?;
            transaction.commit().await?;
            assert!(!changed, "the row was not the one that was read");
            assert_eq!(
                database.get_secret(&ring, &name("a.one")).await?.as_slice(),
                b"new"
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn sec_16_missing_keys_stop_the_start_when_the_database_needs_them() -> TestResult {
        each_database("sec_16_keys_missing", async |db| {
            let database = open(db).await?;
            let folder = tempfile::tempdir()?;
            let keys_folder = folder.path().join("keys");
            // First start, empty database: the keys are made.
            let ring = database.open_keys(&keys_folder).await?;
            database
                .put_secret(&ring, &name("a.one"), b"1", at(1))
                .await?;
            // The same database, the key files gone: no new keys, a clear refusal.
            std::fs::remove_dir_all(&keys_folder)?;
            let refused = database.open_keys(&keys_folder).await;
            assert!(
                matches!(refused, Err(StoreError::KeysMissing)),
                "{refused:?}"
            );
            assert!(!keys_folder.exists(), "nothing was made");
            Ok(())
        })
        .await
    }
}
