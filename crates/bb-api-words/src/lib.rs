//! The words of the API description that more than one program needs, each written once: the marks
//! that `bb-api` puts on a route, the words of a paged list, the three words for a route that needs no
//! permission of the catalogue, and the names of the actors and answers of the permission matrix (SEC-MATRIX).
//! `sec_matrix::RECORD_PATH` is the exception that is no word of the API: the path of the record in
//! the repository.
//!
//! The crate has no dependencies and does no work, so `bb-api` (which writes the words), the server
//! tests (which read them) and `xtask` (whose checks read the generated description) can all
//! import it without pulling in the server (coding-standards §1, one definition).

#![forbid(unsafe_code)]

/// The marks `bb-api` writes into the API description, from the route table (`bb-api`
/// `permissions::ROUTES`). No handler writes one by hand.
pub mod marks {
    /// What a route needs: a key of the permission catalogue, or one of the words in
    /// [`crate::needs`].
    pub const PERMISSION: &str = "x-permission";
    /// A route whose answer holds personal data: never stored by a browser or a cache.
    pub const NEVER_STORED: &str = "x-never-stored";
    /// A route whose answer hands out a secret: never stored, never compressed.
    pub const SECRET: &str = "x-secret";
}

/// The words of a paged list: the query parameters a list route takes, and the properties of its
/// answer (PERF-02). `bb-api` writes them, ARCH-LEAN holds the generated description to them.
pub mod paging {
    /// The query parameter that says which fields an answer holds.
    pub const FIELDS: &str = "fields";
    /// The query parameter that says where a page starts.
    pub const CURSOR: &str = "cursor";
    /// The query parameter that says how long a page is; it has a maximum.
    pub const LIMIT: &str = "limit";
    /// The query parameter that asks for related objects inside the items.
    pub const INCLUDE: &str = "include";
    /// The property of an answer that holds the list; the only name of a list that is paged.
    pub const ITEMS: &str = "items";
    /// The property of a page that holds the position of the next page (`null` on the last page).
    pub const NEXT_CURSOR: &str = "next_cursor";
    /// The property of the answer of a list with a hard cap that says the cap cut it off.
    pub const TRUNCATED: &str = "truncated";
}

/// The words for a route that needs no permission of the catalogue.
pub mod needs {
    /// Anybody, also nobody signed in.
    pub const PUBLIC: &str = "public";
    /// A signed-in person, nothing more.
    pub const SIGNED_IN: &str = "signed-in";
    /// A person's own sign-in data: password, passkeys, second step, sessions.
    pub const OWN_ACCOUNT: &str = "own-account";
    /// All three, for the code that checks a declaration is a key or one of these.
    pub const WITHOUT_CATALOGUE: [&str; 3] = [PUBLIC, SIGNED_IN, OWN_ACCOUNT];
}

/// The names the permission matrix (SEC-MATRIX) writes into its record.
pub mod sec_matrix {
    /// The record of what the matrix covered. A path in the repository (from its root), not a word of
    /// the API.
    pub const RECORD_PATH: &str = "tools/checks/sec-matrix-record.txt";

    /// The environment variable that makes the matrix test write the record instead of comparing
    /// with it, and the value it needs.
    pub const WRITE_VARIABLE: &str = "BB_SEC_MATRIX_RECORD";
    /// The value of [`WRITE_VARIABLE`] that writes.
    pub const WRITE_VALUE: &str = "write";

    /// The start of the name of the test that runs the matrix.
    const MATRIX_TEST: &str = "sec_01_every_route";

    /// The command that writes the record again, after a route or an actor is added.
    #[must_use]
    pub fn write_command() -> String {
        format!(
            "{WRITE_VARIABLE}={WRITE_VALUE} cargo xtask db run -- cargo test -p bb-server {MATRIX_TEST}"
        )
    }

    /// Nobody is signed in.
    pub const NOBODY: &str = "nobody";
    /// A person without a role.
    pub const NO_ROLE: &str = "no-role";
    /// Each starting role is an actor named `role-` and the key of the role
    /// (`bb_policy::StartingRole::code`); [`role_actor`] makes the name.
    const ROLE_PREFIX: &str = "role-";
    /// A role on the parent project.
    pub const LEAD_ON_PARENT: &str = "lead-on-parent";
    /// A role on a sibling project only.
    pub const LEAD_ON_SIBLING_ONLY: &str = "lead-on-sibling-only";
    /// *No access* on the project.
    pub const NO_ACCESS_ON_TARGET: &str = "no-access-on-target";
    /// *No access* on the project, under a lead.
    pub const NO_ACCESS_UNDER_LEAD: &str = "no-access-under-lead";
    /// The person who holds exactly the permission the route declares.
    pub const EXACTLY_THE_PERMISSION: &str = "exactly-the-permission";
    /// The access key that may only see.
    pub const KEY_SEE_ONLY: &str = "key-see-only";
    /// The access key for another project.
    pub const KEY_FOR_ANOTHER_PROJECT: &str = "key-for-another-project";

    /// The actor that holds the starting role with this key.
    #[must_use]
    pub fn role_actor(key: &str) -> String {
        format!("{ROLE_PREFIX}{key}")
    }

    /// The actors every record has besides the ones that hold a starting role.
    pub const FIXED_ACTORS: [&str; 7] = [
        NOBODY,
        NO_ROLE,
        LEAD_ON_PARENT,
        LEAD_ON_SIBLING_ONLY,
        NO_ACCESS_ON_TARGET,
        NO_ACCESS_UNDER_LEAD,
        EXACTLY_THE_PERMISSION,
    ];
    /// The actors with an access key, wanted once the description has a route that makes a key.
    pub const KEY_ACTORS: [&str; 2] = [KEY_SEE_ONLY, KEY_FOR_ANOTHER_PROJECT];

    /// The request is let in.
    pub const ALLOWED: &str = "allowed";
    /// Nobody is signed in.
    pub const NOT_SIGNED_IN: &str = "not-signed-in";
    /// The actor knows the place and lacks the permission.
    pub const FORBIDDEN: &str = "forbidden";
    /// The actor may not know about the project.
    pub const NOT_FOUND: &str = "not-found";
    /// An access key at a route of the owner's own sign-in data.
    pub const NOT_FOR_KEYS: &str = "not-for-keys";
    /// What an answer is called in the record.
    pub const ANSWERS: [&str; 5] = [ALLOWED, NOT_SIGNED_IN, FORBIDDEN, NOT_FOUND, NOT_FOR_KEYS];
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_name_of_the_matrix_is_there_once() {
        let actors: BTreeSet<&str> = sec_matrix::FIXED_ACTORS
            .iter()
            .chain(&sec_matrix::KEY_ACTORS)
            .copied()
            .collect();
        assert_eq!(
            actors.len(),
            sec_matrix::FIXED_ACTORS.len() + sec_matrix::KEY_ACTORS.len()
        );
        let answers: BTreeSet<&str> = sec_matrix::ANSWERS.iter().copied().collect();
        assert_eq!(answers.len(), sec_matrix::ANSWERS.len());
    }

    #[test]
    fn a_role_actor_is_the_prefix_and_the_key() {
        assert_eq!(sec_matrix::role_actor("guest"), "role-guest");
    }
}
