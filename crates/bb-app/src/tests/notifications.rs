//! The notifications reader and the list (NTF-01, SEC-06, ADR-0033 §1-§2), run against the
//! in-memory store: who hears of what, never of their own changes, nothing a person may not see,
//! and again nothing once they lost access.

use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::edit::{EditWhat, NoticeKind};
use bb_core::fields::Field;
use bb_core::id::{GroupId, IssueId, ProjectId, RoleId, UserId};
use bb_core::notification::{NotificationFilter, NotificationKind, Via};
use bb_core::people_kinds::PeopleKind;
use bb_core::settings::SettingValue;
use bb_core::technical::TechnicalName;
use bb_policy::{Permission, PersonRef};

use super::scenarios::{a_project, text, token};
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::auth::Started;
use crate::comments::{AddComment, add_comment};
use crate::editing::{
    AskToFinish, EditAnyway, TakeLockQuery, ask_to_finish, edit_anyway, take_lock,
};
use crate::follow::{FollowIssue, FollowersQuery, follow_issue, followers};
use crate::issues::{ChangeIssue, change_issue};
use crate::notifications::{
    NotificationItem, NotificationPage, NotificationWhat, NotificationsQuery, ReadNotifications,
    UnreadQuery, Which, notifications, read_notifications, unread_notifications,
};
use crate::people::{AddGroupMember, CreateGroup, add_group_member, create_group};
use crate::roles::{
    AssignRole, CreateRole, Given, SetFieldLevel, SetPeopleLevel, Subject, assign_role,
    create_role, set_level,
};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

/// Ada (the administrator), Bob and Carol who may work in the project, and an issue Ada reported.
pub(super) struct Crew {
    pub(super) world: World,
    pub(super) project: ProjectId,
    pub(super) role: RoleId,
    pub(super) issue: IssueId,
    /// The version of the issue, counted by hand as the tests change it.
    pub(super) version: i64,
    pub(super) bob: Started,
    pub(super) carol: Started,
}

impl Crew {
    pub(super) async fn new() -> Result<Self, String> {
        let world = World::new().await.map_err(text)?;
        let project = a_project(&world, "ACME").await?;
        let role = create_role(
            &world.ctx(),
            CreateRole {
                session_token: token(&world),
                name: "Worker".to_owned(),
                permissions: vec![
                    Permission::SeeProjects,
                    Permission::SeeIssues,
                    Permission::CreateIssues,
                    Permission::ChangeIssues,
                    Permission::Comment,
                    Permission::EditAnyway,
                ],
                two_step: false,
            },
        )
        .await
        .map_err(text)?
        .into_value()
        .id;
        levels(
            &world,
            role,
            &[
                Field::Title,
                Field::Description,
                Field::Status,
                Field::Priority,
                Field::AssignedTo,
            ],
            FieldLevel::CanChange,
        )
        .await?;
        let bob = world
            .person_named("bob@example.test", "Bob Baker")
            .await
            .map_err(text)?;
        let carol = world
            .person_named("carol@example.test", "Carol Crane")
            .await
            .map_err(text)?;
        let issue = an_issue(&world, project).await?;
        let crew = Self {
            world,
            project,
            role,
            issue,
            version: 1,
            bob,
            carol,
        };
        for person in [&crew.bob, &crew.carol] {
            crew.give_role(person.person.id).await?;
        }
        // What the setup did is over before the tests begin.
        crew.world.clock.advance(1);
        Ok(crew)
    }

    async fn give_role(&self, person: UserId) -> Result<(), String> {
        assign_role(
            &self.world.ctx(),
            AssignRole {
                session_token: token(&self.world),
                subject: Subject::Person(person),
                given: Given::Role(self.role),
                project: None,
                applies_to: AppliesTo::Organization,
            },
        )
        .await
        .map(|_| ())
        .map_err(text)
    }

    pub(super) async fn follow(&self, who: &Started) -> Result<(), String> {
        follow_issue(
            &self.world.ctx(),
            FollowIssue {
                session_token: who.session_token.clone(),
                issue: self.issue.to_string(),
            },
        )
        .await
        .map(|_| ())
        .map_err(text)?;
        self.world.clock.advance(1);
        Ok(())
    }

    /// A change of the issue by `who`, with the version the tests count.
    pub(super) async fn change(&mut self, who: &str, change: Change) -> Result<(), String> {
        let (status, title, priority, assigned_to) = match change {
            Change::Status(status) => (Some(status.key()), None, None, None),
            Change::Title(title) => (None, Some(title.to_owned()), None, None),
            Change::Priority(priority) => (None, None, Some(priority), None),
            Change::Assign(people) => (None, None, None, Some(people)),
        };
        change_issue(
            &self.world.ctx(),
            ChangeIssue {
                session_token: who.to_owned(),
                issue: self.issue.to_string(),
                version: self.version,
                issue_type: None,
                title,
                status,
                priority,
                assigned_to,
                fields: std::collections::BTreeMap::new(),
            },
        )
        .await
        .map_err(text)?;
        self.version += 1;
        self.world.clock.advance(1);
        Ok(())
    }

    pub(super) async fn comment(&self, who: &str, words: &str) -> Result<(), String> {
        let added = add_comment(
            &self.world.ctx(),
            AddComment {
                session_token: who.to_owned(),
                issue: self.issue.to_string(),
                text: words.to_owned(),
            },
        )
        .await
        .map(|_| ())
        .map_err(text);
        self.world.clock.advance(1);
        added
    }

    /// The notifications a person reads now.
    pub(super) async fn heard(&self, who: &str) -> Result<Vec<NotificationItem>, String> {
        self.heard_under(who, NotificationFilter::All).await
    }

    async fn heard_under(
        &self,
        who: &str,
        filter: NotificationFilter,
    ) -> Result<Vec<NotificationItem>, String> {
        Ok(notifications(
            &self.world.ctx(),
            NotificationsQuery {
                session_token: who.to_owned(),
                filter,
                cursor: None,
                limit: 50,
            },
        )
        .await
        .map_err(text)?
        .into_value()
        .items)
    }

    /// The organization hears of this kind of event in the app, whatever a person chose.
    pub(super) async fn switch_on(&self, kind: NotificationKind) -> Result<(), String> {
        self.switch_on_by(kind, Via::App).await
    }

    /// The organization hears of this kind of event this way.
    pub(super) async fn switch_on_by(
        &self,
        kind: NotificationKind,
        via: Via,
    ) -> Result<(), String> {
        change_setting(
            &self.world.ctx(),
            ChangeSetting {
                version: None,
                session_token: token(&self.world),
                scope: SettingsScope::Organization,
                key: kind.setting(via).definition().key().to_owned(),
                value: SettingValue::Bool(true),
            },
        )
        .await
        .map(|_| ())
        .map_err(text)
    }
}

/// The statuses of the workflow *Simple* that these tests change an issue to.
#[derive(Clone, Copy)]
pub(super) enum Status {
    Open,
    InProgress,
    Done,
}

impl Status {
    /// The key of the status.
    fn key(self) -> bb_core::workflow::StatusKey {
        match self {
            Self::Open => bb_core::workflow::starter::open(),
            Self::InProgress => bb_core::workflow::starter::in_progress(),
            Self::Done => bb_core::workflow::starter::done(),
        }
    }
}

pub(super) enum Change {
    Status(Status),
    Title(&'static str),
    Priority(bb_core::issue::Priority),
    Assign(Vec<UserId>),
}

/// Gives a role the level for the fields, everywhere.
pub(super) async fn levels(
    world: &World,
    role: RoleId,
    fields: &[Field],
    level: FieldLevel,
) -> Result<(), String> {
    for field in fields {
        set_level(
            &world.ctx(),
            SetFieldLevel {
                session_token: token(world),
                role,
                project: None,
                issue_type: None,
                field: (*field).into(),
                level: Some(level),
            },
        )
        .await
        .map_err(text)?;
    }
    Ok(())
}

/// The first page of the notifications of Ada.
async fn page_of(crew: &Crew) -> Result<NotificationPage, String> {
    Ok(notifications(
        &crew.world.ctx(),
        NotificationsQuery {
            session_token: token(&crew.world),
            filter: NotificationFilter::All,
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map_err(text)?
    .into_value())
}

/// How many notifications Ada has not read.
async fn unread_of(crew: &Crew) -> Result<usize, String> {
    Ok(unread_notifications(
        &crew.world.ctx(),
        UnreadQuery {
            session_token: token(&crew.world),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .unread)
}

pub(super) fn kinds(items: &[NotificationItem]) -> Vec<NotificationKind> {
    items.iter().map(|item| item.kind).collect()
}

#[tokio::test]
async fn ntf_01_a_follower_hears_of_a_status_change_but_not_of_their_own() -> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    // Ada reported the issue, so she follows it; Bob chooses to.
    crew.follow(&crew.bob).await?;
    crew.change(&bob, Change::Status(Status::Done)).await?;
    let heard_by_ada = crew.heard(&ada).await?;
    assert_eq!(kinds(&heard_by_ada), [NotificationKind::FollowedActivity]);
    assert!(matches!(
        heard_by_ada[0].what,
        NotificationWhat::Changes { .. }
    ));
    assert!(
        crew.heard(&bob).await?.is_empty(),
        "nobody is told of their own change"
    );
    // Ada's own change does not tell her either, but Bob hears it.
    crew.change(&ada, Change::Status(Status::InProgress))
        .await?;
    assert_eq!(crew.heard(&ada).await?.len(), 1);
    assert_eq!(
        kinds(&crew.heard(&bob).await?),
        [NotificationKind::FollowedActivity]
    );
    Ok(())
}

#[tokio::test]
async fn ntf_01_a_comment_on_a_followed_issue_is_activity_and_the_comment_is_read_when_it_is_shown()
-> Outcome {
    let crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.comment(&bob, "It happens every time.").await?;
    let heard = crew.heard(&ada).await?;
    assert_eq!(kinds(&heard), [NotificationKind::FollowedActivity]);
    let NotificationWhat::Comment { comment } = &heard[0].what else {
        return Err("a comment".into());
    };
    assert_eq!(comment.text, "It happens every time.");
    assert_eq!(heard[0].comment, Some(comment.id));
    Ok(())
}

#[tokio::test]
async fn ntf_01_other_changes_to_fields_wait_until_a_person_turns_them_on() -> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.change(&bob, Change::Title("A better title")).await?;
    assert!(
        crew.heard(&ada).await?.is_empty(),
        "starts off (ADR-0033 §1)"
    );
    crew.switch_on(NotificationKind::FollowedFields).await?;
    crew.change(&bob, Change::Title("A still better title"))
        .await?;
    assert_eq!(
        kinds(&crew.heard(&ada).await?),
        [NotificationKind::FollowedFields]
    );
    Ok(())
}

#[tokio::test]
async fn ntf_01_an_assignment_tells_the_person_assigned_and_not_the_one_who_assigns() -> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    let (carol, carol_id) = (crew.carol.session_token.clone(), crew.carol.person.id);
    crew.change(&ada, Change::Assign(vec![carol_id])).await?;
    assert_eq!(
        kinds(&crew.heard(&carol).await?),
        [NotificationKind::Assigned]
    );
    assert!(crew.heard(&bob).await?.is_empty());
    assert!(
        crew.heard(&ada).await?.is_empty(),
        "Ada made the change herself"
    );
    // Adding Bob tells Bob, and Carol, who was assigned before, hears nothing more.
    crew.change(&ada, Change::Assign(vec![carol_id, crew.bob.person.id]))
        .await?;
    assert_eq!(
        kinds(&crew.heard(&bob).await?),
        [NotificationKind::Assigned]
    );
    assert_eq!(crew.heard(&carol).await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn ntf_01_a_mention_reaches_the_person_and_the_members_of_a_group() -> Outcome {
    let crew = Crew::new().await?;
    let ada = token(&crew.world);
    let group: GroupId = create_group(
        &crew.world.ctx(),
        CreateGroup {
            session_token: ada.clone(),
            name: "Night shift".to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id;
    add_group_member(
        &crew.world.ctx(),
        AddGroupMember {
            session_token: ada.clone(),
            group,
            person: crew.carol.person.id,
        },
    )
    .await
    .map_err(text)?;
    crew.comment(&ada, "Please look at this, @bob.baker and @night.shift.")
        .await?;
    let (bob, carol) = (
        crew.heard(&crew.bob.session_token).await?,
        crew.heard(&crew.carol.session_token).await?,
    );
    assert_eq!(kinds(&bob), [NotificationKind::Mentioned]);
    assert_eq!(kinds(&carol), [NotificationKind::Mentioned]);
    // The Mentions filter shows the same.
    assert_eq!(
        crew.heard_under(&crew.bob.session_token, NotificationFilter::Mentions)
            .await?
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn ntf_01_a_person_who_is_mentioned_and_follows_hears_once_as_mentioned() -> Outcome {
    let crew = Crew::new().await?;
    let ada = token(&crew.world);
    crew.follow(&crew.bob).await?;
    crew.comment(&ada, "@bob.baker what do you think?").await?;
    assert_eq!(
        kinds(&crew.heard(&crew.bob.session_token).await?),
        [NotificationKind::Mentioned]
    );
    Ok(())
}

#[tokio::test]
async fn ntf_01_reading_the_events_again_after_a_stop_makes_no_second_notification() -> Outcome {
    let mut crew = Crew::new().await?;
    let bob = crew.bob.session_token.clone();
    crew.change(&bob, Change::Status(Status::Done)).await?;
    let ada = token(&crew.world);
    assert_eq!(crew.heard(&ada).await?.len(), 1);
    // The reader stopped after it stored the rows and before it moved its place on.
    crew.world.store.lock().state.notification_position = 0;
    assert_eq!(crew.heard(&ada).await?.len(), 1);
    assert_eq!(crew.world.store.lock().state.notifications.len(), 1);
    Ok(())
}

#[tokio::test]
async fn sec_06_a_notification_leaves_out_a_field_the_recipient_cannot_see() -> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    // Ada's role hides the priority from Bob and Carol; she follows the issue and wants to hear of
    // every field.
    let hider = create_role(
        &crew.world.ctx(),
        CreateRole {
            session_token: ada.clone(),
            name: "Blind".to_owned(),
            permissions: vec![
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::ChangeIssues,
            ],
            two_step: false,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id;
    levels(
        &crew.world,
        hider,
        &[Field::Title, Field::Status],
        FieldLevel::CanSee,
    )
    .await?;
    levels(&crew.world, hider, &[Field::Priority], FieldLevel::Hidden).await?;
    let dave = crew
        .world
        .person_named("dave@example.test", "Dave Dunn")
        .await
        .map_err(text)?;
    assign_role(
        &crew.world.ctx(),
        AssignRole {
            session_token: ada.clone(),
            subject: Subject::Person(dave.person.id),
            given: Given::Role(hider),
            project: None,
            applies_to: AppliesTo::Organization,
        },
    )
    .await
    .map_err(text)?;
    crew.follow(&dave).await?;
    crew.switch_on(NotificationKind::FollowedFields).await?;
    // One change of a field Dave may see and one he may not.
    crew.change(&bob, Change::Priority(bb_core::issue::Priority::Critical))
        .await?;
    assert!(
        crew.heard(&dave.session_token).await?.is_empty(),
        "a change to a field he cannot see tells him nothing"
    );
    crew.change(&bob, Change::Title("A new title")).await?;
    let heard = crew.heard(&dave.session_token).await?;
    assert_eq!(heard.len(), 1);
    let NotificationWhat::Changes { fields } = &heard[0].what else {
        return Err("changes".into());
    };
    assert_eq!(
        fields.iter().map(|f| f.field.as_str()).collect::<Vec<_>>(),
        ["title"]
    );
    // And not a trace of the priority in the stored row.
    let stored = crew.world.store.lock().state.notifications.clone();
    assert!(
        stored
            .iter()
            .filter(|row| row.recipient == dave.person.id)
            .all(|row| row.changes.iter().all(|change| change.field != "priority"))
    );
    Ok(())
}

#[tokio::test]
async fn sec_06_a_person_who_lost_access_gets_nothing_more() -> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.follow(&crew.carol).await?;
    crew.change(&bob, Change::Status(Status::Done)).await?;
    let carol = crew.carol.session_token.clone();
    assert_eq!(crew.heard(&carol).await?.len(), 1);
    // Carol loses access to the project: what she heard of it before is not shown any more.
    assign_role(
        &crew.world.ctx(),
        AssignRole {
            session_token: ada.clone(),
            subject: Subject::Person(crew.carol.person.id),
            given: Given::NoAccess,
            project: Some(crew.project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await
    .map_err(text)?;
    assert!(
        crew.heard(&carol).await?.is_empty(),
        "yesterday's notification is not shown to a person who lost access"
    );
    // And nothing new is made for her.
    crew.change(&ada, Change::Status(Status::InProgress))
        .await?;
    assert!(crew.heard(&carol).await?.is_empty());
    let rows_for_carol = crew
        .world
        .store
        .lock()
        .state
        .notifications
        .iter()
        .filter(|row| row.recipient == crew.carol.person.id)
        .count();
    assert_eq!(rows_for_carol, 1, "only the one she had before");
    Ok(())
}

#[tokio::test]
async fn sec_06_a_follower_without_access_is_dropped() -> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.follow(&crew.carol).await?;
    let followers_now = |crew: &Crew| {
        crew.world
            .store
            .lock()
            .state
            .followers
            .iter()
            .filter(|follower| follower.issue == crew.issue)
            .map(|follower| follower.person)
            .collect::<Vec<_>>()
    };
    assert!(followers_now(&crew).contains(&crew.carol.person.id));
    let events_before = crew.world.store.event_count();
    assign_role(
        &crew.world.ctx(),
        AssignRole {
            session_token: ada.clone(),
            subject: Subject::Person(crew.carol.person.id),
            given: Given::NoAccess,
            project: Some(crew.project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await
    .map_err(text)?;
    crew.change(&bob, Change::Status(Status::Done)).await?;
    crew.heard(&ada).await?;
    assert!(
        !followers_now(&crew).contains(&crew.carol.person.id),
        "the next change drops a follower who cannot see the issue"
    );
    assert!(crew.world.store.event_count() > events_before);
    // The person follows nothing they cannot see, and the followers a reader sees do not name
    // them either way.
    let seen = followers(
        &crew.world.ctx(),
        FollowersQuery {
            session_token: ada,
            issue: crew.issue.to_string(),
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assert!(seen.items.iter().all(|person| !matches!(
        person,
        PersonRef::Name { id, .. } if *id == crew.carol.person.id
    )));
    Ok(())
}

#[tokio::test]
async fn sec_06_the_author_is_shown_by_name_to_a_reader_who_sees_authors_by_name() -> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.change(&bob, Change::Status(Status::Done)).await?;
    let named = crew.heard(&ada).await?;
    assert!(matches!(
        named[0].by,
        Some(PersonRef::Name { ref name, .. }) if name == "Bob Baker"
    ));
    Ok(())
}

#[tokio::test]
async fn ntf_01_asking_to_finish_and_taking_over_tell_the_person_who_was_editing() -> Outcome {
    let crew = Crew::new().await?;
    let (bob, carol) = (
        crew.bob.session_token.clone(),
        crew.carol.session_token.clone(),
    );
    let what = EditWhat {
        record: crate::auth::entity(bb_core::event::RecordKind::Issue, crew.issue),
        part: TechnicalName::new("description").map_err(text)?,
    };
    let editor = bb_core::id::EditorId::from_random([3; 16]);
    take_lock(
        &crew.world.ctx(),
        TakeLockQuery {
            session_token: bob.clone(),
            what: what.clone(),
            editor,
        },
    )
    .await
    .map_err(text)?;
    ask_to_finish(
        &crew.world.ctx(),
        AskToFinish {
            session_token: carol.clone(),
            what: what.clone(),
        },
    )
    .await
    .map_err(text)?;
    let heard = crew.heard(&bob).await?;
    assert_eq!(kinds(&heard), [NotificationKind::EditNotice]);
    assert!(matches!(
        heard[0].what,
        NotificationWhat::EditNotice {
            notice: NoticeKind::FinishAsked,
            ..
        }
    ));
    edit_anyway(
        &crew.world.ctx(),
        EditAnyway {
            session_token: carol.clone(),
            what,
            editor: bb_core::id::EditorId::from_random([4; 16]),
        },
    )
    .await
    .map_err(text)?;
    let heard = crew.heard(&bob).await?;
    assert_eq!(heard.len(), 2);
    assert!(heard.iter().any(|item| matches!(
        item.what,
        NotificationWhat::EditNotice {
            notice: NoticeKind::TakenOver,
            ..
        }
    )));
    assert!(
        crew.heard(&carol).await?.is_empty(),
        "the person who asks is not told"
    );
    Ok(())
}

#[tokio::test]
async fn ntf_01_mark_all_as_read_leaves_what_came_after_the_newest_shown_unread() -> Outcome {
    let mut crew = Crew::new().await?;
    let bob = crew.bob.session_token.clone();
    crew.change(&bob, Change::Status(Status::Done)).await?;
    crew.change(&bob, Change::Status(Status::InProgress))
        .await?;
    // The person saw the list with the older one; the newer one came after.
    let page = page_of(&crew).await?;
    assert_eq!(page.items.len(), 2);
    let shown_newest = page.items[1].id;
    read_notifications(
        &crew.world.ctx(),
        ReadNotifications {
            session_token: token(&crew.world),
            which: Which::UpTo(shown_newest),
        },
    )
    .await
    .map_err(text)?;
    assert_eq!(unread_of(&crew).await?, 1);
    let unread = crew
        .heard_under(&token(&crew.world), NotificationFilter::Unread)
        .await?;
    assert_eq!(unread.len(), 1);
    assert_eq!(unread[0].id, page.items[0].id);
    Ok(())
}

#[tokio::test]
async fn ntf_01_mark_as_read_marks_what_is_named_and_the_count_follows() -> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.change(&bob, Change::Status(Status::Done)).await?;
    crew.change(&bob, Change::Status(Status::InProgress))
        .await?;
    let page = page_of(&crew).await?;
    assert_eq!((page.items.len(), unread_of(&crew).await?), (2, 2));
    read_notifications(
        &crew.world.ctx(),
        ReadNotifications {
            session_token: ada.clone(),
            which: Which::These(vec![page.items[0].id]),
        },
    )
    .await
    .map_err(text)?;
    let page = page_of(&crew).await?;
    assert_eq!(unread_of(&crew).await?, 1);
    assert_eq!(page.items.iter().filter(|item| item.read).count(), 1);
    read_notifications(
        &crew.world.ctx(),
        ReadNotifications {
            session_token: ada.clone(),
            which: Which::UpTo(super::scenarios_notifications::the_newest_there_can_be()),
        },
    )
    .await
    .map_err(text)?;
    assert_eq!(unread_of(&crew).await?, 0);
    // Nobody can mark another person's notifications.
    let bobs = read_notifications(
        &crew.world.ctx(),
        ReadNotifications {
            session_token: bob,
            which: Which::These(vec![page.items[0].id]),
        },
    )
    .await;
    assert!(bobs.is_ok(), "it changes nothing for Bob, who has none");
    assert!(
        crew.heard_under(&ada, NotificationFilter::Unread)
            .await?
            .is_empty()
    );
    Ok(())
}

#[tokio::test]
async fn ntf_01_a_page_has_a_cursor_and_the_cursor_goes_on_where_it_stopped() -> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    for status in [Status::Done, Status::InProgress, Status::Done] {
        crew.change(&bob, Change::Status(status)).await?;
    }
    let first = notifications(
        &crew.world.ctx(),
        NotificationsQuery {
            session_token: ada.clone(),
            filter: NotificationFilter::All,
            cursor: None,
            limit: 2,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assert_eq!(first.items.len(), 2);
    let rest = notifications(
        &crew.world.ctx(),
        NotificationsQuery {
            session_token: ada,
            filter: NotificationFilter::All,
            cursor: first.next_cursor,
            limit: 2,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assert_eq!(rest.items.len(), 1);
    assert!(rest.next_cursor.is_none());
    Ok(())
}

#[tokio::test]
async fn sec_06_a_person_who_may_not_see_authors_does_not_see_who_did_it() -> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    // Carol follows; her role sees *Authors* as a role label only.
    crew.follow(&crew.carol).await?;
    set_level(
        &crew.world.ctx(),
        SetPeopleLevel {
            session_token: ada.clone(),
            role: crew.role,
            project: None,
            kind: PeopleKind::Authors,
            level: Some(PeopleLevel::Hidden),
        },
    )
    .await
    .map_err(text)?;
    crew.change(&bob, Change::Status(Status::Done)).await?;
    let heard = crew.heard(&crew.carol.session_token).await?;
    assert_eq!(heard.len(), 1);
    let shown = serde_json::to_string(&heard[0].by).map_err(text)?;
    assert!(!shown.contains("Bob"), "{shown}");
    assert!(!shown.contains(&crew.bob.person.id.to_string()), "{shown}");
    Ok(())
}

#[tokio::test]
async fn ntf_01_one_request_for_the_list_looks_at_a_bounded_number_of_rows_and_says_there_is_more()
-> Outcome {
    use bb_core::id::{IssueId, NotificationId};
    use bb_core::{notification::Notification, paging::LOOKED_AT_MAX};

    use crate::context::SystemIds;

    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.change(&bob, Change::Status(Status::Done)).await?;
    assert_eq!(crew.heard(&ada).await?.len(), 1);
    let real = crew
        .world
        .store
        .lock()
        .state
        .notifications
        .iter()
        .find(|row| row.recipient == crew.world.admin.person.id)
        .cloned()
        .ok_or("no notification")?;
    // Newer than the real one, many rows about an issue that is not there (so Ada may not see
    // them): more than one request may look at.
    for number in 0..u16::try_from(LOOKED_AT_MAX + 100)? {
        // A version 7 ID that sorts after any real one: the time part is all ones.
        let mut bytes = [0xFF_u8; 16];
        (bytes[6], bytes[8]) = (0x70, 0x80);
        bytes[14..].copy_from_slice(&number.to_be_bytes());
        crew.world
            .store
            .lock()
            .state
            .notifications
            .push(Notification {
                id: NotificationId::from_bytes(bytes)?,
                issue: Some(IssueId::generate(&mut SystemIds)?),
                ..real.clone()
            });
    }
    let mut cursor = None;
    let mut asked = 0;
    let mut found = Vec::new();
    loop {
        let page = notifications(
            &crew.world.ctx(),
            NotificationsQuery {
                session_token: ada.clone(),
                filter: NotificationFilter::All,
                cursor: cursor.clone(),
                limit: 50,
            },
        )
        .await
        .map_err(text)?
        .into_value();
        asked += 1;
        found.extend(page.items);
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert!(asked >= 2, "the first request stops at the limit: {asked}");
    assert_eq!(found.len(), 1, "and the next one goes on to the real one");
    Ok(())
}
