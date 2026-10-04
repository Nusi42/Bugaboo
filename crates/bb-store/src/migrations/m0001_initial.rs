//! Migration 1: event log and outbox, job queue, security log, secrets and settings.

use sea_query::{ColumnDef, Expr, ForeignKey, Index, InsertStatement, Query, Table};

use super::Step;
use crate::events::OUTBOX_HEAD;
use crate::schema::{
    Event, Job, Outbox, OutboxPosition, Secret, SecurityLog, SecurityLogCheckpoint, SettingRow,
};

pub(super) fn steps() -> Result<Vec<Step>, sea_query::error::Error> {
    let mut steps = Vec::new();
    steps.extend(event_log_and_outbox());
    steps.extend(job_queue());
    steps.extend(security_log());
    steps.extend(secrets_and_settings());
    steps.push(Step::Seed(outbox_head()?));
    Ok(steps)
}

fn event_log_and_outbox() -> Vec<Step> {
    vec![
        // The event log (ADR-0007). Rows are only ever added.
        Step::Table(
            Table::create()
                .table(Event::Table)
                .col(ColumnDef::new(Event::Id).blob().not_null().primary_key())
                .col(ColumnDef::new(Event::Time).big_integer().not_null())
                .col(ColumnDef::new(Event::ActorKind).text().not_null())
                .col(ColumnDef::new(Event::ActorId).blob())
                .col(ColumnDef::new(Event::Channel).text().not_null())
                .col(ColumnDef::new(Event::EntityKind).text().not_null())
                .col(ColumnDef::new(Event::EntityId).blob().not_null())
                .col(ColumnDef::new(Event::Changes).text().not_null())
                .col(ColumnDef::new(Event::Reason).text())
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("event_by_entity")
                .table(Event::Table)
                .col(Event::EntityKind)
                .col(Event::EntityId)
                .col(Event::Id)
                .to_owned(),
        ),
        // The outbox: one entry per event, in the order the changes were committed.
        Step::Table(
            Table::create()
                .table(Outbox::Table)
                .col(
                    ColumnDef::new(Outbox::Position)
                        .big_integer()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(Outbox::EventId)
                        .blob()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(Outbox::CreatedAt).big_integer().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(Outbox::Table, Outbox::EventId)
                        .to(Event::Table, Event::Id),
                )
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(OutboxPosition::Table)
                .col(
                    ColumnDef::new(OutboxPosition::Consumer)
                        .text()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(OutboxPosition::Position)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(OutboxPosition::UpdatedAt)
                        .big_integer()
                        .not_null(),
                )
                .to_owned(),
        ),
    ]
}

fn job_queue() -> Vec<Step> {
    vec![
        // The job queue (ADR-0017).
        Step::Table(
            Table::create()
                .table(Job::Table)
                .col(ColumnDef::new(Job::Id).blob().not_null().primary_key())
                .col(ColumnDef::new(Job::Kind).text().not_null())
                .col(ColumnDef::new(Job::Payload).text().not_null())
                .col(ColumnDef::new(Job::RunAt).big_integer().not_null())
                .col(ColumnDef::new(Job::Attempts).big_integer().not_null())
                .col(ColumnDef::new(Job::MaxAttempts).big_integer().not_null())
                .col(ColumnDef::new(Job::State).text().not_null())
                .col(ColumnDef::new(Job::LeaseOwner).text())
                .col(ColumnDef::new(Job::LeaseUntil).big_integer())
                .col(ColumnDef::new(Job::LastFailure).text())
                .col(ColumnDef::new(Job::CreatedAt).big_integer().not_null())
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("job_by_state_and_time")
                .table(Job::Table)
                .col(Job::State)
                .col(Job::RunAt)
                .to_owned(),
        ),
    ]
}

fn security_log() -> Vec<Step> {
    vec![
        // The security log (SEC-15) and the checkpoint left when old blocks are deleted.
        Step::Table(
            Table::create()
                .table(SecurityLog::Table)
                .col(
                    ColumnDef::new(SecurityLog::Seq)
                        .big_integer()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(SecurityLog::Time).big_integer().not_null())
                .col(ColumnDef::new(SecurityLog::Person).blob())
                .col(ColumnDef::new(SecurityLog::Action).text().not_null())
                .col(ColumnDef::new(SecurityLog::SubjectKind).text())
                .col(ColumnDef::new(SecurityLog::SubjectId).blob())
                .col(ColumnDef::new(SecurityLog::Network).text())
                .col(ColumnDef::new(SecurityLog::PreviousHash).blob().not_null())
                .col(ColumnDef::new(SecurityLog::Hash).blob().not_null())
                .to_owned(),
        ),
        Step::Table(
            Table::create()
                .table(SecurityLogCheckpoint::Table)
                .col(
                    ColumnDef::new(SecurityLogCheckpoint::Seq)
                        .big_integer()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(SecurityLogCheckpoint::Hash)
                        .blob()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SecurityLogCheckpoint::Signature)
                        .blob()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SecurityLogCheckpoint::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .to_owned(),
        ),
    ]
}

fn secrets_and_settings() -> Vec<Step> {
    vec![
        // Secrets (SEC-16), envelope encrypted.
        Step::Table(
            Table::create()
                .table(Secret::Table)
                .col(ColumnDef::new(Secret::Name).text().not_null().primary_key())
                .col(ColumnDef::new(Secret::KeyId).text().not_null())
                .col(ColumnDef::new(Secret::WrappedKey).blob().not_null())
                .col(ColumnDef::new(Secret::WrappedNonce).blob().not_null())
                .col(ColumnDef::new(Secret::Nonce).blob().not_null())
                .col(ColumnDef::new(Secret::Ciphertext).blob().not_null())
                .col(ColumnDef::new(Secret::UpdatedAt).big_integer().not_null())
                .to_owned(),
        ),
        // Setting values per level (ADR-0018). The scope is empty on the organization level.
        Step::Table(
            Table::create()
                .table(SettingRow::Table)
                .col(ColumnDef::new(SettingRow::Key).text().not_null())
                .col(ColumnDef::new(SettingRow::Level).text().not_null())
                .col(ColumnDef::new(SettingRow::Scope).blob().not_null())
                .col(ColumnDef::new(SettingRow::Value).text().not_null())
                .col(ColumnDef::new(SettingRow::Locked).boolean().not_null())
                .col(
                    ColumnDef::new(SettingRow::UpdatedAt)
                        .big_integer()
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .col(SettingRow::Key)
                        .col(SettingRow::Level)
                        .col(SettingRow::Scope),
                )
                .to_owned(),
        ),
    ]
}

/// The head of the outbox: the last position handed out (see `events`).
fn outbox_head() -> Result<InsertStatement, sea_query::error::Error> {
    let mut head = Query::insert();
    head.into_table(OutboxPosition::Table)
        .columns([
            OutboxPosition::Consumer,
            OutboxPosition::Position,
            OutboxPosition::UpdatedAt,
        ])
        .values([Expr::val(OUTBOX_HEAD), Expr::val(0_i64), Expr::val(0_i64)])?;
    Ok(head)
}
