//! Which people a search of the fake store finds: the same rule as the real store's `people_page`
//! (the name key: capitals and repeated spaces do not count, accents do; the characters of a
//! pattern mean themselves).

use bb_core::names::name_key;
use bb_core::person::Person;

use crate::auth::SearchIn;

/// Whether `person` is found by `search`: every word is in the name (or the address), or the name
/// starts with the whole text.
pub(super) fn holds(person: &Person, search: Option<&str>, look_at: SearchIn) -> bool {
    let text = search.unwrap_or_default();
    let name = name_key(&person.display_name);
    if look_at == SearchIn::NameStarts {
        return name.starts_with(&name_key(text));
    }
    text.split_whitespace().all(|word| {
        let word = name_key(word);
        name.contains(&word)
            || (look_at == SearchIn::NamesAndAddresses
                && person.email.to_ascii_lowercase().contains(&word))
    })
}
