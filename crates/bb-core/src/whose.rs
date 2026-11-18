//! Whose a record is, and which members of its changes name a person (ADR-0044, decisions 1 and
//! 6): the one place the walk of a person's data reads the second hop and the people in the
//! changes from. The use cases spell the members with the names here.

use crate::event::RecordKind;
use crate::fields::{Field, FieldKey};
use crate::personal::{Entity, EntityMarks, Link};

/// Whose a record of a kind is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whose {
    /// The record is the person: its ID is the person's.
    ThePerson,
    /// The record belongs to a person: it is stored as the entity of `of.marks`, with the ID of the
    /// record in its field `of.id`, and its field `of.value` names the person.
    Owner {
        /// Where the person a record belongs to is read.
        of: FieldOf,
    },
    /// The record is no person's: the people in it are found by its own links.
    Nobody,
}

/// A field of the rows of an entity, found by the ID of the row: where the walk reads what it needs
/// from another table (the person a record belongs to, the type of an own field, the address of a
/// person). A test checks that both fields are fields of the entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldOf {
    /// The marks of the entity.
    pub marks: EntityMarks,
    /// Its field that holds the ID of a row.
    pub id: &'static str,
    /// Its field that holds what is read.
    pub value: &'static str,
}

/// The field `value` of the entity `E`, by its field `id`.
const fn field_of<E: Entity>(value: &'static str) -> FieldOf {
    FieldOf {
        marks: E::MARKS,
        id: ID,
        value,
    }
}

/// The name of the field that holds the ID of a record, in every entity that `whose` and the walk
/// read by it.
const ID: &str = "id";

/// Where the walk reads the type of an own field (`LinkScope::FieldType`).
pub const OWN_FIELD_TYPE: FieldOf = field_of::<crate::own_fields::OwnField>("kind");

/// Where the walk reads the e-mail address of a person (`LinkScope::Email`).
pub const PERSON_ADDRESS: FieldOf = field_of::<crate::person::Person>("email");

const fn owner<E: Entity>(owner: &'static str) -> Whose {
    Whose::Owner {
        of: field_of::<E>(owner),
    }
}

/// Whose a record of `kind` is. A `match` without a catch-all, so a new kind has to say.
#[must_use]
pub const fn whose(kind: RecordKind) -> Whose {
    use RecordKind as K;
    match kind {
        K::Person | K::Timer | K::WorkTimes | K::Share => Whose::ThePerson,
        K::AccessKey => owner::<crate::access_key::AccessKey>("owner"),
        K::Agent => owner::<crate::agent::Agent>("person"),
        K::AgentWork => owner::<crate::agent_records::IssueWork>("person"),
        K::HandOver => owner::<crate::agent_records::HandOver>("person"),
        K::StandingRule => owner::<crate::agent_records::StandingRule>("person"),
        K::ChangeRun => owner::<crate::many_changes::ManyChangeRun>("person"),
        K::Assignment => owner::<crate::access::Assignment>("person"),
        K::DataExport => owner::<crate::data_export::DataExport>("person"),
        K::ConnectedApp => owner::<crate::oauth::OAuthGrant>("person"),
        K::Passkey => owner::<crate::sign_in::StoredPasskey>("person"),
        K::SavedSearch => owner::<crate::saved_search::SavedSearch>("maker"),
        K::Session => owner::<crate::sign_in::SignInSession>("person"),
        K::Theme => owner::<crate::theme::OwnTheme>("maker"),
        K::TimeEntry => owner::<crate::time::entry::TimeEntry>("person"),
        K::WorkDay => owner::<crate::working_time::record::WorkDay>("person"),
        K::AwayTime => owner::<crate::work_times::record::AwayTime>("person"),
        K::Attachment
        | K::Field
        | K::Comment
        | K::Follower
        | K::Group
        | K::Invitation
        | K::Issue
        | K::IssueType
        | K::Label
        | K::LabelGroup
        | K::OAuthClient
        | K::Project
        | K::Relation
        | K::Layout
        | K::RelationKind
        | K::Workflow
        | K::Role
        | K::Setting
        | K::SetupCode
        | K::StartingRoles
        | K::Tag
        | K::StarterFields => Whose::Nobody,
    }
}

/// The names of the members of the changes that hold a person, as the events store them. The use
/// cases spell the members with these.
pub mod members {
    /// The person of a work day, of work times or of a share, and the person responsible for an
    /// agent.
    pub const PERSON: &str = "person";
    /// Who follows an issue or leaves it.
    pub const FOLLOWER: &str = "follower";
    /// The notes of an edit lock, with the person who holds it inside.
    pub const EDIT_LOCK: &str = "edit_lock";
    /// A row of *Who has which role* that is taken away, with its person inside.
    pub const ASSIGNMENT: &str = "assignment";
    /// The person an agent is offered to, who has not accepted yet.
    pub const OFFERED_TO: &str = "offered_to";
    /// The person who asked for the renewal of the issue IDs of a project that gave up.
    pub const ISSUE_IDS_FAILED: &str = "issue-ids-failed";
    /// The person on the bar a time factor was set for.
    pub const TIME_FACTOR_FOR: &str = "time-factor-for";
}

/// A member of the changes of an event that holds the ID of a person (or a list of them, or an
/// object with one inside) in its values before and after, in the events about one kind of record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonInChanges {
    /// The kind of record of the events.
    pub kind: RecordKind,
    /// Which member.
    pub member: Member,
    /// What the person is to the event.
    pub link: Link,
}

/// Which member of the changes holds a person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Member {
    /// The member with this name.
    Named(&'static str),
    /// Every member that is an own field (named by the field's ID) of a type that holds people.
    OwnField,
}

const fn named(kind: RecordKind, member: &'static str, link: Link) -> PersonInChanges {
    PersonInChanges {
        kind,
        member: Member::Named(member),
        link,
    }
}

/// Every member of the changes that names a person, by kind of record: the one source of what a
/// person is to an event that names them in its changes. The in-memory store of the use-case tests
/// stops a test whose event or notification holds a person in a member that is not here.
pub const IN_CHANGES: &[PersonInChanges] = {
    use RecordKind as K;
    use members as m;
    &[
        named(K::WorkDay, m::PERSON, Link::Subject),
        named(K::WorkTimes, m::PERSON, Link::Subject),
        named(K::AwayTime, m::PERSON, Link::Subject),
        named(K::Share, m::PERSON, Link::Subject),
        named(K::Follower, m::FOLLOWER, Link::Subject),
        named(K::Issue, Field::ReportedBy.code(), Link::Author),
        named(K::Issue, Field::AssignedTo.code(), Link::Subject),
        named(K::Assignment, m::ASSIGNMENT, Link::Holder),
        named(K::Agent, m::OFFERED_TO, Link::Recipient),
        named(K::Agent, m::PERSON, Link::Holder),
        named(K::Project, m::ISSUE_IDS_FAILED, Link::Recipient),
        named(K::Issue, m::TIME_FACTOR_FOR, Link::Subject),
        named(K::Issue, m::EDIT_LOCK, Link::Holder),
        named(K::Project, m::EDIT_LOCK, Link::Holder),
        named(K::Comment, m::EDIT_LOCK, Link::Holder),
        PersonInChanges {
            kind: K::Issue,
            member: Member::OwnField,
            link: Link::Subject,
        },
    ]
};

/// What the person is to an event about a record of `kind` whose change `member` names them; none
/// when the member is not declared.
#[must_use]
pub fn link_in_changes(kind: &str, member: &str) -> Option<Link> {
    IN_CHANGES
        .iter()
        .find(|declared| {
            declared.kind.code() == kind
                && match declared.member {
                    Member::Named(name) => name == member,
                    Member::OwnField => {
                        matches!(FieldKey::from_code(member), Some(FieldKey::Own(_)))
                    }
                }
        })
        .map(|declared| declared.link)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_field(marks: EntityMarks, field: &str) -> bool {
        marks.fields().iter().any(|mark| mark.name() == field)
    }

    #[test]
    fn priv_02_every_field_the_walk_reads_from_another_table_is_a_field_of_its_entity() {
        let owners = RecordKind::ALL
            .iter()
            .filter_map(|kind| match whose(*kind) {
                Whose::Owner { of } => Some(of),
                Whose::ThePerson | Whose::Nobody => None,
            });
        for of in owners.chain([OWN_FIELD_TYPE, PERSON_ADDRESS]) {
            assert!(has_field(of.marks, of.id), "{}.{}", of.marks.name(), of.id);
            assert!(
                has_field(of.marks, of.value),
                "{}.{}",
                of.marks.name(),
                of.value
            );
        }
    }

    #[test]
    fn priv_02_a_member_of_the_changes_says_what_the_person_is_to_the_event() {
        assert_eq!(
            link_in_changes("issue", Field::ReportedBy.code()),
            Some(Link::Author)
        );
        assert_eq!(
            link_in_changes("work_day", members::PERSON),
            Some(Link::Subject)
        );
        // The notes of an edit lock hold the person who holds it, under `members::PERSON` inside.
        for kind in ["issue", "project", "comment"] {
            assert_eq!(
                link_in_changes(kind, members::EDIT_LOCK),
                Some(Link::Holder)
            );
        }
        assert_eq!(link_in_changes("issue", "title"), None);
        assert_eq!(link_in_changes("project", members::PERSON), None);
    }
}
