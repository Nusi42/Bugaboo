//! The rule that tells when two names are the same name, for the editors of the browser that keep
//! a draft (the statuses of a workflow). It is `bb_core::names`, the rule of the server; nothing is
//! written again here.

use wasm_bindgen::prelude::wasm_bindgen;

/// The key by which names are told apart (`bb_core::names::name_key`): lower case, with the spaces
/// squeezed. Two names with the same key are the same name.
#[must_use]
#[wasm_bindgen]
pub fn name_key(name: &str) -> String {
    bb_core::names::name_key(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_browser_gets_the_key_of_the_server() {
        for name in ["In progress", "in  PROGRESS ", "Ünï  code"] {
            assert_eq!(name_key(name), bb_core::names::name_key(name));
        }
        assert_eq!(name_key("In  progress"), name_key("in progress"));
    }
}
