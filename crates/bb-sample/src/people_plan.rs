//! Who is in the sample organization: 50 people with the mix of roles of architecture §11, made
//! from the seed alone. The plan lists the people; `generate` makes the people it names through the
//! use cases.
//!
//! The role of each person is recorded here; `organization` gives it to them as assignments, directly
//! or through a group. The same goes for the settings of each person (time zone and region) and
//! for the few people who have an access key.

use bb_app::access::StartingRole;
use bb_core::access_key::KeyChoice;
use bb_core::settings::{
    DEVICE, REGION_DATE_FORMAT, REGION_FIRST_DAY, REGION_NUMBER_FORMAT, SettingDefinition,
};

use crate::error::SampleError;
use crate::names::NameList;
use crate::random::SeededRandom;

/// Every sample address ends with this domain: it is reserved for examples, so a message to a
/// sample person can never reach a real inbox (TEST-SAMPLE-DATA).
pub const EMAIL_DOMAIN: &str = "bugaboo.example";

/// The mix of roles of the 50 people (architecture §11).
pub const ROLE_MIX: [(StartingRole, usize); 6] = [
    (StartingRole::Administrator, 2),
    (StartingRole::ProjectLead, 4),
    (StartingRole::TeamMember, 30),
    (StartingRole::Contributor, 8),
    (StartingRole::Viewer, 4),
    (StartingRole::Guest, 2),
];

/// How many people the sample has.
pub const PEOPLE: usize = 50;

/// The people the demos name, the same whatever the seed: the administrator who runs the demo (the
/// first person, made with the setup code), Ana (the person whose access is checked) and Ben.
const FIXED: [(&str, &str, StartingRole); 3] = [
    ("Ada", "Brandt", StartingRole::Administrator),
    ("Ana", "Moreno", StartingRole::Contributor),
    ("Ben", "Hale", StartingRole::TeamMember),
];

/// Who gets a code app for the tests of the second step, by role: a few people, never the people
/// the demos name.
const CODE_APPS: [(StartingRole, usize); 4] = [
    (StartingRole::ProjectLead, 1),
    (StartingRole::TeamMember, 2),
    (StartingRole::Contributor, 1),
    (StartingRole::Viewer, 1),
];

/// Who gets an access key, by role: a few people, never the people the demos name.
const KEY_HOLDERS: [(StartingRole, usize); 2] = [
    (StartingRole::ProjectLead, 1),
    (StartingRole::TeamMember, 2),
];

/// The access keys of the sample: what each is called and what it may do. Keys are given to the
/// people of [`KEY_HOLDERS`] in this order.
pub const ACCESS_KEYS: [(&str, KeyChoice); 3] = [
    ("Build server", KeyChoice::SeeOnly),
    ("Release script", KeyChoice::SeeAndChange),
    ("Reporting", KeyChoice::SeeOnly),
];

/// The time zones the people of the sample live in: spread over the world.
pub const TIME_ZONES: [&str; 10] = [
    "Europe/Berlin",
    "Europe/London",
    "Europe/Lisbon",
    "America/New_York",
    "America/Sao_Paulo",
    "Africa/Lagos",
    "Asia/Kolkata",
    "Asia/Tokyo",
    "Australia/Sydney",
    "UTC",
];

/// The region a person chose in *My settings*: time zone, how dates and numbers are written and
/// the first day of the week.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    /// A name of the IANA list.
    pub time_zone: String,
    /// A choice of the setting *Date format*.
    pub date_format: String,
    /// A choice of the setting *Number format*.
    pub number_format: String,
    /// A choice of the setting *First day of the week*.
    pub first_day: String,
}

/// One person of the plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SamplePerson {
    /// The name others see: a given name and a family name from the list.
    pub name: String,
    /// The e-mail address, under [`EMAIL_DOMAIN`].
    pub email: String,
    /// The role the person gets once roles exist.
    pub role: StartingRole,
    /// Whether the person gets a code app.
    pub code_app: bool,
    /// Which of the [`ACCESS_KEYS`] the person gets, if any.
    pub access_key: Option<usize>,
    /// What the person chose in *My settings › Region*.
    pub region: Region,
    fixed: bool,
}

impl SamplePerson {
    fn new(first: &str, last: &str, role: StartingRole, fixed: bool) -> Self {
        Self {
            name: format!("{first} {last}"),
            email: format!(
                "{}.{}@{EMAIL_DOMAIN}",
                first.to_ascii_lowercase(),
                last.to_ascii_lowercase()
            ),
            role,
            code_app: false,
            access_key: None,
            region: Region {
                time_zone: DEVICE.to_owned(),
                date_format: DEVICE.to_owned(),
                number_format: DEVICE.to_owned(),
                first_day: DEVICE.to_owned(),
            },
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
pub fn sample_people(seed: u64, names: &NameList) -> Result<Vec<SamplePerson>, SampleError> {
    let mut random = SeededRandom::new(seed);
    let mut roles: Vec<StartingRole> = ROLE_MIX
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
    let mut next_key = 0;
    for (role, count) in CODE_APPS {
        let chosen = people
            .iter_mut()
            .filter(|person| person.role == role && !person.fixed)
            .take(count);
        for person in chosen {
            person.code_app = true;
        }
    }
    for (role, count) in KEY_HOLDERS {
        let chosen = people
            .iter_mut()
            .filter(|person| person.role == role && !person.fixed && !person.code_app)
            .take(count);
        for person in chosen {
            person.access_key = Some(next_key);
            next_key += 1;
        }
    }
    // The last thing drawn from the seed, so nothing above changes when a setting is added.
    for person in &mut people {
        person.region = Region {
            time_zone: TIME_ZONES[random.below(TIME_ZONES.len())].to_owned(),
            date_format: choice_of(&mut random, REGION_DATE_FORMAT.definition()),
            number_format: choice_of(&mut random, REGION_NUMBER_FORMAT.definition()),
            first_day: choice_of(&mut random, REGION_FIRST_DAY.definition()),
        };
    }
    Ok(people)
}

/// A choice of a setting that is not "whatever my device says".
fn choice_of(random: &mut SeededRandom, setting: &SettingDefinition) -> String {
    let choices: Vec<&str> = setting
        .format()
        .choices()
        .iter()
        .map(bb_core::settings::Choice::value)
        .filter(|value| *value != DEVICE)
        .collect();
    choices
        .get(random.below(choices.len()))
        .map_or(DEVICE, |value| value)
        .to_owned()
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
        let people = sample_people(42, &names()).unwrap();
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
            let people = sample_people(seed, &names()).unwrap();
            assert_eq!(people[0].name, "Ada Brandt");
            assert_eq!(people[0].role, StartingRole::Administrator);
            let find = |name: &str| people.iter().find(|person| person.name == name).unwrap();
            assert_eq!(find("Ana Moreno").role, StartingRole::Contributor);
            assert_eq!(find("Ana Moreno").email, "ana.moreno@bugaboo.example");
            assert_eq!(find("Ben Hale").role, StartingRole::TeamMember);
            assert!(!find("Ana Moreno").code_app && !people[0].code_app);
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_people_and_another_seed_other_people() {
        let first = sample_people(42, &names()).unwrap();
        assert_eq!(first, sample_people(42, &names()).unwrap());
        assert_ne!(first, sample_people(43, &names()).unwrap());
    }

    #[test]
    fn every_address_is_under_the_reserved_domain_and_every_name_is_from_the_list() {
        let list = names();
        for person in sample_people(42, &list).unwrap() {
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
        let people = sample_people(42, &names()).unwrap();
        let with: Vec<&SamplePerson> = people.iter().filter(|person| person.code_app).collect();
        assert_eq!(with.len(), 5);
        assert!(with.iter().all(|person| !person.is_fixed()));
    }

    #[test]
    fn three_people_have_an_access_key_and_never_the_people_the_demos_name() {
        let people = sample_people(42, &names()).unwrap();
        let mut keys: Vec<(usize, StartingRole)> = people
            .iter()
            .filter_map(|person| person.access_key.map(|at| (at, person.role)))
            .collect();
        keys.sort_by_key(|(at, _)| *at);
        assert_eq!(keys.len(), ACCESS_KEYS.len());
        assert_eq!(
            keys.iter().map(|(at, _)| *at).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert!(
            people
                .iter()
                .filter(|person| person.access_key.is_some())
                .all(|person| !person.is_fixed() && !person.code_app)
        );
    }

    #[test]
    fn the_people_live_in_many_time_zones_and_write_dates_and_numbers_in_chosen_ways() {
        let people = sample_people(42, &names()).unwrap();
        let zones: BTreeSet<&str> = people
            .iter()
            .map(|person| person.region.time_zone.as_str())
            .collect();
        assert!(zones.len() >= 6, "{zones:?}");
        assert!(zones.iter().all(|zone| TIME_ZONES.contains(zone)));
        for person in &people {
            let region = &person.region;
            for (setting, value) in [
                (REGION_DATE_FORMAT.definition(), &region.date_format),
                (REGION_NUMBER_FORMAT.definition(), &region.number_format),
                (REGION_FIRST_DAY.definition(), &region.first_day),
            ] {
                assert!(setting.format().accepts(value), "{value}");
                assert_ne!(value, DEVICE, "a choice, not the device");
            }
        }
    }

    #[test]
    fn a_list_that_is_too_short_is_an_error_and_not_an_endless_loop() {
        let short = NameList::parse("first = [\"Ana\"]\nlast = [\"Moreno\", \"Hale\"]").unwrap();
        assert!(sample_people(42, &short).is_err());
    }
}
