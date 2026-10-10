//! Edit locks, drafts and saving, part: drafts (ISS-17, ISS-11, ADR-0019).

use super::*;

#[tokio::test]
async fn iss_11_a_draft_is_readable_only_by_its_author() -> TestResult {
    on_both_databases("iss_11_private", async |stage| {
        let editing = stage.describing();
        stage
            .keep_draft(&stage.ben, &editing, "Ben's private text")
            .await?;
        // Nobody else reads it, not even somebody who may take over or administrate.
        for other in [stage.ada(), &stage.cleo, &stage.dan] {
            assert!(stage.draft(other, &editing).await?.draft.is_none());
        }
        // Their own draft for the same target is another draft.
        stage
            .keep_draft(&stage.cleo, &editing, "Cleo's text")
            .await?;
        let bens = stage.draft(&stage.ben, &editing).await?.draft;
        assert_eq!(
            bens.map(|draft| draft.text).as_deref(),
            Some("Ben's private text")
        );
        // A draft of a new record belongs to the person, the project and the form.
        let new_note = DraftTarget::NewRecord {
            project: stage.project.id,
            kind: stage.note.kind.clone(),
        };
        stage
            .keep_draft(&stage.ben, &new_note, "A new note")
            .await?;
        assert!(stage.draft(&stage.cleo, &new_note).await?.draft.is_none());
        // Nobody keeps a draft for what they may not change or make.
        let outsider = stage.org.person().await?;
        assert_eq!(
            refused(stage.keep_draft(&outsider, &editing, "x").await),
            Some(ErrorCode::NotFound)
        );
        assert_eq!(
            refused(stage.draft(&outsider, &new_note).await),
            Some(ErrorCode::NotFound)
        );
        // Throwing away deletes only the author's own.
        stage
            .rig()
            .api()
            .throw_away_draft(
                Rig::info(),
                ThrowAwayDraftQuery {
                    session_token: stage.ben.session.clone(),
                    target: editing.clone(),
                },
            )
            .await?;
        assert!(stage.draft(&stage.ben, &editing).await?.draft.is_none());
        assert!(stage.draft(&stage.cleo, &editing).await?.draft.is_some());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_11_the_clearing_deletes_old_drafts_and_lock_rows_that_ran_out() -> TestResult {
    on_both_databases("iss_11_the_clearing", async |stage| {
        let target = stage.describing();
        stage
            .keep_draft(&stage.cleo, &target, "left behind")
            .await?;
        stage.lock(&stage.ben).await?;
        stage.pass(DRAFT_KEEP_DAYS * 86_400 + 1);
        let drafts = async || -> Result<bool, Failure> {
            Ok(stage
                .rig()
                .storage
                .draft(stage.cleo.id, target.clone(), Timestamp::UNIX_EPOCH)
                .await?
                .is_some())
        };
        let lock_rows = async || -> Result<bool, Failure> {
            Ok(stage
                .rig()
                .storage
                .edit_lock(stage.what.clone())
                .await?
                .is_some())
        };
        assert!(
            drafts().await? && lock_rows().await?,
            "stored until cleared"
        );
        let ctx = stage.rig().services.ctx(Rig::info());
        bb_app::editing::purge_drafts(&ctx, bb_app::editing::PurgeDraftsQuery).await?;
        assert!(!drafts().await?, "the draft is past its keep period");
        assert!(!lock_rows().await?, "the lease ran out");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_11_a_draft_goes_after_save_and_after_its_keep_period() -> TestResult {
    on_both_databases("iss_11_keep_period", async |stage| {
        let target = stage.describing();
        // After a save.
        let (token, _) = stage.lock(&stage.ben).await?;
        stage.keep_draft(&stage.ben, &target, "typing").await?;
        stage.save(&stage.ben, token, "done", 1).await?;
        assert!(stage.draft(&stage.ben, &target).await?.draft.is_none());
        // After 30 days. On the day itself it is still there; a second later it is as good as gone,
        // also before the job that deletes it has run.
        stage
            .keep_draft(&stage.cleo, &target, "left behind")
            .await?;
        // A month is longer than a session: Cleo signs in again each time.
        stage.pass(DRAFT_KEEP_DAYS * 86_400);
        let cleo = stage.signed_in_again(CLEO).await?;
        assert!(stage.draft(&cleo, &target).await?.draft.is_some());
        stage.pass(1);
        let cleo = stage.signed_in_again(CLEO).await?;
        assert!(stage.draft(&cleo, &target).await?.draft.is_none());
        let stored = |author: UserId| {
            stage
                .rig()
                .storage
                .draft(author, target.clone(), Timestamp::UNIX_EPOCH)
        };
        assert!(
            stored(stage.cleo.id).await?.is_some(),
            "the row is still there"
        );
        // The job deletes it, and a second run finds nothing to do.
        let ctx = stage.rig().services.ctx(Rig::info());
        let gone = bb_app::editing::purge_drafts(&ctx, bb_app::editing::PurgeDraftsQuery).await?;
        assert_eq!(gone, 1);
        assert!(stored(stage.cleo.id).await?.is_none());
        let again = bb_app::editing::purge_drafts(&ctx, bb_app::editing::PurgeDraftsQuery).await?;
        assert_eq!(again, 0);
        Ok(())
    })
    .await
}
