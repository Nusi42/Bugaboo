//! What the database holds after a run, read through the store: the only place that reads it.

use std::error::Error;

use bb_app::member::comment;
use bb_core::edit::{DraftTarget, EditWhat};
use bb_core::event::{DomainEvent, EntityKind, EntityRef, RecordKind};
use bb_core::fields::Field;
use bb_core::id::{EntityId, IssueId, UserId};
use bb_core::technical::TechnicalName;
use bb_store::Database;

use crate::book::{Book, DraftEnd, last_drafts};

/// The most events of one kind that are read: far more than a run makes.
const MOST_EVENTS: u64 = 1_000_000;

/// The error of reading: anything that went wrong, with its words.
pub type Failure = Box<dyn Error + Send + Sync>;

/// What the database holds about one issue.
#[derive(Debug)]
pub struct IssueFacts {
    /// The readable ID of the issue.
    pub key: String,
    /// The version after each event about the issue that counted one, oldest first.
    pub event_versions: Vec<i64>,
    /// How many events wrote a comment on the issue.
    pub comment_events: usize,
    /// The version the issue row has.
    pub version: i64,
    /// The description the issue row has.
    pub description: String,
}

/// A lock row.
#[derive(Debug)]
pub struct LockRow {
    /// The ID of the record the lock is on.
    pub record: String,
    /// When the lease of the lock ends, in seconds since 1970.
    pub expires_at: i64,
}

/// The draft of a person for the description of an issue, if the database has one.
#[derive(Debug)]
pub struct StoredDraft {
    /// The ID of the person.
    pub holder: String,
    /// The readable ID of the issue.
    pub key: String,
    /// Its text.
    pub text: Option<String>,
}

/// Everything the rules look at.
#[derive(Debug, Default)]
pub struct Facts {
    /// The issues of the run.
    pub issues: Vec<IssueFacts>,
    /// Every lock row.
    pub locks: Vec<LockRow>,
    /// The drafts that the book names.
    pub drafts: Vec<StoredDraft>,
}

fn describe(what: &str, error: impl std::fmt::Display) -> Failure {
    format!("{what}: {error}").into()
}

/// Whether a comment event names this issue.
fn is_comment_on(event: &DomainEvent, issue: &str) -> bool {
    !event.counted_version()
        && event.changes.iter().any(|change| {
            change.field == comment::ISSUE
                && change.after.as_ref().and_then(|after| after.as_str()) == Some(issue)
        })
}

fn description_of(id: &str) -> Result<EditWhat, Failure> {
    Ok(EditWhat {
        record: EntityRef {
            kind: RecordKind::Issue.into(),
            id: id.parse::<EntityId>().map_err(|e| describe(id, e))?,
        },
        part: TechnicalName::new(Field::Description.code()).map_err(|e| describe("a part", e))?,
    })
}

/// Every lock row the database holds, an expired one too.
///
/// # Errors
/// Fails when the database cannot be read.
pub async fn lock_rows(database: &Database) -> Result<Vec<LockRow>, Failure> {
    Ok(database
        .edit_lock_rows()
        .await?
        .into_iter()
        .map(|lock| LockRow {
            record: lock.record.id.to_string(),
            expires_at: lock.expires_at.as_second(),
        })
        .collect())
}

/// Reads what the rules look at: the issues of the book, every lock row and the drafts the book
/// names.
///
/// # Errors
/// Fails when the database cannot be read or the book names an ID that is none.
pub async fn gather(database: &Database, book: &Book) -> Result<Facts, Failure> {
    let comments = database
        .events_of_kind(&EntityKind::from(RecordKind::Comment), MOST_EVENTS)
        .await?;
    let mut facts = Facts::default();
    for work in &book.issues {
        let what = description_of(&work.id)?;
        let events = database.events_about(&what.record, MOST_EVENTS).await?;
        let id: IssueId = work.id.parse().map_err(|e| describe(&work.id, e))?;
        let row = database
            .stored_issue(id)
            .await?
            .ok_or_else(|| describe(&work.key, "the issue row is missing"))?;
        facts.issues.push(IssueFacts {
            key: work.key.clone(),
            event_versions: events
                .iter()
                .filter_map(DomainEvent::version_after)
                .collect(),
            comment_events: comments
                .iter()
                .filter(|event| is_comment_on(event, &work.id))
                .count(),
            version: row.version,
            description: row.description,
        });
    }
    facts.locks = lock_rows(database).await?;
    for drafted in last_drafts(book) {
        if drafted.ending == DraftEnd::Saved {
            continue;
        }
        let work = book
            .issues
            .iter()
            .find(|work| work.key == drafted.key)
            .ok_or_else(|| describe(&drafted.key, "a draft on an issue the book does not name"))?;
        let holder: UserId = drafted
            .holder
            .parse()
            .map_err(|e| describe(&drafted.holder, e))?;
        let draft = database
            .stored_draft(holder, DraftTarget::Editing(description_of(&work.id)?))
            .await?;
        facts.drafts.push(StoredDraft {
            holder: drafted.holder.clone(),
            key: drafted.key.clone(),
            text: draft.map(|draft| draft.text),
        });
    }
    Ok(facts)
}
