use bb_core::event::{EntityRef, FieldChange};
use bb_core::id::{EntityId, FieldId, UserId};
use bb_core::own_fields::FieldType;
use bb_core::personal::{Entity, Link, LinkScope, PersonLink};

// An entity that names a person needs the message `personal-kind.<its name>`, so the example
// borrows the name of one that has it.
#[derive(Entity)]
pub struct Comment {
    #[personal(category = "content", basis = "contract")]
    pub text: String,
    #[personal(category = "identity", basis = "legitimate_interests")]
    #[person(author)]
    pub author: UserId,
    #[personal(category = "identity", basis = "legitimate_interests")]
    #[person(recipient)]
    pub readers: Vec<Option<UserId>>,
    #[not_personal]
    #[person_for(record, link = "subject")]
    pub about: EntityRef,
    #[not_personal]
    pub field: FieldId,
    #[personal(category = "activity", basis = "legitimate_interests")]
    #[person_for(field_type = "person", key = "field", link = "subject")]
    #[person_for(field_type = "people", key = "field", link = "subject")]
    pub value: Option<EntityId>,
    #[not_personal]
    #[names_no_person]
    pub record: EntityId,
    #[personal(category = "identity", basis = "contract")]
    #[person_email(recipient)]
    pub email: String,
    #[personal(category = "content", basis = "legitimate_interests")]
    #[person_in_changes]
    pub changes: Vec<FieldChange>,
    #[personal(category = "content", basis = "legitimate_interests")]
    #[person_in_json(subject)]
    pub payload: serde_json::Value,
    #[personal(category = "content", basis = "contract")]
    #[names_no_person]
    pub options: Option<serde_json::Value>,
}

// The derive writes every link into the constant, in the order of the fields.
fn main() {
    let links = <Comment as Entity>::MARKS.person_links();
    assert_eq!(links.entity(), "Comment");
    let person_in = |field_type| LinkScope::FieldType {
        key: "field",
        field_type,
    };
    assert_eq!(
        links.links(),
        [
            PersonLink::new("author", Some(Link::Author), LinkScope::Every),
            PersonLink::new("readers", Some(Link::Recipient), LinkScope::Every),
            PersonLink::new(
                "about",
                Some(Link::Subject),
                LinkScope::Record { kind: None }
            ),
            PersonLink::new("value", Some(Link::Subject), person_in(FieldType::Person)),
            PersonLink::new("value", Some(Link::Subject), person_in(FieldType::People)),
            PersonLink::new("email", Some(Link::Recipient), LinkScope::Email),
            PersonLink::new("changes", None, LinkScope::Changes),
            PersonLink::new("payload", Some(Link::Subject), LinkScope::Json),
        ]
    );
    assert_eq!(links.of_field("text").count(), 0);
    assert_eq!(links.of_field("record").count(), 0);
}
