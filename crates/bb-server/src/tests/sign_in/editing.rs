//! Edit locks, drafts and the check of versions through the real wiring, on both databases (ISS-17, ISS-11,
//! ADR-0019): who is editing, the lease of a minute, *Edit anyway* that keeps the first person's text,
//! a save that is refused without losing the text, the drafts that only their author reads, and the
//! structured answer to a stale version.
//!
//! The record is a *note*, a kind of record that exists only in the tests (`TestNotes`): locks and
//! drafts work on any record by its kind and ID, so no table of a real record is needed.

use std::sync::Arc;

use bb_app::access::{Permission, PersonRef};
use bb_app::auth::AppError;
use bb_app::context::SystemIds;
use bb_app::editing::EditStore as _;
use bb_app::editing::testing::TestNotes;
use bb_app::editing::{
    AskToFinish, CheckVersionsQuery, DraftSlot, DraftText, EditAnyway, HeartbeatAnswer,
    HeartbeatQuery, LockOutcome, LockStatus, ReadDraftQuery, ReleaseLockQuery, ReleaseStuckLock,
    SaveDraftQuery, TakeLockQuery, TextSave, ThrowAwayDraftQuery, Versions,
};
use bb_app::people::RenameGroup;
use bb_app::projects::{ChangeProject, ProjectView};
use bb_app::roles::{ChangeRole, SetPeopleLevel};
use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::edit::{
    DRAFT_KEEP_DAYS, DraftTarget, EditWhat, HEARTBEAT_SECONDS, LEASE_SECONDS, NoticeKind,
    VERSIONS_MAX,
};
use bb_core::event::{Actor, Channel, DomainEvent, EntityRef, FieldChange};
use bb_core::id::{EditorId, EntityId, EventId, LockToken, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_core::technical::TechnicalName;
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;
use serde_json::json;

use super::policy_rig::{Member, Org, Run};
use super::rig::{PASSWORD, Rig, refused};

/// What a person needs to edit a note.
const EDITING: [Permission; 3] = [
    Permission::SeeProjects,
    Permission::ChangeIssues,
    Permission::MakeIssues,
];
/// What a person needs to take the editing over.
const TAKING_OVER: [Permission; 5] = [
    Permission::SeeProjects,
    Permission::ChangeIssues,
    Permission::MakeIssues,
    Permission::EditAnyway,
    Permission::ChangeProjectSettings,
];

/// The e-mail addresses of the people of the stage.
const BEN: &str = "ben@example.test";
const CLEO: &str = "cleo@example.test";
const DAN: &str = "dan@example.test";
const EVE: &str = "eve@example.test";

/// An organization with a project, a note in it, and four people: Ada (administrator), Ben (edits,
/// sees editors as *Someone*), Cleo (edits, sees names) and Dan (edits, may take over, sees names).
struct Stage<'a> {
    org: Org<'a>,
    notes: TestNotes,
    project: ProjectView,
    note: EntityRef,
    what: EditWhat,
    ben: Member,
    cleo: Member,
    dan: Member,
}

impl<'a> Stage<'a> {
    async fn start(rig: &'a Rig, notes: &TestNotes) -> Run<Stage<'a>> {
        let org = Org::start(rig).await?;
        let project = org.project(None, "ACME").await?;
        notes.set_project(project.id);
        let note = notes.add(EntityId::from(UserId::generate(&mut SystemIds)));
        let what = EditWhat {
            record: note.clone(),
            part: TechnicalName::new("description")?,
        };
        let hidden = org.role("Editor", &EDITING).await?;
        let seen = org.role("Editor who sees names", &EDITING).await?;
        let chief = org.role("Chief", &TAKING_OVER).await?;
        for role in [seen.id, chief.id] {
            for kind in [PeopleKind::Editing, PeopleKind::LastChangedBy] {
                set_people_level(&org, role, kind, PeopleLevel::Names).await?;
            }
        }
        let person = async |email: &str| -> Run<Member> {
            let started = rig.person(&org.admin.session, email).await?;
            Ok(Member {
                id: started.person.id,
                session: started.session_token,
            })
        };
        let (ben, cleo, dan) = (person(BEN).await?, person(CLEO).await?, person(DAN).await?);
        org.give(&ben, hidden.id, project.id).await?;
        org.give(&cleo, seen.id, project.id).await?;
        org.give(&dan, chief.id, project.id).await?;
        Ok(Self {
            org,
            notes: notes.clone(),
            project,
            note,
            what,
            ben,
            cleo,
            dan,
        })
    }

    fn ada(&self) -> &Member {
        &self.org.admin
    }

    fn rig(&self) -> &Rig {
        self.org.rig
    }

    async fn take(&self, by: &Member, editor: EditorId) -> Result<LockOutcome, AppError> {
        Ok(self
            .rig()
            .api()
            .take_lock(
                Rig::info(),
                TakeLockQuery {
                    session_token: by.session.clone(),
                    what: self.what.clone(),
                    editor,
                },
            )
            .await?
            .into_value())
    }

    /// The token of the lock that a person takes (the thing must be free for them).
    async fn lock(&self, by: &Member) -> Result<(LockToken, EditorId), Failure> {
        let editor = EditorId::generate(&mut SystemIds);
        match self.take(by, editor).await? {
            LockOutcome::Taken(taken) => Ok((taken.token, editor)),
            LockOutcome::Held(held) => Err(format!("held by {:?}", held.holder).into()),
        }
    }

    async fn heartbeat(
        &self,
        by: &Member,
        token: LockToken,
        text: Option<&str>,
    ) -> Result<HeartbeatAnswer, AppError> {
        Ok(self
            .rig()
            .api()
            .heartbeat(
                Rig::info(),
                HeartbeatQuery {
                    session_token: by.session.clone(),
                    what: self.what.clone(),
                    token,
                    draft: text.map(|text| DraftText {
                        text: text.to_owned(),
                        base_version: Some(1),
                    }),
                },
            )
            .await?
            .into_value())
    }

    async fn edit_anyway(&self, by: &Member) -> Result<LockOutcome, AppError> {
        Ok(self
            .rig()
            .api()
            .edit_anyway(
                Rig::info(),
                EditAnyway {
                    session_token: by.session.clone(),
                    what: self.what.clone(),
                    editor: EditorId::generate(&mut SystemIds),
                },
            )
            .await?
            .into_value())
    }

    async fn draft(&self, by: &Member, target: &DraftTarget) -> Result<DraftSlot, AppError> {
        Ok(self
            .rig()
            .api()
            .read_draft(
                Rig::info(),
                ReadDraftQuery {
                    session_token: by.session.clone(),
                    target: target.clone(),
                },
            )
            .await?
            .into_value())
    }

    async fn keep_draft(
        &self,
        by: &Member,
        target: &DraftTarget,
        text: &str,
    ) -> Result<(), AppError> {
        self.rig()
            .api()
            .save_draft(
                Rig::info(),
                SaveDraftQuery {
                    session_token: by.session.clone(),
                    target: target.clone(),
                    draft: DraftText {
                        text: text.to_owned(),
                        base_version: Some(1),
                    },
                },
            )
            .await?;
        Ok(())
    }

    async fn check_versions(
        &self,
        by: &Member,
        records: Vec<EntityRef>,
    ) -> Result<Versions, AppError> {
        Ok(self
            .rig()
            .api()
            .check_versions(
                Rig::info(),
                CheckVersionsQuery {
                    session_token: by.session.clone(),
                    records,
                },
            )
            .await?
            .into_value())
    }

    async fn let_go(&self, by: &Member, token: LockToken) -> Result<(), AppError> {
        self.rig()
            .api()
            .release_lock(
                Rig::info(),
                ReleaseLockQuery {
                    session_token: by.session.clone(),
                    what: self.what.clone(),
                    token,
                },
            )
            .await?;
        Ok(())
    }

    /// A person after a long time away: their session ran out, so they sign in again.
    async fn signed_in_again(&self, email: &str) -> Run<Member> {
        let started = self.rig().sign_in(email, PASSWORD).await?;
        Ok(Member {
            id: started.person.id,
            session: started.session_token,
        })
    }

    fn describing(&self) -> DraftTarget {
        DraftTarget::Editing(self.what.clone())
    }

    /// Saves the note under a lock the way the route of a record will: the text, the token and
    /// the version the person started from, together. The note counts its version up.
    async fn save(
        &self,
        by: &Member,
        token: LockToken,
        text: &str,
        version: i64,
    ) -> Result<(), AppError> {
        let mut ids = SystemIds;
        let event = DomainEvent {
            id: EventId::generate(&mut ids)
                .map_err(|error| AppError::Internal(error.to_string()))?,
            time: self.rig().clock_now(),
            actor: Actor::Person(by.id),
            channel: Channel::Ui,
            entity: self.note.clone(),
            changes: vec![FieldChange {
                field: "description".to_owned(),
                before: None,
                after: Some(json!(text)),
            }],
            reason: None,
        };
        let (notes, note) = (self.notes.clone(), self.note.clone());
        self.rig()
            .services
            .ctx(Rig::info())
            .save_text(
                by.id,
                bb_app::access::Place::Project(self.project.id),
                &TextSave {
                    what: self.what.clone(),
                    token,
                    text: text.to_owned(),
                    version,
                },
                &event,
                move |_transaction| {
                    Box::pin(async move {
                        if notes.version(&note) == Some(version) {
                            notes.change(&note);
                            Ok(Some(()))
                        } else {
                            Ok(None)
                        }
                    })
                },
            )
            .await
    }

    fn pass(&self, seconds: i64) {
        self.rig().clock.advance(seconds);
    }
}

async fn set_people_level(
    org: &Org<'_>,
    role: RoleId,
    kind: PeopleKind,
    level: PeopleLevel,
) -> Run<()> {
    org.rig
        .api()
        .set_people_level(
            Rig::info(),
            SetPeopleLevel {
                session_token: org.admin.session.clone(),
                role,
                project: None,
                kind,
                level: Some(level),
            },
        )
        .await?;
    Ok(())
}

/// How a person is shown, by name.
fn shown_as(member: &Member, name: &str) -> PersonRef {
    PersonRef::Name {
        id: member.id,
        name: name.to_owned(),
    }
}

async fn on_both_databases<F>(name: &'static str, body: F) -> TestResult
where
    F: for<'a> AsyncFn(&'a Stage<'a>) -> TestResult + Copy + Send + Sync,
{
    each_database(name, async move |db| {
        let notes = TestNotes::new()?;
        let rig = Rig::start_with_records(db, Arc::new(notes.clone())).await?;
        let stage = Stage::start(&rig, &notes).await?;
        body(&stage).await
    })
    .await
}

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
