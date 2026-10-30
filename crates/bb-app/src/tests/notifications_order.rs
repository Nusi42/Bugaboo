//! The order of the list of notifications (NTF-01, UX §4.11): newest first by when it happened,
//! not by when the reader came to the event.

use super::notifications::{Change, Crew, Status};
use super::scenarios::{text, token};
use super::world::Outcome;
use crate::notifications::{
    ReadNotifications, UnreadQuery, Which, read_notifications, unread_notifications,
};

#[tokio::test]
async fn ntf_01_the_list_is_newest_first_by_when_it_happened_whenever_the_reader_read_it() -> Outcome
{
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    // Long after Ada began to follow the issue, so the earlier change below is still news to her.
    crew.world.clock.advance(1000);
    crew.change(&bob, Change::Status(Status::Done)).await?;
    // The reader comes to the first change now.
    assert_eq!(crew.heard(&ada).await?.len(), 1);
    // A change that happened earlier reaches the reader only later (a clock set back, as the
    // sample organization does, or a change that took long to commit).
    crew.world.clock.advance(-100);
    crew.change(&bob, Change::Status(Status::InProgress))
        .await?;
    crew.world.clock.advance(200);
    let heard = crew.heard(&ada).await?;
    let times: Vec<_> = heard.iter().map(|item| item.at).collect();
    let mut newest_first = times.clone();
    newest_first.sort_by(|a, b| b.cmp(a));
    assert_eq!(times.len(), 2);
    assert_eq!(
        times, newest_first,
        "the one that happened last comes first"
    );
    Ok(())
}

#[tokio::test]
async fn ntf_01_mark_all_as_read_leaves_unread_what_was_made_after_the_newest_shown_even_for_an_earlier_event()
-> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.world.clock.advance(1000);
    crew.change(&bob, Change::Status(Status::Done)).await?;
    // The person's page shows one notification: the newest she has seen.
    let shown = crew.heard(&ada).await?;
    assert_eq!(shown.len(), 1);
    let shown_newest = shown[0].id;
    // A change that happened earlier reaches the reader only after she looked.
    crew.world.clock.advance(-100);
    crew.change(&bob, Change::Status(Status::InProgress))
        .await?;
    crew.world.clock.advance(200);
    assert_eq!(unread_count(&crew).await?, 2);
    read_notifications(
        &crew.world.ctx(),
        ReadNotifications {
            session_token: ada.clone(),
            which: Which::UpTo(shown_newest),
        },
    )
    .await
    .map_err(text)?;
    // She has not seen the second one: it stays unread.
    assert_eq!(unread_count(&crew).await?, 1);
    Ok(())
}

/// How many notifications Ada has not read.
async fn unread_count(crew: &Crew) -> Result<usize, String> {
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
