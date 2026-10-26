//! Saved searches (QRY-06, CFG-02, ADR-0034): who makes, shares, changes and deletes them, and who
//! is reached. The rules, run against the in-memory store; the same rules run on both databases
//! through the API in `bb-server`.

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::{GroupId, ProjectId, SavedSearchId};
use bb_core::saved_search::{Presentation, SearchSet, ShareKind, ShareTarget};
use bb_policy::StartingRole;

use super::scenarios::{a_project, text, token};
use super::scenarios_searches::{
    a_group_of, a_presentation, choose_list, list, list_of, open, save, share, unshare, with_role,
};
use super::world::{Outcome, World};
use crate::auth::{AppError, Started, by, changed, entity};
use crate::people::{DeleteGroup, delete_group};
use crate::ports::ChangeStore as _;
use crate::saved_searches::{
    ChangeSearch, DeleteSearch, RestoreSearch, SaveCopy, SavedSearchStore as _, ShareChoicesQuery,
    ShareView, change_search, delete_search, restore_search, save_copy, share_choices,
};

/// The people of a small company: Ada the administrator; Ben, a team member of ACME only; Cleo, a
/// team member of the whole organization; Dan, a guest in ACME; Eve, with no role at all.
pub(super) struct Crew {
    pub(super) world: World,
    pub(super) acme: ProjectId,
    pub(super) web: ProjectId,
    pub(super) ben: Started,
    pub(super) cleo: Started,
    pub(super) dan: Started,
    pub(super) eve: Started,
}

impl Crew {
    pub(super) async fn new() -> Result<Self, String> {
        let world = World::new().await?;
        let acme = a_project(&world, "ACME").await?;
        let web = a_project(&world, "WEB").await?;
        let person = |email: &'static str, name: &'static str| {
            let world = &world;
            async move { world.person_named(email, name).await.map_err(text) }
        };
        let ben = person("ben@example.test", "Ben Baker").await?;
        let cleo = person("cleo@example.test", "Cleo Cook").await?;
        let dan = person("dan@example.test", "Dan Dyer").await?;
        let eve = person("eve@example.test", "Eve Eld").await?;
        with_role(&world, &ben, StartingRole::TeamMember, Some(acme)).await?;
        with_role(&world, &cleo, StartingRole::TeamMember, None).await?;
        with_role(&world, &dan, StartingRole::Guest, Some(acme)).await?;
        Ok(Self {
            world,
            acme,
            web,
            ben,
            cleo,
            dan,
            eve,
        })
    }

    pub(super) fn ada(&self) -> String {
        token(&self.world)
    }
}

pub(super) fn refused<T>(result: &Result<T, AppError>, code: ErrorCode) -> bool {
    matches!(result, Err(AppError::Refused(found)) if *found == code)
}

/// The searches shared with the person.
pub(super) async fn shared(
    crew: &Crew,
    who: &Started,
) -> Result<Vec<crate::saved_searches::SavedSearchView>, AppError> {
    list(&crew.world, &who.session_token, SearchSet::Shared).await
}

fn names(views: &[crate::saved_searches::SavedSearchView]) -> Vec<&str> {
    views.iter().map(|view| view.name.as_str()).collect()
}

#[tokio::test]
async fn qry_06_a_saved_search_is_private_when_it_is_made_and_keeps_how_it_is_shown() -> Outcome {
    let crew = Crew::new().await?;
    let made = save(
        &crew.world,
        &crew.ben.session_token,
        "My bugs",
        "status is open",
    )
    .await?;
    assert!(made.mine && made.shared_with.is_empty());
    assert_eq!(made.presentation, a_presentation());
    assert_eq!(made.text, "status is open");
    // Nobody else lists it or opens it, and the address answers as for one that is not there.
    for other in [&crew.cleo, &crew.dan, &crew.eve] {
        for scope in [SearchSet::Mine, SearchSet::Shared] {
            assert!(
                list(&crew.world, &other.session_token, scope)
                    .await?
                    .is_empty()
            );
        }
        let missing = open(&crew.world, &other.session_token, made.id).await;
        assert!(refused(&missing, ErrorCode::NotFound));
    }
    let again = open(&crew.world, &crew.ben.session_token, made.id).await?;
    assert_eq!(again, made);
    Ok(())
}

#[tokio::test]
async fn qry_06_only_the_maker_changes_a_saved_search() -> Outcome {
    let crew = Crew::new().await?;
    let made = save(
        &crew.world,
        &crew.ben.session_token,
        "My bugs",
        "status is open",
    )
    .await?;
    share(
        &crew.world,
        &crew.ben.session_token,
        made.id,
        ShareTarget::Project(crew.acme),
    )
    .await?;
    // Dan sees it (it is shared with ACME), and may not change, share or delete it.
    let dan = &crew.dan.session_token;
    let change = change_search(
        &crew.world.ctx(),
        ChangeSearch {
            session_token: dan.clone(),
            search: made.id,
            version: made.version,
            name: Some("Mine now".to_owned()),
            text: None,
            presentation: None,
        },
    )
    .await;
    assert!(refused(&change, ErrorCode::Forbidden));
    let sharing = share(&crew.world, dan, made.id, ShareTarget::Project(crew.acme)).await;
    assert!(refused(&sharing, ErrorCode::Forbidden));
    let stopping = unshare(&crew.world, dan, made.id, ShareTarget::Project(crew.acme)).await;
    assert!(refused(&stopping, ErrorCode::Forbidden));
    let deleting = delete_search(
        &crew.world.ctx(),
        DeleteSearch {
            session_token: dan.clone(),
            search: made.id,
        },
    )
    .await;
    assert!(refused(&deleting, ErrorCode::Forbidden));
    // Eve does not see it: for her it is not there.
    let hidden = delete_search(
        &crew.world.ctx(),
        DeleteSearch {
            session_token: crew.eve.session_token.clone(),
            search: made.id,
        },
    )
    .await;
    assert!(refused(&hidden, ErrorCode::NotFound));
    // The maker does, and it is the same search afterwards.
    let renamed = change_search(
        &crew.world.ctx(),
        ChangeSearch {
            session_token: crew.ben.session_token.clone(),
            search: made.id,
            version: made.version,
            name: Some("Open bugs".to_owned()),
            text: Some("status is open and priority is high".to_owned()),
            presentation: Some(Presentation::default()),
        },
    )
    .await?
    .into_value();
    assert_eq!(renamed.id, made.id);
    assert_eq!(renamed.name, "Open bugs");
    assert_eq!(renamed.version, made.version + 1);
    assert_eq!(renamed.presentation, Presentation::default());
    let seen = open(&crew.world, dan, made.id).await?;
    assert_eq!(seen.name, "Open bugs");
    Ok(())
}

#[tokio::test]
async fn qry_06_a_change_made_from_an_old_version_is_refused() -> Outcome {
    let crew = Crew::new().await?;
    let made = save(&crew.world, &crew.ben.session_token, "My bugs", "").await?;
    let rename = |version: i64, name: &str| ChangeSearch {
        session_token: crew.ben.session_token.clone(),
        search: made.id,
        version,
        name: Some(name.to_owned()),
        text: None,
        presentation: None,
    };
    change_search(&crew.world.ctx(), rename(made.version, "First")).await?;
    let stale = change_search(&crew.world.ctx(), rename(made.version, "Second")).await;
    // The API turns the stale record into the answer that names the version the search has now.
    assert!(matches!(stale, Err(AppError::Stale(_))), "{stale:?}");
    // Nothing changed is nothing written: the version stays.
    let same = change_search(&crew.world.ctx(), rename(made.version + 1, "First"))
        .await?
        .into_value();
    assert_eq!(same.version, made.version + 1);
    let too_long = change_search(
        &crew.world.ctx(),
        rename(made.version + 1, &"x".repeat(101)),
    )
    .await;
    assert!(refused(&too_long, ErrorCode::InvalidRequest));
    Ok(())
}

#[tokio::test]
async fn qry_06_sharing_needs_share_searches_at_the_place() -> Outcome {
    let crew = Crew::new().await?;
    let ben = &crew.ben.session_token;
    let made = save(&crew.world, ben, "My bugs", "status is open").await?;
    // Ben is a team member of ACME: he shares with ACME, not with WEB, which he does not see.
    let hidden = share(&crew.world, ben, made.id, ShareTarget::Project(crew.web)).await;
    assert!(refused(&hidden, ErrorCode::NotFound));
    let shared = share(&crew.world, ben, made.id, ShareTarget::Project(crew.acme)).await?;
    assert_eq!(shared.shared_with.len(), 1);
    assert!(matches!(&shared.shared_with[0], ShareView::Project { id, .. } if *id == crew.acme));
    // A guest sees ACME and has no *Share searches* there.
    let dans = save(&crew.world, &crew.dan.session_token, "Dan's", "").await?;
    let guest = share(
        &crew.world,
        &crew.dan.session_token,
        dans.id,
        ShareTarget::Project(crew.acme),
    )
    .await;
    assert!(refused(&guest, ErrorCode::Forbidden));
    // Sharing again changes nothing, and no event is written for it.
    let events = crew.world.store.event_count();
    share(&crew.world, ben, made.id, ShareTarget::Project(crew.acme)).await?;
    assert_eq!(crew.world.store.event_count(), events);
    // What the *Share* window offers is what the person may share with.
    let projects = share_choices(
        &crew.world.ctx(),
        ShareChoicesQuery {
            session_token: ben.clone(),
            kind: ShareKind::Project,
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    assert_eq!(
        projects
            .items
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>(),
        [crew.acme.to_string()]
    );
    Ok(())
}

#[tokio::test]
async fn qry_06_a_group_needs_being_in_it_or_manage_groups() -> Outcome {
    let crew = Crew::new().await?;
    let night: GroupId = a_group_of(&crew.world, "Night shift", &[&crew.cleo]).await?;
    let target = ShareTarget::Group(night);
    let share_by = |who: &Started, name: &'static str| {
        let crew = &crew;
        let session = who.session_token.clone();
        async move {
            let made = save(&crew.world, &session, name, "").await?;
            share(&crew.world, &session, made.id, target).await
        }
    };
    // Cleo is in the group and holds *Share searches* for the whole organization.
    assert!(share_by(&crew.cleo, "Cleo's").await.is_ok());
    // Ben holds it in ACME only: that is not for the whole organization.
    assert!(refused(
        &share_by(&crew.ben, "Ben's").await,
        ErrorCode::Forbidden
    ));
    // Cleo's team member role does not let her share with a group she is not in.
    let day: GroupId = a_group_of(&crew.world, "Day shift", &[&crew.ben]).await?;
    let cleo = &crew.cleo.session_token;
    let made = save(&crew.world, cleo, "Cleo's other", "").await?;
    let not_in = share(&crew.world, cleo, made.id, ShareTarget::Group(day)).await;
    assert!(refused(&not_in, ErrorCode::Forbidden));
    // Ada manages groups: she needs not be in the group.
    let ada = crew.ada();
    let adas = save(&crew.world, &ada, "Ada's", "").await?;
    assert!(
        share(&crew.world, &ada, adas.id, ShareTarget::Group(day))
            .await
            .is_ok()
    );
    // A group that is not there is not found for her, and forbidden for the others.
    let nowhere = GroupId::from_parts(jiff::Timestamp::UNIX_EPOCH, [9; 10])?;
    let gone = share(&crew.world, &ada, adas.id, ShareTarget::Group(nowhere)).await;
    assert!(refused(&gone, ErrorCode::NotFound));
    let unknown = share(&crew.world, cleo, made.id, ShareTarget::Group(nowhere)).await;
    assert!(refused(&unknown, ErrorCode::Forbidden));
    // The window offers Cleo her group and Ada all of them.
    let groups_for = |session: &str| {
        let session = session.to_owned();
        let world = &crew.world;
        async move {
            share_choices(
                &world.ctx(),
                ShareChoicesQuery {
                    session_token: session,
                    kind: ShareKind::Group,
                    cursor: None,
                    limit: 50,
                },
            )
            .await
            .map(|page| {
                page.into_value()
                    .items
                    .into_iter()
                    .map(|choice| choice.name)
                    .collect::<Vec<_>>()
            })
        }
    };
    assert_eq!(groups_for(cleo).await?, ["Night shift"]);
    assert_eq!(groups_for(&ada).await?, ["Day shift", "Night shift"]);
    assert!(groups_for(&crew.ben.session_token).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn qry_06_a_search_shared_with_a_place_is_listed_for_the_people_it_reaches_only() -> Outcome {
    let crew = Crew::new().await?;
    let night = a_group_of(&crew.world, "Night shift", &[&crew.cleo]).await?;
    let ada = crew.ada();
    let for_acme = save(&crew.world, &ada, "For ACME", "").await?;
    let for_night = save(&crew.world, &ada, "For the night", "").await?;
    let _private = save(&crew.world, &ada, "Mine", "").await?;
    share(
        &crew.world,
        &ada,
        for_acme.id,
        ShareTarget::Project(crew.acme),
    )
    .await?;
    share(&crew.world, &ada, for_night.id, ShareTarget::Group(night)).await?;
    // Ben and Dan see ACME; Cleo is in the group and, as a team member of the whole organization,
    // sees ACME too; Eve sees nothing.
    assert_eq!(names(&shared(&crew, &crew.ben).await?), ["For ACME"]);
    assert_eq!(names(&shared(&crew, &crew.dan).await?), ["For ACME"]);
    assert_eq!(
        names(&shared(&crew, &crew.cleo).await?),
        ["For ACME", "For the night"]
    );
    assert!(shared(&crew, &crew.eve).await?.is_empty());
    // Ada's own list holds all three, with whom each is shared with.
    let mine = list(&crew.world, &ada, SearchSet::Mine).await?;
    assert_eq!(names(&mine), ["For ACME", "For the night", "Mine"]);
    assert!(shared_view(&mine[1]).contains("Night shift"));
    // A receiver is told who made it, and not whom else it is shared with.
    let seen = open(&crew.world, &crew.ben.session_token, for_acme.id).await?;
    assert!(!seen.mine && seen.shared_with.is_empty());
    // Stopping the share takes it from the receivers at once; a group that is deleted does too.
    unshare(
        &crew.world,
        &ada,
        for_acme.id,
        ShareTarget::Project(crew.acme),
    )
    .await?;
    assert!(shared(&crew, &crew.ben).await?.is_empty());
    delete_group(
        &crew.world.ctx(),
        DeleteGroup {
            session_token: ada.clone(),
            group: night,
        },
    )
    .await?;
    assert!(shared(&crew, &crew.cleo).await?.is_empty());
    // The history of the search says that it is no longer shared with the group.
    let said = crew
        .world
        .store
        .lock()
        .events
        .iter()
        .filter(|event| {
            event.entity.kind == bb_core::event::EntityKind::from(RecordKind::SavedSearch)
                && event.changes.iter().any(|change| {
                    change.field == "shared_with"
                        && change.before.is_some()
                        && change.after.is_none()
                })
        })
        .count();
    assert_eq!(
        said, 2,
        "one event for the search of the group, one for the project"
    );
    let after = open(&crew.world, &ada, for_night.id).await?;
    assert!(after.shared_with.is_empty());
    Ok(())
}

fn shared_view(view: &crate::saved_searches::SavedSearchView) -> String {
    format!("{:?}", view.shared_with)
}

#[tokio::test]
async fn qry_06_save_a_copy_is_private_and_mine() -> Outcome {
    let crew = Crew::new().await?;
    let ada = crew.ada();
    let original = save(&crew.world, &ada, "Open bugs", "status is open").await?;
    share(
        &crew.world,
        &ada,
        original.id,
        ShareTarget::Project(crew.acme),
    )
    .await?;
    let copy = save_copy(
        &crew.world.ctx(),
        SaveCopy {
            session_token: crew.dan.session_token.clone(),
            search: original.id,
            name: None,
        },
    )
    .await?
    .into_value();
    assert_ne!(copy.id, original.id);
    assert!(copy.mine && copy.shared_with.is_empty());
    assert_eq!(
        (&copy.name, &copy.text, &copy.presentation),
        (&original.name, &original.text, &original.presentation)
    );
    // Private: nobody else sees it, not even the maker of the original.
    let hidden = open(&crew.world, &ada, copy.id).await;
    assert!(refused(&hidden, ErrorCode::NotFound));
    let mine = list(&crew.world, &crew.dan.session_token, SearchSet::Mine).await?;
    assert_eq!(mine.len(), 1);
    // The original is as it was, and a person who does not see it cannot copy it.
    assert_eq!(
        open(&crew.world, &ada, original.id).await?.text,
        original.text
    );
    let denied = save_copy(
        &crew.world.ctx(),
        SaveCopy {
            session_token: crew.eve.session_token.clone(),
            search: original.id,
            name: Some("Stolen".to_owned()),
        },
    )
    .await;
    assert!(refused(&denied, ErrorCode::NotFound));
    // The copy can be named.
    let named = save_copy(
        &crew.world.ctx(),
        SaveCopy {
            session_token: crew.ben.session_token.clone(),
            search: original.id,
            name: Some("Bugs of mine".to_owned()),
        },
    )
    .await?
    .into_value();
    assert_eq!(named.name, "Bugs of mine");
    Ok(())
}

#[tokio::test]
async fn qry_06_delete_goes_to_the_trash_with_undo() -> Outcome {
    let crew = Crew::new().await?;
    let ben = &crew.ben.session_token;
    let made = save(&crew.world, ben, "My bugs", "status is open").await?;
    share(&crew.world, ben, made.id, ShareTarget::Project(crew.acme)).await?;
    let trash = |who: &str| DeleteSearch {
        session_token: who.to_owned(),
        search: made.id,
    };
    delete_search(&crew.world.ctx(), trash(ben)).await?;
    // In the trash it is not listed, not found, and not deleted twice.
    assert!(list(&crew.world, ben, SearchSet::Mine).await?.is_empty());
    assert!(
        list(&crew.world, &crew.dan.session_token, SearchSet::Shared)
            .await?
            .is_empty()
    );
    assert!(refused(
        &open(&crew.world, ben, made.id).await,
        ErrorCode::NotFound
    ));
    assert!(refused(
        &delete_search(&crew.world.ctx(), trash(ben)).await,
        ErrorCode::NotFound
    ));
    // Whoever may not take it out is not told that it is there.
    let untrash = |who: &str| RestoreSearch {
        session_token: who.to_owned(),
        search: made.id,
    };
    for other in [&crew.dan, &crew.eve, &crew.cleo] {
        let denied = restore_search(&crew.world.ctx(), untrash(&other.session_token)).await;
        assert!(refused(&denied, ErrorCode::NotFound));
    }
    // *Undo* brings it back as it was, still shared.
    let back = restore_search(&crew.world.ctx(), untrash(ben))
        .await?
        .into_value()
        .search
        .ok_or("the maker is shown the search")?;
    assert_eq!(back.name, "My bugs");
    assert_eq!(back.shared_with.len(), 1);
    assert_eq!(back.version, made.version + 2);
    assert_eq!(
        names(&list(&crew.world, &crew.dan.session_token, SearchSet::Shared).await?),
        ["My bugs"]
    );
    // Taking out what is not in the trash is not found.
    assert!(refused(
        &restore_search(&crew.world.ctx(), untrash(ben)).await,
        ErrorCode::NotFound
    ));
    Ok(())
}

#[tokio::test]
async fn qry_06_somebody_with_change_organization_settings_deletes_any_saved_search() -> Outcome {
    let crew = Crew::new().await?;
    let ben = &crew.ben.session_token;
    let private = save(&crew.world, ben, "Private", "").await?;
    let shared = save(&crew.world, ben, "Shared", "").await?;
    share(&crew.world, ben, shared.id, ShareTarget::Project(crew.acme)).await?;
    let ada = crew.ada();
    for search in [private.id, shared.id] {
        delete_search(
            &crew.world.ctx(),
            DeleteSearch {
                session_token: ada.clone(),
                search,
            },
        )
        .await?;
    }
    assert!(list(&crew.world, ben, SearchSet::Mine).await?.is_empty());
    // She takes it back as well, and the maker sees it again; she changes nothing else of it.
    restore_search(
        &crew.world.ctx(),
        RestoreSearch {
            session_token: ada.clone(),
            search: shared.id,
        },
    )
    .await?;
    assert_eq!(
        names(&list(&crew.world, ben, SearchSet::Mine).await?),
        ["Shared"]
    );
    // Taking a private search out of the trash tells her nothing of its name or text.
    let private_back = restore_search(
        &crew.world.ctx(),
        RestoreSearch {
            session_token: ada.clone(),
            search: private.id,
        },
    )
    .await?
    .into_value();
    assert_eq!(private_back.search, None);
    let change = change_search(
        &crew.world.ctx(),
        ChangeSearch {
            session_token: ada,
            search: shared.id,
            version: shared.version + 2,
            name: Some("Hers".to_owned()),
            text: None,
            presentation: None,
        },
    )
    .await;
    assert!(refused(&change, ErrorCode::Forbidden));
    Ok(())
}

#[tokio::test]
async fn qry_06_the_project_list_is_a_search_shared_with_the_project_that_a_lead_chooses() -> Outcome
{
    let crew = Crew::new().await?;
    let lead = crew
        .world
        .person_named("lea@example.test", "Lea Lead")
        .await
        .map_err(text)?;
    with_role(
        &crew.world,
        &lead,
        StartingRole::ProjectLead,
        Some(crew.acme),
    )
    .await?;
    let ben = &crew.ben.session_token;
    let made = save(&crew.world, ben, "ACME board", "status is open").await?;
    // Without a list of its own, the project shows the built-in default: none from the server.
    assert!(
        list_of(&crew.world, &lead.session_token, "ACME")
            .await?
            .search
            .is_none()
    );
    // A search that is not shared with the project cannot be its list.
    let early = choose_list(&crew.world, &lead.session_token, "ACME", Some(made.id)).await;
    assert!(refused(&early, ErrorCode::NotFound));
    share(&crew.world, ben, made.id, ShareTarget::Project(crew.acme)).await?;
    let lead_token = &lead.session_token;
    // A team member has no *Change project settings*; the lead does.
    let denied = choose_list(&crew.world, ben, "ACME", Some(made.id)).await;
    assert!(refused(&denied, ErrorCode::Forbidden));
    let chosen = choose_list(&crew.world, lead_token, "ACME", Some(made.id)).await?;
    assert_eq!(chosen.search.map(|view| view.id), Some(made.id));
    // Everyone who sees the project is shown it; Eve does not see the project.
    for who in [ben, &crew.dan.session_token, &crew.cleo.session_token] {
        let shown = list_of(&crew.world, who, "ACME").await?;
        assert_eq!(
            shown.search.map(|view| view.name),
            Some("ACME board".to_owned())
        );
    }
    assert!(refused(
        &list_of(&crew.world, &crew.eve.session_token, "ACME").await,
        ErrorCode::NotFound
    ));
    // WEB has none, and a search shared with ACME cannot be the list of WEB.
    let other = choose_list(&crew.world, &crew.ada(), "WEB", Some(made.id)).await;
    assert!(refused(&other, ErrorCode::InvalidRequest));
    // In the trash it is gone from the list, and *Undo* brings it back.
    delete_search(
        &crew.world.ctx(),
        DeleteSearch {
            session_token: ben.clone(),
            search: made.id,
        },
    )
    .await?;
    assert!(
        list_of(&crew.world, lead_token, "ACME")
            .await?
            .search
            .is_none()
    );
    restore_search(
        &crew.world.ctx(),
        RestoreSearch {
            session_token: ben.clone(),
            search: made.id,
        },
    )
    .await?;
    assert!(
        list_of(&crew.world, lead_token, "ACME")
            .await?
            .search
            .is_some()
    );
    // Stopping the share takes the list away: the project shows the built-in default again.
    unshare(&crew.world, ben, made.id, ShareTarget::Project(crew.acme)).await?;
    assert!(
        list_of(&crew.world, lead_token, "ACME")
            .await?
            .search
            .is_none()
    );
    // And so does choosing none.
    share(&crew.world, ben, made.id, ShareTarget::Project(crew.acme)).await?;
    choose_list(&crew.world, lead_token, "ACME", Some(made.id)).await?;
    let cleared = choose_list(&crew.world, lead_token, "ACME", None).await?;
    assert!(cleared.search.is_none());
    Ok(())
}

#[tokio::test]
async fn qry_06_a_person_who_is_removed_takes_the_private_searches_along_and_the_shared_ones_stay()
-> Outcome {
    let crew = Crew::new().await?;
    let ben = &crew.ben.session_token;
    let private = save(&crew.world, ben, "Private", "").await?;
    let shared = save(&crew.world, ben, "Shared", "").await?;
    share(&crew.world, ben, shared.id, ShareTarget::Project(crew.acme)).await?;
    let maker = crew.ben.person.id;
    let event = crew.world.ctx().event(
        by(maker),
        entity(RecordKind::Person, maker),
        vec![changed("person")],
    )?;
    let gone = crew
        .world
        .store
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                crate::tests::fake::FakeStore::delete_private_searches_of(transaction, maker).await
            })
        })
        .await
        .map_err(|error| error.to_string())?;
    assert_eq!(gone, [private.id]);
    let still: Option<SavedSearchId> = crew
        .world
        .store
        .saved_search(shared.id)
        .await
        .map_err(|error| error.to_string())?
        .map(|search| search.id);
    assert_eq!(still, Some(shared.id));
    // The shared search is still seen by its receivers, made by the same person row.
    let seen = open(&crew.world, &crew.dan.session_token, shared.id).await?;
    assert!(!seen.mine);
    Ok(())
}

#[tokio::test]
async fn perf_05_a_page_of_forty_saved_searches_costs_as_many_reads_as_a_page_of_five() -> Outcome {
    let crew = Crew::new().await?;
    let reads_for = |scope: SearchSet| {
        let crew = &crew;
        async move {
            let before = crew.world.store.reads();
            list(&crew.world, &crew.dan.session_token, scope).await?;
            Ok::<usize, AppError>(crew.world.store.reads() - before)
        }
    };
    let makers = [&crew.ben, &crew.cleo];
    let mut counts = Vec::new();
    for round in [5_usize, 40] {
        for at in 0..round {
            let maker = makers[at % makers.len()];
            let made = save(
                &crew.world,
                &maker.session_token,
                &format!("Search {at}"),
                "",
            )
            .await?;
            share(
                &crew.world,
                &maker.session_token,
                made.id,
                ShareTarget::Project(crew.acme),
            )
            .await?;
        }
        counts.push(reads_for(SearchSet::Shared).await?);
    }
    assert_eq!(counts[0], counts[1], "the reads grew with the page");
    Ok(())
}

#[tokio::test]
async fn sec_06_a_hidden_saved_search_and_a_missing_one_run_the_same_reads() -> Outcome {
    let crew = Crew::new().await?;
    let made = save(&crew.world, &crew.ben.session_token, "Private", "").await?;
    let missing = SavedSearchId::from_parts(jiff::Timestamp::UNIX_EPOCH, [5; 10])?;
    let eve = &crew.eve.session_token;
    let measure = |id: SavedSearchId| {
        let crew = &crew;
        async move {
            let before = crew.world.store.reads();
            let answer = open(&crew.world, eve, id).await;
            (answer, crew.world.store.reads() - before)
        }
    };
    let (hidden_answer, hidden_reads) = measure(made.id).await;
    let (missing_answer, missing_reads) = measure(missing).await;
    assert_eq!(hidden_answer, missing_answer);
    assert_eq!(hidden_reads, missing_reads);
    Ok(())
}
