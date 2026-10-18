//! Migration 20: `mention` loses `read_at` (M4-F83).
//!
//! Nothing sets or reads it since the notifications replaced the *Mentions* block (M4 WP-08); a
//! notification keeps its own `read_at`. The column is taken away the way migration 3 took away
//! `person.administrator`: one `ALTER TABLE … DROP COLUMN`, which both SQLite (3.35 and later) and
//! PostgreSQL run in place. No index or key of `mention` names the column, so SQLite accepts it.

use sea_query::Table;

use super::Step;
use crate::schema::{MentionRow, Retired};

pub(super) fn steps() -> Vec<Step> {
    vec![Step::Alter(
        Table::alter()
            .table(MentionRow::Table)
            .drop_column(Retired::ReadAt)
            .to_owned(),
    )]
}
