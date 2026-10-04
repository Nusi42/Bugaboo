//! The list of all entities, for the personal-data map (ADR-0015).
//!
//! Every struct with `#[derive(Entity)]` is named here once. DATA-MARKS fails when one is missing,
//! so *Download my data*, *Remove a person* and the record of processing cannot forget it.

crate::entities! {
    crate::event::DomainEvent,
    crate::job::Job,
    crate::migration::MigrationRecord,
    crate::outbox::OutboxEntry,
    crate::outbox::OutboxPosition,
    crate::person::Person,
    crate::secret::StoredSecret,
    crate::security_log::SecurityLogCheckpoint,
    crate::security_log::SecurityLogEntry,
    crate::settings::StoredSetting,
}

#[cfg(test)]
mod tests {
    use super::ALL;
    use crate::personal::Personal;

    #[test]
    fn priv_02_names_are_unique_and_every_field_has_a_mark() {
        let mut names = std::collections::HashSet::new();
        for entity in ALL {
            assert!(
                names.insert(entity.name()),
                "{} is listed twice",
                entity.name()
            );
            assert!(!entity.fields().is_empty(), "{}", entity.name());
            for field in entity.fields() {
                // A mark is either personal data with its category and basis, or not personal.
                assert!(matches!(
                    field.personal_data(),
                    Personal::Yes { .. } | Personal::No
                ));
            }
        }
    }

    #[test]
    fn the_person_entity_marks_id_name_address_and_creation_as_personal() {
        let person = ALL.iter().find(|entity| entity.name() == "Person").unwrap();
        assert_eq!(person.personal_count(), 4);
    }
}
