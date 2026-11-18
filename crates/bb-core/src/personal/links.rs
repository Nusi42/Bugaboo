//! The links of a person (ADR-0044, decision 1): what a person is to a row that names them.
//!
//! Beside its personal-data mark, every field that can name a person says when it does and what
//! the person is to the row: always (`#[person(author)]`), in the rows about some kinds of record
//! (`#[person_for(record, …)]`, as `crate::event::whose` says for each kind), in the rows whose own
//! field is of a type that holds people (`#[person_for(field_type = …)]`), as the e-mail address of
//! the person (`#[person_email(…)]`) or in the values of the changes it holds
//! (`#[person_in_changes(…)]`). A reference to a record that never names a person says so
//! (`#[names_no_person]`). `#[derive(Entity)]` writes the links into the entity's marks
//! ([`PersonLinks`]): the list the walk of a person's data reads. The members of the changes of an
//! event that name a person are declared per kind of record ([`PayloadPerson`]), by the use cases
//! that write them.

use std::collections::{BTreeMap, BTreeSet};

use crate::own_fields::FieldType;

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
    /// The field holds an e-mail address; it names the person whose address it is
    /// (`#[person_email(…)]`).
    Email,
    /// The field holds the changes of a record (`FieldChange`), whose values can name a person:
    /// the members that do are declared for each kind of record ([`PayloadPerson`])
    /// (`#[person_in_changes(…)]`).
    Changes,
}

/// One field of an entity that names a person, when it does, and what the person is to the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonLink {
    field: &'static str,
    link: Link,
    scope: LinkScope,
}

impl PersonLink {
    /// The field `field` names a person in the rows `scope` says. Written by `#[derive(Entity)]`.
    #[must_use]
    pub const fn new(field: &'static str, link: Link, scope: LinkScope) -> Self {
        Self { field, link, scope }
    }

    /// The name of the field.
    #[must_use]
    pub const fn field(&self) -> &'static str {
        self.field
    }

    /// What the person is to the row.
    #[must_use]
    pub const fn link(&self) -> Link {
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

    /// The links of the field `field`.
    pub fn of_field(&self, field: &str) -> impl Iterator<Item = &'static PersonLink> {
        self.links.iter().filter(move |link| link.field == field)
    }
}

/// A member of the changes of an event that holds the ID of a person (or a list of them, or an
/// object with one inside) in its values before and after, in the events about one kind of record
/// (ADR-0044, decisions 1 and 6). The use cases that write the member declare it beside its name,
/// so the walk finds the event and *Remove a person* knows which values to scrub.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayloadPerson {
    /// The kind of record of the events.
    pub kind: crate::event::RecordKind,
    /// The name of the member in the changes; see [`PayloadMember`].
    pub member: PayloadMember,
    /// What the person is to the event.
    pub link: Link,
}

/// Which member of the changes holds a person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadMember {
    /// The member with this name.
    Named(&'static str),
    /// Every member that is an own field (named by the field's ID) of a type that holds people.
    OwnField,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_keeps_its_field_word_and_scope() {
        const LINKS: &[PersonLink] = &[
            PersonLink::new("author", Link::Author, LinkScope::Every),
            PersonLink::new("about", Link::Subject, LinkScope::Record { kind: None }),
        ];
        let link = PersonLink::new(
            "about",
            Link::Subject,
            LinkScope::FieldType {
                key: "field",
                field_type: FieldType::People,
            },
        );
        assert_eq!(link.field(), "about");
        assert_eq!(link.link(), Link::Subject);
        let all = PersonLinks::new("Note", LINKS);
        assert_eq!(all.of_field("about").count(), 1);
        assert_eq!(all.entity(), "Note");
    }
}
