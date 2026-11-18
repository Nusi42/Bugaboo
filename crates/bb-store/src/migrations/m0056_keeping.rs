//! Migration 56: the indexes the purges of *Keep and delete rules* need to find old rows by their
//! moment (PRIV-05, ADR-0045, decision 5). Each step of a purge chooses at most a step of rows by a
//! moment; without an index on it every step would read the whole table. The tables that had one
//! already (notifications, tool calls, drafts, notes, locks, away times, jobs) keep theirs. Every
//! table of the trash has its moment of deletion already (`deleted_at`), so no column is added.

use sea_query::Index;

use super::Step;
use crate::schema::{
    AttachmentRow, IssueRow, OAuthCodeRow, OAuthTokenRow, SignInSession, SignUpMailRow,
    TimeEntryRow,
};

pub(super) fn steps() -> Vec<Step> {
    vec![
        // The IP addresses and the longest life of a session (`sessions.purge`).
        Step::Index(
            Index::create()
                .name("session_by_start")
                .table(SignInSession::Table)
                .col(SignInSession::CreatedAt)
                .to_owned(),
        ),
        // The memory of sign-up e-mails (`sessions.purge`).
        Step::Index(
            Index::create()
                .name("sign_up_mail_by_time")
                .table(SignUpMailRow::Table)
                .col(SignUpMailRow::SentAt)
                .to_owned(),
        ),
        // The issues and the attachments of the trash (`files.clean-up`).
        Step::Index(
            Index::create()
                .name("issue_by_deletion")
                .table(IssueRow::Table)
                .col(IssueRow::DeletedAt)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("attachment_by_deletion")
                .table(AttachmentRow::Table)
                .col(AttachmentRow::DeletedAt)
                .to_owned(),
        ),
        // The codes and tokens of AI apps that ran out (`oauth.clean-old-grants`).
        Step::Index(
            Index::create()
                .name("oauth_code_by_end")
                .table(OAuthCodeRow::Table)
                .col(OAuthCodeRow::ExpiresAt)
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("oauth_token_by_end")
                .table(OAuthTokenRow::Table)
                .col(OAuthTokenRow::ExpiresAt)
                .to_owned(),
        ),
        // The time entries past their keep period, counted for everybody by an administrator.
        Step::Index(
            Index::create()
                .name("time_entry_by_day")
                .table(TimeEntryRow::Table)
                .col(TimeEntryRow::Day)
                .to_owned(),
        ),
    ]
}
