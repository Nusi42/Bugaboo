//! The time zones the server knows: the names of the zone data that comes with it (`jiff` with its
//! bundled data, the feature `zone-data` of this crate, which the server turns on). A zone is one of
//! these names exactly, so a name is stored the way the list writes it; the browser offers the same
//! list (`cargo xtask gen time-zones` writes it from [`names`]).

use jiff::tz::{self, TimeZone};

/// The zone of a person and an organization who chose none, and the one zone every list has.
pub const UTC: &str = "UTC";

/// The areas of the world whose zones a person picks from: a zone named for a city of one of
/// them (`Europe/Berlin`), never a zone of the old names kept for programs (`Etc/GMT+5`, `US/Eastern`,
/// `Zulu`, `EST5EDT`).
const AREAS: [&str; 10] = [
    "Africa",
    "America",
    "Antarctica",
    "Arctic",
    "Asia",
    "Atlantic",
    "Australia",
    "Europe",
    "Indian",
    "Pacific",
];

/// The zones a person is offered (the browser's list): the zones of the areas of the world, and
/// UTC, sorted. The server still accepts every name it knows ([`named`]), so a value stored before
/// stays valid.
#[must_use]
pub fn offered_names() -> Vec<String> {
    known_names()
        .into_iter()
        .filter(|name| {
            name == UTC
                || name
                    .split_once('/')
                    .is_some_and(|(area, _)| AREAS.contains(&area))
        })
        .collect()
}

/// The names of every zone the server knows, sorted.
#[must_use]
pub fn known_names() -> Vec<String> {
    let mut names: Vec<String> = tz::db()
        .available()
        .map(|name| name.as_str().to_owned())
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// The zone of a name of the list; `None` for a name the server does not know, also one that is
/// only written another way (`europe/berlin`).
#[must_use]
pub fn named(name: &str) -> Option<TimeZone> {
    if name == UTC {
        return Some(TimeZone::UTC);
    }
    let zone = TimeZone::get(name).ok()?;
    // The zone data finds a name whatever its capitals; the list writes each name one way.
    (zone.iana_name() == Some(name) || tz::db().available().any(|known| known.as_str() == name))
        .then_some(zone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m2_f72_the_server_knows_the_zones_of_its_data_and_no_others() {
        for known in [
            "UTC",
            "Europe/Berlin",
            "America/New_York",
            "Pacific/Kiritimati",
        ] {
            assert!(named(known).is_some(), "{known}");
        }
        for unknown in [
            "Europe/Atlantis",
            "Mars/Olympus",
            "europe/berlin",
            "",
            "Europe/",
        ] {
            assert!(named(unknown).is_none(), "{unknown}");
        }
    }

    #[test]
    fn m2_f72_a_person_is_offered_the_zones_of_the_world_and_utc_only() {
        let offered = offered_names();
        assert!(offered.iter().any(|name| name == "Europe/Berlin"));
        assert!(offered.iter().any(|name| name == UTC));
        for program_name in [
            "Etc/GMT+5",
            "Factory",
            "posixrules",
            "ROK",
            "Zulu",
            "UCT",
            "EST5EDT",
            "GMT0",
            "US/Eastern",
        ] {
            assert!(
                !offered.iter().any(|name| name == program_name),
                "{program_name}"
            );
            // The server still knows the ones it has, so a value stored before stays valid.
        }
        assert!(named("Etc/GMT+5").is_some());
        assert!(offered.iter().all(|name| named(name).is_some()));
    }

    #[test]
    fn m2_f72_the_list_is_sorted_and_every_name_on_it_is_known() {
        let names = known_names();
        assert!(names.len() > 300, "{}", names.len());
        assert!(names.is_sorted());
        assert!(names.iter().any(|name| name == UTC));
        assert!(names.iter().all(|name| named(name).is_some()));
    }
}
