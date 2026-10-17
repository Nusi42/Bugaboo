//! Handles (M3-F70, M4-Q9, ADR-0033 §6): made from the name, unique, changeable, and a mention that
//! names the person, so an old text keeps naming them.

use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::notification::NotificationKind;
use bb_core::people_kinds::PeopleKind;

use super::notifications::Crew;
use super::scenarios::{text, token};
use super::world::Outcome;
use crate::auth::AppError;
use crate::comments::{MentionChoicesQuery, mention_choices};
use crate::notifications::{
    ChangeHandle, CreateMissingHandles, MyHandleQuery, change_handle, create_missing_handles,
    my_handle,
};
use crate::roles::{SetPeopleLevel, set_level};

async fn handle_of(crew: &Crew, session: &str) -> Result<String, String> {
    Ok(my_handle(
        &crew.world.ctx(),
        MyHandleQuery {
            session_token: session.to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .handle)
}

async fn change(crew: &Crew, session: &str, handle: &str) -> Result<String, AppError> {
    Ok(change_handle(
        &crew.world.ctx(),
        ChangeHandle {
            session_token: session.to_owned(),
            handle: handle.to_owned(),
        },
    )
    .await?
    .into_value()
    .handle)
}

#[tokio::test]
async fn m3_f70_a_person_gets_a_handle_from_their_name_and_a_second_one_with_the_name_gets_a_number()
-> Outcome {
    let crew = Crew::new().await?;
    assert_eq!(handle_of(&crew, &token(&crew.world)).await?, "ada.admin");
    assert_eq!(
        handle_of(&crew, &crew.bob.session_token).await?,
        "bob.baker"
    );
    let another = crew
        .world
        .person_named("bob2@example.test", "Bob Baker")
        .await?;
    assert_eq!(
        handle_of(&crew, &another.session_token).await?,
        "bob.baker.2"
    );
    Ok(())
}

#[tokio::test]
async fn m3_f70_a_handle_is_changed_and_nobody_can_take_one_that_was_somebodys() -> Outcome {
    let crew = Crew::new().await?;
    let bob = crew.bob.session_token.clone();
    assert_eq!(change(&crew, &bob, "bobby").await?, "bobby");
    assert_eq!(handle_of(&crew, &bob).await?, "bobby");
    // The one Bob gave up is still his: Carol cannot take it.
    let carol = crew.carol.session_token.clone();
    assert_eq!(
        change(&crew, &carol, "bob.baker").await.err(),
        Some(AppError::Refused(ErrorCode::NameTaken))
    );
    // Nor the one he has.
    assert_eq!(
        change(&crew, &carol, "bobby").await.err(),
        Some(AppError::Refused(ErrorCode::NameTaken))
    );
    // He can take his old one back.
    assert_eq!(change(&crew, &bob, "bob.baker").await?, "bob.baker");
    // A text that is no handle is refused.
    for bad in ["", "Bob", "bob baker", "@bob", ".bob", &"a".repeat(65)] {
        assert_eq!(
            change(&crew, &bob, bad).await.err(),
            Some(AppError::Refused(ErrorCode::InvalidRequest)),
            "{bad:?}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn m3_f70_a_mention_names_the_person_and_an_old_handle_still_names_them() -> Outcome {
    let crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.comment(&ada, "Please look, @bob.baker.").await?;
    assert_eq!(
        crew.heard(&bob).await?.len(),
        1,
        "the handle of the name names him"
    );
    change(&crew, &bob, "bobby").await?;
    // The old text still shows him: the old handle is his, and the chip of the old text stays.
    crew.comment(&ada, "And again, @bob.baker.").await?;
    let kinds: Vec<_> = crew
        .heard(&bob)
        .await?
        .iter()
        .map(|item| item.kind)
        .collect();
    assert_eq!(
        kinds,
        [NotificationKind::Mentioned, NotificationKind::Mentioned]
    );
    // The new one names him too.
    crew.comment(&ada, "Last time, @bobby.").await?;
    assert_eq!(crew.heard(&bob).await?.len(), 3);
    Ok(())
}

#[tokio::test]
async fn m3_f70_the_choices_after_the_at_sign_are_handles_and_follow_who_sees_people() -> Outcome {
    let crew = Crew::new().await?;
    let ada = token(&crew.world);
    change(&crew, &crew.bob.session_token, "robert").await?;
    let offered = |session: String| {
        let world = &crew.world;
        let issue = crew.issue.to_string();
        async move {
            mention_choices(
                &world.ctx(),
                MentionChoicesQuery {
                    session_token: session,
                    issue,
                    search: Some("rob".to_owned()),
                    limit: 20,
                },
            )
            .await
            .map(|choices| {
                choices
                    .into_value()
                    .items
                    .into_iter()
                    .map(|choice| (choice.handle, choice.name))
                    .collect::<Vec<_>>()
            })
        }
    };
    assert_eq!(
        offered(ada.clone()).await?,
        [("robert".to_owned(), "Bob Baker".to_owned())],
        "found by the handle he chose, not by his name"
    );
    // A person who sees people by role only is offered no person.
    set_level(
        &crew.world.ctx(),
        SetPeopleLevel {
            session_token: ada,
            role: crew.role,
            project: None,
            kind: PeopleKind::Mentions,
            level: Some(PeopleLevel::RoleOnly),
        },
    )
    .await
    .map_err(text)?;
    assert!(offered(crew.carol.session_token.clone()).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn m3_f70_the_people_made_before_handles_get_theirs_when_the_server_starts() -> Outcome {
    let crew = Crew::new().await?;
    // The state of an older server: nobody has a handle.
    crew.world.store.lock().state.handles.clear();
    let made = create_missing_handles(&crew.world.ctx(), CreateMissingHandles).await?;
    assert_eq!(made, 3, "Ada, Bob and Carol");
    assert_eq!(
        handle_of(&crew, &crew.carol.session_token).await?,
        "carol.crane"
    );
    // Safe to run again.
    assert_eq!(
        create_missing_handles(&crew.world.ctx(), CreateMissingHandles).await?,
        0
    );
    Ok(())
}
