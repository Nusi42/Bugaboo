//! Migration 21: the words of the kinds of notifications and of notes about editing are written in
//! words joined by hyphens, like every other word of the API (M4 WP-16, API change A1): a
//! notification of `followed_activity` is now `followed-activity`, a note `finish_asked` is
//! `finish-asked`. The two tables are the only places that keep these words.
//!
//! The old and the new words are written out here, not read from the program's vocabulary: the
//! migration says what a database of migration 20 holds and what the program asks for from now on,
//! however the vocabulary changes later.

use sea_query::{Expr, ExprTrait as _, Query};

use super::Step;
use crate::schema::{EditNoticeRow, NotificationRow};

/// The kinds of notifications that changed.
const NOTIFICATION_KINDS: [(&str, &str); 4] = [
    ("followed_activity", "followed-activity"),
    ("followed_fields", "followed-fields"),
    ("run_ended", "run-ended"),
    ("edit_notice", "edit-notice"),
];

/// The kinds of notes about editing that changed.
const NOTICE_KINDS: [(&str, &str); 2] = [
    ("finish_asked", "finish-asked"),
    ("taken_over", "taken-over"),
];

pub(super) fn steps() -> Vec<Step> {
    let notifications = NOTIFICATION_KINDS.iter().map(|(old, new)| {
        Step::Update(
            Query::update()
                .table(NotificationRow::Table)
                .value(NotificationRow::Kind, *new)
                .and_where(Expr::col(NotificationRow::Kind).eq(*old))
                .to_owned(),
        )
    });
    let notices = NOTICE_KINDS.iter().map(|(old, new)| {
        Step::Update(
            Query::update()
                .table(EditNoticeRow::Table)
                .value(EditNoticeRow::Kind, *new)
                .and_where(Expr::col(EditNoticeRow::Kind).eq(*old))
                .to_owned(),
        )
    });
    notifications.chain(notices).collect()
}

#[cfg(test)]
mod tests {
    use bb_core::edit::NoticeKind;
    use bb_core::notification::NotificationKind;
    use bb_core::words::Word;
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, Order, Query};

    use super::super::Database;
    use super::*;
    use crate::schema::PersonRow;
    use crate::test_support::at;

    /// Every word of the two vocabularies that has a hyphen is the new word of a pair, and every
    /// new word of a pair is a word of the vocabulary: the migration and the program agree.
    #[test]
    fn m4_wp16_the_pairs_are_the_hyphenated_words_of_the_vocabularies() {
        fn hyphenated<W: Word>() -> Vec<&'static str> {
            W::ALL
                .iter()
                .map(|word| word.code())
                .filter(|code| code.contains('-'))
                .collect()
        }
        let new_notifications: Vec<&str> = NOTIFICATION_KINDS.iter().map(|pair| pair.1).collect();
        assert_eq!(new_notifications, hyphenated::<NotificationKind>());
        let new_notices: Vec<&str> = NOTICE_KINDS.iter().map(|pair| pair.1).collect();
        assert_eq!(new_notices, hyphenated::<NoticeKind>());
    }

    /// A person, so the rows that name one have somebody to name.
    async fn insert_person(database: &Database, person: &[u8]) -> TestResult {
        let mut made = Query::insert();
        made.into_table(PersonRow::Table)
            .columns([
                PersonRow::Id,
                PersonRow::DisplayName,
                PersonRow::Email,
                PersonRow::CreatedAt,
            ])
            .values([
                Expr::val(person.to_vec()),
                Expr::val("Ana"),
                Expr::val("ana@example.test"),
                Expr::val(1_i64),
            ])?;
        database.execute(&made).await?;
        Ok(())
    }

    /// A notification of the kind, as a database of migration 20 holds it.
    async fn insert_notification(
        database: &Database,
        person: &[u8],
        number: u8,
        kind: &str,
    ) -> TestResult {
        let mut row = Query::insert();
        row.into_table(NotificationRow::Table)
            .columns([
                NotificationRow::Id,
                NotificationRow::Recipient,
                NotificationRow::Kind,
                NotificationRow::Event,
                NotificationRow::Once,
                NotificationRow::At,
                NotificationRow::Changes,
                NotificationRow::InApp,
                NotificationRow::Mail,
            ])
            .values([
                Expr::val(vec![10 + number; 16]),
                Expr::val(person.to_vec()),
                Expr::val(kind),
                Expr::val(vec![20_u8; 16]),
                Expr::val(format!("once-{number}")),
                Expr::val(2_i64),
                Expr::val("[]"),
                Expr::val(true),
                Expr::val("never"),
            ])?;
        database.execute(&row).await?;
        Ok(())
    }

    /// A note about editing of the kind, as a database of migration 20 holds it.
    async fn insert_notice(database: &Database, person: &[u8], kind: &str) -> TestResult {
        let mut row = Query::insert();
        row.into_table(EditNoticeRow::Table)
            .columns([
                EditNoticeRow::Recipient,
                EditNoticeRow::RecordKind,
                EditNoticeRow::RecordId,
                EditNoticeRow::Part,
                EditNoticeRow::Kind,
                EditNoticeRow::FromPerson,
                EditNoticeRow::At,
            ])
            .values([
                Expr::val(person.to_vec()),
                Expr::val("issue"),
                Expr::val(vec![30_u8; 16]),
                Expr::val("description"),
                Expr::val(kind),
                Expr::val(person.to_vec()),
                Expr::val(3_i64),
            ])?;
        database.execute(&row).await?;
        Ok(())
    }

    #[tokio::test]
    async fn m4_wp16_migration_21_gives_the_kinds_the_hyphenated_words_and_keeps_the_rest()
    -> TestResult {
        each_database("migration_21_kebab_words", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(20, at(0)).await?;
            let person = vec![1_u8; 16];
            insert_person(&database, &person).await?;
            // One row of each kind: the old words and one that did not change.
            let kinds = ["mentioned", "followed_activity", "run_ended", "edit_notice"];
            for (number, kind) in kinds.iter().enumerate() {
                insert_notification(&database, &person, u8::try_from(number)?, kind).await?;
            }
            for kind in ["finish_asked", "released"] {
                insert_notice(&database, &person, kind).await?;
            }

            database.migrate(at(1)).await?;

            let mut notifications = Query::select();
            notifications
                .column(NotificationRow::Kind)
                .from(NotificationRow::Table)
                .order_by(NotificationRow::Id, Order::Asc);
            let found: Vec<String> = database
                .fetch_all(&notifications)
                .await?
                .iter()
                .map(|row| row.get::<String>(NotificationRow::Kind))
                .collect::<Result<_, _>>()?;
            assert_eq!(
                found,
                ["mentioned", "followed-activity", "run-ended", "edit-notice"]
            );
            let mut notices = Query::select();
            notices
                .column(EditNoticeRow::Kind)
                .from(EditNoticeRow::Table)
                .order_by(EditNoticeRow::Kind, Order::Asc);
            let found: Vec<String> = database
                .fetch_all(&notices)
                .await?
                .iter()
                .map(|row| row.get::<String>(EditNoticeRow::Kind))
                .collect::<Result<_, _>>()?;
            assert_eq!(found, ["finish-asked", "released"]);
            Ok(())
        })
        .await
    }
}
