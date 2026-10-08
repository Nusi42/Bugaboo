//! Access keys on both databases (AUTH-07): a key is stored with its scope and found by the hash
//! of its secret only, only its owner deletes it, the list pages newest first, the last use is
//! kept to the minute, and the questions about a second step follow roles, groups and the
//! organization switch (AUTH-03).

use std::collections::BTreeSet;
use std::sync::Arc;

use bb_app::access::PolicyStore as _;
use bb_app::auth::{AccessKeyStore, AccountStore, AuthStore, SecondStepStore};
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::access::{AppliesTo, Assignment, Group, GroupMember, Role};
use bb_core::id::{AssignmentId, GroupId, IdSource, RoleId};
use bb_core::person::Person;
use bb_core::settings::{AUTH_ACCESS_KEY_LONGEST_DAYS, SECURITY_TWO_STEP_REQUIRED, SettingValue};
use bb_core::sign_in::{StoredPasskey, TwoStep};
use bb_testing::{TestDatabase, TestResult, each_database};

use super::*;
use crate::anchor::LogSeal;
use crate::auth::AuthStorage;
use crate::settings::SettingChange;
use crate::test_support::{at, event, ids, open};

struct Fixture {
    storage: AuthStorage,
    _folder: tempfile::TempDir,
}

async fn fixture(db: &TestDatabase) -> Result<Fixture, Box<dyn std::error::Error + Send + Sync>> {
    let database = open(db).await?;
    let folder = tempfile::tempdir()?;
    std::fs::create_dir_all(folder.path().join("logs"))?;
    let keys = Arc::new(database.open_keys(&folder.path().join("keys")).await?);
    let seal = LogSeal::new(&keys, folder.path().join("logs").join("head.log"));
    Ok(Fixture {
        storage: AuthStorage::new(database, keys, seal),
        _folder: folder,
    })
}

/// Runs `write` as a change with its event, the way a use case does.
async fn change<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    let event = event(source, 5);
    ChangeStore::record_change(storage, &event, write).await
}

async fn a_person(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    email: &str,
) -> Result<Person, StoreError> {
    let person = Person {
        id: UserId::generate(source),
        display_name: "Test Person".to_owned(),
        email: email.to_owned(),
        created_at: at(0),
    };
    let stored = person.clone();
    change(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::insert_person(tx, &stored, "hash", at(0)).await })
    })
    .await?;
    Ok(person)
}

fn a_key(
    source: &mut impl IdSource,
    owner: UserId,
    tag: u8,
    choice: KeyChoice,
    created: i64,
) -> AccessKeyRecord {
    AccessKeyRecord {
        key: AccessKey {
            id: AccessKeyId::generate(source),
            owner,
            name: format!("Key {tag}"),
            prefix: format!("{tag:08}"),
            secret_hash: Digest::new([tag; 32]),
            choice,
            created_at: at(created),
            expires_at: at(created + 1000),
            last_used_at: None,
        },
        permissions: Vec::new(),
        projects: Vec::new(),
    }
}

async fn store_key(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    record: &AccessKeyRecord,
) -> Result<(), StoreError> {
    let record = record.clone();
    change(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::insert_access_key(tx, &record).await })
    })
    .await
}

#[tokio::test]
async fn auth_07_a_key_is_stored_with_its_scope_and_found_by_the_hash_of_its_secret_only()
-> TestResult {
    each_database("auth_07_store_key", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ada = a_person(&storage, &mut source, "ada@example.test").await?;
        let project = ProjectId::generate(&mut source)?;
        change(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_project(
                    tx,
                    &bb_core::project::Project {
                        id: project,
                        parent: None,
                        short_name: bb_core::project::ShortName::new("WEB")
                            .map_err(|_| StoreError::Corrupt("a name"))?,
                        name: "Web".to_owned(),
                        version: 1,
                    },
                )
                .await
            })
        })
        .await?;
        let mut record = a_key(&mut source, ada.id, 1, KeyChoice::Permissions, 10);
        record.permissions = vec![Permission::SeeProjects, Permission::CheckAccess];
        record.projects = vec![project];
        store_key(&storage, &mut source, &record).await?;
        // Found by the hash; the scope comes with it.
        let found = storage
            .access_key_by_hash(&record.key.secret_hash)
            .await?
            .ok_or("not found by its hash")?;
        assert_eq!(found.key, record.key);
        assert_eq!(
            found.permissions.iter().copied().collect::<BTreeSet<_>>(),
            BTreeSet::from([Permission::SeeProjects, Permission::CheckAccess])
        );
        assert_eq!(found.projects, vec![project]);
        // Another hash finds nothing: the prefix alone is not a way in.
        assert!(
            storage
                .access_key_by_hash(&Digest::new([9; 32]))
                .await?
                .is_none()
        );
        // The prefix and the hash are each unique: a second key with either is refused.
        let same_prefix = AccessKeyRecord {
            key: AccessKey {
                id: AccessKeyId::generate(&mut source),
                secret_hash: Digest::new([77; 32]),
                ..record.key.clone()
            },
            ..record.clone()
        };
        let result = store_key(&storage, &mut source, &same_prefix).await;
        assert!(AuthStorage::is_duplicate(
            &result.err().ok_or("a second key with the same prefix")?
        ));
        assert_eq!(storage.access_key_count(ada.id).await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_only_the_owner_deletes_a_key_and_the_scope_goes_with_it() -> TestResult {
    each_database("auth_07_delete_key", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ada = a_person(&storage, &mut source, "ada@example.test").await?;
        let bob = a_person(&storage, &mut source, "bob@example.test").await?;
        let mut record = a_key(&mut source, ada.id, 1, KeyChoice::Permissions, 10);
        record.permissions = vec![Permission::SeeProjects];
        store_key(&storage, &mut source, &record).await?;
        let id = record.key.id;
        let by_bob = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_access_key(tx, bob.id, id).await })
        })
        .await?;
        assert!(!by_bob, "Bob has no such key");
        assert_eq!(storage.access_key_count(ada.id).await?, 1);
        let owner = ada.id;
        let by_ada = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_access_key(tx, owner, id).await })
        })
        .await?;
        assert!(by_ada);
        assert!(
            storage
                .access_key_by_hash(&record.key.secret_hash)
                .await?
                .is_none()
        );
        let again = change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_access_key(tx, owner, id).await })
        })
        .await?;
        assert!(!again);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_the_list_pages_newest_first_and_the_last_use_is_kept_to_the_minute() -> TestResult
{
    each_database("auth_07_list_and_touch", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ada = a_person(&storage, &mut source, "ada@example.test").await?;
        let bob = a_person(&storage, &mut source, "bob@example.test").await?;
        let mut all = Vec::new();
        for (tag, created) in [(1_u8, 10_i64), (2, 20), (3, 30), (4, 30)] {
            let record = a_key(&mut source, ada.id, tag, KeyChoice::SeeOnly, created);
            store_key(&storage, &mut source, &record).await?;
            all.push(record);
        }
        let bobs = a_key(&mut source, bob.id, 9, KeyChoice::SeeOnly, 40);
        store_key(&storage, &mut source, &bobs).await?;
        // Newest first; the same moment by ID. Bob's key is not in Ada's list.
        let first = storage.access_keys_of(ada.id, None, 2).await?;
        assert_eq!(first.len(), 2);
        assert!(first[0].key.created_at == at(30) && first[1].key.created_at == at(30));
        assert!(first[0].key.id < first[1].key.id);
        let last = &first[1].key;
        let rest = storage
            .access_keys_of(ada.id, Some((last.created_at, last.id)), 10)
            .await?;
        let names: Vec<&str> = rest.iter().map(|record| record.key.name.as_str()).collect();
        assert_eq!(names, ["Key 2", "Key 1"]);
        // Using a key writes the minute once: an earlier moment changes nothing.
        let id = all[0].key.id;
        storage.touch_access_key(id, at(60)).await?;
        storage.touch_access_key(id, at(30)).await?;
        let kept = storage
            .access_key_by_hash(&all[0].key.secret_hash)
            .await?
            .ok_or("no key")?;
        assert_eq!(kept.key.last_used_at, Some(at(60)));
        storage.touch_access_key(id, at(120)).await?;
        let later = storage
            .access_key_by_hash(&all[0].key.secret_hash)
            .await?
            .ok_or("no key")?;
        assert_eq!(later.key.last_used_at, Some(at(120)));
        Ok(())
    })
    .await
}

type Failure = Box<dyn std::error::Error + Send + Sync>;

/// Ada has the strict role (it has the switch for two-step sign-in) herself, Bob has the relaxed
/// one, and Carol is in a group that has the strict one.
struct Organization {
    ada: Person,
    bob: Person,
    carol: Person,
    strict: RoleId,
    relaxed: RoleId,
}

fn role(id: RoleId, name: &str, position: i64, two_step: bool) -> Role {
    Role {
        id,
        name: name.to_owned(),
        starting: None,
        position,
        two_step,
    }
}

fn assignment(
    id: AssignmentId,
    person: Option<UserId>,
    group: Option<GroupId>,
    role: RoleId,
) -> Assignment {
    Assignment {
        id,
        person,
        group,
        role: Some(role),
        project: None,
        applies_to: AppliesTo::Organization,
        given_at: at(1),
    }
}

async fn strict_and_relaxed(
    storage: &AuthStorage,
    source: &mut impl IdSource,
) -> Result<Organization, Failure> {
    let ada = a_person(storage, source, "ada@example.test").await?;
    let bob = a_person(storage, source, "bob@example.test").await?;
    let carol = a_person(storage, source, "carol@example.test").await?;
    let strict = role(RoleId::generate(source)?, "Strict", 0, true);
    let relaxed = role(RoleId::generate(source)?, "Relaxed", 1, false);
    let group = Group {
        id: GroupId::generate(source)?,
        name: "Strict people".to_owned(),
    };
    let member = GroupMember {
        group: group.id,
        person: carol.id,
        added_at: at(1),
    };
    let assignments = [
        assignment(
            AssignmentId::generate(source)?,
            Some(ada.id),
            None,
            strict.id,
        ),
        assignment(
            AssignmentId::generate(source)?,
            Some(bob.id),
            None,
            relaxed.id,
        ),
        assignment(
            AssignmentId::generate(source)?,
            None,
            Some(group.id),
            strict.id,
        ),
    ];
    let roles = [strict.clone(), relaxed.clone()];
    change(storage, source, move |tx| {
        Box::pin(async move {
            for role in &roles {
                AuthStorage::insert_role(tx, role, &[], &[], &[]).await?;
            }
            AuthStorage::insert_group(tx, &group).await?;
            AuthStorage::insert_group_member(tx, &member).await?;
            for assignment in &assignments {
                AuthStorage::insert_assignment(tx, assignment).await?;
            }
            Ok(())
        })
    })
    .await?;
    Ok(Organization {
        ada,
        bob,
        carol,
        strict: strict.id,
        relaxed: relaxed.id,
    })
}

#[tokio::test]
async fn auth_03_a_role_with_the_switch_asks_for_a_second_step_directly_or_through_a_group()
-> TestResult {
    each_database("auth_03_role_switch", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let world = strict_and_relaxed(&storage, &mut source).await?;
        assert!(storage.second_step_required(world.ada.id).await?);
        assert!(!storage.second_step_required(world.bob.id).await?);
        assert!(storage.second_step_required(world.carol.id).await?);
        // Somebody who holds nothing is asked nothing.
        let dora = a_person(&storage, &mut source, "dora@example.test").await?;
        assert!(!storage.second_step_required(dora.id).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_a_confirmed_code_app_or_a_passkey_is_a_second_step_and_nothing_else_is()
-> TestResult {
    each_database("auth_03_has_second_step", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let world = strict_and_relaxed(&storage, &mut source).await?;
        let (ada, carol) = (world.ada.id, world.carol.id);
        assert!(!storage.has_second_step(ada).await?);
        change(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::upsert_two_step(
                    tx,
                    &TwoStep {
                        person: ada,
                        confirmed: false,
                        last_step: 0,
                        created_at: at(2),
                    },
                )
                .await
            })
        })
        .await?;
        assert!(!storage.has_second_step(ada).await?, "not confirmed yet");
        let passkey = StoredPasskey {
            id: bb_core::id::PasskeyId::generate(&mut source),
            person: carol,
            credential_id: vec![1, 2, 3],
            data: "{}".to_owned(),
            label: "Phone".to_owned(),
            created_at: at(3),
            last_used_at: None,
        };
        change(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::confirm_two_step(tx, ada).await?;
                AuthStorage::insert_passkey(tx, &passkey).await
            })
        })
        .await?;
        assert!(storage.has_second_step(ada).await?);
        assert!(storage.has_second_step(carol).await?);
        assert!(!storage.has_second_step(world.bob.id).await?);
        // Of the people with the strict role (Ada herself, Carol through the group) nobody is
        // left without a second step; of those with the relaxed role there is Bob.
        assert!(
            storage
                .people_without_second_step(world.strict)
                .await?
                .is_empty()
        );
        assert_eq!(
            storage.people_without_second_step(world.relaxed).await?,
            vec![world.bob.id]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_03_the_organization_switch_asks_everybody() -> TestResult {
    each_database("auth_03_organization_switch_store", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let world = strict_and_relaxed(&storage, &mut source).await?;
        assert!(!storage.second_step_required(world.bob.id).await?);
        let key = SECURITY_TWO_STEP_REQUIRED.definition().key().to_owned();
        storage
            .database()
            .set_setting(
                &event(&mut source, 6),
                &SettingChange {
                    key,
                    scope: bb_core::settings::Scope::Organization,
                    value: SettingValue::Bool(true),
                    locked: false,
                },
            )
            .await?;
        assert!(storage.second_step_required(world.bob.id).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_the_life_of_a_key_has_a_default_and_a_longest_time_that_never_pass_a_year()
-> TestResult {
    each_database("auth_07_key_life_settings", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let settings = storage.auth_settings().await?;
        assert_eq!(settings.access_key_default_days, 90);
        assert_eq!(settings.access_key_longest_days, 365);
        // A value stored beyond the bounds (by hand) is cut to them when it is read.
        let key = AUTH_ACCESS_KEY_LONGEST_DAYS.definition().key().to_owned();
        let result = storage
            .database()
            .set_setting(
                &event(&mut source, 6),
                &SettingChange {
                    key,
                    scope: bb_core::settings::Scope::Organization,
                    value: SettingValue::Integer(4000),
                    locked: false,
                },
            )
            .await;
        // It is refused when it is stored, too.
        assert!(result.is_err());
        Ok(())
    })
    .await
}
