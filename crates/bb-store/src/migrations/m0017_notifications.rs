//! Migration 17: notifications, handles and the confirmation of addresses (NTF-01, NTF-02,
//! M3-F70, M1-F38, ADR-0033).
//!
//! `notification` has one row for each person and event (or comment, for a mention): the unique key
//! `(recipient, once)` is what keeps a reader that reads an event again from making it twice. The
//! row keeps the fields the person could see when it was built, and where its e-mail is. A person
//! who is removed leaves no row (PRIV-04): the rows go with them. `person_handle` has one row
//! for each person, and the handle is unique. `address_confirmation` has a row only for an account
//! that waits for its address to be confirmed.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::Step;
use crate::schema::{AddressConfirmationRow, NotificationRow, PersonHandleRow, PersonRow};

pub(super) fn steps() -> Vec<Step> {
    let mut steps = notifications();
    steps.extend(handles());
    steps.extend(confirmations());
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
                .col(
                    ColumnDef::new(PersonHandleRow::Person)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(PersonHandleRow::Handle).text().not_null())
                .col(
                    ColumnDef::new(PersonHandleRow::ChangedAt)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(PersonHandleRow::Table, PersonHandleRow::Person)
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        // A handle names one person: two people who ask for it at the same moment cannot both
        // get it.
        Step::Index(
            Index::create()
                .name("person_handle_by_handle")
                .table(PersonHandleRow::Table)
                .col(PersonHandleRow::Handle)
                .unique()
                .to_owned(),
        ),
    ]
}

fn confirmations() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(AddressConfirmationRow::Table)
                .col(
                    ColumnDef::new(AddressConfirmationRow::Person)
                        .blob()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(AddressConfirmationRow::TokenHash)
                        .blob()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(AddressConfirmationRow::CreatedAt)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(AddressConfirmationRow::ExpiresAt)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(
                            AddressConfirmationRow::Table,
                            AddressConfirmationRow::Person,
                        )
                        .to(PersonRow::Table, PersonRow::Id),
                )
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("address_confirmation_by_token")
                .table(AddressConfirmationRow::Table)
                .col(AddressConfirmationRow::TokenHash)
                .unique()
                .to_owned(),
        ),
        Step::Index(
            Index::create()
                .name("address_confirmation_by_expiry")
                .table(AddressConfirmationRow::Table)
                .col(AddressConfirmationRow::ExpiresAt)
                .to_owned(),
        ),
    ]
}
