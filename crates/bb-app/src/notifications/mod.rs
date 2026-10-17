//! Notifications (NTF-01, NTF-02, SEC-06, ADR-0033): what people are told about, in the app and by
//! e-mail.
//!
//! - `plan.rs`: what an event means for notifications, as far as that needs no store (pure).
//! - `build.rs`: the outbox reader `notifications`; idempotent, resumes from the place it recorded;
//!   checks the recipient's mask and *Who sees people* when it builds a row.
//! - `read.rs`: the list and *Mark as read*; checks again when a row is shown.
//! - `mail.rs`: the job `mail.send` and the e-mail a notification makes; checks again when it is
//!   sent.

mod build;
mod compose;
mod handles;
mod keep;
mod mail;
mod mail_settings;
mod plan;
mod read;
mod store;
mod types;

pub(crate) use handles::give_handle;
pub use handles::{
    ChangeHandle, CreateMissingHandles, HandleStore, MyHandle, MyHandleQuery, change_handle,
    create_missing_handles, my_handle,
};
pub(crate) use mail::{MailJob, send_failure_for_job};
pub use mail_settings::{SendTestEmail, SetMailPassword, send_test_email, set_mail_password};
pub use read::{notifications, read_notifications, unread_notifications};
pub use store::NotificationStore;
pub use types::*;
