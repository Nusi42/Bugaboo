//! Handles (M3-F70, M4-Q9, ADR-0033 §6): made from the name, unique, changeable, and a mention that
//! names the person, so an old text keeps naming them.

use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::handle::CHANGE_EVERY_DAYS;
use bb_core::notification::NotificationKind;
use bb_core::people_kinds::PeopleKind;
use bb_core::time::SECONDS_PER_DAY;

use super::notifications::Crew;
use super::scenarios::{text, token};
use super::world::Outcome;
use crate::auth::AppError;
use crate::comments::{MentionChoicesQuery, mention_choices};
use crate::notifications::{
    ChangeHandle, CreateMissingHandles, MyHandleQuery, change_handle, create_missing_handles,
    my_handle,
};
use crate::people::{CreateGroup, create_group};
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

/// Lets `seconds` pass for the changes of handles: a session would not last the days between two
/// changes, so the moment of each change is moved back instead of the clock forward.
fn let_pass(crew: &Crew, seconds: i64) {
    for handle in &mut crew.world.store.lock().state.handles {
        handle.changed_at = handle
            .changed_at
            .checked_sub(jiff::SignedDuration::from_secs(seconds))
            .unwrap_or(handle.changed_at);
    }
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
    // He can take his old one back, once the time between two changes has passed.
    let_pass(&crew, CHANGE_EVERY_DAYS * SECONDS_PER_DAY + 1);
    assert_eq!(change(&crew, &bob, "bob.baker").await?, "bob.baker");
    // A text that is no handle is refused.
    for bad in [
        "",
        "Bob",
        "bob baker",
        "@bob",
        ".bob",
        "böb",
        "ボブ",
        &"a".repeat(65),
    ] {
        assert_eq!(
            change(&crew, &bob, bad).await.err(),
            Some(AppError::Refused(ErrorCode::InvalidRequest)),
            "{bad:?}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn m3_f70_a_mention_names_the_person_with_that_handle_now_and_an_old_text_keeps_the_person()
-> Outcome {
    let crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.comment(&ada, "Please look, @bob.baker.").await?;
    assert_eq!(
        crew.heard(&bob).await?.len(),
        1,
        "the handle of the name names him"
    );
    change(&crew, &bob, "bobby").await?;
    // The text of before stores the person, so it still shows him.
    assert_eq!(crew.heard(&bob).await?.len(), 1);
    // The handle he gave up names nobody in a new text, and nobody else can take it.
    crew.comment(&ada, "And again, @bob.baker.").await?;
    assert_eq!(
        crew.heard(&bob).await?.len(),
        1,
        "a former handle names nobody"
    );
    assert_eq!(
        change(&crew, &crew.carol.session_token, "bob.baker")
            .await
            .err(),
        Some(AppError::Refused(ErrorCode::NameTaken))
    );
    // The new one names him.
    crew.comment(&ada, "Last time, @bobby.").await?;
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
    Ok(())
}

#[tokio::test]
async fn m3_f70_a_name_with_letters_outside_ascii_gets_an_ascii_handle_or_the_fallback() -> Outcome
{
    let crew = Crew::new().await?;
    let german = crew
        .world
        .person_named("juergen@example.test", "Jürgen Groß")
        .await?;
    assert_eq!(
        handle_of(&crew, &german.session_token).await?,
        "juergen.gross"
    );
    let russian = crew
        .world
        .person_named("ivan@example.test", "Иван Петров")
        .await?;
    assert_eq!(handle_of(&crew, &russian.session_token).await?, "person");
    let another = crew
        .world
        .person_named("pyotr@example.test", "Пётр")
        .await?;
    assert_eq!(handle_of(&crew, &another.session_token).await?, "person.2");
    Ok(())
}

#[tokio::test]
async fn m3_f70_nobody_takes_the_handle_of_a_group_or_a_role() -> Outcome {
    let crew = Crew::new().await?;
    let ada = token(&crew.world);
    create_group(
        &crew.world.ctx(),
        CreateGroup {
            session_token: ada.clone(),
            name: "Night Shift".to_owned(),
        },
    )
    .await
    .map_err(text)?;
    let bob = crew.bob.session_token.clone();
    for taken in ["night.shift", "project.lead"] {
        assert_eq!(
            change(&crew, &bob, taken).await.err(),
            Some(AppError::Refused(ErrorCode::NameTaken)),
            "{taken}"
        );
    }
    // A person named like the group gets a number, as when two people share a name.
    let person = crew
        .world
        .person_named("night@example.test", "Night Shift")
        .await?;
    assert_eq!(
        handle_of(&crew, &person.session_token).await?,
        "night.shift.2"
    );
    Ok(())
}

#[tokio::test]
async fn m3_f70_a_handle_is_changed_once_in_a_period_and_the_first_one_does_not_count() -> Outcome {
    let crew = Crew::new().await?;
    let bob = crew.bob.session_token.clone();
    // The first change comes right after the account was made.
    change(&crew, &bob, "bobby").await?;
    assert_eq!(
        change(&crew, &bob, "robert").await.err(),
        Some(AppError::Refused(ErrorCode::HandleChangedLately))
    );
    let_pass(&crew, CHANGE_EVERY_DAYS * SECONDS_PER_DAY - 60);
    assert_eq!(
        change(&crew, &bob, "robert").await.err(),
        Some(AppError::Refused(ErrorCode::HandleChangedLately)),
        "not yet"
    );
    let_pass(&crew, 120);
    assert_eq!(change(&crew, &bob, "robert").await?, "robert");
    // Asking for the handle one has is no change.
    assert_eq!(change(&crew, &bob, "robert").await?, "robert");
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

#[tokio::test]
async fn m3_f70_of_two_changes_of_one_handle_at_the_same_moment_one_is_kept() -> Outcome {
    let crew = Crew::new().await?;
    let bob = crew.bob.session_token.clone();
    let (first, second) = tokio::join!(change(&crew, &bob, "bobby"), change(&crew, &bob, "robert"));
    let results = [first, second];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(results.iter().any(|result| matches!(
        result,
        Err(AppError::Refused(ErrorCode::HandleChangedLately))
    )));
    Ok(())
}
