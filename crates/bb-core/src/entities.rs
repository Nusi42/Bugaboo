//! The list of all entities, for the personal-data map (ADR-0015).
//!
//! Every struct with `#[derive(Entity)]` is named here once. DATA-MARKS fails when one is missing,
//! so *Download my data*, *Remove a person* and the record of processing cannot forget it.

crate::entities! {
    crate::access::Assignment,
    crate::access::Group,
    crate::access::GroupMember,
    crate::access::LegacyAdministrator,
    crate::access::ProjectFieldLevel,
    crate::access::ProjectPeopleLevel,
    crate::access::Role,
    crate::access::RoleFieldLevel,
    crate::access::RolePeopleLevel,
    crate::access::RolePermission,
    crate::access::StartingRolesMade,
    crate::event::DomainEvent,
    crate::job::Job,
    crate::migration::MigrationRecord,
    crate::outbox::OutboxEntry,
    crate::outbox::OutboxPosition,
    crate::person::Person,
    crate::project::Project,
    crate::secret::StoredSecret,
    crate::security_log::SecurityLogCheckpoint,
    crate::security_log::SecurityLogEntry,
    crate::settings::StoredSetting,
    crate::sign_in::BackupCode,
    crate::sign_in::Invitation,
    crate::sign_in::PasswordLogin,
    crate::sign_in::SetupCode,
    crate::sign_in::SignInSession,
    crate::sign_in::StoredPasskey,
    crate::sign_in::TwoStep,
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
