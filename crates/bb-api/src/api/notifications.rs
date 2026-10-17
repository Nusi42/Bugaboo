//! Notifications and the mail server (NTF-01, NTF-02, SEC-06, SEC-16, ADR-0033): the list of what
//! the person is told about below `/api/v1/notifications`, and the password and the test e-mail of
//! the mail server below `/api/v1/mail`.
//!
//! A notification is shaped for the person at the moment they ask: a field they may not see is not
//! in it, the people in it are shown as *Who sees people* says, and a notification about an issue
//! they cannot see any more is not listed at all (ADR-0033 §2). A row is flat (ARCH-LEAN): the
//! changes of one notification are items of their own that share its `id`, so a page may hold a few
//! more items than `limit`.

use axum::extract::State;
use axum::response::Response;
use bb_app::notifications::{
    MailServerQuery, NotificationItem, NotificationWhat, NotificationsQuery, ReadNotifications,
    SendTestEmail, SetMailPassword, UnreadQuery, Which,
};
use bb_core::edit::NoticeKind;
use bb_core::id::NotificationId;
use bb_core::mail::MailServerReadiness;
use bb_core::notification::{NotificationFilter, NotificationKind};
use bb_core::own_fields::FieldType;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{
    AuthHandle, Info, InvalidAnswer, NotSignedInAnswer, Session, TooSoonAnswer, answer,
    answer_authorized, done,
};
use super::body_id;
use super::change_side::sides_of;
use super::projects::ProjectRef;
use super::relations::IssueRef;
use crate::errors::{ApiError, ApiJson, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};
use bb_app::history::FieldChangeView;

const NOTIFICATIONS_LIST: &str = "notifications";

/// The fields of an item of the list that can be asked for. An item holds only the ones that
/// belong to what it says.
const NOTIFICATION_FIELDS: [FieldSpec; 28] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("kind"),
    FieldSpec::by_default("at"),
    FieldSpec::by_default("read"),
    FieldSpec::by_default("issue"),
    FieldSpec::by_default("by"),
    FieldSpec::by_default("comment"),
    FieldSpec::by_default("text"),
    FieldSpec::by_default("html"),
    FieldSpec::by_default("notice"),
    FieldSpec::by_default("part"),
    FieldSpec::by_default("run"),
    FieldSpec::by_default("run_done"),
    FieldSpec::by_default("field"),
    FieldSpec::by_default("field_name"),
    FieldSpec::by_default("field_kind"),
    FieldSpec::by_default("before_text"),
    FieldSpec::by_default("after_text"),
    FieldSpec::by_default("before_value"),
    FieldSpec::by_default("after_value"),
    FieldSpec::by_default("before_names"),
    FieldSpec::by_default("after_names"),
    FieldSpec::by_default("before_people"),
    FieldSpec::by_default("after_people"),
    FieldSpec::by_default("before_issue"),
    FieldSpec::by_default("after_issue"),
    FieldSpec::by_default("before_project"),
    FieldSpec::by_default("after_project"),
];

/// One item of the list of notifications.
#[derive(Serialize, ToSchema)]
pub(crate) struct NotificationEntry {
    /// The ID of the notification; the items of one notification share it. *Mark as read* names it.
    id: String,
    /// What the notification is about.
    kind: NotificationKind,
    /// When it happened.
    at: String,
    /// Whether the person has read it.
    read: bool,
    /// The issue it is about, when the person may see it.
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<IssueRef>,
    /// Who did it, as the person may see people of the kind *Authors*; left out for Bugaboo itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    by: Option<PersonRef>,
    /// A comment: its ID, which *Open* goes to.
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    /// A comment: the text, in Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    /// A comment: the text as safe HTML, made on the server for this person.
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    /// A note about editing: somebody asked the person to finish, or took the editing over.
    #[serde(skip_serializing_if = "Option::is_none")]
    notice: Option<NoticeKind>,
    /// A note about editing: the part of the record that was being edited.
    #[serde(skip_serializing_if = "Option::is_none")]
    part: Option<String>,
    /// The end of a run: the ID of the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    run: Option<String>,
    /// The end of a run: whether it ended well.
    #[serde(skip_serializing_if = "Option::is_none")]
    run_done: Option<bool>,
    /// A change: the key of the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    /// A change of an own field: its name.
    #[serde(skip_serializing_if = "Option::is_none")]
    field_name: Option<String>,
    /// A change of an own field: its kind, which says how `before_value` and `after_value` are
    /// worded.
    #[serde(skip_serializing_if = "Option::is_none")]
    field_kind: Option<FieldType>,
    /// A change: what the field was, in words.
    #[serde(skip_serializing_if = "Option::is_none")]
    before_text: Option<String>,
    /// A change: what the field is now, in words.
    #[serde(skip_serializing_if = "Option::is_none")]
    after_text: Option<String>,
    /// A change of an own field that a screen words itself (yes or no, a length of time, a
    /// number, a day): what the field was, as the JSON of its kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    before_value: Option<Value>,
    /// The same for what the field is now.
    #[serde(skip_serializing_if = "Option::is_none")]
    after_value: Option<Value>,
    /// A change of a field that is a list of names (labels, tags): what it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    before_names: Option<Vec<String>>,
    /// The same for what the field is now.
    #[serde(skip_serializing_if = "Option::is_none")]
    after_names: Option<Vec<String>>,
    /// A change of a field that holds people: who they were, each as the reader may see people.
    #[serde(skip_serializing_if = "Option::is_none")]
    before_people: Option<Vec<PersonRef>>,
    /// The same for who they are now.
    #[serde(skip_serializing_if = "Option::is_none")]
    after_people: Option<Vec<PersonRef>>,
    /// A change of a field that is an issue: the issue it was, when the reader may see it.
    #[serde(skip_serializing_if = "Option::is_none")]
    before_issue: Option<IssueRef>,
    /// The same for what it is now.
    #[serde(skip_serializing_if = "Option::is_none")]
    after_issue: Option<IssueRef>,
    /// A change of the project: the project the issue was in, when the reader may see it.
    #[serde(skip_serializing_if = "Option::is_none")]
    before_project: Option<ProjectRef>,
    /// The same for the project it is in now.
    #[serde(skip_serializing_if = "Option::is_none")]
    after_project: Option<ProjectRef>,
}

impl Masked for NotificationEntry {}

/// The start of every item of a notification.
fn start(item: &NotificationItem) -> NotificationEntry {
    NotificationEntry {
        id: item.id.to_string(),
        kind: item.kind,
        at: item.at.to_string(),
        read: item.read,
        issue: item.issue.as_ref().map(IssueRef::from),
        by: item.by.clone(),
        comment: item.comment.map(|comment| comment.to_string()),
        text: None,
        html: None,
        notice: None,
        part: None,
        run: None,
        run_done: None,
        field: None,
        field_name: None,
        field_kind: None,
        before_text: None,
        after_text: None,
        before_value: None,
        after_value: None,
        before_names: None,
        after_names: None,
        before_people: None,
        after_people: None,
        before_issue: None,
        after_issue: None,
        before_project: None,
        after_project: None,
    }
}

/// The item of one changed field.
fn change_entry(item: &NotificationItem, change: &FieldChangeView) -> NotificationEntry {
    let mut entry = start(item);
    entry.field = Some(change.field.clone());
    entry.field_name.clone_from(&change.name);
    let (before, after) = sides_of(change);
    entry.field_kind = before.kind.or(after.kind);
    (entry.before_text, entry.before_value) = (before.text, before.value);
    (entry.before_names, entry.before_people) = (before.names, before.people);
    entry.before_issue = before.issue;
    entry.before_project = before.project;
    (entry.after_text, entry.after_value) = (after.text, after.value);
    (entry.after_names, entry.after_people) = (after.names, after.people);
    entry.after_issue = after.issue;
    entry.after_project = after.project;
    entry
}

/// The items of one notification: a change has one for each field, the rest have one.
fn entries_of(item: &NotificationItem) -> Vec<NotificationEntry> {
    match &item.what {
        NotificationWhat::Changes { fields } => fields
            .iter()
            .map(|change| change_entry(item, change))
            .collect(),
        NotificationWhat::Comment { comment } => {
            let mut entry = start(item);
            entry.text = Some(comment.text.clone());
            entry.html = Some(comment.html.clone());
            vec![entry]
        }
        NotificationWhat::EditNotice { notice, part } => {
            let mut entry = start(item);
            entry.notice = Some(*notice);
            entry.part = Some(part.clone());
            vec![entry]
        }
        NotificationWhat::RunEnded { run, done } => {
            let mut entry = start(item);
            entry.run = Some(run.to_string());
            entry.run_done = Some(*done);
            vec![entry]
        }
        NotificationWhat::Plain => vec![start(item)],
    }
}

/// Which notifications to list.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct FilterParams {
    /// All of them (the default), the unread ones, or the ones that mention the person.
    #[param(inline)]
    filter: Option<NotificationFilter>,
}

/// How many notifications the person has not read.
#[derive(Serialize, ToSchema)]
pub(crate) struct UnreadBody {
    /// The number, at most 100: only what the person may still see is counted.
    unread: usize,
}

impl Masked for UnreadBody {}

/// The notifications to mark as read.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ReadNotificationsBody {
    /// The IDs of the notifications, at most 100; or leave it out and set `up_to`.
    #[serde(default)]
    ids: Vec<String>,
    /// Mark every unread notification as read up to and including this one, the newest the person
    /// has seen: ones that came after it stay unread.
    #[serde(default)]
    up_to: Option<String>,
}

/// The password of the mail server.
#[derive(Deserialize, ToSchema)]
pub(crate) struct MailPasswordBody {
    /// The password. Leave it out, or leave it empty, to remove it (a mail server that asks for
    /// none). It is stored encrypted and never sent back.
    #[serde(default)]
    password: Option<String>,
}

/// What the mail server has that is no setting.
#[derive(Serialize, ToSchema)]
pub(crate) struct MailServerStatusBody {
    /// A password is stored. The password itself is never sent.
    credential_set: bool,
    /// What the server judges of the mail server as entered: `ready`, `incomplete` (the host or
    /// the sender is missing) or `insecure` (a secret would go without encryption to another
    /// machine).
    readiness: MailServerReadiness,
}

impl Masked for MailServerStatusBody {}

/// The notifications of the person, newest first, a page at a time. A notification about an issue
/// the person cannot see any more is not in the list.
#[utoipa::path(
    get,
    path = "/api/v1/notifications",
    operation_id = "listNotifications",
    tag = "notifications",
    params(ListParams, FilterParams),
    responses((status = 200, description = "The notifications.", body = Page<NotificationEntry>), InvalidAnswer, NotSignedInAnswer)
)]
async fn list_notifications(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
    ApiQuery(which): ApiQuery<FilterParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(NOTIFICATIONS_LIST);
    let page = auth
        .api()?
        .notifications(
            info,
            NotificationsQuery {
                session_token,
                filter: which.filter.unwrap_or(NotificationFilter::All),
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = page.map(|page| Listed {
        items: page.items.iter().flat_map(entries_of).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&NOTIFICATION_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(
        NOTIFICATIONS_LIST,
        &list,
        &fields,
    )?))
}

/// How many notifications the person has not read: the number on the menu entry.
#[utoipa::path(
    get,
    path = "/api/v1/notifications/unread",
    operation_id = "getUnreadNotifications",
    tag = "notifications",
    responses((status = 200, description = "The number.", body = UnreadBody), NotSignedInAnswer)
)]
async fn get_unread_notifications(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let count = auth
        .api()?
        .unread_notifications(info, UnreadQuery { session_token })
        .await?;
    answer_authorized(&count.map(|count| UnreadBody {
        unread: count.unread,
    }))
}

/// Marks notifications of the person as read: the ones named, or all of them.
#[utoipa::path(
    post,
    path = "/api/v1/notifications/read",
    operation_id = "readNotifications",
    tag = "notifications",
    request_body = ReadNotificationsBody,
    responses((status = 204, description = "They are marked."), InvalidAnswer, NotSignedInAnswer)
)]
async fn read_notifications(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<ReadNotificationsBody>,
) -> Result<Response, ApiError> {
    let which = if let Some(up_to) = &body.up_to {
        Which::UpTo(body_id::<NotificationId>(up_to)?)
    } else {
        Which::These(
            body.ids
                .iter()
                .map(|text| body_id::<NotificationId>(text))
                .collect::<Result<Vec<_>, _>>()?,
        )
    };
    auth.api()?
        .read_notifications(
            info,
            ReadNotifications {
                session_token,
                which,
            },
        )
        .await?;
    Ok(done())
}

/// Enters, changes or removes the password of the mail server. It is stored encrypted like other
/// secrets and never sent back.
#[utoipa::path(
    put,
    path = "/api/v1/mail/password",
    operation_id = "setMailPassword",
    tag = "notifications",
    request_body = MailPasswordBody,
    responses((status = 204, description = "The password is stored."), InvalidAnswer, NotSignedInAnswer)
)]
async fn set_mail_password(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<MailPasswordBody>,
) -> Result<Response, ApiError> {
    auth.api()?
        .set_mail_password(
            info,
            SetMailPassword {
                session_token,
                password: body.password,
            },
        )
        .await?;
    Ok(done())
}

/// What the page of the mail server needs to know that the settings do not hold: whether a
/// password is entered, never what it is.
#[utoipa::path(
    get,
    path = "/api/v1/mail",
    operation_id = "getMailServer",
    tag = "notifications",
    responses((status = 200, description = "Whether it is entered.", body = MailServerStatusBody), NotSignedInAnswer)
)]
async fn get_mail_password(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let state = auth
        .api()?
        .mail_server_status(info, MailServerQuery { session_token })
        .await?;
    answer_authorized(&state.map(|state| MailServerStatusBody {
        credential_set: state.credential_set,
        readiness: state.readiness,
    }))
}

/// Sends a test e-mail to the person who asks, through the mail server as it is entered, even
/// while e-mail is still off.
#[utoipa::path(
    post,
    path = "/api/v1/mail/test",
    operation_id = "sendTestEmail",
    tag = "notifications",
    responses((status = 204, description = "The mail server took the e-mail."), NotSignedInAnswer, TooSoonAnswer)
)]
async fn send_test_email(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    auth.api()?
        .send_test_email(info, SendTestEmail { session_token })
        .await?;
    Ok(done())
}

/// The routes of notifications and of the mail server.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_notifications))
        .routes(routes!(get_unread_notifications))
        .routes(routes!(read_notifications))
        .routes(routes!(set_mail_password, get_mail_password))
        .routes(routes!(send_test_email))
}

#[cfg(test)]
mod tests {
    use bb_core::issue::IssueIcon;

    use super::*;
    use crate::api::issues::IssueTypeRef;
    use crate::api::projects::ProjectRef;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_an_item() {
        let issue = || IssueRef {
            id: String::new(),
            key: String::new(),
            project: ProjectRef {
                id: String::new(),
                short_name: String::new(),
                name: String::new(),
            },
            issue_type: IssueTypeRef {
                id: String::new(),
                name: String::new(),
                icon: IssueIcon::Bug,
                retired: false,
            },
            title: None,
            status: Some(crate::api::workflows::StatusRef::blank()),
            priority: Some(bb_core::issue::Priority::Normal),
        };
        let project = || ProjectRef {
            id: String::new(),
            short_name: String::new(),
            name: String::new(),
        };
        let item = NotificationEntry {
            id: String::new(),
            kind: NotificationKind::Mentioned,
            at: String::new(),
            read: false,
            issue: Some(issue()),
            by: Some(PersonRef::Someone),
            comment: Some(String::new()),
            text: Some(String::new()),
            html: Some(String::new()),
            notice: Some(NoticeKind::FinishAsked),
            part: Some(String::new()),
            run: Some(String::new()),
            run_done: Some(true),
            field: Some(String::new()),
            field_name: Some(String::new()),
            field_kind: Some(FieldType::YesNo),
            before_text: Some(String::new()),
            after_text: Some(String::new()),
            before_value: Some(Value::Null),
            after_value: Some(Value::Null),
            before_names: Some(vec![]),
            after_names: Some(vec![]),
            before_people: Some(vec![]),
            after_people: Some(vec![]),
            before_issue: Some(issue()),
            after_issue: Some(issue()),
            before_project: Some(project()),
            after_project: Some(project()),
        };
        crate::lean::assert_fields_are_the_keys_of(&NOTIFICATION_FIELDS, &item);
    }

    #[test]
    fn ntf_01_the_documents_name_the_cap_the_core_sets() {
        // The text of the API description is written by hand; the number in it is the constant.
        let document = serde_json::to_string(&crate::openapi::document()).unwrap_or_default();
        let said = format!("at most {}", bb_core::notification::AT_ONCE_MAX);
        assert!(document.matches(&said).count() >= 2, "{said}");
    }
}
