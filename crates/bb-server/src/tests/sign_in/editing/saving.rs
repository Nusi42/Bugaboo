//! Edit locks, drafts and saving, part: saving (ISS-17, ISS-11, ADR-0019).

use super::*;

#[tokio::test]
async fn iss_17_a_save_with_a_lost_lock_is_refused_and_the_text_kept() -> TestResult {
    on_both_databases("iss_17_lost_lock", async |stage| {
        // The lock ran out.
        let (token, _) = stage.lock(&stage.ben).await?;
        stage.pass(LEASE_SECONDS);
        let error = stage
            .save(&stage.ben, token, "Ben's text", 1)
            .await
            .unwrap_err();
        assert_eq!(error.code(), Some(ErrorCode::EditLockLost));
        let kept = stage.draft(&stage.ben, &stage.describing()).await?;
        assert_eq!(
            kept.draft.map(|draft| draft.text).as_deref(),
            Some("Ben's text")
        );
        assert_eq!(
            stage.notes.version(&stage.note),
            Some(1),
            "nothing was saved"
        );
        // Somebody took it over.
        let (token, _) = stage.lock(&stage.ben).await?;
        stage.edit_anyway(&stage.dan).await?;
        let error = stage
            .save(&stage.ben, token, "Ben's other text", 1)
            .await
            .unwrap_err();
        assert_eq!(error.code(), Some(ErrorCode::EditTakenOver));
        let kept = stage.draft(&stage.ben, &stage.describing()).await?;
        assert_eq!(
            kept.draft.map(|draft| draft.text).as_deref(),
            Some("Ben's other text")
        );
        // A token that is not the lock's own.
        let error = stage
            .save(
                &stage.dan,
                LockToken::generate(&mut SystemIds),
                "Dan's text",
                1,
            )
            .await
            .unwrap_err();
        assert_eq!(error.code(), Some(ErrorCode::EditTakenOver));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_a_save_with_the_lock_and_the_version_goes_through_and_lets_go() -> TestResult {
    on_both_databases("iss_17_saved", async |stage| {
        let (token, _) = stage.lock(&stage.ben).await?;
        stage
            .keep_draft(&stage.ben, &stage.describing(), "typing")
            .await?;
        stage.save(&stage.ben, token, "Ben's text", 1).await?;
        assert_eq!(stage.notes.version(&stage.note), Some(2));
        // The lock is gone, the draft is gone, the change is in the event log.
        let (cleos, _) = stage.lock(&stage.cleo).await?;
        stage.let_go(&stage.cleo, cleos).await?;
        assert!(
            stage
                .draft(&stage.ben, &stage.describing())
                .await?
                .draft
                .is_none()
        );
        assert_eq!(
            stage
                .rig()
                .database
                .events_about(&stage.note, 10)
                .await?
                .len(),
            1
        );
        // The version moved on: the next save from the old version is stale, and the text is kept.
        let (token, _) = stage.lock(&stage.ben).await?;
        stage.notes.change(&stage.note);
        let error = stage
            .save(&stage.ben, token, "Ben's second text", 2)
            .await
            .unwrap_err();
        assert_eq!(error.code(), Some(ErrorCode::VersionChanged));
        let kept = stage.draft(&stage.ben, &stage.describing()).await?;
        assert_eq!(
            kept.draft.map(|draft| draft.text).as_deref(),
            Some("Ben's second text")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_a_stale_version_answers_who_changed_what() -> TestResult {
    on_both_databases("iss_17_stale_version", async |stage| {
        let api = stage.rig().api();
        let change = |by: &Member, version: i64, name: &str| {
            api.change_project(
                Rig::info(),
                ChangeProject {
                    session_token: by.session.clone(),
                    project: stage.project.id,
                    version,
                    name: Some(name.to_owned()),
                    short_name: None,
                },
            )
        };
        // Ada renames the project; Dan, who looked at version 1, is told by whom and what.
        let renamed = change(stage.ada(), stage.project.version, "Acme Corporation").await?;
        let AppError::Conflict(conflict) = change(&stage.dan, stage.project.version, "Mine")
            .await
            .unwrap_err()
        else {
            return Err("no structured answer".into());
        };
        assert_eq!(conflict.version, Some(renamed.into_value().version));
        assert_eq!(
            conflict.changed_by,
            Some(shown_as(stage.ada(), "Ada Admin"))
        );
        assert_eq!(conflict.fields, ["name"]);
        assert_eq!(conflict.current["name"], json!("Acme Corporation"));
        assert!(conflict.changed_at.is_some());
        // A person who sees who changed things as *Someone* is told no name.
        let manager = stage
            .org
            .role(
                "Manager",
                &[Permission::SeeProjects, Permission::ChangeProjectSettings],
            )
            .await?;
        let started = stage.rig().person(&stage.ada().session, EVE).await?;
        let eve = Member {
            id: started.person.id,
            session: started.session_token,
        };
        stage.org.give(&eve, manager.id, stage.project.id).await?;
        let AppError::Conflict(for_eve) = change(&eve, stage.project.version, "Eve's")
            .await
            .unwrap_err()
        else {
            return Err("no structured answer for Eve".into());
        };
        assert_eq!(for_eve.changed_by, Some(PersonRef::Someone));
        // Ben may not change the project at all: forbidden, and nothing about who changed it.
        assert_eq!(
            refused(change(&stage.ben, stage.project.version, "Ben's").await),
            Some(ErrorCode::Forbidden)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_17_a_stale_group_or_role_answers_who_changed_what() -> TestResult {
    on_both_databases("iss_17_stale_group_and_role", async |stage| {
        let api = stage.rig().api();
        let ada = stage.ada();
        let group = stage.org.group("Backend").await?;
        let rename = |version: i64, name: &str| {
            api.rename_group(
                Rig::info(),
                RenameGroup {
                    session_token: ada.session.clone(),
                    group,
                    version,
                    name: name.to_owned(),
                },
            )
        };
        let renamed = rename(1, "Platform").await?.into_value();
        assert_eq!(renamed.version, 2);
        let AppError::Conflict(conflict) = rename(1, "Mine").await.unwrap_err() else {
            return Err("no structured answer for the group".into());
        };
        assert_eq!(conflict.version, Some(2));
        assert_eq!(conflict.fields, ["name"]);
        assert_eq!(conflict.current["name"], json!("Platform"));
        assert_eq!(conflict.changed_by, Some(shown_as(ada, "Ada Admin")));
        // The same for a role: the permissions are one whole, so a stale list is refused.
        let role = stage.org.role("Reader", &[Permission::SeeProjects]).await?;
        let change = |version: i64, permissions: Vec<Permission>| {
            api.change_role(
                Rig::info(),
                ChangeRole {
                    session_token: ada.session.clone(),
                    role: role.id,
                    version,
                    name: None,
                    permissions: Some(permissions),
                    two_step: None,
                },
            )
        };
        let wider = [Permission::SeeProjects, Permission::MakeIssues];
        assert_eq!(
            change(role.version, wider.to_vec())
                .await?
                .into_value()
                .version,
            2
        );
        let AppError::Conflict(conflict) = change(role.version, vec![Permission::SeeProjects])
            .await
            .unwrap_err()
        else {
            return Err("no structured answer for the role".into());
        };
        assert_eq!(conflict.version, Some(2));
        assert!(conflict.fields.contains(&"permissions".to_owned()));
        assert_eq!(
            conflict.current["permissions"],
            json!(["see-projects", "make-issues"])
        );
        Ok(())
    })
    .await
}
