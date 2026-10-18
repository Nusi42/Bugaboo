//! Adding a file to an issue and the notifications (NTF-01, ADR-0033 §1): the followers hear of it.

use bb_core::history::FileEvent;
use bb_core::notification::NotificationKind;

use super::notifications::{Crew, kinds};
use super::scenarios::{text, token};
use super::world::Outcome;
use crate::notifications::NotificationWhat;

/// A file `name` on the issue of the crew, added by the session `who`.
async fn add_file(crew: &Crew, who: &str, name: &str) -> Result<(), String> {
    let started = crate::files::start_upload(
        &crew.world.ctx(),
        crate::files::StartUploadQuery {
            session_token: who.to_owned(),
            issue: crew.issue.to_string(),
            comment: None,
            name: name.to_owned(),
            size: super::scenarios_files::A_FILE.len() as u64,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    crate::files::send_part(
        &crew.world.ctx(),
        crate::files::SendPartQuery {
            session_token: who.to_owned(),
            upload: started.id,
            offset: 0,
            bytes: super::scenarios_files::A_FILE.to_vec(),
        },
    )
    .await
    .map_err(text)?;
    crate::files::finish_upload(
        &crew.world.ctx(),
        crate::files::FinishUpload {
            session_token: who.to_owned(),
            upload: started.id,
        },
    )
    .await
    .map_err(text)?;
    crew.world.clock.advance(1);
    Ok(())
}

#[tokio::test]
async fn ntf_01_adding_a_file_tells_the_followers_and_not_the_one_who_added_it() -> Outcome {
    let crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    add_file(&crew, &bob, "jam.txt").await?;
    let heard = crew.heard(&ada).await?;
    assert_eq!(kinds(&heard), [NotificationKind::FollowedActivity]);
    assert!(
        matches!(&heard[0].what, NotificationWhat::File { event: FileEvent::Added, name: Some(name) } if name == "jam.txt"),
        "{:?}",
        heard[0].what
    );
    assert!(crew.heard(&bob).await?.is_empty());
    Ok(())
}
