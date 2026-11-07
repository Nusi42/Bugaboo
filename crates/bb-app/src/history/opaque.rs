//! A name for an ID that tells nothing about when the ID was made (PRIV-09): an ID the server makes
//! holds the moment it was made, so a reader who may not see that moment is given the first bytes of
//! a hash of it, and the rows that are put in order for such a reader are tied by the same hash.

use sha2::{Digest, Sha256};

/// How many bytes of the hash the name keeps.
const OPAQUE_ID_BYTES: usize = 16;

/// The name of an ID: the first bytes of its SHA-256, as hexadecimal text. It is the same every time
/// and follows no order of making.
#[must_use]
pub fn opaque_id(id: &str) -> String {
    use std::fmt::Write as _;
    Sha256::digest(id.as_bytes())
        .iter()
        .take(OPAQUE_ID_BYTES)
        .fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priv_09_an_opaque_id_is_stable_and_has_no_uuid_in_it() {
        let id = "01a3185c-5000-73b2-8f37-20757db0778e";
        assert_eq!(opaque_id(id), opaque_id(id));
        assert_ne!(
            opaque_id(id),
            opaque_id("01a3185c-5000-73b2-8f37-20757db0778f")
        );
        assert_eq!(opaque_id(id).len(), 2 * OPAQUE_ID_BYTES);
        assert!(!opaque_id(id).contains('-'));
    }
}
