//! The pages of a search that is put together from streams: the issues that have the words in the
//! title and then the others, and the projects of the sight one by one. Every way of finding the
//! words gives the same pages as asking about every issue, and a cut after the first rows of a
//! stream never changes which issues come first (PERF-01, M5-F46, M5-F48).

use bb_app::search::SearchRequest;
use bb_testing::{TestResult, each_database};

use super::Form;
use super::statements::Forms;
use super::tests::{Data, request, seed};
use super::tests_rest::comment_and_index;
use crate::AuthStorage;
use crate::test_support::fixture;

/// Every title of a search, a page of `limit` at a time, with the words found as `forms` says.
async fn walk(
    storage: &AuthStorage,
    ask: &SearchRequest,
    limit: u64,
    forms: Forms,
) -> Result<Vec<String>, String> {
    let mut titles = Vec::new();
    let mut after = None;
    loop {
        let mut page = ask.clone();
        page.limit = limit;
        page.after = after;
        let hits = storage
            .search_page_in(&page, forms)
            .await
            .map_err(|error| format!("{error:?}"))?;
        after = hits.last().map(|hit| hit.issue.id);
        let more = !hits.is_empty();
        titles.extend(hits.into_iter().map(|hit| hit.issue.title));
        if !more {
            return Ok(titles);
        }
    }
}

/// The comments that put `page` and `typo` in texts of issues that do not have them in the title.
async fn commented(storage: &AuthStorage, data: &mut Data) -> TestResult {
    let (slow, docs, idea, crash) = (
        data.issues[3].id,
        data.issues[2].id,
        data.issues[5].id,
        data.issues[4].id,
    );
    comment_and_index(
        storage,
        data,
        &[
            (slow, "a typo was noted"),
            (docs, "a page with a typo"),
            (idea, "the page jams"),
            (crash, "page crash"),
        ],
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn perf_02_every_way_of_finding_the_words_gives_the_same_pages() -> TestResult {
    each_database("search_streams_forms", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        commented(storage, &mut data).await?;
        let (each, first, second) = (Form::Each, Form::Driven(0), Form::Driven(1));
        let both = |title, rest| Forms { title, rest };
        let one_word = [
            Forms::EACH,
            both(first, first),
            both(first, each),
            both(each, first),
        ];
        let two_words = [
            Forms::EACH,
            both(first, second),
            both(second, first),
            both(second, second),
        ];
        // The title hit first, then the newest.
        let searches: [(&str, &[Forms], &[&str]); 6] = [
            (
                "contains page",
                &one_word,
                &["Slow page", "Idea", "Crash on save", "Docs typo"],
            ),
            (
                "contains page and contains typo",
                &one_word,
                &["Slow page", "Docs typo"],
            ),
            (
                "contains typo and contains page",
                &one_word,
                &["Docs typo", "Slow page"],
            ),
            (
                "contains \"page typo\"",
                &two_words,
                &["Slow page", "Docs typo"],
            ),
            (
                "contains \"typo page\"",
                &two_words,
                &["Slow page", "Docs typo"],
            ),
            (
                "contains page and priority is high",
                &one_word,
                &["Slow page", "Crash on save"],
            ),
        ];
        for (text, all_forms, expected) in searches {
            for forms in all_forms {
                for limit in [1, 2, 20] {
                    let ask = request(&data, text, None, limit);
                    assert_eq!(
                        walk(storage, &ask, limit, *forms).await?,
                        expected,
                        "{text} by {forms:?} {limit} at a time"
                    );
                }
            }
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_02_the_words_of_a_search_are_found_by_the_rarest_one_when_it_is_rare() -> TestResult {
    each_database("search_streams_probe", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        commented(storage, &mut data).await?;
        let ask = request(&data, "contains \"page typo\"", None, 20);
        let forms = async |at_most| {
            storage
                .forms_of(&ask, at_most)
                .await
                .map_err(|error| format!("{error:?}"))
        };
        // In titles `page` has one row and so has `typo` (the first is kept); in all texts `page`
        // has four and `typo` three.
        let driven = |title, rest| Forms {
            title: Form::Driven(title),
            rest: Form::Driven(rest),
        };
        assert_eq!(forms(5).await?, driven(0, 1));
        // A word with that many rows or more is not a rare one.
        assert_eq!(
            forms(4).await?,
            Forms {
                title: Form::Driven(0),
                rest: Form::Driven(1)
            }
        );
        assert_eq!(
            forms(3).await?,
            Forms {
                title: Form::Driven(0),
                rest: Form::Each
            }
        );
        assert_eq!(forms(1).await?, Forms::EACH);
        // No words to decide the order by: nothing to ask.
        let plain = request(&data, "priority is high", None, 20);
        assert_eq!(
            storage
                .forms_of(&plain, 5)
                .await
                .map_err(|error| format!("{error:?}"))?,
            Forms::EACH
        );
        Ok(())
    })
    .await
}

/// The titles of the issues of the tests, the newest first.
fn newest_first(data: &Data) -> Vec<String> {
    let mut all: Vec<_> = data.issues.iter().chain([&data.hidden]).collect();
    all.sort_by_key(|each| std::cmp::Reverse((each.created_at, each.id)));
    all.into_iter().map(|each| each.title.clone()).collect()
}

#[tokio::test]
async fn perf_02_a_page_over_a_few_projects_is_the_first_rows_of_each_put_together() -> TestResult {
    each_database("search_streams_projects", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let newest = newest_first(&data);
        // Two projects where the person sees everything.
        let mut both = request(&data, "", None, 20);
        both.sight.all = vec![data.project, data.hidden.project];
        for limit in [1, 2, 3, 20] {
            assert_eq!(
                walk(storage, &both, limit, Forms::EACH).await?,
                newest,
                "{limit}"
            );
        }
        // The other way round, and with a filter.
        let mut oldest = request(&data, "sort by created, oldest first", None, 20);
        oldest.sight = both.sight.clone();
        let mut expected = newest.clone();
        expected.reverse();
        for limit in [1, 2, 20] {
            assert_eq!(
                walk(storage, &oldest, limit, Forms::EACH).await?,
                expected,
                "{limit}"
            );
        }
        let mut high = request(&data, "priority is high", None, 20);
        high.sight = both.sight.clone();
        assert_eq!(
            walk(storage, &high, 1, Forms::EACH).await?,
            ["Crash on save", "Slow page", "Printer jams"]
        );
        // One project where everything is seen and one where only what the person reported.
        let mut mixed = request(&data, "", None, 20);
        mixed.sight.all = vec![data.project];
        mixed.sight.own = vec![data.hidden.project];
        for limit in [1, 2, 20] {
            assert_eq!(
                walk(storage, &mixed, limit, Forms::EACH).await?,
                newest,
                "{limit}"
            );
        }
        // Somebody who reported nothing in the second project reads nothing of it.
        mixed.sight.person = data.issues[5].reported_by;
        let without: Vec<String> = newest
            .iter()
            .filter(|title| *title != "Hidden one")
            .cloned()
            .collect();
        assert_eq!(walk(storage, &mixed, 2, Forms::EACH).await?, without);
        Ok(())
    })
    .await
}
