//! The port and the index on both databases: the same words find the same issues in the same
//! order (ARCH-DB-PARITY).

mod reader;

use bb_core::id::{EntityId, FieldId, IssueId};
use bb_core::text_index::TextSource;
use bb_core::text_words::{prepare, search_words};
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
/// and are found by the same words on both databases: the tokenizer of SQLite (`ascii`) and the
/// words split at spaces of PostgreSQL keep the stored word as it is (M5-F53).
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

/// The characters a test of the tokenizers goes over: the ones the review named (a final sigma, the
/// combining iota, letters of the category "other symbol" such as a circled letter), the cases of
/// the tests above, whole blocks that fold or take apart (Latin, Greek, the extended Greek with the
/// iota below, Cyrillic, the circled letters, the combining marks), and every 211th letter and
/// number there is, so a script no block names is in too.
fn tokenizer_characters() -> Vec<char> {
    let blocks = [
        0x20..0x250,
        0x300..0x370,
        0x370..0x400,
        0x400..0x500,
        0x1f00..0x2000,
        0x2460..0x2500,
        0xff00..0xfff0,
    ];
    let named =
        "ςσΣ\u{345}\u{3b9}\u{399}ⓐⒶ\u{130}ıİßẞﬁǅ\u{1e9e}\u{a8}\u{3d1}\u{2126}\u{212a}\u{1c5}";
    let mut all: Vec<char> = named.chars().collect();
    all.extend(blocks.into_iter().flatten().filter_map(char::from_u32));
    all.extend(
        ('\0'..=char::MAX)
            .filter(|c| c.is_alphanumeric())
            .step_by(211),
    );
    all.sort_unstable();
    all.dedup();
    all
}

/// The two databases keep a stored word as it is (M5-F53, ADR-0016): the tokenizer of SQLite
/// (`ascii`) splits at the spaces and nowhere else and folds nothing, and PostgreSQL compares the
/// words split at spaces. For each character: every word that Rust prepares from a text with it is
/// found, no piece cut out of a word is found as a word, and the character written in lower case
/// as its upper case lower-cases it (a final sigma becomes the middle one, the combining iota
/// becomes the letter) is another word and is not found.
#[tokio::test]
async fn iss_13_the_databases_keep_a_stored_word_as_rust_prepared_it() -> TestResult {
    each_database("iss_13_tokenizers", async |db| {
        let fixture = fixture(db).await?;
        let mut ids = ids();
        let world = world(&fixture.storage, &mut ids, 1).await;
        let issue = world.issues[0].id;
        let characters = tokenizer_characters();
        let mut fields = Vec::new();
        let mut tx = fixture.storage.database().begin().await?;
        for letter in &characters {
            let field = FieldId::generate(&mut ids)?;
            let text = format!("x{letter}x {letter}y {letter}");
            index_text(&mut tx, issue, TextSource::Field, field.into(), &text).await?;
            fields.push((field, text));
        }
        tx.commit().await?;
        let mut wrong = Vec::new();
        for (letter, (field, text)) in characters.iter().zip(&fields) {
            let scope = TextScope::Field(*field);
            let stored = prepare(text);
            let asked = |words: &str, must_be_found: bool| {
                let words = words.to_owned();
                let storage = &fixture.storage;
                async move {
                    let found = !found(storage, &words, scope).await?.is_empty();
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
                        (found == must_be_found).then_some(()),
                    )
                }
            };
            for word in &stored {
                if asked(word, true).await?.is_none() {
                    wrong.push(format!("{:04X} {word:?} is not found", u32::from(*letter)));
                }
                // The start of a word is found by its start, and a piece is not a word.
                let chars: Vec<char> = word.chars().collect();
                let n = chars.len();
                let pieces = [
                    chars[..n - 1].iter().collect::<String>(),
                    chars[1..].iter().collect::<String>(),
                ];
                for piece in pieces {
                    let is_a_word = stored.contains(&piece);
                    if n > 1
                        && !piece.is_empty()
                        && prepare(&piece) == [piece.clone()]
                        && !is_a_word
                        && asked(&piece, false).await?.is_none()
                    {
                        wrong.push(format!(
                            "{:04X} {piece:?} of {word:?} is found",
                            u32::from(*letter)
                        ));
                    }
                }
                // Another word that a fold of the tokenizer would make the same.
                let folded: String = word
                    .chars()
                    .flat_map(char::to_uppercase)
                    .flat_map(char::to_lowercase)
                    .collect();
                if folded != *word
                    && prepare(&folded) == [folded.clone()]
                    && !stored.contains(&folded)
                    && asked(&folded, false).await?.is_none()
                {
                    wrong.push(format!(
                        "{:04X} {folded:?} finds {word:?}",
                        u32::from(*letter)
                    ));
                }
            }
        }
        assert!(wrong.is_empty(), "{} mistakes: {wrong:?}", wrong.len());
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
