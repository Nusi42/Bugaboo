//! The adapter of `bb_app::privacy::removal::RemovalStore` (PRIV-04, ADR-0044, decision 6): the
//! rows a removal deletes and the changes it scrubs, found by the conditions of the walk and by
//! nothing else.

use bb_app::privacy::removal::{PersonWords, RemovalStore, Scrub, ScrubPlace, ScrubbedTable};
use bb_core::event::{DomainEvent, FieldChange};
use bb_core::id::{CommentId, IssueId, JobId, UserId};
use bb_core::job::JobState;
use bb_core::personal::{Entity as _, EntityMarks, Link, LinkScope};
use bb_core::removal::PendingRemoval;
use bb_core::secret::StoredSecret;
use bb_core::technical::TechnicalName;
use bb_core::whose::MADE_ONLY_BY_THEIR_OWNER;
use jiff::Timestamp;
use sea_query::{
    Alias, Cond, Expr, ExprTrait as _, Iden, Order, Query, SelectStatement, UnionType,
};
use zeroize::Zeroizing;

use super::{UNSTORED_LINK, column, linked, record_parts, table_of};
use crate::AuthStorage;
use crate::db::{Database, Dialect, Row, StoredId as _, Transaction, to_micros};
use crate::error::{SecretProblem, StoreError};
use crate::events::ACTOR_PERSON;
use crate::like;
use crate::schema::{
    Event, HistoryEntryRow, Job, NotificationRow, PersonHandleRow, PersonRow, RemovalPendingRow,
    tables,
};
use crate::secrets::{sealed_secret, write_secret};
use crate::text_index::{compact_text_index, forget_comments, reindex_issues};
use crate::walk_index::{Indexed, KeyPart, after_key, name};

impl RemovalStore for AuthStorage {
    async fn reindex_removed_texts(
        &self,
        issues: &[IssueId],
        comments: &[CommentId],
    ) -> Result<(), StoreError> {
        let mut transaction = self.database().begin().await?;
        forget_comments(&mut transaction, comments).await?;
        reindex_issues(&mut transaction, issues).await?;
        transaction.commit().await
    }

    async fn compact_removed_texts(&self) -> Result<(), StoreError> {
        compact_text_index(self.database()).await
    }

    async fn scrub_step(
        &self,
        scrub: &Scrub,
        from: &ScrubPlace,
    ) -> Result<Option<ScrubPlace>, StoreError> {
        self.scrub_words_step(scrub, from).await
    }

    async fn seal_removal_words(
        &self,
        person: UserId,
        words: &PersonWords,
        at: Timestamp,
    ) -> Result<StoredSecret, StoreError> {
        let value = Zeroizing::new(serde_json::to_vec(words)?);
        sealed_secret(
            &self.key_ring().newest()?,
            &words_secret(person)?,
            &value,
            at,
        )
    }

    async fn removal_words(&self, person: UserId) -> Result<Option<PersonWords>, StoreError> {
        match self
            .database()
            .get_secret(self.key_ring(), &words_secret(person)?)
            .await
        {
            Ok(value) => Ok(Some(serde_json::from_slice(&value)?)),
            Err(StoreError::Secret(SecretProblem::Missing)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn stopped_removals(&self) -> Result<Vec<PendingRemoval>, StoreError> {
        self.database()
            .fetch_all(&stopped_select())
            .await?
            .iter()
            .map(read_pending)
            .collect()
    }

    async fn pending_removal_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<PendingRemoval>, StoreError> {
        let mut select = pending_select();
        select.and_where(
            Expr::col((RemovalPendingRow::Table, RemovalPendingRow::Person)).eq(person.stored()),
        );
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_pending)
            .transpose()
    }

    async fn start_removal(
        transaction: &mut Transaction,
        pending: &PendingRemoval,
        words: &StoredSecret,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(RemovalPendingRow::Table)
            .columns(PENDING_COLUMNS)
            .values([
                Expr::val(pending.person.stored()),
                Expr::val(pending.comments_go),
                Expr::val(pending.job.stored()),
                Expr::val(pending.step.as_str()),
                Expr::val(to_micros(pending.created_at)),
                Expr::val(to_micros(pending.updated_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a removal that is not finished"))?;
        transaction.execute(&insert).await?;
        write_secret(transaction, words).await
    }

    async fn move_removal(
        transaction: &mut Transaction,
        person: UserId,
        from: JobId,
        to: JobId,
        step: &str,
        at: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(RemovalPendingRow::Table)
            .value(RemovalPendingRow::Job, to.stored())
            .value(RemovalPendingRow::Step, step)
            .value(RemovalPendingRow::UpdatedAt, to_micros(at))
            .and_where(Expr::col(RemovalPendingRow::Person).eq(person.stored()))
            .and_where(Expr::col(RemovalPendingRow::Job).eq(from.stored()));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn end_removal(
        transaction: &mut Transaction,
        person: UserId,
        job: JobId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(RemovalPendingRow::Table)
            .and_where(Expr::col(RemovalPendingRow::Person).eq(person.stored()))
            .and_where(Expr::col(RemovalPendingRow::Job).eq(job.stored()));
        if transaction.execute(&delete).await? == 0 {
            return Ok(false);
        }
        Database::delete_secret_in(transaction, &words_secret(person)?).await?;
        Ok(true)
    }

    async fn hide_comments_of(
        transaction: &mut Transaction,
        author: UserId,
        at: Timestamp,
    ) -> Result<u64, StoreError> {
        crate::comments::hide_comments_of(transaction, author, at).await
    }

    async fn person_words(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<PersonWords>, StoreError> {
        let mut select = Query::select();
        select
            .columns([PersonRow::DisplayName, PersonRow::Email])
            .from(PersonRow::Table)
            .and_where(Expr::col(PersonRow::Id).eq(person.stored()));
        let Some(row) = transaction.fetch_optional(&select).await? else {
            return Ok(None);
        };
        let mut handles = Query::select();
        handles
            .column(PersonHandleRow::Handle)
            .from(PersonHandleRow::Table)
            .and_where(Expr::col(PersonHandleRow::Person).eq(person.stored()));
        let handles = transaction
            .fetch_all(&handles)
            .await?
            .iter()
            .map(|row| row.get(PersonHandleRow::Handle))
            .collect::<Result<_, _>>()?;
        Ok(Some(PersonWords {
            name: row.get(PersonRow::DisplayName)?,
            address: row.get(PersonRow::Email)?,
            handles,
        }))
    }

    async fn delete_linked(
        transaction: &mut Transaction,
        person: UserId,
        kind: &EntityMarks,
        links: &[Link],
    ) -> Result<u64, StoreError> {
        let all = tables();
        let Some(table) = all.iter().find(|table| table.marks.name() == kind.name()) else {
            return Ok(0);
        };
        let Some(any) = linked(table, &all, person, links, transaction.dialect())? else {
            return Ok(0);
        };
        let mut delete = Query::delete();
        delete
            .from_table(Alias::new(table.name.as_str()))
            .cond_where(any);
        transaction.execute(&delete).await
    }

    async fn delete_jobs_with_address(
        transaction: &mut Transaction,
        address: &str,
    ) -> Result<u64, StoreError> {
        if address.is_empty() {
            return Ok(0);
        }
        let mut delete = Query::delete();
        delete
            .from_table(Job::Table)
            .and_where(like::containing_any_case(
                Expr::col(Job::Payload),
                &as_in_json(address),
            ));
        transaction.execute(&delete).await
    }

    async fn scrub_records(
        transaction: &mut Transaction,
        person: UserId,
        scrub: &Scrub,
    ) -> Result<u64, StoreError> {
        let Some(about) = events_about(person, scrub, transaction.dialect())? else {
            return Ok(0);
        };
        let mut changed = 0;
        for table in ScrubbedTable::ALL {
            let scrubbed = columns_of(*table);
            changed += rewrite(
                transaction,
                &scrubbed,
                Some(Cond::all().add(Expr::col(scrubbed.event.clone()).in_subquery(about.clone()))),
                Scrubbing::Every,
            )
            .await?;
        }
        Ok(changed)
    }
}

/// The name of the secret that holds the words of the removal of `person`.
fn words_secret(person: UserId) -> Result<TechnicalName, StoreError> {
    TechnicalName::new(&format!("privacy.removal.{person}"))
        .map_err(|_| StoreError::Corrupt("the name of the words of a removal"))
}

/// The columns of a row of a removal that is not finished, in the order it is written.
const PENDING_COLUMNS: [RemovalPendingRow; 6] = [
    RemovalPendingRow::Person,
    RemovalPendingRow::CommentsGo,
    RemovalPendingRow::Job,
    RemovalPendingRow::Step,
    RemovalPendingRow::CreatedAt,
    RemovalPendingRow::UpdatedAt,
];

/// The rows of the removals that are not finished, with their job beside them (`LEFT JOIN`).
fn pending_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns(PENDING_COLUMNS.map(|column| (RemovalPendingRow::Table, column)))
        .from(RemovalPendingRow::Table)
        .left_join(
            Job::Table,
            Expr::col((Job::Table, Job::Id))
                .equals((RemovalPendingRow::Table, RemovalPendingRow::Job)),
        );
    select
}

/// The removals that stopped: their job is on the dead-letter list, or gone.
fn stopped_select() -> SelectStatement {
    let mut select = pending_select();
    select.cond_where(
        Cond::any()
            .add(Expr::col((Job::Table, Job::Id)).is_null())
            .add(Expr::col((Job::Table, Job::State)).eq(JobState::Dead.code())),
    );
    select
}

fn read_pending(row: &Row) -> Result<PendingRemoval, StoreError> {
    Ok(PendingRemoval {
        person: row.id(RemovalPendingRow::Person)?,
        comments_go: row.get(RemovalPendingRow::CommentsGo)?,
        job: row.id(RemovalPendingRow::Job)?,
        step: row.get(RemovalPendingRow::Step)?,
        created_at: row.time(RemovalPendingRow::CreatedAt)?,
        updated_at: row.time(RemovalPendingRow::UpdatedAt)?,
    })
}

impl Database {
    /// How many removals stopped (their job gave up or is gone): what the status page shows until
    /// `privacy.resume-removals` gives each a new job.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn stopped_removal_count(&self) -> Result<u64, StoreError> {
        Ok(u64::try_from(self.fetch_all(&stopped_select()).await?.len()).unwrap_or(u64::MAX))
    }
}

/// The columns of a table whose changes a removal scrubs: those of its index of the walk (the
/// table, the key, the changes), the column of its event and the column of its reason, if any.
fn columns_of(table: ScrubbedTable) -> Scrubbed {
    let (of, event, reason) = match table {
        ScrubbedTable::Events => (Indexed::Event, name(&Event::Id), Some(name(&Event::Reason))),
        ScrubbedTable::History => (Indexed::History, name(&HistoryEntryRow::Event), None),
        ScrubbedTable::Notifications => {
            (Indexed::Notification, name(&NotificationRow::Event), None)
        }
    };
    Scrubbed {
        table: of.table(),
        changes: of.changes(),
        event,
        key: of.key(),
        reason,
    }
}

/// The IDs of the events about a record that `scrub` names: of one of its kinds and the person's,
/// as the walk finds them, or one of its records; and the events the person made themselves about
/// a record of one of its kinds that only their owner makes (`MADE_ONLY_BY_THEIR_OWNER`), which
/// can be gone by now (the name of an access key they deleted). What the person did to the records
/// of others (the roles they gave, the work times they edited) stays. Each way is a select of its
/// own, joined with `UNION ALL`, so an index answers each, as the arms of the walk; none when
/// `scrub` names no record.
fn events_about(
    person: UserId,
    scrub: &Scrub,
    dialect: Dialect,
) -> Result<Option<SelectStatement>, StoreError> {
    let all = tables();
    let events = table_of(&all, DomainEvent::MARKS)?;
    let record = events
        .marks
        .person_links()
        .links()
        .iter()
        .find(|link| matches!(link.scope(), LinkScope::Record { kind: None }))
        .ok_or(UNSTORED_LINK)?;
    let kind_column = column(events, record.field(), false)?;
    let id_column = column(events, record.field(), true)?;
    let mut parts = record_parts(
        &all,
        &scrub.kinds,
        &kind_column,
        &id_column,
        (person, dialect),
    )?;
    for named in &scrub.records {
        parts.push(
            Cond::all()
                .add(Expr::col(kind_column.clone()).eq(named.kind.as_str()))
                .add(Expr::col(id_column.clone()).eq(named.id.stored())),
        );
    }
    let own: Vec<&str> = scrub
        .kinds
        .iter()
        .filter(|kind| MADE_ONLY_BY_THEIR_OWNER.contains(kind))
        .map(|kind| kind.code())
        .collect();
    if !own.is_empty() {
        parts.push(
            Cond::all()
                .add(Expr::col(Event::ActorKind).eq(ACTOR_PERSON))
                .add(Expr::col(Event::ActorId).eq(person.stored()))
                .add(Expr::col(kind_column).is_in(own)),
        );
    }
    let mut parts = parts.into_iter().map(|part| {
        let mut select = Query::select();
        select.column(Event::Id).from(Event::Table).cond_where(part);
        select
    });
    let Some(mut about) = parts.next() else {
        return Ok(None);
    };
    for part in parts {
        about.union(UnionType::All, part);
    }
    Ok(Some(about))
}

/// `word` as JSON writes it inside a string, so a word with a quote is found as it is stored.
fn as_in_json(word: &str) -> String {
    let quoted = serde_json::Value::String(word.to_owned()).to_string();
    quoted
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(&quoted)
        .to_owned()
}

/// How many rows one page of the scrub of the records reads in its change: the rows about the
/// person's records, which the index of each table finds.
const SCRUB_PAGE: u64 = 500;

/// How many rows one step of the scrub of the words reads, outside any change: enough that a large
/// table takes few steps, few enough that a step holds no writer up for long (only the rows of it
/// that change are written, in one short change).
pub(crate) const SCRUB_STEP_ROWS: u64 = 5_000;

/// A table whose changes are scrubbed: its column of changes, the column of its event, the columns
/// of its key (IDs, 16 bytes each), and the column of the reason, if it has one.
struct Scrubbed {
    table: Alias,
    changes: Alias,
    event: Alias,
    key: Vec<Alias>,
    reason: Option<Alias>,
}

/// How the changes of a row are scrubbed.
#[derive(Clone, Copy)]
enum Scrubbing<'s> {
    /// Every value goes: the change is about a record of the person.
    Every,
    /// The values that hold one of the person's words go.
    Holding(&'s Scrub),
}

impl Scrubbing<'_> {
    fn takes_text(self, text: &str) -> bool {
        match self {
            Self::Every => true,
            Self::Holding(scrub) => scrub.holds(text),
        }
    }

    fn takes(self, value: Option<&serde_json::Value>) -> bool {
        match self {
            Self::Every => value.is_some(),
            Self::Holding(scrub) => value.is_some_and(|value| scrub.holds_value(value)),
        }
    }
}

/// The changes `stored` with the values `how` names taken out.
fn scrubbed(stored: &str, how: Scrubbing<'_>) -> Result<String, StoreError> {
    let corrupt = |_| StoreError::Corrupt("the changes of an event");
    let mut changes: Vec<FieldChange> = serde_json::from_str(stored).map_err(corrupt)?;
    for change in &mut changes {
        if how.takes(change.before.as_ref()) {
            change.before = None;
        }
        if how.takes(change.after.as_ref()) {
            change.after = None;
        }
    }
    serde_json::to_string(&changes).map_err(corrupt)
}

/// The names the steps of a scrub read the changes and the reason of a row by (the key is read by
/// [`KeyPart`]).
#[derive(Clone, Copy, Iden)]
enum Read {
    Changes,
    Reason,
}

/// A row of a scrubbed table, as a step reads it.
struct ReadRow {
    key: Vec<Vec<u8>>,
    changes: String,
    reason: Option<String>,
}

/// The select of the key, the changes and the reason of the rows of `table`.
fn read_rows(table: &Scrubbed) -> SelectStatement {
    let mut select = Query::select();
    select.from(table.table.clone());
    KeyPart::select(&mut select, &table.key);
    select.expr_as(Expr::col(table.changes.clone()), Read::Changes);
    match &table.reason {
        Some(reason) => select.expr_as(Expr::col(reason.clone()), Read::Reason),
        None => select.expr_as(Expr::val(Option::<String>::None), Read::Reason),
    };
    for column in &table.key {
        select.order_by(column.clone(), Order::Asc);
    }
    select
}

fn read_row(table: &Scrubbed, row: &Row) -> Result<ReadRow, StoreError> {
    Ok(ReadRow {
        key: KeyPart::read(row, table.key.len())?,
        changes: row.get(Read::Changes)?,
        reason: row.get(Read::Reason)?,
    })
}

/// The scrubbed changes and whether the reason goes, when the row changes; none when it does not.
fn scrubbed_row(row: &ReadRow, how: Scrubbing<'_>) -> Result<Option<(String, bool)>, StoreError> {
    let new = scrubbed(&row.changes, how)?;
    let drop_reason = row
        .reason
        .as_deref()
        .is_some_and(|text| how.takes_text(text));
    Ok((new != row.changes || drop_reason).then_some((new, drop_reason)))
}

/// Writes the scrubbed changes of `row`, only while its changes are still those it was read with:
/// a removal of somebody else that scrubbed the row meanwhile is not undone. Whether it was
/// written.
async fn write_scrubbed(
    transaction: &mut Transaction,
    table: &Scrubbed,
    row: &ReadRow,
    (new, drop_reason): (String, bool),
) -> Result<u64, StoreError> {
    let mut update = Query::update();
    update
        .table(table.table.clone())
        .value(table.changes.clone(), new)
        .and_where(Expr::col(table.changes.clone()).eq(row.changes.clone()));
    if let (true, Some(column)) = (drop_reason, &table.reason) {
        update.value(column.clone(), Option::<String>::None);
    }
    for (column, value) in table.key.iter().zip(&row.key) {
        update.and_where(Expr::col(column.clone()).eq(value.clone()));
    }
    transaction.execute(&update).await
}

/// Writes the scrubbed changes (and reason) of each row of `table` that matches `rows` (every row
/// for none), going through them by key a page at a time: each row is read once with its key and
/// updated by it.
async fn rewrite(
    transaction: &mut Transaction,
    table: &Scrubbed,
    rows: Option<Cond>,
    how: Scrubbing<'_>,
) -> Result<u64, StoreError> {
    let mut rewritten = 0;
    let mut last: Option<Vec<Vec<u8>>> = None;
    loop {
        let mut select = read_rows(table);
        select.limit(SCRUB_PAGE);
        if let Some(rows) = &rows {
            select.cond_where(rows.clone());
        }
        if let Some(last) = &last {
            select.cond_where(after_key(&table.key, last));
        }
        let page = transaction.fetch_all(&select).await?;
        for found in &page {
            let row = read_row(table, found)?;
            if let Some(scrubbed) = scrubbed_row(&row, how)? {
                rewritten += write_scrubbed(transaction, table, &row, scrubbed).await?;
            }
            last = Some(row.key);
        }
        if (page.len() as u64) < SCRUB_PAGE {
            return Ok(rewritten);
        }
    }
}

impl AuthStorage {
    /// One step of the scrub of the words (`RemovalStore::scrub_step`): the next
    /// [`SCRUB_STEP_ROWS`] rows of the table of `from`, read by key outside any change, and the
    /// rows of them that hold one of the words read again and written in one short change.
    pub(crate) async fn scrub_words_step(
        &self,
        scrub: &Scrub,
        from: &ScrubPlace,
    ) -> Result<Option<ScrubPlace>, StoreError> {
        let table = columns_of(from.table);
        let corrupt = || StoreError::Corrupt("the place of a scrub");
        let after = from
            .after
            .iter()
            .map(|part| {
                bb_core::hex::decode::<16>(part)
                    .map(|id| id.to_vec())
                    .ok_or_else(corrupt)
            })
            .collect::<Result<Vec<_>, _>>()?;
        // The rows of the step: the next SCRUB_STEP_ROWS by key, up to the last of them.
        let mut end = Query::select();
        end.from(table.table.clone())
            .limit(1)
            .offset(SCRUB_STEP_ROWS - 1);
        KeyPart::select(&mut end, &table.key);
        for column in &table.key {
            end.order_by(column.clone(), Order::Asc);
        }
        let mut select = read_rows(&table);
        if !after.is_empty() {
            end.cond_where(after_key(&table.key, &after));
            select.cond_where(after_key(&table.key, &after));
        }
        let end: Option<Vec<Vec<u8>>> = match self.database().fetch_optional(&end).await? {
            Some(row) => Some(KeyPart::read(&row, table.key.len())?),
            None => None,
        };
        if let Some(end) = &end {
            select.cond_where(after_key(&table.key, end).not());
        }
        // Of them, the database reads only the rows that may hold a word (folded the way it folds,
        // `like::may_hold_folded`); Rust decides.
        let columns: Vec<Alias> = std::iter::once(table.changes.clone())
            .chain(table.reason.clone())
            .collect();
        if let Some(may_hold) = candidates(&columns, &scrub.looked_for()) {
            select.cond_where(may_hold);
        }
        let mut holding = Vec::new();
        for found in &self.database().fetch_all(&select).await? {
            let row = read_row(&table, found)?;
            if scrubbed_row(&row, Scrubbing::Holding(scrub))?.is_some() {
                holding.push(row.key);
            }
        }
        if !holding.is_empty() {
            let mut transaction = self.database().begin().await?;
            for key in &holding {
                let mut select = read_rows(&table);
                for (column, value) in table.key.iter().zip(key) {
                    select.and_where(Expr::col(column.clone()).eq(value.clone()));
                }
                let Some(found) = transaction.fetch_optional(&select).await? else {
                    continue;
                };
                let row = read_row(&table, &found)?;
                if let Some(scrubbed) = scrubbed_row(&row, Scrubbing::Holding(scrub))?
                    && write_scrubbed(&mut transaction, &table, &row, scrubbed).await? == 0
                {
                    // Changed between the read and the write: the step is tried again.
                    return Err(StoreError::Busy);
                }
            }
            transaction.commit().await?;
        }
        Ok(match end {
            Some(key) => Some(ScrubPlace {
                table: from.table,
                after: key.iter().map(|part| bb_core::hex::encode(part)).collect(),
            }),
            None => from.table.next().map(|table| ScrubPlace {
                table,
                after: Vec::new(),
            }),
        })
    }
}

/// The rows of `columns` that may hold one of `words` (folded in Rust): the candidates of
/// [`like::may_hold_folded`], or every row (`None`) when a word keeps too little to narrow by.
fn candidates(columns: &[Alias], words: &[String]) -> Option<Cond> {
    let mut holding = Cond::any();
    for word in words {
        for column in columns {
            holding = holding.add(like::may_hold_folded(Expr::col(column.clone()), word)?);
        }
    }
    Some(holding)
}

#[cfg(test)]
#[path = "removal_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "removal_pending_tests.rs"]
pub(crate) mod pending_tests;

/// The name of the one value the sweep reads.
#[cfg(any(test, feature = "testing"))]
#[derive(Clone, Copy, Iden)]
enum Swept {
    Value,
}

/// The value the sweep read, as text: bytes that are not UTF-8 (SQLite casts a stored ID as it is)
/// are read with the characters they cannot hold replaced.
///
/// # Errors
/// The error of reading it as text, when it can be read neither way.
#[cfg(any(test, feature = "testing"))]
fn swept_text(row: &Row) -> Result<Option<String>, StoreError> {
    row.get::<Option<String>>(Swept::Value).or_else(|error| {
        row.get::<Option<Vec<u8>>>(Swept::Value)
            .map(|bytes| bytes.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
            .map_err(|_| error)
    })
}

#[cfg(any(test, feature = "testing"))]
impl AuthStorage {
    /// The columns (`table.column`) of every stored table that hold one of `texts` in some row,
    /// folded as names are (`bb_core::names::name_key`): the sweep that proves a removal leaves no
    /// name, handle or address behind. The database picks the candidates as a scrub does, and Rust
    /// decides; a candidate that cannot be read as text counts as found, so the sweep fails rather
    /// than overlooks.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn columns_holding(&self, texts: &[&str]) -> Result<Vec<String>, StoreError> {
        use bb_core::names::name_key;
        let folded: Vec<String> = texts.iter().map(|text| name_key(text)).collect();
        let mut found = Vec::new();
        for table in tables() {
            for (column, _) in &table.columns {
                let as_text = sea_query::Func::cast_as(
                    Expr::col(Alias::new(column.as_str())),
                    Alias::new("text"),
                );
                let mut select = Query::select();
                select
                    .expr_as(as_text.clone(), Swept::Value)
                    .from(Alias::new(table.name.as_str()));
                let mut any = Cond::any();
                let mut narrowed = true;
                for word in &folded {
                    match like::may_hold_folded(as_text.clone(), word) {
                        Some(may) => any = any.add(may),
                        None => narrowed = false,
                    }
                }
                if narrowed {
                    select.cond_where(any);
                }
                let holds = self.database().fetch_all(&select).await?.iter().any(|row| {
                    match swept_text(row) {
                        Ok(value) => value.is_some_and(|value| {
                            let value = name_key(&value);
                            folded.iter().any(|word| value.contains(word.as_str()))
                        }),
                        Err(_) => true,
                    }
                });
                if holds {
                    found.push(format!("{}.{column}", table.name));
                }
            }
        }
        found.sort();
        found.dedup();
        Ok(found)
    }
}
