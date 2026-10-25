//! The members of the events that the use cases write, as `event_members` builds them: the names
//! and the order are what the history and the notifications read, for an event of a person and an
//! event of the large dataset alike (ADR-0007, architecture §11).

use bb_core::event::FieldChange;
use bb_core::files::{Attachment, MediaType, PreviewStatus, StoredFile};
use bb_core::id::{AttachmentId, CommentId, IssueId, IssueTypeId, LabelId, ProjectId, UserId};
use bb_core::issue::{Issue, Priority};
use bb_core::own_fields::ChoiceColor;
use bb_core::readable_id::IssueCode;
use bb_core::security_log::Digest;
use bb_core::tags::Label;
use bb_core::workflow::StatusKey;
use jiff::Timestamp;
use serde_json::{Value, json};

use crate::context::SystemIds;
use crate::event_members as members;

/// The names of the members, in order.
fn names(changes: &[FieldChange]) -> Vec<&str> {
    changes.iter().map(|change| change.field.as_str()).collect()
}

/// The value a member was set to.
fn after<'a>(changes: &'a [FieldChange], name: &str) -> &'a Value {
    changes
        .iter()
        .find(|change| change.field == name)
        .and_then(|change| change.after.as_ref())
        .unwrap_or_else(|| panic!("no member {name}"))
}

fn key(text: &str) -> StatusKey {
    StatusKey::new(text).unwrap()
}

fn issue(ids: &mut SystemIds) -> Issue {
    Issue {
        id: IssueId::generate(ids).unwrap(),
        project: ProjectId::generate(ids).unwrap(),
        issue_type: IssueTypeId::generate(ids).unwrap(),
        code: IssueCode::new("7").unwrap(),
        title: "A title".to_owned(),
        description: "A text".to_owned(),
        status: key("open"),
        priority: Priority::High,
        reported_by: UserId::generate(ids),
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        closed_at: None,
        deleted_at: None,
        version: 1,
    }
}

#[test]
fn iss_01_a_new_issue_gives_its_members_in_the_order_of_the_form() {
    let mut ids = SystemIds;
    let issue = issue(&mut ids);
    let helper = UserId::generate(&mut ids);
    let own = vec![members::own_value_set(
        bb_core::id::FieldId::generate(&mut ids).unwrap(),
        bb_core::own_fields::FieldType::Text,
        &bb_core::own_fields_value::FieldValue::Text("x".to_owned()),
    )];
    let changes = members::issue_created(&issue, &[helper], own);
    let own_name = changes[8].field.clone();
    assert_eq!(
        names(&changes),
        [
            "project",
            "issue-type",
            "title",
            "description",
            "status",
            "priority",
            "reported-by",
            "assigned-to",
            own_name.as_str(),
        ]
    );
    assert_eq!(after(&changes, "title"), &json!("A title"));
    assert_eq!(after(&changes, "status"), &json!("open"));
    assert_eq!(after(&changes, "assigned-to"), &json!([helper.to_string()]));
    assert!(changes.iter().all(|change| change.before.is_none()));
}

#[test]
fn wf_01_a_status_change_gives_the_status_then_the_counted_up_version() {
    let changes = members::status_changed(&key("open"), &key("done"), 4);
    assert_eq!(names(&changes), ["status", "version"]);
    assert_eq!(changes[0].before, Some(json!("open")));
    assert_eq!(changes[0].after, Some(json!("done")));
    assert_eq!(changes[1].before, Some(json!(4)));
    assert_eq!(changes[1].after, Some(json!(5)));
}

#[test]
fn tag_01_a_tag_put_on_gives_the_names_then_the_counted_up_version() {
    let tags = members::tag_put_on(&[], &["red".to_owned()], 2);
    assert_eq!(names(&tags), ["tags", "version"]);
    assert_eq!(after(&tags, "tags"), &json!(["red"]));
    assert_eq!(tags[1].after, Some(json!(3)));
}

#[test]
fn tag_02_a_label_put_on_gives_the_labels_then_the_counted_up_version() {
    let label = Label {
        id: LabelId::generate(&mut SystemIds).unwrap(),
        group: None,
        name: "urgent".to_owned(),
        color: ChoiceColor::Cat5,
        position: 0,
        retired_at: None,
        version: 1,
    };
    let labels = members::label_put_on(&[], std::slice::from_ref(&label), 5);
    assert_eq!(names(&labels), ["labels", "version"]);
    assert_eq!(
        after(&labels, "labels"),
        &json!([{"group": null, "name": "urgent"}])
    );
    assert_eq!(labels[1].before, Some(json!(5)));
    assert_eq!(labels[1].after, Some(json!(6)));
}

#[test]
fn iss_03_a_sub_issue_put_under_a_parent_gives_the_parent_then_the_version() {
    let mut ids = SystemIds;
    let parent = IssueId::generate(&mut ids).unwrap();
    let changes = members::parent_changed(None, Some(parent), 1);
    assert_eq!(names(&changes), ["parent", "version"]);
    assert_eq!(changes[0].before, None);
    assert_eq!(changes[0].after, Some(json!(parent.to_string())));
    assert_eq!(changes[1].after, Some(json!(2)));
}

#[test]
fn att_01_a_file_on_a_comment_gives_the_comment_member_last() {
    let mut ids = SystemIds;
    let issue = IssueId::generate(&mut ids).unwrap();
    let comment = CommentId::generate(&mut ids).unwrap();
    let file = StoredFile {
        hash: Digest::new([7; 32]),
        size: 10,
        media_type: MediaType::new("image/png").unwrap(),
        preview: PreviewStatus::Pending,
        created_at: Timestamp::UNIX_EPOCH,
    };
    let attachment = |comment| Attachment {
        id: AttachmentId::generate(&mut SystemIds).unwrap(),
        issue,
        comment,
        file: file.hash,
        name: "a.png".to_owned(),
        added_by: UserId::generate(&mut SystemIds),
        added_at: Timestamp::UNIX_EPOCH,
        deleted_at: None,
        preview: PreviewStatus::Pending,
        version: 1,
    };
    let on_issue = members::attachment_added(&attachment(None), &file);
    assert_eq!(names(&on_issue), ["issue", "name", "file", "media_type"]);
    let on_comment = members::attachment_added(&attachment(Some(comment)), &file);
    assert_eq!(
        names(&on_comment),
        ["issue", "name", "file", "media_type", "comment"]
    );
    assert_eq!(after(&on_comment, "comment"), &json!(comment.to_string()));
}
