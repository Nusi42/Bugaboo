//! What a key may do: its choice, its projects, its owner's roles and the answer for a key that does not work (AUTH-07).

use super::*;

#[tokio::test]
async fn auth_07_a_key_is_shown_once_and_only_its_hash_is_stored() -> TestResult {
    each_database("auth_07_shown_once", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let created = create(&rig, &bob, KeyChoice::SeeOnly, &[], &[]).await?;
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
        let stored = bb_app::auth::AccessKeyStore::access_key_by_hash(&rig.storage, &secret.hash())
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
        // *See only*: *Check access* (See), not *Create projects* (Change) or *Invite people* (Manage).
        let see = create(&rig, &bob, KeyChoice::SeeOnly, &[], &[])
            .await?
            .secret;
        assert_eq!(judge(check(&rig, &see, bob.id, None).await), None);
        assert_eq!(
            judge(create_project(&rig, &see, "SEEONLY").await),
            Some(ErrorCode::Forbidden)
        );
        assert_eq!(
            judge(invitations(&rig, &see).await),
            Some(ErrorCode::Forbidden)
        );
        // *See and change* adds the kind *Change*.
        let change = create(&rig, &bob, KeyChoice::SeeAndChange, &[], &[])
            .await?
            .secret;
        assert_eq!(judge(check(&rig, &change, bob.id, None).await), None);
        assert_eq!(judge(create_project(&rig, &change, "CHANGE1").await), None);
        assert_eq!(
            judge(invitations(&rig, &change).await),
            Some(ErrorCode::Forbidden)
        );
        // *Everything I can do*: all of Bob's permissions, and no more (and never inviting people
        // or changing roles, whatever the owner holds).
        let all = create(&rig, &bob, KeyChoice::Everything, &[], &[])
            .await?
            .secret;
        assert_eq!(judge(check(&rig, &all, bob.id, None).await), None);
        assert_eq!(judge(create_project(&rig, &all, "ALLOFIT").await), None);
        // Not even the one that says everything invites people: that stays in the app.
        assert_eq!(
            judge(invitations(&rig, &all).await),
            Some(ErrorCode::Forbidden)
        );
        // Single permissions: only those.
        let single = create(
            &rig,
            &bob,
            KeyChoice::Permissions,
            &[Permission::CreateProjects],
            &[],
        )
        .await?
        .secret;
        assert_eq!(judge(create_project(&rig, &single, "SINGLE1").await), None);
        assert_eq!(
            judge(check(&rig, &single, bob.id, None).await),
            Some(ErrorCode::Forbidden)
        );
        // A key never holds more than its owner: Ada may see the status of the server, Bob may
        // not, and a key of Bob with that permission (a single permission, or everything) cannot
        // either.
        let greedy = create(
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
        let ada = create(&rig, &org.admin, KeyChoice::Everything, &[], &[])
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
        let key = create(&rig, &bob, KeyChoice::SeeOnly, &[], &[web.id])
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
        let key = create(&rig, &bob, KeyChoice::Everything, &[], &[])
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
        let works = create(&rig, &bob, KeyChoice::Everything, &[], &[]).await?;
        let expires = later(rig.clock_now(), 2 * DAY).to_string();
        let runs_out = create_with(&rig, &bob, KeyChoice::Everything, &[], &[], Some(expires))
            .await?
            .secret;
        let deleted = create(&rig, &bob, KeyChoice::Everything, &[], &[]).await?;
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
        let key = create(&rig, &bob, KeyChoice::Everything, &[], &[]).await?;
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
