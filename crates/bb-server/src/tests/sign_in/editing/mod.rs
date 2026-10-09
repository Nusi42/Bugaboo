//! Edit locks, drafts and the check of versions through the real wiring, on both databases (ISS-17, ISS-11,
//! ADR-0019): who is editing, the lease of a minute, *Edit anyway* that keeps the first person's text,
//! a save that is refused without losing the text, the drafts that only their author reads, and the
//! structured answer to a stale version.
//!
//! The record is a *note*, a kind of record that exists only in the tests (`TestNotes`): locks and
//! drafts work on any record by its kind and ID, so no table of a real record is needed.

mod drafts;
mod locks;
mod saving;

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
