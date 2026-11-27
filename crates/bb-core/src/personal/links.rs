//! The links of a person (ADR-0044, decision 1): what a person is to a row that names them.
//!
//! Beside its personal-data mark, every field that can name a person says when it does and what
//! the person is to the row: always (`#[person(author)]`), in the rows about some kinds of record
//! (`#[person_for(record, …)]`, as `crate::event::whose` says for each kind), in the rows whose own
//! field is of a type that holds people (`#[person_for(field_type = …)]`), in the rows that are a
//! part of a record of the person (`#[person_for(part_of)]`), as the e-mail address of
//! the person (`#[person_email(…)]`), in the values of the changes it holds (`#[person_in_changes]`,
//! where each member says what the person is to the event in `crate::whose::IN_CHANGES`) or in
//! the JSON it holds (`#[person_in_json(…)]`). A reference or JSON that never names a person says
//! so (`#[names_no_person]`). `#[derive(Entity)]` writes the links into the entity's marks
//! ([`PersonLinks`]): the list the walk of a person's data reads.

use std::collections::{BTreeMap, BTreeSet};

use crate::event::RecordKind;
use crate::own_fields::FieldType;
use crate::whose::{FieldOf, Whose, field_of, whose};

words! {
    /// What a person is to a row that names them. The words are the ones in `#[person(…)]`,
    /// `#[person_email(…)]` and `#[person_for(link = …)]`.
    pub enum Link {
        /// The person wrote or made it: a comment, a draft, an issue they reported.
        Author => "author",
        /// It is addressed to the person: a notification, a mention, an invitation.
        Recipient => "recipient",
        /// It is about the person: their own record, their work day, their settings.
        Subject => "subject",
        /// The person did it: the actor of an event, who changed or decided something.
        Actor => "actor",
        /// The person holds or owns it: a session, a lock, an access key, an agent.
        Holder => "holder",
    }
    messages name = "person-link."
}

/// A type that holds the ID of a person, so a stored field of this type needs a link that says
/// what the person is to the row (`#[person(…)]`). DATA-MARKS reads the list of these types from
/// the `names_a_person!` below and fails a field of one of them that has no link.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not hold the ID of a person, so it cannot carry `#[person(...)]`",
    note = "a link says what a person is to the row; only a field that names a person has one (ADR-0044)"
)]
pub trait NamesAPerson {}

macro_rules! names_a_person {
    ($($ty:ty),+ $(,)?) => { $(impl NamesAPerson for $ty {})+ };
}

names_a_person!(crate::id::UserId, crate::event::Actor);

impl<T: NamesAPerson> NamesAPerson for Option<T> {}
impl<T: NamesAPerson> NamesAPerson for Vec<T> {}
impl<T: NamesAPerson + ?Sized> NamesAPerson for Box<T> {}
impl<T: NamesAPerson> NamesAPerson for BTreeSet<T> {}
impl<K, T: NamesAPerson> NamesAPerson for BTreeMap<K, T> {}

/// A type that refers to a record of any kind, which can be a person: a field of this type says in
/// which rows it names a person (`#[person_for(…)]`) or that it never does (`#[names_no_person]`).
/// DATA-MARKS reads the list of these types from the `refers_to_a_record!` below.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a reference to a record, so it cannot carry `#[person_for(...)]` or `#[names_no_person]`",
    note = "a field that always names a person carries `#[person(...)]` (ADR-0044)"
)]
pub trait RefersToARecord {}

macro_rules! refers_to_a_record {
    ($($ty:ty),+ $(,)?) => { $(impl RefersToARecord for $ty {})+ };
}

refers_to_a_record!(crate::id::EntityId, crate::event::EntityRef);

impl<T: RefersToARecord> RefersToARecord for Option<T> {}

/// The ID of a record that belongs to a person, so a field of it can carry
/// `#[person_for(part_of)]`: the rows that hold it are a part of that record (the boxes of a
/// dashboard, the breaks of a work day, the versions of a comment) and are the person's when the
/// record is. `OWNER` says where the person of the record is read, once for the type of its ID; what
/// the person is to a part is what the field of `OWNER` says they are to the record. A field of such
/// a type that is no part of its record says `#[names_no_person]` (DATA-MARKS, which reads the list
/// from `ids_of_owned_records!` below).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not the ID of a record a person owns, so it cannot carry `#[person_for(part_of)]`",
    note = "add the type to `ids_of_owned_records!` in `bb_core::personal`, with where the person of its record is read"
)]
pub trait IdOfAnOwnedRecord {
    /// Where the person of the record is read: its entity, its field of the ID and its field that
    /// names the person.
    const OWNER: FieldOf;
    /// What the person is to a part: the link of the field of `OWNER` that names them.
    const LINK: Link = link_of(Self::OWNER);
}

/// Where the person of a record of `kind` is read, as `whose` says. A kind that is the person
/// themselves or nobody's has no owner to read: in the `const` of `ids_of_owned_records!` it stops
/// the build.
const fn owned(kind: RecordKind) -> FieldOf {
    let owner = whose(kind);
    assert!(
        matches!(owner, Whose::Owner { .. }),
        "only the ID of a record that belongs to a person can be listed"
    );
    match owner {
        Whose::Owner { of } => of,
        // Never reached: the assertion above stops the build first.
        Whose::ThePerson | Whose::Nobody => crate::whose::PERSON_ADDRESS,
    }
}

/// The link of the field `of.value` of the entity `of.marks`, which names the person in every row.
/// A field without one stops the build, in the `const` of `IdOfAnOwnedRecord::LINK`.
const fn link_of(of: FieldOf) -> Link {
    let links = of.marks.person_links().links();
    let mut found = None;
    let mut at = 0;
    while at < links.len() {
        let link = links[at];
        if found.is_none()
            && crate::words::same_text(link.field, of.value)
            && matches!(link.scope, LinkScope::Every)
        {
            found = link.link;
        }
        at += 1;
    }
    assert!(
        found.is_some(),
        "the field that names the person of a record carries no #[person(...)]"
    );
    match found {
        Some(word) => word,
        // Never reached: the assertion above stops the build first.
        None => Link::Subject,
    }
}

macro_rules! ids_of_owned_records {
    ($($ty:ty => $owner:expr),+ $(,)?) => {
        $(
            impl IdOfAnOwnedRecord for $ty {
                const OWNER: FieldOf = $owner;
            }
            // A field of the ID that is no part of its record says so with `#[names_no_person]`.
            impl CanNameAPerson for $ty {}
            impl CanNameAPerson for Option<$ty> {}
        )+
    };
}

ids_of_owned_records!(
    crate::id::DashboardId => owned(RecordKind::Dashboard),
    crate::id::SavedSearchId => owned(RecordKind::SavedSearch),
    crate::id::WorkDayId => owned(RecordKind::WorkDay),
    crate::id::AccessKeyId => owned(RecordKind::AccessKey),
    crate::id::GrantId => owned(RecordKind::ConnectedApp),
    crate::id::StandingRuleId => owned(RecordKind::StandingRule),
    // A comment is nobody's in `whose` (the people in its events are found by their own links),
    // yet its versions are its author's, as the comment is: its own link says so.
    crate::id::CommentId => field_of::<crate::comment::Comment>("author"),
);

/// A type that holds an e-mail address as text, so a field of it can carry
/// `#[person_email(…)]`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not the text of an e-mail address, so it cannot carry `#[person_email(...)]`"
)]
pub trait HoldsAnAddress {}

impl HoldsAnAddress for String {}
impl HoldsAnAddress for Option<String> {}

/// A type that holds the changes of a record, whose values can name a person, so a field of it
/// carries `#[person_in_changes(…)]`. DATA-MARKS reads the list from `carries_changes!` below.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not hold the changes of a record, so it cannot carry `#[person_in_changes(...)]`"
)]
pub trait CarriesChanges {}

macro_rules! carries_changes {
    ($($ty:ty),+ $(,)?) => { $(impl CarriesChanges for $ty {})+ };
}

carries_changes!(crate::event::FieldChange);

impl<T: CarriesChanges> CarriesChanges for Vec<T> {}

/// A type that holds JSON, which can hold anything, a person too, so a field of it carries
/// `#[person_in_json(…)]` or `#[names_no_person]`. DATA-MARKS reads the list from `holds_json!`
/// below.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not hold JSON, so it cannot carry `#[person_in_json(...)]`"
)]
pub trait HoldsJson {}

macro_rules! holds_json {
    ($($ty:ty),+ $(,)?) => { $(impl HoldsJson for $ty {})+ };
}

holds_json!(serde_json::Value);

impl<T: HoldsJson> HoldsJson for Option<T> {}

/// A type that can name a person without always doing so: a reference to a record or JSON. A field
/// of it that never names one says so with `#[names_no_person]`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot name a person, so it needs no `#[names_no_person]`"
)]
pub trait CanNameAPerson {}

impl<T: RefersToARecord> CanNameAPerson for T {}
impl CanNameAPerson for serde_json::Value {}
impl CanNameAPerson for Option<serde_json::Value> {}

/// In which rows a field names a person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkScope {
    /// In every row: the field holds the ID of a person (`#[person(…)]`).
    Every,
    /// In the rows about a record that is a person or belongs to one, as
    /// [`crate::event::whose`] says for its kind: the field is a reference to a record, a kind
    /// and an ID (`#[person_for(record, link = …)]`), or an ID whose kind is in another field
    /// (`#[person_for(record, kind = "entity_kind", link = …)]`).
    Record {
        /// The field that holds the kind of the record, when the field itself holds only its ID.
        kind: Option<&'static str>,
    },
    /// In the rows whose own field is of this type: `key` is the field of the row that holds the
    /// ID of the own field (`#[person_for(field_type = …, key = …)]`).
    FieldType {
        /// The field of the row that names the own field.
        key: &'static str,
        /// The type of the own field in whose rows the field names a person.
        field_type: FieldType,
    },
    /// In the rows that are a part of a record of the person: the field holds the ID of the record,
    /// whose person is read where `of` says ([`IdOfAnOwnedRecord`]), and the person is to the part
    /// what they are to the record (`#[person_for(part_of)]`). The boxes of a dashboard name nobody
    /// themselves, yet they are the dashboard's maker's as much as it is.
    PartOf {
        /// Where the person of the record is read.
        of: FieldOf,
    },
    /// The field holds an e-mail address; it names the person whose address it is
    /// (`#[person_email(…)]`).
    Email,
    /// The field holds the changes of a record (`FieldChange`), whose values can name a person:
    /// the members that do, and what the person is to the event, are declared for each kind of
    /// record in `crate::whose::IN_CHANGES` (`#[person_in_changes]`).
    Changes,
    /// The field holds JSON, which names the person when it holds their ID as a text anywhere
    /// inside (`#[person_in_json(…)]`).
    Json,
}

/// One field of an entity that names a person, when it does, and what the person is to the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonLink {
    field: &'static str,
    link: Option<Link>,
    scope: LinkScope,
}

impl PersonLink {
    /// The field `field` names a person in the rows `scope` says. Written by `#[derive(Entity)]`.
    #[must_use]
    pub const fn new(field: &'static str, link: Option<Link>, scope: LinkScope) -> Self {
        Self { field, link, scope }
    }

    /// The name of the field.
    #[must_use]
    pub const fn field(&self) -> &'static str {
        self.field
    }

    /// What the person is to the row; none for the changes of a record, where each member says
    /// it (`crate::whose::IN_CHANGES`).
    #[must_use]
    pub const fn link(&self) -> Option<Link> {
        self.link
    }

    /// In which rows the field names a person.
    #[must_use]
    pub const fn scope(&self) -> LinkScope {
        self.scope
    }
}

/// The fields of one entity that name a person, with their links: the list the walk of a person's
/// data reads (ADR-0044, decision 1). Generated by `#[derive(Entity)]` from the same definition
/// as the marks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonLinks {
    entity: &'static str,
    links: &'static [PersonLink],
}

impl PersonLinks {
    /// The links of the entity `entity`.
    #[must_use]
    pub(crate) const fn new(entity: &'static str, links: &'static [PersonLink]) -> Self {
        Self { entity, links }
    }

    /// The name of the entity.
    #[must_use]
    pub const fn entity(&self) -> &'static str {
        self.entity
    }

    /// Its links, in the order the fields are declared; empty when no field names a person.
    #[must_use]
    pub const fn links(&self) -> &'static [PersonLink] {
        self.links
    }

    /// The entity of the record whose parts the rows are (`LinkScope::PartOf`); none for an entity
    /// whose rows are records of their own. A part goes and stays with its record.
    #[must_use]
    pub fn whole(&self) -> Option<super::EntityMarks> {
        self.links.iter().find_map(|link| match link.scope {
            LinkScope::PartOf { of } => Some(of.marks),
            _ => None,
        })
    }

    /// The links of the field `field`.
    pub fn of_field(&self, field: &str) -> impl Iterator<Item = &'static PersonLink> {
        self.links.iter().filter(move |link| link.field == field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_keeps_its_field_word_and_scope() {
        const LINKS: &[PersonLink] = &[
            PersonLink::new("author", Some(Link::Author), LinkScope::Every),
            PersonLink::new(
                "about",
                Some(Link::Subject),
                LinkScope::Record { kind: None },
            ),
        ];
        let link = PersonLink::new(
            "about",
            Some(Link::Subject),
            LinkScope::FieldType {
                key: "field",
                field_type: FieldType::People,
            },
        );
        assert_eq!(link.field(), "about");
        assert_eq!(link.link(), Some(Link::Subject));
        let all = PersonLinks::new("Note", LINKS);
        assert_eq!(all.of_field("about").count(), 1);
        assert_eq!(all.entity(), "Note");
    }
}
