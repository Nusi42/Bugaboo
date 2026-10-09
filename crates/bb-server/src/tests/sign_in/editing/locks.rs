//! Edit locks, drafts and saving, part: locks (ISS-17, ISS-11, ADR-0019).

use super::*;

#[tokio::test]
async fn iss_17_a_second_person_sees_who_is_editing_or_someone() -> TestResult {
    on_both_databases("iss_17_who_is_editing", async |stage| {
        let (ada_token, ada_editor) = stage.lock(stage.ada()).await?;
        // Ben sees editors as *Someone*, Cleo by name.
        let LockOutcome::Held(for_ben) = stage
            .take(&stage.ben, EditorId::generate(&mut SystemIds))
            .await?
        else {
            return Err("Ben got the lock".into());
        };
        assert_eq!(for_ben.holder, PersonRef::Someone);
        assert!(!for_ben.can_edit_anyway && !for_ben.yours_elsewhere);
        let LockOutcome::Held(for_cleo) = stage
            .take(&stage.cleo, EditorId::generate(&mut SystemIds))
            .await?
        else {
            return Err("Cleo got the lock".into());
        };
        assert_eq!(for_cleo.holder, shown_as(stage.ada(), "Ada Admin"));
        // Dan may take the editing over, and is told so.
        let LockOutcome::Held(for_dan) = stage
            .take(&stage.dan, EditorId::generate(&mut SystemIds))
            .await?
        else {
            return Err("Dan got the lock".into());
        };
        assert!(for_dan.can_edit_anyway);
        // The same person in another window is told it is their own.
        let LockOutcome::Held(elsewhere) = stage
            .take(stage.ada(), EditorId::generate(&mut SystemIds))
            .await?
        else {
            return Err("Ada got a second lock".into());
        };
        assert!(elsewhere.yours_elsewhere);
        // The same editor asking again keeps its token.
        let LockOutcome::Taken(again) = stage.take(stage.ada(), ada_editor).await? else {
            return Err("Ada lost her own lock".into());
        };
        assert_eq!(again.token, ada_token);
        // Nobody who may not change the note gets a lock, or learns anything about it.
        let outsider = stage.org.person().await?;
        let unknown = refused(
            stage
                .take(&outsider, EditorId::generate(&mut SystemIds))
                .await,
        );
        assert_eq!(unknown, Some(ErrorCode::NotFound));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_a_lock_frees_itself_a_minute_after_the_last_heartbeat() -> TestResult {
    on_both_databases("iss_17_the_lease", async |stage| {
        let (token, _) = stage.lock(stage.ada()).await?;
        let editor = || EditorId::generate(&mut SystemIds);
        // A heartbeat every 15 seconds keeps the lock for as long as they come.
        for _ in 0..8 {
            stage.pass(HEARTBEAT_SECONDS);
            let beat = stage.heartbeat(stage.ada(), token, None).await?;
            assert!(matches!(beat.lock, LockStatus::Held { .. }), "{beat:?}");
            assert!(matches!(
                stage.take(&stage.ben, editor()).await?,
                LockOutcome::Held(_)
            ));
        }
        // The page is closed: the last heartbeat was a moment ago. Just before a minute is over the
        // lock is still Ada's, at the minute it is free.
        stage.pass(LEASE_SECONDS - 1);
        assert!(matches!(
            stage.take(&stage.ben, editor()).await?,
            LockOutcome::Held(_)
        ));
        stage.pass(1);
        let LockOutcome::Taken(bens) = stage.take(&stage.ben, editor()).await? else {
            return Err("the lock did not free itself".into());
        };
        // Ada comes back: the lock is somebody else's now, and her text is kept, not dropped.
        let late = stage
            .heartbeat(stage.ada(), token, Some("Ada's text"))
            .await?;
        assert!(
            matches!(late.lock, LockStatus::TakenOver { .. }),
            "{late:?}"
        );
        assert!(late.draft_saved);
        let beat = stage.heartbeat(&stage.ben, bens.token, None).await?;
        assert!(matches!(beat.lock, LockStatus::Held { .. }));
        // Nobody holds it after the editor is closed, and a lock that ran out with nobody after
        // it is told so.
        stage.pass(LEASE_SECONDS);
        let ran_out = stage.heartbeat(&stage.ben, bens.token, None).await?;
        assert_eq!(ran_out.lock, LockStatus::RanOut);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_letting_go_frees_the_lock_at_once_and_only_for_its_token() -> TestResult {
    on_both_databases("iss_17_release", async |stage| {
        let (token, _) = stage.lock(stage.ada()).await?;
        let release = |by: &Member, token: LockToken| {
            stage.rig().api().release_lock(
                Rig::info(),
                ReleaseLockQuery {
                    session_token: by.session.clone(),
                    what: stage.what.clone(),
                    token,
                },
            )
        };
        // Another token holds nothing: the lock stays.
        release(&stage.ben, LockToken::generate(&mut SystemIds)).await?;
        assert!(matches!(
            stage
                .take(&stage.ben, EditorId::generate(&mut SystemIds))
                .await?,
            LockOutcome::Held(_)
        ));
        release(stage.ada(), token).await?;
        assert!(matches!(
            stage
                .take(&stage.ben, EditorId::generate(&mut SystemIds))
                .await?,
            LockOutcome::Taken(_)
        ));
        // Letting go twice is no error.
        release(stage.ada(), token).await?;
        // Letting go is no change anybody reads in the event log, and no entry of the security log.
        let events = stage.rig().database.events_about(&stage.note, 100).await?;
        assert!(events.is_empty(), "{events:?}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_17_releasing_a_stuck_lock_is_in_the_security_log() -> TestResult {
    on_both_databases("sec_17_stuck_lock", async |stage| {
        let release = |by: &Member| {
            stage.rig().api().release_stuck_lock(
                Rig::info(),
                ReleaseStuckLock {
                    session_token: by.session.clone(),
                    what: stage.what.clone(),
                },
            )
        };
        let logged = async || -> Result<Vec<_>, Failure> {
            Ok(stage
                .rig()
                .database
                .security_log_entries(0, 10_000)
                .await?
                .into_iter()
                .filter(|entry| entry.action.as_str() == SecurityAction::EditLockReleased.code())
                .collect())
        };
        let (token, _) = stage.lock(&stage.ben).await?;
        // Without *Edit anyway* nothing is released.
        assert_eq!(
            refused(release(&stage.cleo).await),
            Some(ErrorCode::Forbidden)
        );
        assert!(logged().await?.is_empty());
        release(&stage.dan).await?;
        let entries = logged().await?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].person, Some(stage.dan.id));
        assert_eq!(entries[0].subject.as_ref(), Some(&stage.note));
        // The lock is gone; Ben's heartbeat says so.
        let beat = stage.heartbeat(&stage.ben, token, None).await?;
        assert_eq!(beat.lock, LockStatus::RanOut);
        // Nothing stuck: nothing to release, and nothing logged.
        assert_eq!(
            refused(release(&stage.dan).await),
            Some(ErrorCode::NotFound)
        );
        assert_eq!(logged().await?.len(), 1);
        // Letting go of one's own lock is not an administrator's act.
        let (token, _) = stage.lock(&stage.ben).await?;
        stage
            .rig()
            .api()
            .release_lock(
                Rig::info(),
                ReleaseLockQuery {
                    session_token: stage.ben.session.clone(),
                    what: stage.what.clone(),
                    token,
                },
            )
            .await?;
        assert_eq!(logged().await?.len(), 1);
        // A lease that ran out is not somebody's editing any more: it is cleared, with no entry
        // in the log and no note.
        let (token, _) = stage.lock(&stage.ben).await?;
        stage.pass(LEASE_SECONDS);
        release(&stage.dan).await?;
        assert_eq!(logged().await?.len(), 1);
        let beat = stage.heartbeat(&stage.ben, token, None).await?;
        assert!(beat.notices.is_empty());
        // The person's own lock, released the administrator's way, is not logged either.
        stage.lock(&stage.dan).await?;
        release(&stage.dan).await?;
        assert_eq!(logged().await?.len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_the_person_whose_lock_was_released_is_told_who_released_it() -> TestResult {
    on_both_databases("iss_17_released_note", async |stage| {
        let (token, _) = stage.lock(&stage.ben).await?;
        stage
            .rig()
            .api()
            .release_stuck_lock(
                Rig::info(),
                ReleaseStuckLock {
                    session_token: stage.dan.session.clone(),
                    what: stage.what.clone(),
                },
            )
            .await?;
        let beat = stage.heartbeat(&stage.ben, token, None).await?;
        assert_eq!(beat.lock, LockStatus::RanOut);
        assert_eq!(beat.notices.len(), 1);
        assert_eq!(beat.notices[0].kind, NoticeKind::Released);
        // Ben sees editors as *Someone*: the note is made with the people kind of editing.
        assert_eq!(beat.notices[0].from, PersonRef::Someone);
        // Each note is told once.
        let again = stage.heartbeat(&stage.ben, token, None).await?;
        assert!(again.notices.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_a_token_is_no_use_to_another_person() -> TestResult {
    on_both_databases("iss_17_token_holder", async |stage| {
        let (token, _) = stage.lock(&stage.ben).await?;
        // Cleo has Ben's token, but she is not the holder: her heartbeat does not renew the lease
        // and her release does not let go of it.
        stage.pass(LEASE_SECONDS - 10);
        let beat = stage.heartbeat(&stage.cleo, token, None).await?;
        assert!(matches!(beat.lock, LockStatus::TakenOver { .. }));
        stage.let_go(&stage.cleo, token).await?;
        stage.pass(5);
        let own = stage.heartbeat(&stage.ben, token, None).await?;
        assert!(
            matches!(own.lock, LockStatus::Held { .. }),
            "{:?}",
            own.lock
        );
        // Nor does a save with it: nothing is written and Ben's lock stays.
        let error = stage
            .save(&stage.cleo, token, "Cleo's text", 1)
            .await
            .unwrap_err();
        assert_eq!(error.code(), Some(ErrorCode::EditTakenOver));
        assert_eq!(stage.notes.version(&stage.note), Some(1));
        stage.save(&stage.ben, token, "Ben's text", 1).await?;
        assert_eq!(stage.notes.version(&stage.note), Some(2));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_checking_versions_answers_the_versions_of_what_is_on_the_screen() -> TestResult {
    on_both_databases("iss_17_check_versions", async |stage| {
        let project = EntityRef {
            kind: bb_core::event::RecordKind::Project.into(),
            id: EntityId::from(stage.project.id),
        };
        let nowhere = EntityRef {
            kind: stage.note.kind.clone(),
            id: EntityId::from(UserId::generate(&mut SystemIds)),
        };
        let asked = vec![stage.note.clone(), project, nowhere];
        let versions = |checked: Versions| {
            checked
                .items
                .iter()
                .map(|item| item.version)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            versions(stage.check_versions(&stage.ben, asked.clone()).await?),
            [Some(1), Some(stage.project.version), None]
        );
        stage.notes.change(&stage.note);
        assert_eq!(
            versions(stage.check_versions(&stage.ben, asked.clone()).await?)[0],
            Some(2)
        );
        // Somebody who may not know the project sees no version of it.
        let outsider = stage.org.person().await?;
        assert_eq!(
            versions(stage.check_versions(&outsider, asked).await?),
            [None, None, None]
        );
        // Only so many at once.
        let many = vec![stage.note.clone(); usize::try_from(VERSIONS_MAX)? + 1];
        assert_eq!(
            refused(stage.check_versions(&stage.ben, many).await),
            Some(ErrorCode::TooLarge)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_edit_anyway_keeps_the_first_persons_text_as_their_draft() -> TestResult {
    on_both_databases("iss_17_edit_anyway", async |stage| {
        let (token, _) = stage.lock(&stage.ben).await?;
        stage
            .heartbeat(&stage.ben, token, Some("Ben's long text"))
            .await?;
        // Somebody without the permission cannot take over.
        assert_eq!(
            refused(stage.edit_anyway(&stage.cleo).await),
            Some(ErrorCode::Forbidden)
        );
        // Cleo asks Ben to finish; Ben reads it at his next heartbeat, once.
        stage
            .rig()
            .api()
            .ask_to_finish(
                Rig::info(),
                AskToFinish {
                    session_token: stage.cleo.session.clone(),
                    what: stage.what.clone(),
                },
            )
            .await?;
        let asked = stage.heartbeat(&stage.ben, token, None).await?;
        assert_eq!(asked.notices.len(), 1);
        assert_eq!(asked.notices[0].kind, NoticeKind::FinishAsked);
        assert_eq!(
            asked.notices[0].from,
            PersonRef::Someone,
            "Ben sees editors as Someone"
        );
        assert!(
            stage
                .heartbeat(&stage.ben, token, None)
                .await?
                .notices
                .is_empty()
        );
        // Dan takes over.
        let LockOutcome::Taken(dans) = stage.edit_anyway(&stage.dan).await? else {
            return Err("Dan did not get the lock".into());
        };
        // Ben's text is his draft still, and nobody else reads it.
        let kept = stage.draft(&stage.ben, &stage.describing()).await?;
        assert_eq!(
            kept.draft.map(|draft| draft.text).as_deref(),
            Some("Ben's long text")
        );
        assert!(
            stage
                .draft(&stage.dan, &stage.describing())
                .await?
                .draft
                .is_none()
        );
        // At his next heartbeat Ben learns it, and what he typed since is kept as well.
        let told = stage
            .heartbeat(&stage.ben, token, Some("Ben's longer text"))
            .await?;
        assert!(
            matches!(told.lock, LockStatus::TakenOver { .. }),
            "{told:?}"
        );
        assert_eq!(told.notices.len(), 1);
        assert_eq!(told.notices[0].kind, NoticeKind::TakenOver);
        assert!(told.draft_saved);
        let kept = stage.draft(&stage.ben, &stage.describing()).await?;
        assert_eq!(
            kept.draft.map(|draft| draft.text).as_deref(),
            Some("Ben's longer text")
        );
        // Dan holds the lock; Ben's old token is worth nothing.
        assert!(matches!(
            stage.heartbeat(&stage.dan, dans.token, None).await?.lock,
            LockStatus::Held { .. }
        ));
        // The takeover and the request are facts in the event log.
        let events = stage.rig().database.events_about(&stage.note, 100).await?;
        assert_eq!(events.len(), 2, "{events:?}");
        Ok(())
    })
    .await
}
