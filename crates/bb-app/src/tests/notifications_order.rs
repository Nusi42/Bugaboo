//! The order of the list of notifications (NTF-01, UX §4.11): newest first by when it happened,
//! not by when the reader came to the event.

use super::notifications::{Change, Crew, Status};
use super::scenarios::token;
use super::world::Outcome;

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
