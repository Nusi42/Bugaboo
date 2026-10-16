//! Listing and deleting keys, the security log, the limit on guessing and the last use (AUTH-07, SEC-14, SEC-15).

use super::*;

#[tokio::test]
async fn auth_07_only_the_owner_lists_and_deletes_a_key_and_a_deleted_key_is_gone() -> TestResult {
    each_database("auth_07_owner_only", async |db| {
        let rig = Rig::start_with(db, Tries::Roomy).await?;
        let org = Org::start(&rig).await?;
        let bob = bob(&org).await?;
        let key = create(&rig, &bob, KeyChoice::Everything, &[], &[]).await?;
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
        let key = create(&rig, &bob, KeyChoice::SeeOnly, &[], &[]).await?;
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
        let key = create(&rig, &bob, KeyChoice::Everything, &[], &[])
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
        let key = create(&rig, &bob, KeyChoice::Everything, &[], &[])
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
