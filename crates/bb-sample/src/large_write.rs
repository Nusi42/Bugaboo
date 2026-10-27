//! Writing the issues of the large dataset: batch after batch in big transactions, with the events
//! of each batch appended in the order they happened, then the counters of the projects, and at the
//! end what a server does by itself: the history and the notifications are read from the events.

use bb_app::comments::CommentStore as _;
use bb_app::context::Clock as _;
use bb_app::fields::FieldStore as _;
use bb_app::files::AttachmentStore as _;
use bb_app::follow::FollowStore as _;
use bb_app::issues::IssueStore as _;
use bb_app::relations::RelationStore as _;
use bb_app::tags::TagStore as _;
use bb_core::event::{EntityKind, RecordKind};
use bb_core::technical::TechnicalName;
use bb_store::AuthStorage;
use jiff::Timestamp;

use crate::error::{SampleError, Step};
use crate::generate::World;
use crate::large_build::{Bundle, Maker};
use crate::large_org::Org;
use crate::large_state::{Bank, Counters};

/// How the issues are spread in time and how many there are.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Span {
    pub(crate) issues: u32,
    /// The first moment, in seconds since 1970.
    pub(crate) begin: i64,
    /// The last moment.
    pub(crate) end: i64,
}

/// The name the outbox keeps its last position under.
const OUTBOX_HEAD: &str = "head";

fn cumulative(parts: impl Iterator<Item = u64>) -> Vec<u64> {
    let mut sum = 0_u64;
    parts
        .map(|part| {
            sum += part;
            sum
        })
        .collect()
}

async fn write_batch(storage: &AuthStorage, bundles: &[Bundle]) -> Result<(), SampleError> {
    let mut events: Vec<_> = bundles
        .iter()
        .flat_map(|bundle| bundle.events.iter().cloned())
        .collect();
    // Events come in the order they happened; the sort is stable, so an issue's own events
    // keep their order.
    events.sort_by_key(|event| event.time);
    storage
        .database()
        .record_changes(&events, async |transaction| {
            for bundle in bundles {
                AuthStorage::insert_issue(transaction, &bundle.issue, &bundle.assigned).await?;
                for (field, cells) in &bundle.values {
                    AuthStorage::put_field_value(
                        transaction,
                        EntityKind::from(RecordKind::Issue),
                        bundle.issue.id.into(),
                        *field,
                        cells,
                    )
                    .await?;
                }
                AuthStorage::add_followers(transaction, &bundle.followers).await?;
                for comment in &bundle.comments {
                    AuthStorage::insert_comment(transaction, comment).await?;
                }
                for link in &bundle.tags {
                    AuthStorage::insert_issue_tag(transaction, link).await?;
                }
                for link in &bundle.labels {
                    AuthStorage::insert_issue_label(transaction, link).await?;
                }
                if let Some(parent) = &bundle.parent {
                    AuthStorage::set_issue_parent(transaction, parent).await?;
                }
                if let Some(relation) = &bundle.relation {
                    AuthStorage::insert_relation(transaction, relation).await?;
                }
                for attachment in &bundle.attachments {
                    AuthStorage::insert_attachment(transaction, attachment).await?;
                }
            }
            Ok(())
        })
        .await?;
    Ok(())
}

/// Writes `span.issues` issues and then reads the history and the notifications from the events.
///
/// # Errors
/// [`SampleError`] when a write or a read fails.
pub(crate) async fn run(
    world: &World<'_>,
    bank: &Bank,
    org: &Org,
    span: Span,
) -> Result<(), SampleError> {
    let storage = world.storage;
    let database = storage.database();
    database
        .record_changes(&[], async |transaction| {
            for tag in &bank.tags {
                AuthStorage::insert_tag(transaction, tag).await?;
            }
            for file in &bank.files {
                AuthStorage::insert_stored_file(transaction, file).await?;
            }
            Ok(())
        })
        .await?;
    let head = TechnicalName::new(OUTBOX_HEAD)
        .map_err(|error| SampleError::step(Step::Dataset, &error))?;
    let first = database.outbox_position(&head).await?;
    let mut maker = Maker {
        bank,
        org,
        seed: world.seed,
        total: span.issues,
        begin: span.begin,
        end: span.end,
        counters: Counters::new(org.places.len()),
        cumulative: cumulative(org.places.iter().map(|place| place.weight)),
        type_cumulative: {
            let mut sum = 0_u32;
            bank.names
                .types
                .iter()
                .map(|share| {
                    sum += share.percent;
                    sum
                })
                .collect()
        },
    };
    let batch = bank.plan.batch.max(1);
    let mut next = 0_u32;
    while next < span.issues {
        let upto = span.issues.min(next.saturating_add(batch));
        let bundles = (next..upto)
            .map(|index| maker.build(index))
            .collect::<Result<Vec<_>, _>>()?;
        write_batch(storage, &bundles).await?;
        next = upto;
    }
    database
        .record_changes(&[], async |transaction| {
            for (place, last) in org.places.iter().zip(&maker.counters.last) {
                if *last > 0 {
                    transaction.set_issue_counter(place.id, *last).await?;
                }
            }
            Ok(())
        })
        .await?;

    // What a server does by itself: the history of every issue is read from the events, and the
    // notifications of the newest events are made (the older ones were read long ago).
    let ctx = world.services.ctx(world.request());
    world
        .clock
        .set(Timestamp::from_second(span.end).unwrap_or(world.start));
    ctx.catch_up_history()
        .await
        .map_err(|error| SampleError::step(Step::History, &error))?;
    // The index of the text search is built from the stored texts in big steps (the events of a
    // million issues would be read one by one), and the reader starts at the end, so the first
    // round of a server does not read them again.
    ctx.rebuild_text_index()
        .await
        .map_err(|error| SampleError::step(Step::TextIndex, &error))?;
    let last = database.outbox_position(&head).await?;
    let window = (last - first) * i64::from(bank.plan.notified_percent) / 100;
    database
        .outbox_set(
            bb_core::notification::CONSUMER,
            last - window,
            world.clock.now(),
        )
        .await?;
    ctx.catch_up_notifications()
        .await
        .map_err(|error| SampleError::step(Step::Notifications, &error))?;
    Ok(())
}
