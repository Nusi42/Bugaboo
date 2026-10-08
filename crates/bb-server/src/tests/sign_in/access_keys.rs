//! Access keys (AUTH-07, SEC-14, SEC-15, SEC-16, ADR-0025 decision 8), through the real wiring:
//! the use cases of `bb-app`, the store of `bb-store` and, for the HTTP tests, the router of
//! `bb-api`. Every flow runs on SQLite and on PostgreSQL (`each_database`, ARCH-DB-PARITY).
//!
//! The test organization of `policy_rig` has the administrator Ada and people without a role;
//! Bob gets the role *Checker*: *Check access* (a *See* permission), *Make projects* (*Change*)
//! and *Invite people* (*Manage*), one of each kind a key can keep.

use std::fmt::Write as _;

use bb_app::access::Permission;
use bb_app::auth::{
    AccessKeyRulesQuery, AccessKeyView, AccessKeysQuery, AppError, ChangePassword,
    CreatedAccessKey, DeleteAccessKey, InvitationsQuery, MakeAccessKey, ProjectChoicesQuery,
    RequestInfo, StartCodeApp, TwoStepQuery,
};
use bb_app::projects::MakeProject;
use bb_app::roles::{CheckAccessQuery, RemoveAssignment};
use bb_auth::AccessKeySecret;
use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::id::ProjectId;
use bb_core::network::NetworkPrefix;
use bb_core::sign_in::SecurityAction;
use bb_core::technical::TechnicalName;
use bb_store::Verification;
use bb_testing::{TestResult, each_database};
use jiff::Timestamp;

use super::http::ADA_EMAIL;
use super::policy_rig::{Member, Org, Run};
use super::rig::{PASSWORD, Rig, Tries, refused};

const DAY: i64 = 24 * 3600;

/// The moment `seconds` after `moment`.
pub(super) fn later(moment: Timestamp, seconds: i64) -> Timestamp {
    Timestamp::from_second(moment.as_second() + seconds).unwrap_or(moment)
}

/// What Bob may do in these tests: one permission of each kind a key keeps.
const CHECKER: [Permission; 4] = [
    Permission::SeeProjects,
    Permission::CheckAccess,
    Permission::MakeProjects,
    Permission::ManageInvitations,
];

/// A key of `owner` that runs out when it should, for the tests that use it over HTTP.
pub(super) async fn made(
    rig: &Rig,
    owner: &Member,
    choice: KeyChoice,
    expires_at: Option<String>,
) -> Run<CreatedAccessKey> {
    Ok(make_with(rig, owner, choice, &[], &[], expires_at).await?)
}

/// The secret of a new key of `owner`.
pub(super) async fn key_of(rig: &Rig, owner: &Member, choice: KeyChoice) -> Run<String> {
    Ok(made(rig, owner, choice, None).await?.secret)
}

/// A key of `owner`: the answer, or why it was refused.
pub(super) async fn make(
    rig: &Rig,
    owner: &Member,
    choice: KeyChoice,
    permissions: &[Permission],
    projects: &[ProjectId],
) -> Result<CreatedAccessKey, AppError> {
    make_with(rig, owner, choice, permissions, projects, None).await
}

async fn make_with(
    rig: &Rig,
    owner: &Member,
    choice: KeyChoice,
    permissions: &[Permission],
    projects: &[ProjectId],
    expires_at: Option<String>,
) -> Result<CreatedAccessKey, AppError> {
    Ok(rig
        .api()
        .make_access_key(
            Rig::info(),
            MakeAccessKey {
                session_token: owner.session.clone(),
                password: PASSWORD.to_owned(),
                name: "Nightly report".to_owned(),
                choice,
                permissions: permissions.to_vec(),
                projects: projects.to_vec(),
                expires_at,
            },
        )
        .await?
        .into_value())
}

/// *Check access* for Bob at a project, asked with `credential` (a key or a session).
pub(super) async fn check(
    rig: &Rig,
    credential: &str,
    person: bb_core::id::UserId,
    place: Option<ProjectId>,
) -> Result<(), AppError> {
    rig.api()
        .check_access(
            Rig::info(),
            CheckAccessQuery {
                session_token: credential.to_owned(),
                person,
                place,
            },
        )
        .await
        .map(|_| ())
}

/// A request that every key of a person with the role *Checker* may make, the one the tests of
/// keys use to see whether a key works: *Check access* for the administrator.
pub(super) async fn probe(rig: &Rig, credential: &str) -> Result<(), AppError> {
    let ada = rig
        .person_id(ADA_EMAIL)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    check(rig, credential, ada, None).await
}

/// *Make projects* at the top level, asked with `credential`.
async fn make_project(rig: &Rig, credential: &str, short_name: &str) -> Result<(), AppError> {
    rig.api()
        .make_project(
            Rig::info(),
            MakeProject {
                session_token: credential.to_owned(),
                parent: None,
                short_name: short_name.to_owned(),
                name: format!("Project {short_name}"),
            },
        )
        .await
        .map(|_| ())
}

/// *Invite people*'s list, asked with `credential`.
async fn invitations(rig: &Rig, credential: &str) -> Result<(), AppError> {
    rig.api()
        .invitations(
            Rig::info(),
            InvitationsQuery {
                session_token: credential.to_owned(),
                cursor: None,
                limit: 10,
            },
        )
        .await
        .map(|_| ())
}

/// Bob, with the role *Checker* for the whole organization.
pub(super) async fn bob(org: &Org<'_>) -> Run<Member> {
    let bob = org.person().await?;
    let role = org.role("Checker", &CHECKER).await?;
    org.give_everywhere(&bob, role.id).await?;
    Ok(bob)
}

async fn keys_of(rig: &Rig, owner: &Member) -> Run<Vec<AccessKeyView>> {
    Ok(rig
        .api()
        .access_keys(
            Rig::info(),
            AccessKeysQuery {
                session_token: owner.session.clone(),
                cursor: None,
                limit: 100,
            },
        )
        .await?
        .into_value()
        .items)
}

#[tokio::test]
async fn auth_07_a_key_is_shown_once_and_only_its_hash_is_stored() -> TestResult {
    each_database("auth_07_shown_once", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let created = make(&rig, &bob, KeyChoice::SeeOnly, &[], &[]).await?;
        let key = created.secret.clone();
        // `bb_`, the public prefix, `_` and 256 bits.
        assert!(key.starts_with("bb_"), "{key}");
        assert_eq!(key.len(), 3 + 8 + 1 + 52);
        assert_eq!(&key[3..11], created.key.prefix);
        // The list names the key by its prefix and holds no secret, now or later.
        let listed = keys_of(&rig, &bob).await?;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].prefix, created.key.prefix);
        assert!(!format!("{listed:?}").contains(&key[12..]));
        // What is stored is the hash of the secret, found by it, and not the key itself.
        let secret = AccessKeySecret::parse(&key).ok_or("not a key")?;
        let stored = bb_app::auth::AuthStore::access_key_by_hash(&rig.storage, &secret.hash())
            .await?
            .ok_or("the hash finds no key")?;
        assert_eq!(stored.key.id, created.key.id);
        assert_eq!(stored.key.owner, bob.id);
        assert_eq!(stored.key.prefix, created.key.prefix);
        assert_ne!(
            stored.key.secret_hash.as_bytes().as_slice(),
            key.as_bytes(),
            "the hash is not the key"
        );
        // No event and no entry of the security log holds the key, its secret or its hash.
        let consumer = TechnicalName::new("test-reader")?;
        let mut written = String::new();
        for (_, event) in rig.database.outbox_after(&consumer, 100_000).await? {
            written.push_str(&serde_json::to_string(&event)?);
        }
        for entry in rig.database.security_log_entries(0, 10_000).await? {
            write!(written, "{entry:?}")?;
        }
        assert!(written.contains("access_key"), "the change was recorded");
        for secret in [
            &key[3..],
            &key[12..],
            &format!("{:?}", stored.key.secret_hash),
        ] {
            if secret.len() > 8 {
                assert!(!written.contains(secret), "a secret was written down");
            }
        }
        // The key opens a door a person with this role may use.
        check(&rig, &key, bob.id, None).await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_a_key_does_what_its_choice_allows_and_never_more_than_its_owner() -> TestResult {
    each_database("auth_07_choice_and_owner", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let judge = |result: Result<(), AppError>| refused(result);
        // *See only*: *Check access* (See), not *Make projects* (Change) or *Invite people* (Manage).
        let see = make(&rig, &bob, KeyChoice::SeeOnly, &[], &[]).await?.secret;
        assert_eq!(judge(check(&rig, &see, bob.id, None).await), None);
        assert_eq!(
            judge(make_project(&rig, &see, "SEEONLY").await),
            Some(ErrorCode::Forbidden)
        );
        assert_eq!(
            judge(invitations(&rig, &see).await),
            Some(ErrorCode::Forbidden)
        );
        // *See and change* adds the kind *Change*.
        let change = make(&rig, &bob, KeyChoice::SeeAndChange, &[], &[])
            .await?
            .secret;
        assert_eq!(judge(check(&rig, &change, bob.id, None).await), None);
        assert_eq!(judge(make_project(&rig, &change, "CHANGE1").await), None);
        assert_eq!(
            judge(invitations(&rig, &change).await),
            Some(ErrorCode::Forbidden)
        );
        // *Everything I can do*: all of Bob's permissions, and no more (and never inviting people
        // or changing roles, whatever the owner holds).
        let all = make(&rig, &bob, KeyChoice::Everything, &[], &[])
            .await?
            .secret;
        assert_eq!(judge(check(&rig, &all, bob.id, None).await), None);
        assert_eq!(judge(make_project(&rig, &all, "ALLOFIT").await), None);
        // Not even the one that says everything invites people: that stays in the app.
        assert_eq!(
            judge(invitations(&rig, &all).await),
            Some(ErrorCode::Forbidden)
        );
        // Single permissions: only those.
        let single = make(
            &rig,
            &bob,
            KeyChoice::Permissions,
            &[Permission::MakeProjects],
            &[],
        )
        .await?
        .secret;
        assert_eq!(judge(make_project(&rig, &single, "SINGLE1").await), None);
        assert_eq!(
            judge(check(&rig, &single, bob.id, None).await),
            Some(ErrorCode::Forbidden)
        );
        // A key never holds more than its owner: Ada may see the status of the server, Bob may
        // not, and a key of Bob with that permission (a single permission, or everything) cannot
        // either.
        let greedy = make(
            &rig,
            &bob,
            KeyChoice::Permissions,
            &[Permission::SeeStatus],
            &[],
        )
        .await?
        .secret;
        let status = |credential: String| {
            let rig = &rig;
            async move {
                rig.api()
                    .see_server_status(
                        Rig::info(),
                        bb_app::auth::SeeServerStatusQuery {
                            session_token: credential,
                        },
                    )
                    .await
            }
        };
        assert_eq!(
            refused(status(greedy.clone()).await),
            Some(ErrorCode::Forbidden)
        );
        assert_eq!(
            refused(status(all.clone()).await),
            Some(ErrorCode::Forbidden)
        );
        // The keys of the administrator hold what she holds (and no more than keys may).
        let ada = make(&rig, &org.admin, KeyChoice::Everything, &[], &[])
            .await?
            .secret;
        assert!(status(ada).await.is_ok());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_a_key_for_projects_counts_in_them_and_their_sub_projects_only() -> TestResult {
    each_database("auth_07_key_projects", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let web = org.project(None, "WEB").await?;
        let api = org.project(Some(web.id), "API").await?;
        let other = org.project(None, "OTHER").await?;
        let key = make(&rig, &bob, KeyChoice::SeeOnly, &[], &[web.id])
            .await?
            .secret;
        // In the project and in its sub-project: yes.
        assert_eq!(refused(check(&rig, &key, bob.id, Some(web.id)).await), None);
        assert_eq!(refused(check(&rig, &key, bob.id, Some(api.id)).await), None);
        // In another project the key is not there at all, like a project that does not exist.
        assert_eq!(
            refused(check(&rig, &key, bob.id, Some(other.id)).await),
            Some(ErrorCode::NotFound)
        );
        // Nothing for the whole organization.
        assert_eq!(
            refused(check(&rig, &key, bob.id, None).await),
            Some(ErrorCode::Forbidden)
        );
        // Bob himself still sees both.
        assert_eq!(
            refused(check(&rig, &bob.session, bob.id, Some(other.id)).await),
            None
        );
        // The list names the project of the key.
        let listed = keys_of(&rig, &bob).await?;
        assert_eq!(listed[0].projects.len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_a_key_follows_its_owner_losing_a_role_at_once() -> TestResult {
    each_database("auth_07_follows_owner", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let bob = org.person().await?;
        let role = org.role("Checker", &CHECKER).await?;
        let row = org.give_everywhere(&bob, role.id).await?;
        let key = make(&rig, &bob, KeyChoice::Everything, &[], &[])
            .await?
            .secret;
        assert_eq!(refused(probe(&rig, &key).await), None);
        // The administrator takes the role away: the very next request of the key is refused,
        // and the key itself is still there.
        rig.api()
            .remove_assignment(
                Rig::info(),
                RemoveAssignment {
                    session_token: org.admin.session.clone(),
                    assignment: row.id,
                },
            )
            .await?;
        assert_eq!(refused(probe(&rig, &key).await), Some(ErrorCode::Forbidden));
        assert_eq!(keys_of(&rig, &bob).await?.len(), 1);
        // Given back, the key works again: it never held anything of its own.
        org.give_everywhere(&bob, role.id).await?;
        assert_eq!(refused(probe(&rig, &key).await), None);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_an_expired_a_deleted_and_an_unknown_key_get_the_same_answer() -> TestResult {
    each_database("auth_07_same_answer", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let works = make(&rig, &bob, KeyChoice::Everything, &[], &[]).await?;
        let expires = later(rig.clock_now(), 2 * DAY).to_string();
        let runs_out = make_with(&rig, &bob, KeyChoice::Everything, &[], &[], Some(expires))
            .await?
            .secret;
        let deleted = make(&rig, &bob, KeyChoice::Everything, &[], &[]).await?;
        rig.api()
            .delete_access_key(
                Rig::info(),
                DeleteAccessKey {
                    session_token: bob.session.clone(),
                    id: deleted.key.id,
                },
            )
            .await?;
        // A key that was never made, in the right form; and things that are not keys at all.
        let unknown = AccessKeySecret::generate()?.encode().as_str().to_owned();
        assert_eq!(refused(probe(&rig, &runs_out).await), None);
        // Three days on, the key has run out.
        rig.clock.advance(3 * DAY);
        let answers = [
            probe(&rig, &runs_out).await,
            probe(&rig, &deleted.secret).await,
            probe(&rig, &unknown).await,
            probe(&rig, "bb_nonsense").await,
            probe(&rig, "bb_").await,
        ];
        for answer in &answers {
            assert_eq!(answer, &answers[0], "the answers differ");
        }
        assert_eq!(answers[0], Err(AppError::Refused(ErrorCode::NotSignedIn)));
        // The key that did not run out keeps working (the session of the owner ran out meanwhile,
        // a key does not need it).
        assert_eq!(refused(probe(&rig, &works.secret).await), None);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_the_prefix_of_a_key_has_to_be_the_stored_one() -> TestResult {
    each_database("auth_07_prefix_is_compared", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let key = make(&rig, &bob, KeyChoice::Everything, &[], &[]).await?;
        assert_eq!(refused(probe(&rig, &key.secret).await), None);
        // The right secret under another public prefix is not the key: one text only.
        let (start, secret) = key.secret.split_at(3 + 8);
        let other = if &start[3..4] == "0" { "1" } else { "0" };
        let altered = format!("bb_{other}{}{secret}", &start[4..]);
        assert_ne!(altered, key.secret);
        assert_eq!(
            refused(probe(&rig, &altered).await),
            Some(ErrorCode::NotSignedIn)
        );
        Ok(())
    })
    .await
}

/// Every use case of the sign-in data (password, second step, sessions, keys) turns the key away.
async fn the_sign_in_data_turns_the_key_away(rig: &Rig, key: &str) -> Run<()> {
    let api = rig.api();
    let not_allowed = Some(ErrorCode::AccessKeyNotAllowed);
    let info = Rig::info;
    let token = || key.to_owned();
    // The password: not changed, not even with the right one.
    let changed = api
        .change_password(
            info(),
            ChangePassword {
                session_token: token(),
                current_password: PASSWORD.to_owned(),
                new_password: "Another-Long-Password-7".to_owned(),
            },
        )
        .await;
    assert_eq!(refused(changed), not_allowed);
    // The second step, sessions and the keys themselves.
    let started = api
        .start_code_app(
            info(),
            StartCodeApp {
                session_token: token(),
                password: PASSWORD.to_owned(),
            },
        )
        .await;
    assert_eq!(refused(started), not_allowed);
    let status = api
        .two_step(
            info(),
            TwoStepQuery {
                session_token: token(),
            },
        )
        .await;
    assert_eq!(refused(status), not_allowed);
    let listed = api
        .sessions(
            info(),
            bb_app::auth::SessionsQuery {
                session_token: token(),
            },
        )
        .await;
    assert_eq!(refused(listed), not_allowed);
    let ended = api
        .sign_out_everywhere(
            info(),
            bb_app::auth::SignOutEverywhere {
                session_token: token(),
            },
        )
        .await;
    assert_eq!(refused(ended), not_allowed);
    let out = api
        .sign_out(
            info(),
            bb_app::auth::SignOut {
                session_token: token(),
            },
        )
        .await;
    assert_eq!(refused(out), not_allowed);
    let who = api
        .authenticate(
            info(),
            bb_app::auth::AuthenticateQuery {
                session_token: token(),
            },
        )
        .await;
    assert_eq!(refused(who), not_allowed);
    let make_another = api
        .make_access_key(
            info(),
            MakeAccessKey {
                session_token: token(),
                password: PASSWORD.to_owned(),
                name: "Another".to_owned(),
                choice: KeyChoice::Everything,
                permissions: vec![],
                projects: vec![],
                expires_at: None,
            },
        )
        .await;
    assert_eq!(refused(make_another), not_allowed);
    let own = api
        .access_keys(
            info(),
            AccessKeysQuery {
                session_token: token(),
                cursor: None,
                limit: 10,
            },
        )
        .await;
    assert_eq!(refused(own), not_allowed);
    Ok(())
}

#[tokio::test]
async fn auth_07_a_key_cannot_change_the_owners_sign_in_data() -> TestResult {
    each_database("auth_07_sign_in_data", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let key = make(&rig, &bob, KeyChoice::Everything, &[], &[])
            .await?
            .secret;
        the_sign_in_data_turns_the_key_away(&rig, &key).await?;
        // Nothing changed: the password is the old one, there is still one key, the session
        // still works, and the person has no second step set up.
        rig.sign_in("person1@example.test", PASSWORD).await?;
        assert_eq!(keys_of(&rig, &bob).await?.len(), 1);
        assert!(rig.permissions(&bob.session).await.is_ok());
        let status = rig
            .api()
            .two_step(
                Rig::info(),
                TwoStepQuery {
                    session_token: bob.session.clone(),
                },
            )
            .await?;
        assert!(!status.code_app && status.passkeys.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_making_a_key_checks_the_password_the_name_and_the_choice() -> TestResult {
    each_database("auth_07_make_rules", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let web = org.project(None, "WEB").await?;
        let invalid = Some(ErrorCode::InvalidRequest);
        // A wrong password: nothing is made.
        let wrong = rig
            .api()
            .make_access_key(
                Rig::info(),
                MakeAccessKey {
                    session_token: bob.session.clone(),
                    password: "Wrong-Password-1".to_owned(),
                    name: "Report".to_owned(),
                    choice: KeyChoice::SeeOnly,
                    permissions: vec![],
                    projects: vec![],
                    expires_at: None,
                },
            )
            .await;
        assert_eq!(refused(wrong), Some(ErrorCode::WrongCredentials));
        // A name that is empty, or too long.
        for name in ["", "   ", &"x".repeat(101)] {
            let result = rig
                .api()
                .make_access_key(
                    Rig::info(),
                    MakeAccessKey {
                        session_token: bob.session.clone(),
                        password: PASSWORD.to_owned(),
                        name: name.to_owned(),
                        choice: KeyChoice::SeeOnly,
                        permissions: vec![],
                        projects: vec![],
                        expires_at: None,
                    },
                )
                .await;
            assert_eq!(refused(result), invalid, "{name:?}");
        }
        // Single permissions only with that choice, and at least one then.
        let none = make(&rig, &bob, KeyChoice::Permissions, &[], &[]).await;
        assert_eq!(refused(none), invalid);
        let both = make(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[Permission::SeeProjects],
            &[],
        )
        .await;
        assert_eq!(refused(both), invalid);
        // A project that does not exist, and a project that Bob may not know about.
        let missing = make(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[ProjectId::from_parts(rig.clock_now(), [1; 10])?],
        )
        .await;
        assert_eq!(refused(missing), Some(ErrorCode::NotFound));
        org.forbid(&bob, web.id).await?;
        let hidden = make(&rig, &bob, KeyChoice::SeeOnly, &[], &[web.id]).await;
        assert_eq!(refused(hidden), Some(ErrorCode::NotFound));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_a_key_lives_90_days_unless_a_date_is_picked_and_never_beyond_the_longest_time()
-> TestResult {
    each_database("auth_07_make_dates", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let invalid = Some(ErrorCode::InvalidRequest);
        // The default is 90 days; a date picked in the past, or beyond a year, is refused.
        let now = rig.clock_now();
        let made = make(&rig, &bob, KeyChoice::SeeOnly, &[], &[]).await?;
        assert_eq!(made.key.expires_at, later(now, 90 * DAY));
        let past = make_with(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[],
            Some(later(now, -DAY).to_string()),
        )
        .await;
        assert_eq!(refused(past), invalid);
        let too_late = make_with(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[],
            Some(later(now, 366 * DAY).to_string()),
        )
        .await;
        assert_eq!(refused(too_late), invalid);
        let not_a_date = make_with(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[],
            Some("tomorrow".to_owned()),
        )
        .await;
        assert_eq!(refused(not_a_date), invalid);
        let in_a_year = make_with(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[],
            Some(later(now, 365 * DAY).to_string()),
        )
        .await?;
        assert_eq!(in_a_year.key.expires_at, later(now, 365 * DAY));
        // The organization shortens the longest time: the default follows it, and a date beyond
        // it is refused.
        for (key, days) in [
            ("auth.access-key-longest-days", 30),
            ("auth.access-key-default-days", 60),
        ] {
            rig.set_organization_setting(key, days).await?;
        }
        let capped = make(&rig, &bob, KeyChoice::SeeOnly, &[], &[]).await?;
        assert_eq!(capped.key.expires_at, later(now, 30 * DAY));
        let beyond = make_with(
            &rig,
            &bob,
            KeyChoice::SeeOnly,
            &[],
            &[],
            Some(later(now, 31 * DAY).to_string()),
        )
        .await;
        assert_eq!(refused(beyond), invalid);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_only_the_owner_lists_and_deletes_a_key_and_a_deleted_key_is_gone() -> TestResult {
    each_database("auth_07_owner_only", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let key = make(&rig, &bob, KeyChoice::Everything, &[], &[]).await?;
        // The administrator has no key and cannot see or delete Bob's.
        assert!(keys_of(&rig, &org.admin).await?.is_empty());
        let result = rig
            .api()
            .delete_access_key(
                Rig::info(),
                DeleteAccessKey {
                    session_token: org.admin.session.clone(),
                    id: key.key.id,
                },
            )
            .await;
        assert_eq!(refused(result), Some(ErrorCode::NotFound));
        assert_eq!(keys_of(&rig, &bob).await?.len(), 1);
        assert_eq!(refused(probe(&rig, &key.secret).await), None);
        // Bob deletes it: at once, and for good; deleting it again finds nothing.
        let delete = || DeleteAccessKey {
            session_token: bob.session.clone(),
            id: key.key.id,
        };
        rig.api().delete_access_key(Rig::info(), delete()).await?;
        assert_eq!(
            refused(probe(&rig, &key.secret).await),
            Some(ErrorCode::NotSignedIn)
        );
        assert!(keys_of(&rig, &bob).await?.is_empty());
        let again = rig.api().delete_access_key(Rig::info(), delete()).await;
        assert_eq!(refused(again), Some(ErrorCode::NotFound));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_15_making_and_deleting_keys_is_in_the_security_log() -> TestResult {
    each_database("sec_15_keys_in_the_log", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let before = rig.database.security_log_entries(0, 1000).await?.len();
        let key = make(&rig, &bob, KeyChoice::SeeOnly, &[], &[]).await?;
        rig.api()
            .delete_access_key(
                Rig::info(),
                DeleteAccessKey {
                    session_token: bob.session.clone(),
                    id: key.key.id,
                },
            )
            .await?;
        // A key that does not work, from a network: the log holds the network, cut short, and
        // neither the key nor its prefix (the prefix of a key that is not known least of all).
        let unknown = AccessKeySecret::generate()?;
        let from = RequestInfo {
            network: NetworkPrefix::from_stored("203.0.113.0"),
            ..RequestInfo::default()
        };
        let result = rig
            .api()
            .invitations(
                from.clone(),
                InvitationsQuery {
                    session_token: unknown.encode().as_str().to_owned(),
                    cursor: None,
                    limit: 10,
                },
            )
            .await;
        assert_eq!(refused(result), Some(ErrorCode::NotSignedIn));
        // The same key of the deleted one is refused too.
        let result = rig
            .api()
            .invitations(
                from,
                InvitationsQuery {
                    session_token: key.secret.clone(),
                    cursor: None,
                    limit: 10,
                },
            )
            .await;
        assert_eq!(refused(result), Some(ErrorCode::NotSignedIn));
        let entries = rig.database.security_log_entries(0, 1000).await?;
        let new: Vec<_> = entries.iter().skip(before).collect();
        let actions: Vec<&str> = new.iter().map(|entry| entry.action.as_str()).collect();
        assert_eq!(
            actions,
            [
                SecurityAction::AccessKeyMade.code(),
                SecurityAction::AccessKeyDeleted.code(),
                SecurityAction::AccessKeyRefused.code(),
                SecurityAction::AccessKeyRefused.code(),
            ]
        );
        // Making and deleting name the person and the key; a refused key names nobody.
        assert_eq!(new[0].person, Some(bob.id));
        assert_eq!(new[1].person, Some(bob.id));
        assert!(new[0].subject.is_some() && new[0].subject == new[1].subject);
        for refused in &new[2..] {
            assert_eq!(refused.person, None);
            assert_eq!(refused.subject, None);
            assert_eq!(
                refused.network.as_ref().map(ToString::to_string).as_deref(),
                Some("203.0.113.0")
            );
        }
        let text = format!("{entries:?}");
        for hidden in [
            unknown.prefix(),
            &unknown.encode(),
            key.key.prefix.as_str(),
            &key.secret,
        ] {
            assert!(!text.contains(hidden), "{hidden} is in the security log");
        }
        // The chain still verifies.
        assert!(matches!(
            rig.database.verify_security_log(&rig.seal).await?,
            Verification::Valid { .. }
        ));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_14_guessing_keys_is_limited_per_network_and_a_working_key_costs_nothing() -> TestResult
{
    each_database("sec_14_guessing_keys", async |db| {
        // Three failed tries per network and minute.
        let rig = Rig::start_with(db, Tries::Few).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let key = make(&rig, &bob, KeyChoice::Everything, &[], &[])
            .await?
            .secret;
        // A working key does not use anything up, however often it is used.
        for _ in 0..30 {
            assert!(probe(&rig, &key).await.is_ok());
        }
        // Wrong keys do: after three, a guess from the network waits; a key that works does not.
        for _ in 0..3 {
            let guess = AccessKeySecret::generate()?.encode().as_str().to_owned();
            assert_eq!(
                refused(probe(&rig, &guess).await),
                Some(ErrorCode::NotSignedIn)
            );
        }
        let guess = AccessKeySecret::generate()?.encode().as_str().to_owned();
        assert!(matches!(
            probe(&rig, &guess).await,
            Err(AppError::Limited { .. })
        ));
        assert!(probe(&rig, &key).await.is_ok());
        // The sessions of the people on that network are not affected.
        assert!(rig.permissions(&bob.session).await.is_ok());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_the_last_use_of_a_key_is_kept_to_the_minute() -> TestResult {
    each_database("auth_07_last_use", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let key = make(&rig, &bob, KeyChoice::Everything, &[], &[])
            .await?
            .secret;
        assert_eq!(keys_of(&rig, &bob).await?[0].last_used_at, None);
        // The clock of the tests starts on a full minute: 37 seconds in, the minute it is.
        let start = rig.clock_now();
        rig.clock.advance(37);
        assert_eq!(refused(probe(&rig, &key).await), None);
        assert_eq!(keys_of(&rig, &bob).await?[0].last_used_at, Some(start));
        // Within the same minute nothing changes; in the next one the minute moves on.
        rig.clock.advance(10);
        assert_eq!(refused(probe(&rig, &key).await), None);
        assert_eq!(keys_of(&rig, &bob).await?[0].last_used_at, Some(start));
        rig.clock.advance(30);
        assert_eq!(refused(probe(&rig, &key).await), None);
        assert_eq!(
            keys_of(&rig, &bob).await?[0].last_used_at,
            Some(later(start, 60))
        );
        Ok(())
    })
    .await
}

/// The short names of the projects a person may choose for a key, a page at a time.
async fn choices(rig: &Rig, who: &Member, limit: u32) -> Run<Vec<Vec<String>>> {
    let mut pages = Vec::new();
    let mut cursor = None;
    loop {
        let page = rig
            .api()
            .project_choices(
                Rig::info(),
                ProjectChoicesQuery {
                    session_token: who.session.clone(),
                    cursor: cursor.clone(),
                    limit,
                },
            )
            .await?
            .into_value();
        pages.push(
            page.items
                .iter()
                .map(|project| project.short_name.clone())
                .collect(),
        );
        cursor = page.next_cursor;
        if cursor.is_none() {
            return Ok(pages);
        }
        assert!(pages.len() < 20, "the pages do not end");
    }
}

#[tokio::test]
async fn auth_07_a_person_chooses_the_projects_of_a_key_from_those_they_know_about() -> TestResult {
    each_database("auth_07_project_choices", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let web = org.project(None, "WEB").await?;
        org.project(Some(web.id), "API").await?;
        org.project(None, "OTHER").await?;
        org.project(None, "ZEBRA").await?;
        // Bob sees every project: by short name, in one page or in several.
        assert_eq!(
            choices(&rig, &bob, 50).await?,
            [["API", "OTHER", "WEB", "ZEBRA"]]
        );
        assert_eq!(
            choices(&rig, &bob, 3).await?,
            [vec!["API", "OTHER", "WEB"], vec!["ZEBRA"]]
        );
        // Somebody who holds a role in one project only knows that project and what is inside it.
        let role = org.role("Reader", &[Permission::SeeProjects]).await?;
        let dora = org.person().await?;
        org.give(&dora, role.id, web.id).await?;
        assert_eq!(choices(&rig, &dora, 50).await?, [["API", "WEB"]]);
        // Somebody without a role knows none, and asks for no cursor that is not one.
        let eve = org.person().await?;
        assert_eq!(choices(&rig, &eve, 50).await?, [Vec::<String>::new()]);
        let bad = rig
            .api()
            .project_choices(
                Rig::info(),
                ProjectChoicesQuery {
                    session_token: bob.session.clone(),
                    cursor: Some("not a count".to_owned()),
                    limit: 10,
                },
            )
            .await;
        assert_eq!(refused(bad), Some(ErrorCode::InvalidRequest));
        // A key cannot ask: choosing is part of making a key.
        let key = make(&rig, &bob, KeyChoice::Everything, &[], &[])
            .await?
            .secret;
        let by_key = rig
            .api()
            .project_choices(
                Rig::info(),
                ProjectChoicesQuery {
                    session_token: key,
                    cursor: None,
                    limit: 10,
                },
            )
            .await;
        assert_eq!(refused(by_key), Some(ErrorCode::AccessKeyNotAllowed));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn auth_07_the_rules_of_a_key_name_its_days_and_the_permissions_to_choose_from() -> TestResult
{
    each_database("auth_07_key_rules", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let ask = || async {
            rig.api()
                .access_key_rules(
                    Rig::info(),
                    AccessKeyRulesQuery {
                        session_token: org.admin.session.clone(),
                    },
                )
                .await
                .map(bb_app::access::Authorized::into_value)
        };
        let rules = ask().await?;
        assert_eq!((rules.default_days, rules.longest_days), (90, 365));
        // Every permission of the catalogue that a key may hold, once, in its order, with its
        // topic: not inviting people or changing roles (`NOT_FOR_KEYS`).
        let keys: Vec<Permission> = rules
            .permissions
            .iter()
            .map(|(permission, _)| *permission)
            .collect();
        let may_be_held: Vec<Permission> = Permission::ALL
            .iter()
            .copied()
            .filter(|permission| permission.keys_may_hold())
            .collect();
        assert_eq!(keys, may_be_held);
        assert!(keys.len() < Permission::ALL.len());
        assert!(
            rules
                .permissions
                .iter()
                .all(|(permission, topic)| permission.topic() == *topic)
        );
        // The default is never more than the longest time.
        rig.set_organization_setting("auth.access-key-longest-days", 30)
            .await?;
        let rules = ask().await?;
        assert_eq!((rules.default_days, rules.longest_days), (30, 30));
        Ok(())
    })
    .await
}
