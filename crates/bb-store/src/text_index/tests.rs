//! The port and the index on both databases: the same words find the same issues in the same
//! order (ARCH-DB-PARITY).

mod reader;

use bb_core::id::{EntityId, FieldId, IssueId};
use bb_core::text_index::TextSource;
use bb_core::text_words::search_words;
use bb_testing::{TestResult, each_database};
use sea_query::{Expr, ExprTrait as _, Order, Query};

use super::{TextMatch, TextScope, TextSearch, index_text, text_matches};
use crate::schema::IssueRow;
use crate::test_support::{fixture, ids, world};

/// An expected hit: the code of the issue and whether its title matched.
fn hit(number: i64, title: i64) -> (String, i64) {
    (number.to_string(), title)
}

/// The issues that a search finds, title hits first and then the newest (the order the compiler of
/// the search asks for), each with its title hit.
async fn found(
    storage: &crate::AuthStorage,
    input: &str,
    scope: TextScope,
) -> Result<Vec<(String, i64)>, Box<dyn std::error::Error + Send + Sync>> {
    let database = storage.database();
    let words = search_words(input);
    let matched = text_matches(
        database.dialect(),
        &TextSearch {
            words: &words,
            scope,
        },
    );
    let mut select = Query::select();
    select
        .column((IssueRow::Table, IssueRow::Code))
        .expr(Expr::col(("text_hit", TextMatch::TITLE_HIT)))
        .from(IssueRow::Table)
        .join_subquery(
            sea_query::JoinType::InnerJoin,
            matched.select,
            "text_hit",
            Expr::col((IssueRow::Table, IssueRow::Id)).equals(("text_hit", TextMatch::ISSUE)),
        )
        .order_by(("text_hit", TextMatch::TITLE_HIT), Order::Desc)
        .order_by((IssueRow::Table, IssueRow::CreatedAt), Order::Desc)
        .order_by((IssueRow::Table, IssueRow::Id), Order::Desc);
    let rows = database.fetch_all(&select).await?;
    rows.iter()
        .map(|row| {
            Ok((
                row.get::<String>(IssueRow::Code)?,
                row.get::<i64>(TextMatch::TITLE_HIT)?,
            ))
        })
        .collect()
}

/// Puts `text` of the source into the index for the first of two issues and finds it with `words`:
/// the second issue has other words and is never found.
async fn finds_through(
    name: &str,
    source: TextSource,
    text: &'static str,
    words: &'static str,
) -> TestResult {
    each_database(name, async |db| {
        let fixture = fixture(db).await?;
        let mut ids = ids();
        let world = world(&fixture.storage, &mut ids, 2).await;
        let issue: Vec<IssueId> = world.issues.iter().map(|made| made.id).collect();
        let field = FieldId::generate(&mut ids)?;
        // The title and the description are the issue's own; the others have a record of their own.
        let reference = match source {
            TextSource::Title | TextSource::Description => EntityId::from(issue[0]),
            TextSource::Field => field.into(),
            TextSource::Comment | TextSource::File => EntityId::from(IssueId::generate(&mut ids)?),
        };
        let mut tx = fixture.storage.database().begin().await?;
        index_text(&mut tx, issue[0], source, reference, text).await?;
        index_text(
            &mut tx,
            issue[1],
            TextSource::Title,
            issue[1].into(),
            "nothing here",
        )
        .await?;
        tx.commit().await?;
        let scope = if source == TextSource::Field {
            TextScope::Field(field)
        } else {
            TextScope::Everywhere
        };
        let title = i64::from(source == TextSource::Title);
        assert_eq!(
            found(&fixture.storage, words, scope).await?,
            [hit(1, title)]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_13_a_word_in_the_title_finds_the_issue() -> TestResult {
    finds_through("iss_13_title", TextSource::Title, "Crash on save", "save").await
}

#[tokio::test]
async fn iss_13_a_word_in_the_description_finds_the_issue() -> TestResult {
    finds_through(
        "iss_13_description",
        TextSource::Description,
        "Steps: open the Größe dialog",
        "dialog",
    )
    .await
}

#[tokio::test]
async fn iss_13_a_word_in_a_comment_finds_the_issue() -> TestResult {
    finds_through(
        "iss_13_comment",
        TextSource::Comment,
        "It CRASHES at once",
        "crashes",
    )
    .await
}

#[tokio::test]
async fn iss_13_a_word_in_a_file_name_finds_the_issue() -> TestResult {
    finds_through(
        "iss_13_file",
        TextSource::File,
        "Protokoll-Übersicht.pdf",
        "protokoll",
    )
    .await
}

#[tokio::test]
async fn iss_13_a_word_in_a_long_text_field_finds_the_issue_when_that_field_is_asked() -> TestResult
{
    finds_through(
        "iss_13_field",
        TextSource::Field,
        "notes about printers",
        "printers",
    )
    .await
}

#[tokio::test]
async fn iss_13_crash_does_not_find_crashes_but_crash_star_does() -> TestResult {
    each_database("iss_13_prefix", async |db| {
        let fixture = fixture(db).await?;
        let mut ids = ids();
        let world = world(&fixture.storage, &mut ids, 3).await;
        let issue: Vec<IssueId> = world.issues.iter().map(|made| made.id).collect();
        let mut tx = fixture.storage.database().begin().await?;
        index_text(
            &mut tx,
            issue[0],
            TextSource::Title,
            issue[0].into(),
            "A crash",
        )
        .await?;
        index_text(
            &mut tx,
            issue[1],
            TextSource::Title,
            issue[1].into(),
            "Many crashes",
        )
        .await?;
        index_text(
            &mut tx,
            issue[2],
            TextSource::Title,
            issue[2].into(),
            "Nothing",
        )
        .await?;
        tx.commit().await?;
        let storage = &fixture.storage;
        let everywhere = TextScope::Everywhere;
        assert_eq!(found(storage, "crash", everywhere).await?, [hit(1, 1)]);
        assert_eq!(
            found(storage, "crash*", everywhere).await?,
            [hit(2, 1), hit(1, 1)]
        );
        assert_eq!(found(storage, "crashes", everywhere).await?, [hit(2, 1)]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_13_accents_and_case_do_not_matter() -> TestResult {
    each_database("iss_13_accents", async |db| {
        let fixture = fixture(db).await?;
        let mut ids = ids();
        let world = world(&fixture.storage, &mut ids, 2).await;
        let issue: Vec<IssueId> = world.issues.iter().map(|made| made.id).collect();
        let mut tx = fixture.storage.database().begin().await?;
        index_text(
            &mut tx,
            issue[0],
            TextSource::Title,
            issue[0].into(),
            "Die Größe, très ÜBER",
        )
        .await?;
        tx.commit().await?;
        let everywhere = TextScope::Everywhere;
        for words in ["GROSSE", "grösse", "tres", "TRÈS", "uber", "Über*"] {
            assert_eq!(
                found(&fixture.storage, words, everywhere).await?,
                [hit(1, 1)],
                "{words}"
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_13_the_words_may_be_in_different_texts_of_one_issue() -> TestResult {
    each_database("iss_13_words_anywhere", async |db| {
        let fixture = fixture(db).await?;
        let mut ids = ids();
        let world = world(&fixture.storage, &mut ids, 4).await;
        let issue: Vec<IssueId> = world.issues.iter().map(|made| made.id).collect();
        let (note, other) = (
            EntityId::from(IssueId::generate(&mut ids)?),
            EntityId::from(IssueId::generate(&mut ids)?),
        );
        let mut tx = fixture.storage.database().begin().await?;
        // 1: both words in the title. 2: one in the title, one in a comment. 3: one word in each
        // of two issues. 4: only one word.
        index_text(
            &mut tx,
            issue[0],
            TextSource::Title,
            issue[0].into(),
            "Login error",
        )
        .await?;
        index_text(
            &mut tx,
            issue[1],
            TextSource::Title,
            issue[1].into(),
            "Login fails",
        )
        .await?;
        index_text(
            &mut tx,
            issue[1],
            TextSource::Comment,
            note,
            "I see an error",
        )
        .await?;
        index_text(
            &mut tx,
            issue[2],
            TextSource::Title,
            issue[2].into(),
            "Login",
        )
        .await?;
        index_text(&mut tx, issue[3], TextSource::Comment, other, "error").await?;
        tx.commit().await?;
        // Title hits come first; the issue with the words in two texts is found, and a word
        // from another issue does not count.
        assert_eq!(
            found(&fixture.storage, "login error", TextScope::Everywhere).await?,
            [hit(1, 1), hit(2, 0)]
        );
        // A word typed twice, or a start of one letter, finds what the whole word finds.
        assert_eq!(
            found(&fixture.storage, "error error e*", TextScope::Everywhere).await?,
            found(&fixture.storage, "error e", TextScope::Everywhere).await?
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_13_the_port_reads_a_field_only_when_that_field_is_asked_for() -> TestResult {
    each_database("iss_13_field_scope", async |db| {
        let fixture = fixture(db).await?;
        let mut ids = ids();
        let world = world(&fixture.storage, &mut ids, 3).await;
        let issue: Vec<IssueId> = world.issues.iter().map(|made| made.id).collect();
        let (wanted, other) = (FieldId::generate(&mut ids)?, FieldId::generate(&mut ids)?);
        let mut tx = fixture.storage.database().begin().await?;
        index_text(
            &mut tx,
            issue[0],
            TextSource::Field,
            wanted.into(),
            "steps to reproduce",
        )
        .await?;
        index_text(
            &mut tx,
            issue[1],
            TextSource::Field,
            other.into(),
            "steps to reproduce",
        )
        .await?;
        index_text(
            &mut tx,
            issue[2],
            TextSource::Title,
            issue[2].into(),
            "steps to reproduce",
        )
        .await?;
        tx.commit().await?;
        let storage = &fixture.storage;
        assert_eq!(
            found(storage, "steps reproduce", TextScope::Field(wanted)).await?,
            [hit(1, 0)]
        );
        // A bare search never reads a field.
        assert_eq!(
            found(storage, "steps reproduce", TextScope::Everywhere).await?,
            [hit(3, 1)]
        );
        Ok(())
    })
    .await
}
