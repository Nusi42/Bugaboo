use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::UserId;
use bb_core::personal::{Entity, Link, PersonLink};

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
    #[person_for(kind = "person", link = "subject")]
    #[person_for(kind = "timer", link = "subject")]
    pub about: EntityRef,
}

// The derive writes every link into the constant, in the order of the fields.
fn main() {
    let links = <Note as Entity>::MARKS.person_links();
    assert_eq!(links.entity(), "Note");
    assert_eq!(
        links.links(),
        [
            PersonLink::new("author", Link::Author),
            PersonLink::new("readers", Link::Recipient),
            PersonLink::for_kind("about", Link::Subject, RecordKind::Person),
            PersonLink::for_kind("about", Link::Subject, RecordKind::Timer),
        ]
    );
    assert_eq!(links.of_field("text").count(), 0);
    assert_eq!(links.of_field("about").count(), 2);
}
