//! Personal-data marks (ADR-0015, PRIV-02).
//!
//! Every stored field says whether it holds personal data. `#[derive(Entity)]` turns the marks of
//! a struct into a constant, and the list of all entities (`entities::ALL`) is what *Download my
//! data*, *Remove a person*, the keep-and-delete rules and the record of processing read. The
//! generated page `docs/privacy/personal-data.md` is written from the same marks.

use jiff::Timestamp;

pub use bb_macros::Entity;

words! {
    /// What kind of personal data a field holds. The words are the ones in
    /// `#[personal(category = …)]`.
    pub enum Category {
        /// Who a person is: name, e-mail address, avatar.
        Identity => "identity",
        /// What proves who a person is: password hash, passkeys, backup codes.
        Credentials => "credentials",
        /// What a person did and when: who changed what, who is assigned.
        Activity => "activity",
        /// Where a person works: their groups and the roles they have, and where (ADR-0025).
        Organization => "organization",
        /// What a person wrote: comments, descriptions, reasons.
        Content => "content",
        /// Time at work: work days and breaks, time entries with their notes and accounting
        /// fields, the running timer, and the reasons for changing a past work day (ADR-0035).
        EmployeeTime => "employee_time",
        /// What identifies a device or connection: network address, browser, session.
        Technical => "technical",
    }
}

words! {
    /// Why the data may be processed: the six legal bases of GDPR Article 6. The words are the
    /// ones in `#[personal(basis = …)]`.
    pub enum Basis {
        /// The person agreed (Art. 6(1)(a)).
        Consent => "consent",
        /// Needed for a contract with the person (Art. 6(1)(b)).
        Contract => "contract",
        /// A law requires it (Art. 6(1)(c)).
        LegalObligation => "legal_obligation",
        /// Needed to protect someone's life (Art. 6(1)(d)).
        VitalInterests => "vital_interests",
        /// Needed for a task in the public interest (Art. 6(1)(e)).
        PublicTask => "public_task",
        /// The organization has a good reason that does not override the person's rights (Art. 6(1)(f)).
        LegitimateInterests => "legitimate_interests",
    }
}

/// A type that cannot hold personal data, so a field of this type may be marked `#[not_personal]`.
///
/// It is implemented for numbers, truth values, moments, the IDs of records (not the ID of a
/// person), the words of the marks, and for types that say so explicitly. Text is not on the
/// list: free text can hold anything, so it is marked `#[personal(…)]`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` may hold personal data, so a field of this type cannot be `#[not_personal]`",
    label = "mark this field `#[personal(category = \"...\", basis = \"...\")]`",
    note = "or implement `NotPersonal` for the type, if it can never hold personal data (DATA-MARKS)"
)]
pub trait NotPersonal {}

macro_rules! not_personal {
    ($($ty:ty),+ $(,)?) => { $(impl NotPersonal for $ty {})+ };
}

not_personal!(
    bool, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64, char,
    Timestamp,
);

impl<T: NotPersonal> NotPersonal for Option<T> {}
impl<T: NotPersonal> NotPersonal for Vec<T> {}
impl<T: NotPersonal + ?Sized> NotPersonal for Box<T> {}

/// What an entity's field holds, as written on the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Personal {
    /// Personal data, with its category and the legal basis for keeping it.
    Yes {
        /// What kind of data it is.
        category: Category,
        /// Why it may be processed.
        basis: Basis,
    },
    /// Not personal data.
    No,
}

/// The mark of one field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldMark {
    name: &'static str,
    personal: Personal,
}

impl FieldMark {
    /// A field that holds personal data (`#[personal(…)]`).
    #[must_use]
    pub const fn personal(name: &'static str, category: Category, basis: Basis) -> Self {
        Self {
            name,
            personal: Personal::Yes { category, basis },
        }
    }

    /// A field that holds no personal data (`#[not_personal]`).
    #[must_use]
    pub const fn not_personal(name: &'static str) -> Self {
        Self {
            name,
            personal: Personal::No,
        }
    }

    /// The name of the field.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// What the field holds.
    #[must_use]
    pub const fn personal_data(&self) -> Personal {
        self.personal
    }
}

/// The marks of all fields of one entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityMarks {
    name: &'static str,
    fields: &'static [FieldMark],
}

impl EntityMarks {
    /// The marks of the entity `name`. Written by `#[derive(Entity)]`.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(name: &'static str, fields: &'static [FieldMark]) -> Self {
        Self { name, fields }
    }

    /// The name of the entity (its struct).
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Its fields, in the order they are declared.
    #[must_use]
    pub const fn fields(&self) -> &'static [FieldMark] {
        self.fields
    }

    /// How many fields hold personal data.
    #[must_use]
    pub fn personal_count(&self) -> usize {
        self.fields
            .iter()
            .filter(|field| matches!(field.personal, Personal::Yes { .. }))
            .count()
    }
}

/// Implemented by `#[derive(Entity)]`; there is no other way to implement it by accident.
pub trait Entity {
    /// The marks of this entity's fields.
    const MARKS: EntityMarks;
}

/// The list of all entities, for the personal-data map. Every entity is named here once:
/// `entities! { Person, DomainEvent }`. DATA-MARKS fails when an entity is missing from the list.
#[macro_export]
macro_rules! entities {
    ($($entity:ty),+ $(,)?) => {
        /// The marks of every entity, in the order of the list.
        pub const ALL: &[$crate::personal::EntityMarks] =
            &[$(<$entity as $crate::personal::Entity>::MARKS),+];
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_count_the_personal_fields() {
        const MARKS: EntityMarks = EntityMarks::new(
            "Demo",
            &[
                FieldMark::personal("email", Category::Identity, Basis::Contract),
                FieldMark::not_personal("id"),
            ],
        );
        assert_eq!(MARKS.personal_count(), 1);
        assert_eq!(MARKS.fields()[1].personal_data(), Personal::No);
        assert_eq!(MARKS.fields()[0].name(), "email");
    }
}
