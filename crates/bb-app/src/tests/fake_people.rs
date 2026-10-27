//! Which people a search of the fake store finds: the same rule as the real store's `people_page`.

use bb_core::names::name_key;
use bb_core::person::Person;

use crate::auth::SearchIn;

/// Whether `person` is found by `search`: every word is in the name (or the address), or the name
/// starts with the whole text.
pub(super) fn holds(person: &Person, search: Option<&str>, look_at: SearchIn) -> bool {
    let name = person.display_name.to_lowercase();
    if look_at == SearchIn::NameStarts {
        return name.starts_with(&name_key(search.unwrap_or_default()));
    }
    let words: String = search
        .unwrap_or_default()
        .chars()
        .filter(|character| !matches!(character, '%' | '_' | '\\'))
        .collect::<String>()
        .to_lowercase();
    words.split_whitespace().all(|word| {
        name.contains(word)
            || (look_at == SearchIn::NamesAndAddresses
                && person.email.to_lowercase().contains(word))
    })
}
