//! Which people a search of the fake store finds: the same rule as the real store's `people_page`
//! (capitals folded the way the database folds them, the characters of ASCII only; the characters
//! of a pattern mean themselves).

use bb_core::names::squeezed;
use bb_core::person::Person;

use crate::auth::SearchIn;

/// Whether `person` is found by `search`: every word is in the name (or the address), or the name
/// starts with the whole text.
pub(super) fn holds(person: &Person, search: Option<&str>, look_at: SearchIn) -> bool {
    let text = search.unwrap_or_default();
    let name = person.display_name.to_ascii_lowercase();
    if look_at == SearchIn::NameStarts {
        return name.starts_with(&squeezed(text).to_ascii_lowercase());
    }
    text.split_whitespace().all(|word| {
        let word = word.to_ascii_lowercase();
        name.contains(&word)
            || (look_at == SearchIn::NamesAndAddresses
                && person.email.to_ascii_lowercase().contains(&word))
    })
}
