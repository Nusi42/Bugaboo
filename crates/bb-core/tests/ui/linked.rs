use bb_core::event::{EntityRef, FieldChange, RecordKind};
use bb_core::id::{EntityId, FieldId, SavedSearchId, UserId};
use bb_core::own_fields::FieldType;
use bb_core::personal::{Entity, Link, LinkScope, PersonLink};

// An entity that names a person needs the message `personal-kind.<its name>` and, for each field,
// `personal-field.<its name>.<the field>`, so the examples borrow the names and fields of records
// that have them, one record for each kind of link.
#[derive(Entity)]
pub struct Comment {
    #[personal(category = "content", basis = "contract")]
    pub text: String,
    #[personal(category = "identity", basis = "legitimate_interests")]
    #[person(author)]
    pub author: UserId,
    #[not_personal]
    #[person_for(record, link = "subject")]
    pub issue: EntityRef,
}

#[derive(Entity)]
pub struct Notification {
    #[personal(category = "identity", basis = "legitimate_interests")]
    #[person(recipient)]
    pub recipient: Vec<Option<UserId>>,
    #[not_personal]
    #[names_no_person]
    pub record: EntityId,
    #[personal(category = "content", basis = "legitimate_interests")]
    #[person_in_changes]
    pub changes: Vec<FieldChange>,
}

#[derive(Entity)]
pub struct StoredFieldValue {
    #[not_personal]
    pub field: FieldId,
    #[personal(category = "activity", basis = "legitimate_interests")]
    #[person_for(field_type = "person", key = "field", link = "subject")]
    #[person_for(field_type = "people", key = "field", link = "subject")]
    pub value_ref: Option<EntityId>,
}

#[derive(Entity)]
pub struct EmailChange {
    #[personal(category = "identity", basis = "contract")]
    #[person_email(recipient)]
    pub new_email: String,
}

#[derive(Entity)]
pub struct SavedSearch {
    #[personal(category = "content", basis = "legitimate_interests")]
    #[person_in_json(subject)]
    pub timeline: serde_json::Value,
}

#[derive(Entity)]
pub struct SearchColumn {
    #[not_personal]
    #[person_for(part_of, link = "author")]
    pub search: SavedSearchId,
}

// The derive writes every link into the constant, in the order of the fields.
fn main() {
    let links = <Comment as Entity>::MARKS.person_links();
    assert_eq!(links.entity(), "Comment");
    assert_eq!(
        links.links(),
        [
            PersonLink::new("author", Some(Link::Author), LinkScope::Every),
            PersonLink::new(
                "issue",
                Some(Link::Subject),
                LinkScope::Record { kind: None }
            ),
        ]
    );
    assert_eq!(links.of_field("text").count(), 0);

    let links = <Notification as Entity>::MARKS.person_links();
    assert_eq!(
        links.links(),
        [
            PersonLink::new("recipient", Some(Link::Recipient), LinkScope::Every),
            PersonLink::new("changes", None, LinkScope::Changes),
        ]
    );
    assert_eq!(links.of_field("record").count(), 0);

    let person_in = |field_type| LinkScope::FieldType {
        key: "field",
        field_type,
    };
    assert_eq!(
        <StoredFieldValue as Entity>::MARKS.person_links().links(),
        [
            PersonLink::new(
                "value_ref",
                Some(Link::Subject),
                person_in(FieldType::Person)
            ),
            PersonLink::new(
                "value_ref",
                Some(Link::Subject),
                person_in(FieldType::People)
            ),
        ]
    );
    assert_eq!(
        <EmailChange as Entity>::MARKS.person_links().links(),
        [PersonLink::new(
            "new_email",
            Some(Link::Recipient),
            LinkScope::Email
        )]
    );
    assert_eq!(
        <SavedSearch as Entity>::MARKS.person_links().links(),
        [PersonLink::new(
            "timeline",
            Some(Link::Subject),
            LinkScope::Json
        )]
    );
    // A part takes the kind of its record from the type of the ID.
    let part = <SearchColumn as Entity>::MARKS.person_links();
    assert_eq!(
        part.links(),
        [PersonLink::new(
            "search",
            Some(Link::Author),
            LinkScope::PartOf {
                kind: RecordKind::SavedSearch
            }
        )]
    );
    assert_eq!(part.whole(), Some(RecordKind::SavedSearch));
    assert_eq!(links.whole(), None);
    // Each field of such a record has the title of its column.
    for marks in [
        <Comment as Entity>::MARKS,
        <Notification as Entity>::MARKS,
        <StoredFieldValue as Entity>::MARKS,
        <EmailChange as Entity>::MARKS,
        <SavedSearch as Entity>::MARKS,
        <SearchColumn as Entity>::MARKS,
    ] {
        assert!(marks.fields().iter().all(|field| field.title().is_some()));
    }
}
