//! Personal-data marks (ADR-0015, PRIV-02).
//!
//! Every stored field says whether it holds personal data. `#[derive(Entity)]` turns the marks of
//! a struct into a constant, and the list of all entities (`entities::ALL`) is what *Download my
//! data*, *Remove a person*, the keep-and-delete rules and the record of processing read. The
//! generated page `docs/privacy/personal-data.md` is written from the same marks.

pub use bb_macros::Entity;

/// What kind of personal data a field holds. The words are the ones in `#[personal(category = …)]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    /// Who a person is: name, e-mail address, avatar.
    Identity,
    /// What proves who a person is: password hash, passkeys, backup codes.
    Credentials,
    /// What a person did and when: who changed what, who is assigned.
    Activity,
    /// What a person wrote: comments, descriptions, reasons.
    Content,
    /// Working hours and the accounting fields of time entries.
    EmployeeTime,
    /// What identifies a device or connection: network address, browser, session.
    Technical,
}

impl Category {
    /// Every category, in the order the personal-data page lists them.
    pub const ALL: [Self; 6] = [
        Self::Identity,
        Self::Credentials,
        Self::Activity,
        Self::Content,
        Self::EmployeeTime,
        Self::Technical,
    ];

    /// The word used in `#[personal(category = "…")]`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Credentials => "credentials",
            Self::Activity => "activity",
            Self::Content => "content",
            Self::EmployeeTime => "employee_time",
            Self::Technical => "technical",
        }
    }
}

/// Why the data may be processed: the six legal bases of GDPR Article 6.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Basis {
    /// The person agreed (Art. 6(1)(a)).
    Consent,
    /// Needed for a contract with the person (Art. 6(1)(b)).
    Contract,
    /// A law requires it (Art. 6(1)(c)).
    LegalObligation,
    /// Needed to protect someone's life (Art. 6(1)(d)).
    VitalInterests,
    /// Needed for a task in the public interest (Art. 6(1)(e)).
    PublicTask,
    /// The organization has a good reason that does not override the person's rights (Art. 6(1)(f)).
    LegitimateInterests,
}

impl Basis {
    /// Every basis, in the order of the article.
    pub const ALL: [Self; 6] = [
        Self::Consent,
        Self::Contract,
        Self::LegalObligation,
        Self::VitalInterests,
        Self::PublicTask,
        Self::LegitimateInterests,
    ];

    /// The word used in `#[personal(basis = "…")]`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Consent => "consent",
            Self::Contract => "contract",
            Self::LegalObligation => "legal_obligation",
            Self::VitalInterests => "vital_interests",
            Self::PublicTask => "public_task",
            Self::LegitimateInterests => "legitimate_interests",
        }
    }
}

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
    fn codes_are_unique_lower_case_words() {
        let mut seen = std::collections::HashSet::new();
        for code in Category::ALL
            .iter()
            .map(|c| c.code())
            .chain(Basis::ALL.iter().map(|b| b.code()))
        {
            assert!(
                code.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{code}"
            );
            assert!(seen.insert(code), "{code} is used twice");
        }
    }

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
