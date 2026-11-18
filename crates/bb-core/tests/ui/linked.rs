use bb_core::event::{EntityRef, FieldChange};
use bb_core::id::{EntityId, FieldId, UserId};
use bb_core::own_fields::FieldType;
use bb_core::personal::{Entity, Link, LinkScope, PersonLink};

#[derive(Entity)]
pub struct Note {
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
    #[person_in_changes(subject)]
    pub changes: Vec<FieldChange>,
}

// The derive writes every link into the constant, in the order of the fields.
fn main() {
    let links = <Note as Entity>::MARKS.person_links();
    assert_eq!(links.entity(), "Note");
    let person_in = |field_type| LinkScope::FieldType {
        key: "field",
        field_type,
    };
    assert_eq!(
        links.links(),
        [
            PersonLink::new("author", Link::Author, LinkScope::Every),
            PersonLink::new("readers", Link::Recipient, LinkScope::Every),
            PersonLink::new("about", Link::Subject, LinkScope::Record { kind: None }),
            PersonLink::new("value", Link::Subject, person_in(FieldType::Person)),
            PersonLink::new("value", Link::Subject, person_in(FieldType::People)),
            PersonLink::new("email", Link::Recipient, LinkScope::Email),
            PersonLink::new("changes", Link::Subject, LinkScope::Changes),
        ]
    );
    assert_eq!(links.of_field("text").count(), 0);
    assert_eq!(links.of_field("record").count(), 0);
}
