//! Migration 17: notifications, handles and the links of sign-ups (NTF-01, NTF-02, M3-F70, M1-F38,
//! ADR-0033).
//!
//! `notification` has one row for each person and event (or comment, for a mention): the unique key
//! `(recipient, once)` is what keeps a reader that reads an event again from making it twice. The
//! row keeps the fields the person could see when it was built, and where its e-mail is. The table
//! has no cascade: the rows of a person who is removed are deleted by the removal (M9, PRIV-04), and
//! all of them go after 90 days anyway. `person_handle` has one row for each handle a person has or
//! had, and a handle names one person for good. `sign_up_invitation` has one row for each link a
//! sign-up on one's own made: an invitation with nobody who invited.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::Step;
use crate::schema::{NotificationRow, PersonHandleRow, PersonRow, SignUpInvitationRow};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = notifications();
    steps.extend(handles());
    steps.extend(sign_up_invitations());
    steps
}

fn notifications() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(NotificationRow::Table)
                .col(
                    ColumnDef::new(NotificationRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(NotificationRow::Recipient).blob().not_null())
                .col(ColumnDef::new(NotificationRow::Kind).text().not_null())
                // No reference to the issue: a notification about an issue that is gone is not
                // shown, and the row is deleted with the others after 90 days.
                .col(ColumnDef::new(NotificationRow::Issue).blob())
                .col(ColumnDef::new(NotificationRow::Event).blob().not_null())
                .col(ColumnDef::new(NotificationRow::Once).text().not_null())
                .col(ColumnDef::new(NotificationRow::Record).blob())
                .col(ColumnDef::new(NotificationRow::By).blob())
                .col(ColumnDef::new(NotificationRow::At).big_integer().not_null())
                .col(ColumnDef::new(NotificationRow::Changes).text().not_null())
                .col(ColumnDef::new(NotificationRow::InApp).boolean().not_null())
                .col(ColumnDef::new(NotificationRow::Mail).text().not_null())
                .col(ColumnDef::new(NotificationRow::ReadAt).big_integer())
                .foreign_key(
                    ForeignKey::create()
                        .from(NotificationRow::Table, NotificationRow::Recipient)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        // A person hears of an event once.
        Step::Index(
            Index::create()
                .name("notification_once")
                .table(NotificationRow::Table)
                .col(NotificationRow::Recipient)
                .col(NotificationRow::Once)
                .unique()
                .to_owned(),
        ),
        // The list of a person, newest first.
        Step::Index(
            Index::create()
                .name("notification_by_recipient")
                .table(NotificationRow::Table)
                .col(NotificationRow::Recipient)
                .col(NotificationRow::Id)
                .to_owned(),
        ),
        // The e-mail that waits for the other changes to an issue.
        Step::Index(
            Index::create()
                .name("notification_mail_waiting")
                .table(NotificationRow::Table)
                .col(NotificationRow::Mail)
                .col(NotificationRow::Recipient)
                .col(NotificationRow::Issue)
                .to_owned(),
        ),
        // The job that deletes old notifications finds them by this.
        Step::Index(
            Index::create()
                .name("notification_by_age")
                .table(NotificationRow::Table)
                .col(NotificationRow::At)
                .to_owned(),
        ),
    ]
}

fn handles() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(PersonHandleRow::Table)
                .col(ColumnDef::new(PersonHandleRow::Person).blob().not_null())
                // A handle names one person, for good: a former handle stays with the person.
                .col(
                    ColumnDef::new(PersonHandleRow::Handle)
                        .text()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(PersonHandleRow::ChangedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(PersonHandleRow::Current).boolean().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(PersonHandleRow::Table, PersonHandleRow::Person)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        // The handles of a person are found from the person.
        Step::Index(
            Index::create()
                .name("person_handle_by_person")
                .table(PersonHandleRow::Table)
                .col(PersonHandleRow::Person)
                .to_owned(),
        ),
    ]
}

fn sign_up_invitations() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(SignUpInvitationRow::Table)
                .col(
                    ColumnDef::new(SignUpInvitationRow::Id)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(SignUpInvitationRow::TokenHash)
                        .blob()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(SignUpInvitationRow::Email).text().not_null())
                .col(
                    ColumnDef::new(SignUpInvitationRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SignUpInvitationRow::ExpiresAt)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(SignUpInvitationRow::UsedAt).big_integer())
                .to_owned(),
        ),
        // The last link made for an address is found by the address.
        Step::Index(
            Index::create()
                .name("sign_up_invitation_by_email")
                .table(SignUpInvitationRow::Table)
                .col(SignUpInvitationRow::Email)
                .col(SignUpInvitationRow::CreatedAt)
                .to_owned(),
        ),
        // The job that clears sessions finds the finished links by this.
        Step::Index(
            Index::create()
                .name("sign_up_invitation_by_expiry")
                .table(SignUpInvitationRow::Table)
                .col(SignUpInvitationRow::ExpiresAt)
                .to_owned(),
        ),
    ]
}
