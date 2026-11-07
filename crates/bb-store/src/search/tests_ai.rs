//! A search for the AI leaves out the texts that an administrator kept from AI (AI-03): a bare
//! `contains` looks in the title and the description with the other texts, so it must not.

use bb_app::search::WithheldText;
use bb_testing::{TestResult, each_database};

use super::tests::{request, seed};
use super::tests_more::titles_of;
use super::tests_rest::comment_and_index;
use crate::test_support::fixture;

#[tokio::test]
async fn ai_03_a_bare_contains_never_matches_a_field_kept_from_ai() -> TestResult {
    each_database("search_bare_contains_withheld", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let login = data.issues[1].id;
        comment_and_index(storage, &mut data, &[(login, "Zebra stripes appear")]).await?;
        let found = |text: &'static str, withheld: WithheldText| {
            let mut ask = request(&data, text, None, 20);
            ask.withheld_text = withheld;
            async move { titles_of(storage, &ask).await }
        };
        let nothing = WithheldText::default();
        let title = WithheldText {
            title: true,
            description: false,
        };
        // The title is found by a bare `contains` for a person, and not for the AI.
        assert_eq!(found("contains crash", nothing).await?, ["Crash on save"]);
        assert!(found("contains crash", title).await?.is_empty());
        // The other texts of the same issue are still found: a comment.
        assert_eq!(found("contains zebra", title).await?, ["Login broken"]);
        // The title is the only place of that word, so nothing else matches it either.
        assert_eq!(found("contains login", title).await?, Vec::<String>::new());
        Ok(())
    })
    .await
}
