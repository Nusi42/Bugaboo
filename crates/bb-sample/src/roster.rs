//! Who is in the sample organization: 50 people with the mix of roles of architecture §11, made
//! from the seed alone. The roster is the plan; `generate` makes the people it names through the
//! use cases.
//!
//! The roles are recorded here and nowhere else in the sample yet: M2 has no use case that gives a
//! person a role. When the policy engine exists (WP-05), WP-13 gives each person the role the
//! roster names, through the extension point [`crate::Later`].

use crate::error::SampleError;
use crate::names::NameList;
use crate::random::SeededRandom;

/// Every sample address ends with this domain: it is reserved for examples, so a message to a
/// sample person can never reach a real inbox (TEST-SAMPLE-DATA).
pub const EMAIL_DOMAIN: &str = "bugaboo.example";

/// A role of the starting roles, as the sample records it (a technical word, not text for people).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Looks after the whole organization.
    Administrator,
    /// Manages projects and who has which role in them.
    ProjectLead,
    /// Changes issues.
    TeamMember,
    /// Creates issues and changes their own.
    Contributor,
    /// Only looks.
    Viewer,
    /// Sees only what they reported.
    Guest,
}

impl Role {
    /// The word the dump and the sign-in file use for the role.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Administrator => "administrator",
            Self::ProjectLead => "project_lead",
            Self::TeamMember => "team_member",
            Self::Contributor => "contributor",
            Self::Viewer => "viewer",
            Self::Guest => "guest",
        }
    }
}

/// The mix of roles of the 50 people (architecture §11).
pub const ROLE_MIX: [(Role, usize); 6] = [
    (Role::Administrator, 2),
    (Role::ProjectLead, 4),
    (Role::TeamMember, 30),
    (Role::Contributor, 8),
    (Role::Viewer, 4),
    (Role::Guest, 2),
];

/// How many people the sample has.
pub const PEOPLE: usize = 50;

/// The people the demos name, the same whatever the seed: the administrator who runs the demo (the
/// first person, made with the setup code), Ana (the person whose access is checked) and Ben.
const FIXED: [(&str, &str, Role); 3] = [
    ("Ada", "Brandt", Role::Administrator),
    ("Ana", "Moreno", Role::Contributor),
    ("Ben", "Hale", Role::TeamMember),
];

/// Who gets a code app for the tests of the second step, by role: a few people, never the people
/// the demos name.
const CODE_APPS: [(Role, usize); 4] = [
    (Role::ProjectLead, 1),
    (Role::TeamMember, 2),
    (Role::Contributor, 1),
    (Role::Viewer, 1),
];

/// One person of the plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SamplePerson {
    /// The name others see: a given name and a family name from the list.
    pub name: String,
    /// The e-mail address, under [`EMAIL_DOMAIN`].
    pub email: String,
    /// The role the person gets once roles exist.
    pub role: Role,
    /// Whether the person gets a code app.
    pub code_app: bool,
    fixed: bool,
}

impl SamplePerson {
    fn new(first: &str, last: &str, role: Role, fixed: bool) -> Self {
        Self {
            name: format!("{first} {last}"),
            email: format!(
                "{}.{}@{EMAIL_DOMAIN}",
                first.to_ascii_lowercase(),
                last.to_ascii_lowercase()
            ),
            role,
            code_app: false,
            fixed,
        }
    }

    /// Whether the demos name this person (the same whatever the seed).
    #[must_use]
    pub const fn is_fixed(&self) -> bool {
        self.fixed
    }
}

/// The 50 people of the seed. The first is the administrator who runs the demo; the rest come in
/// an order that depends on the seed.
///
/// # Errors
/// Fails when the list of names is too short for 50 different people.
pub fn roster(seed: u64, names: &NameList) -> Result<Vec<SamplePerson>, SampleError> {
    let mut random = SeededRandom::new(seed);
    let mut roles: Vec<Role> = ROLE_MIX
        .iter()
        .flat_map(|(role, count)| std::iter::repeat_n(*role, *count))
        .collect();
    for (_, _, role) in FIXED {
        if let Some(at) = roles.iter().position(|taken| *taken == role) {
            roles.swap_remove(at);
        }
    }
    random.shuffle(&mut roles);

    let mut people: Vec<SamplePerson> = FIXED
        .iter()
        .map(|(first, last, role)| SamplePerson::new(first, last, *role, true))
        .collect();
    let mut tries = 0_usize;
    while people.len() < PEOPLE {
        tries += 1;
        let (Some(role), true) = (roles.last().copied(), tries < PEOPLE * 1000) else {
            return Err(SampleError::TooFewNames);
        };
        let first = &names.first()[random.below(names.first().len())];
        let last = &names.last()[random.below(names.last().len())];
        let person = SamplePerson::new(first, last, role, false);
        if people.iter().all(|known| known.email != person.email) {
            roles.pop();
            people.push(person);
        }
    }
    // The first person stays first (the setup code makes the first administrator); the others
    // come in the order of the seed, so the roles are spread over the people.
    if let Some((_, rest)) = people.split_first_mut() {
        random.shuffle(rest);
    }
    for (role, count) in CODE_APPS {
        let chosen = people
            .iter_mut()
            .filter(|person| person.role == role && !person.fixed)
            .take(count);
        for person in chosen {
            person.code_app = true;
        }
    }
    Ok(people)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn names() -> NameList {
        NameList::bundled().unwrap()
    }

    #[test]
    fn there_are_fifty_different_people_with_the_mix_of_architecture_11() {
        let people = roster(42, &names()).unwrap();
        assert_eq!(people.len(), PEOPLE);
        assert_eq!(
            ROLE_MIX.iter().map(|(_, count)| count).sum::<usize>(),
            PEOPLE
        );
        for (role, count) in ROLE_MIX {
            let found = people.iter().filter(|person| person.role == role).count();
            assert_eq!(found, count, "{}", role.code());
        }
        let emails: BTreeSet<&String> = people.iter().map(|person| &person.email).collect();
        assert_eq!(emails.len(), PEOPLE, "no address twice");
    }

    #[test]
    fn the_people_the_demos_name_are_the_same_whatever_the_seed() {
        for seed in [1, 42, 9_999] {
            let people = roster(seed, &names()).unwrap();
            assert_eq!(people[0].name, "Ada Brandt");
            assert_eq!(people[0].role, Role::Administrator);
            let find = |name: &str| people.iter().find(|person| person.name == name).unwrap();
            assert_eq!(find("Ana Moreno").role, Role::Contributor);
            assert_eq!(find("Ana Moreno").email, "ana.moreno@bugaboo.example");
            assert_eq!(find("Ben Hale").role, Role::TeamMember);
            assert!(!find("Ana Moreno").code_app && !people[0].code_app);
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_people_and_another_seed_other_people() {
        let first = roster(42, &names()).unwrap();
        assert_eq!(first, roster(42, &names()).unwrap());
        assert_ne!(first, roster(43, &names()).unwrap());
    }

    #[test]
    fn every_address_is_under_the_reserved_domain_and_every_name_is_from_the_list() {
        let list = names();
        for person in roster(42, &list).unwrap() {
            assert!(
                person.email.ends_with(&format!("@{EMAIL_DOMAIN}")),
                "{}",
                person.email
            );
            assert!(list.contains_full_name(&person.name), "{}", person.name);
        }
    }

    #[test]
    fn a_few_people_have_a_code_app() {
        let people = roster(42, &names()).unwrap();
        let with: Vec<&SamplePerson> = people.iter().filter(|person| person.code_app).collect();
        assert_eq!(with.len(), 5);
        assert!(with.iter().all(|person| !person.is_fixed()));
    }

    #[test]
    fn a_list_that_is_too_short_is_an_error_and_not_an_endless_loop() {
        let short = NameList::parse("first = [\"Ana\"]\nlast = [\"Moreno\", \"Hale\"]").unwrap();
        assert!(roster(42, &short).is_err());
    }
}
