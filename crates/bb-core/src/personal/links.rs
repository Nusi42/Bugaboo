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

use crate::own_fields::FieldType;
use crate::whose::{FieldOf, field_of};

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
/// record is. `OWNER` says where the person of the record is read: the one field of the record that
/// names a person in every row; what the person is to a part is what that field says they are to
/// the record ([`part_link`]). Only `ids_of_owned_records!` below implements it, so no part can say
/// otherwise. A field of such a type that is no part of its record says `#[names_no_person]`
/// (DATA-MARKS, which reads the list from `ids_of_owned_records!`).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not the ID of a record a person owns, so it cannot carry `#[person_for(part_of)]`",
    note = "add the type to `ids_of_owned_records!` in `bb_core::personal`, with its record"
)]
pub trait IdOfAnOwnedRecord: listed::Listed {
    /// Where the person of the record is read: its entity, its field of the ID and its field that
    /// names the person.
    const OWNER: FieldOf;
}

/// The seal of [`IdOfAnOwnedRecord`]: only the list below implements it.
mod listed {
    pub trait Listed {}
}

/// What the person is to a part of a record whose ID is `T`: the link of the field that names the
/// person of the record.
#[must_use]
pub const fn part_link<T: IdOfAnOwnedRecord>() -> Link {
    match the_person_link(T::OWNER.marks).link {
        Some(word) => word,
        // Never reached: `the_person_link` takes only a link of a person in every row.
        None => Link::Subject,
    }
}

/// The one link of the entity of `marks` that names a person in every row. An entity with none,
/// or with more than one, has no one person to belong to: in the `const` of the list it stops the
/// build.
const fn the_person_link(marks: super::EntityMarks) -> PersonLink {
    let links = marks.person_links().links();
    let mut found = None;
    let mut count = 0;
    let mut at = 0;
    while at < links.len() {
        let link = links[at];
        if matches!(link.scope, LinkScope::Every) && link.link.is_some() {
            found = Some(link);
            count += 1;
        }
        at += 1;
    }
    assert!(
        count == 1,
        "a record whose ID is listed names its person in exactly one field, by #[person(...)]"
    );
    match found {
        Some(link) => link,
        // Never reached: the assertion above stops the build first.
        None => PersonLink::new("", None, LinkScope::Every),
    }
}

/// Where the person of the record `E` is read: its one field that names a person in every row.
const fn person_of<E: super::Entity>() -> FieldOf {
    field_of::<E>(the_person_link(E::MARKS).field)
}

/// Each entry is the type of an ID and its record. The person of the record is read from the
/// record's own marks, and the build stops when the record's own ID is of another type.
macro_rules! ids_of_owned_records {
    ($($ty:ty => $record:path),+ $(,)?) => {
        $(
            impl listed::Listed for $ty {}
            impl IdOfAnOwnedRecord for $ty {
                const OWNER: FieldOf = person_of::<$record>();
            }
            const _: fn(&$record) -> $ty = |record| record.id;
            // A field of the ID that is no part of its record says so with `#[names_no_person]`.
            impl CanNameAPerson for $ty {}
            impl CanNameAPerson for Option<$ty> {}
        )+
        /// Where the person of each listed record is read, for the test that `whose` agrees.
        #[cfg(test)]
        const LISTED: &[FieldOf] = &[$(<$ty as IdOfAnOwnedRecord>::OWNER),+];
    };
}

// A comment is nobody's in `whose` (the people in its events are found by their own links), yet
// its versions are its author's, as the comment is.
ids_of_owned_records!(
    crate::id::DashboardId => crate::dashboard::Dashboard,
    crate::id::SavedSearchId => crate::saved_search::SavedSearch,
    crate::id::WorkDayId => crate::working_time::record::WorkDay,
    crate::id::AccessKeyId => crate::access_key::AccessKey,
    crate::id::GrantId => crate::oauth::OAuthGrant,
    crate::id::StandingRuleId => crate::agent_records::StandingRule,
    crate::id::CommentId => crate::comment::Comment,
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

    /// The links by which rows that name a person by one of `words` are counted: every link of the
    /// entity with such a word, never the link of a part, which names the person only as much as
    /// the record it is a part of does (the plan of a removal counts that record).
    pub fn counted(&self, words: &[Link]) -> impl Iterator<Item = &'static PersonLink> {
        self.links.iter().filter(move |link| {
            !matches!(link.scope, LinkScope::PartOf { .. })
                && link.link.is_some_and(|word| words.contains(&word))
        })
    }

    /// The links that make a row a part of a record of the person (`LinkScope::PartOf`): such a
    /// row goes and stays with its record, so it is not counted apart even when its own links name
    /// the person too.
    pub fn parts(&self) -> impl Iterator<Item = &'static PersonLink> {
        self.links
            .iter()
            .filter(|link| matches!(link.scope, LinkScope::PartOf { .. }))
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
    fn priv_03_a_listed_record_reads_its_person_where_whose_does() {
        use crate::event::RecordKind;
        use crate::whose::{Whose, whose};
        for kind in RecordKind::ALL {
            let Whose::Owner { of } = whose(*kind) else {
                continue;
            };
            for listed in LISTED
                .iter()
                .filter(|listed| listed.marks.name() == of.marks.name())
            {
                assert_eq!(*listed, of, "{kind:?}");
            }
        }
        assert_eq!(LISTED.len(), 7);
    }

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
