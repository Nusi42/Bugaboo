//! The port and the index on both databases: the same words find the same issues in the same
//! order (ARCH-DB-PARITY).

mod reader;

use bb_core::id::{EntityId, FieldId, IssueId};
use bb_core::text_index::TextSource;
use bb_core::text_words::search_words;
use bb_testing::{TestResult, each_database};
use sea_query::{Expr, ExprTrait as _, Order, Query};

use super::{TextScope, TextSearch, index_text, text_issues};
use crate::schema::IssueRow;
use crate::test_support::{fixture, ids, world};

/// An expected hit: the code of the issue.
fn hit(number: i64) -> String {
    number.to_string()
}

/// The issues that a search finds, the newest first (the order of the title hits is the
/// compiler's, tested with it in `search`).
pub(crate) async fn found(
    storage: &crate::AuthStorage,
    input: &str,
    scope: TextScope,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let database = storage.database();
    let words = search_words(input);
    let matched = text_issues(
        database.dialect(),
        &TextSearch {
            words: &words,
            scope,
        },
    );
    let mut select = Query::select();
    select
        .column((IssueRow::Table, IssueRow::Code))
        .from(IssueRow::Table)
        .and_where(Expr::col((IssueRow::Table, IssueRow::Id)).in_subquery(matched))
        .order_by((IssueRow::Table, IssueRow::CreatedAt), Order::Desc)
        .order_by((IssueRow::Table, IssueRow::Id), Order::Desc);
    let rows = database.fetch_all(&select).await?;
    rows.iter()
        .map(|row| Ok(row.get::<String>(IssueRow::Code)?))
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
        assert_eq!(found(&fixture.storage, words, scope).await?, [hit(1)]);
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
        assert_eq!(found(storage, "crash", everywhere).await?, [hit(1)]);
        assert_eq!(
            found(storage, "crash*", everywhere).await?,
            [hit(2), hit(1)]
        );
        assert_eq!(found(storage, "crashes", everywhere).await?, [hit(2)]);
        Ok(())
    })
    .await
}

/// The one rule of the words (M5-F53): lower case and NFC, no letter changed into another, so
/// *Müller* is found by *müller* however it is written, and not by *Muller* or *Mueller*.
#[tokio::test]
async fn iss_13_case_and_the_way_a_letter_is_written_do_not_matter_but_the_letter_does()
-> TestResult {
    each_database("iss_13_letters", async |db| {
        let fixture = fixture(db).await?;
        let mut ids = ids();
        let world = world(&fixture.storage, &mut ids, 2).await;
        let issue: Vec<IssueId> = world.issues.iter().map(|made| made.id).collect();
        let mut tx = fixture.storage.database().begin().await?;
        // The first title writes the umlaut as one character in some words and as two in others.
        index_text(
            &mut tx,
            issue[0],
            TextSource::Title,
            issue[0].into(),
            "Die Größe, très ÜBER, Mu\u{308}ller, Æther",
        )
        .await?;
        tx.commit().await?;
        let everywhere = TextScope::Everywhere;
        let found_by = [
            "größe",
            "GRÖ\u{1e9e}E",
            "Gro\u{308}ße",
            "TRÈS",
            "Über*",
            "u\u{308}ber",
            "müller",
            "MÜLLER",
            "Mu\u{308}ller",
            "mül*",
            "æther",
        ];
        for words in found_by {
            assert_eq!(
                found(&fixture.storage, words, everywhere).await?,
                [hit(1)],
                "{words:?}"
            );
        }
        let not_found_by = [
            "grosse", "GROSSE", "grösse", "tres", "uber", "muller", "mueller", "mu*", "aether",
        ];
        for words in not_found_by {
            assert_eq!(
                found(&fixture.storage, words, everywhere).await?,
                Vec::<String>::new(),
                "{words:?}"
            );
        }
        Ok(())
    })
    .await
}

/// Letters that NFC joins, takes apart or leaves with a mark on them stay one word in the index
/// and are found by the same words on both databases: the tokenizer of SQLite (`unicode61`, no
/// accents removed) and `simple` of PostgreSQL keep the stored word as it is (M5-F53).
#[tokio::test]
async fn iss_13_letters_that_nfc_joins_or_leaves_marked_are_found_alike_on_both_databases()
-> TestResult {
    each_database("iss_13_nfc_letters", async |db| {
        let fixture = fixture(db).await?;
        let mut ids = ids();
        let world = world(&fixture.storage, &mut ids, 2).await;
        let issue: Vec<IssueId> = world.issues.iter().map(|made| made.id).collect();
        let mut tx = fixture.storage.database().begin().await?;
        // A capital I with a dot above (it keeps its mark after lower case and NFC), a Japanese
        // syllable written as two characters, and a Devanagari letter with a dot written as one.
        index_text(
            &mut tx,
            issue[0],
            TextSource::Title,
            issue[0].into(),
            "\u{130}stanbul \u{304b}\u{3099}x \u{958}y \u{939}\u{93f}\u{928}\u{94d}\u{926}\u{940}",
        )
        .await?;
        tx.commit().await?;
        let everywhere = TextScope::Everywhere;
        for words in [
            "\u{130}stanbul",
            "i\u{307}stanbul",
            "i\u{307}st*",
            "\u{304c}x",
            "\u{304b}\u{3099}x",
            "\u{915}\u{93c}y",
            "\u{958}y",
            "\u{939}\u{93f}\u{928}",
        ] {
            assert_eq!(
                found(&fixture.storage, words, everywhere).await?,
                [hit(1)],
                "{words:?}"
            );
        }
        // The letters are kept: an I without the dot is another letter.
        for words in [
            "istanbul",
            "istan*",
            "\u{304b}x",
            "\u{915}y",
            // What the tokenizer of SQLite cut inside a word before migration 25: the part
            // after a mark that is not an accent is not a word of its own.
            "y",
            "stanbul",
            "\u{915}",
            "\u{939}\u{928}\u{94d}\u{926}",
        ] {
            assert_eq!(
                found(&fixture.storage, words, everywhere).await?,
                Vec::<String>::new(),
                "{words:?}"
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
        // The issue with the words in two texts is found (the newest first), and a word from
        // another issue does not count.
        assert_eq!(
            found(&fixture.storage, "login error", TextScope::Everywhere).await?,
            [hit(2), hit(1)]
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
            [hit(1)]
        );
        // A bare search never reads a field.
        assert_eq!(
            found(storage, "steps reproduce", TextScope::Everywhere).await?,
            [hit(3)]
        );
        Ok(())
    })
    .await
}
