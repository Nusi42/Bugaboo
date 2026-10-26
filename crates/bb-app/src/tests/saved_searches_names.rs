//! The names in the texts of the saved searches shared with a person (QRY-06, PERF-05): a request
//! resolves a bounded number of them, and a page that reaches the bound ends before the search
//! that needs more, so no search that checks cleanly is left out of the list.

use bb_core::saved_search::{SearchSet, ShareTarget};

use super::saved_searches::Crew;
use super::scenarios_searches::{save, share};
use super::search_run::issue_of;
use super::world::Outcome;
use crate::saved_searches::{SearchesQuery, saved_searches};
use crate::search::NAME_RESOLUTIONS_MAX;

#[tokio::test]
async fn perf_05_more_names_than_a_request_resolves_leave_no_shared_search_out_of_the_list()
-> Outcome {
    let crew = Crew::new().await?;
    // Searches that each name three different issues, so that the names of the list are far more
    // than one request resolves, and the first page cannot hold them all.
    let per_search = 3;
    let searches = NAME_RESOLUTIONS_MAX / per_search + 4;
    let mut made = Vec::new();
    for number in 0..searches {
        let mut text = Vec::new();
        for _ in 0..per_search {
            let issue = issue_of(&crew.world, crew.acme).await?;
            text.push(format!("under ACME-{}", issue.code.as_str()));
        }
        let name = format!("Search {number:03}");
        let each = save(&crew.world, &crew.ada(), &name, &text.join(" or ")).await?;
        share(
            &crew.world,
            &crew.ada(),
            each.id,
            ShareTarget::Project(crew.acme),
        )
        .await?;
        made.push(name);
    }
    // Ben reads the list a page at a time: every search of it, each once, however the pages end.
    let (mut seen, mut cursor, mut pages, mut first_page) = (Vec::new(), None, 0, 0);
    loop {
        let page = saved_searches(
            &crew.world.ctx(),
            SearchesQuery {
                session_token: crew.ben.session_token.clone(),
                scope: SearchSet::Shared,
                cursor,
                limit: 100,
            },
        )
        .await?
        .into_value();
        pages += 1;
        if pages == 1 {
            first_page = page.items.len();
        }
        seen.extend(page.items.into_iter().map(|view| view.name));
        cursor = page.next_cursor;
        assert!(pages < 20, "the list does not end");
        if cursor.is_none() {
            break;
        }
    }
    assert!(
        first_page < made.len(),
        "the first page stopped at the bound"
    );
    assert!(pages > 1);
    seen.sort();
    assert_eq!(seen, made);
    Ok(())
}
